//! 表达式求值（P0-3）：字面量（数字/引号串/裸词=字符串）、f.*/sf.* 变量、
//! 括号、一元（`-` `!`）、算术（`+ - * / %`）、比较（`== != < <= > >=`）、
//! 逻辑（`&& ||`，短路）与空值合并 `??`（右结合）。
//!
//! 优先级从低到高：`||` → `??` → `&&` → 比较 → `+ -` → `* / %` → 一元 → 括号/原子。
//! 逻辑与比较的结果是 `Int(0/1)`，可直接被 `set` 存进变量。

use crate::vars::{Value, Vars};

#[cfg(test)]
mod tests;

/// 求值表达式（返回整数/浮点/字符串）
pub fn eval(expr: &str, vars: &Vars) -> Result<Value, String> {
    let e = expr.trim();
    if e.is_empty() {
        return Err("表达式为空".into());
    }
    let toks = lex(e)?;
    let mut p = Parser { toks: &toks, i: 0 };
    let node = p.parse_or()?;
    if p.i != toks.len() {
        return Err(format!("表达式多余内容：{e}"));
    }
    node.eval(vars)
}

/// v0.3.4 求值并写回赋值目标：`f.a = 表达式` / `f.a += 1` / `sf.arr[0] = x` /
/// `f.d.k = v`（KAG3 eval 主体形态）。返回求值结果。
///
/// 解析策略：按最顶层 `=` / `+= -= *= /= %=` 切分（不在括号/下标内的第一个），
/// 左侧是赋值目标（变量名 / 下标 / 成员路径），右侧是普通表达式。
pub fn eval_assign(expr: &str, vars: &mut Vars) -> Result<Value, String> {
    let e = expr.trim();
    if e.is_empty() {
        return Err("表达式为空".into());
    }
    // 找顶层赋值运算符（深度 0：括号/方括号外）
    let bytes = e.as_bytes();
    let (mut depth, mut hit) = (0i32, None);
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'=' if depth == 0 => {
                // 跳过 == 以及 += -= *= /= %= 的 =（记录复合运算符）
                let prev = if i > 0 { bytes[i - 1] } else { 0 };
                let next = bytes.get(i + 1).copied().unwrap_or(0);
                if next == b'=' {
                    i += 2; // ==
                    continue;
                }
                hit = Some((i, prev));
                break; // 最左顶层 =（左结合唯一目标）
            }
            _ => {}
        }
        i += 1;
    }
    let Some((at, prev)) = hit else {
        return eval(e, vars); // 无赋值：纯表达式求值
    };
    let (compound, target) = match prev {
        b'+' | b'-' | b'*' | b'/' | b'%' => {
            let op = (prev as char).to_string();
            (Some(op), e[..at - 1].trim())
        }
        _ => (None, e[..at].trim()),
    };
    if target.is_empty() {
        return Err(format!("赋值缺少左侧变量：{e}"));
    }
    let rhs_src = e[at + 1..].trim();
    if rhs_src.is_empty() {
        return Err(format!("赋值缺少右侧表达式：{e}"));
    }
    let rhs = eval(rhs_src, vars)?;
    let value = match compound {
        Some(op) => {
            let cur = eval(target, vars).unwrap_or(Value::Int(0));
            bin(&cur, &op, &rhs)?
        }
        None => rhs,
    };
    assign_to(target, value.clone(), vars)?;
    Ok(value)
}

/// 把值写进赋值目标：`f.x` / `sf.x`（整变量）或带下标/成员的容器内定位。
fn assign_to(target: &str, v: Value, vars: &mut Vars) -> Result<(), String> {
    // 目标最后一段的宿主必须是变量：f.a[i]、f.d.k、sf.arr[0].sub
    let (host_src, last) = split_last_accessor(target);
    // last 两种：Index(expr) / Member(key)
    let get_host = |vars: &Vars| -> Result<Value, String> {
        if host_src.is_empty() {
            Err(format!("赋值目标缺少变量名：{target}"))
        } else {
            eval(&host_src, vars)
        }
    };
    match last {
        None => {
            // 纯变量名（f.x / sf.x / x）
            if !target.contains('[') && !target.contains('.') {
                vars.set(target, v);
                return Ok(());
            }
            // eval 需要只读 vars，这里没有未赋值裸词场景：直接覆盖变量名
            vars.set(target, v);
            Ok(())
        }
        Some(Access::Index(idx_expr)) => {
            let mut host = get_host(vars)?;
            let idx = eval(&idx_expr, vars)?;
            let n = idx.as_i64();
            match &mut host {
                Value::Arr(a) => {
                    let i = n.unsigned_abs() as usize;
                    if i >= a.len() {
                        return Err(format!("数组下标越界：{target}[{n}]（长度 {}）", a.len()));
                    }
                    a[i] = v;
                }
                Value::Dict(d) => {
                    let Value::Str(k) = idx.clone() else {
                        return Err(format!("字典下标必须是字符串：{target}"));
                    };
                    upsert(d, k, v);
                }
                _ => return Err(format!("赋值目标不是容器：{target}")),
            }
            write_back(&host_src, host, vars, target)
        }
        Some(Access::Member(key)) => {
            let mut host = get_host(vars)?;
            match &mut host {
                Value::Dict(d) => upsert(d, key.to_string(), v),
                _ => return Err(format!("赋值目标不是字典：{target}")),
            }
            write_back(&host_src, host, vars, target)
        }
    }
}

enum Access {
    Index(String),
    Member(String),
}

/// 拆出最后一个访问器：`f.arr[0]` → ("f.arr", Index("0"))；`f.d.k` → ("f.d", Member("k"))；
/// `f.x` → ("f.x", None)
fn split_last_accessor(target: &str) -> (String, Option<Access>) {
    let t = target.trim();
    if t.ends_with(']') {
        if let Some(open) = t.rfind('[') {
            return (
                t[..open].trim().to_string(),
                Some(Access::Index(t[open + 1..t.len() - 1].trim().to_string())),
            );
        }
    }
    // 成员点段：f./sf. 前缀的两段是变量名，只拆第三段起的最后一段
    let segs: Vec<&str> = t.split('.').collect();
    if segs.len() >= 3 && (segs[0] == "f" || segs[0] == "sf") {
        let var = format!("{}.{}", segs[0], segs[1]);
        let host = segs[2..segs.len() - 1].iter().fold(var, |acc, s| format!("{acc}.{s}"));
        return (host, Some(Access::Member(segs[segs.len() - 1].to_string())));
    }
    (t.to_string(), None)
}

/// 宿主写回：host_src 是变量名时直接 set（容器已就地修改的副本重新写入）
fn write_back(host_src: &str, host: Value, vars: &mut Vars, target: &str) -> Result<(), String> {
    // host_src 必须是纯变量名（f.x / sf.x）；嵌套宿主（f.d.k[0]）暂不支持再嵌套写回
    let name = host_src.trim();
    let ok = !name.contains('[')
        && name.split('.').count() <= 2
        && (name.split('.').count() == 1 || name.starts_with("f.") || name.starts_with("sf."));
    if !ok {
        return Err(format!("赋值目标嵌套过深（暂不支持）：{target}"));
    }
    vars.set(name, host);
    Ok(())
}

fn upsert(d: &mut Vec<(String, Value)>, k: String, v: Value) {
    if let Some(slot) = d.iter_mut().find(|(dk, _)| *dk == k) {
        slot.1 = v;
    } else {
        d.push((k, v));
    }
}

/// if 条件求值（指令已拆为 var / op / val）。
///
/// 两侧都按表达式求值：`if f.a + 1 > f.b * 2 *next` 可用。
/// 唯一的历史特例：左操作数是「裸标识符且未赋值」时按 0（剧本可省初始化），
/// 因为 `eval` 会把未赋值的裸词当字符串字面量。
pub fn eval_cond(var: &str, op: &str, val: &str, vars: &Vars) -> Result<bool, String> {
    let name = var.trim();
    let is_ident = !name.is_empty()
        && !name.contains(' ')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    let a = match vars.get(name) {
        Some(v) => v.clone(),
        None if is_ident && name.parse::<f64>().is_err() => Value::Int(0),
        None => eval(var, vars)?,
    };
    let b = eval(val, vars)?;
    value_cmp(&a, op, &b).ok_or_else(|| format!("条件类型不可比较：{var} {op} {val}"))
}

/// 两值比较（表达式内建比较与 if 共用同一套类型规则）。
/// 返回 None 表示该运算符不支持这两个操作数（如字符串的大小比较）。
pub fn value_cmp(a: &Value, op: &str, b: &Value) -> Option<bool> {
    // 未赋值变量（空串）与数字比较：按 0 处理（剧本可省初始化）
    let blank = |v: &Value| matches!(v, Value::Str(s) if s.is_empty());
    let numeric = |v: &Value| !matches!(v, Value::Str(_)) || blank(v);
    if (blank(a) || blank(b)) && numeric(a) && numeric(b) {
        let (x, y) =
            (if blank(a) { 0.0 } else { a.as_f64() }, if blank(b) { 0.0 } else { b.as_f64() });
        return Some(match op {
            "==" => x == y,
            "!=" => x != y,
            ">=" => x >= y,
            "<=" => x <= y,
            ">" => x > y,
            "<" => x < y,
            _ => return None,
        });
    }
    if matches!(a, Value::Str(_)) || matches!(b, Value::Str(_)) {
        return match op {
            "==" => Some(a == b),
            "!=" => Some(a != b),
            _ => None,
        };
    }
    let (x, y) = (a.as_f64(), b.as_f64());
    Some(match op {
        "==" => x == y,
        "!=" => x != y,
        ">=" => x >= y,
        "<=" => x <= y,
        ">" => x > y,
        "<" => x < y,
        _ => return None,
    })
}

/// 真值：非空字符串、非 0 数字（if 块与逻辑运算共用）
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Str(s) => !s.is_empty(),
        Value::Int(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        // 容器：空 = 假
        Value::Arr(a) => !a.is_empty(),
        Value::Dict(d) => !d.is_empty(),
    }
}

// ── 词法 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Tok {
    Val(Value),
    /// 标识符/裸词：先查变量，查不到按字符串常量
    Ident(String),
    Op(&'static str),
}

fn lex(s: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = s[i..].chars().next().unwrap();
        if c.is_whitespace() {
            i += c.len_utf8();
            continue;
        }
        if c == '\'' || c == '"' {
            let start = i + c.len_utf8();
            let end = s[start..].find(c).ok_or_else(|| format!("引号未闭合：{s}"))?;
            out.push(Tok::Val(Value::Str(s[start..start + end].into())));
            i = start + end + c.len_utf8();
            continue;
        }
        if c.is_ascii_digit()
            || (c == '.' && s[i + 1..].chars().next().is_some_and(|d| d.is_ascii_digit()))
        {
            let start = i;
            while i < bytes.len() {
                let ch = s[i..].chars().next().unwrap();
                if ch.is_ascii_digit() || ch == '.' {
                    i += 1;
                } else {
                    break;
                }
            }
            let t = &s[start..i];
            let v = if let Ok(n) = t.parse::<i64>() {
                Value::Int(n)
            } else if let Ok(f) = t.parse::<f64>() {
                Value::Float(f)
            } else {
                return Err(format!("数字格式无效：{t}"));
            };
            out.push(Tok::Val(v));
            continue;
        }
        if c.is_alphanumeric() || c == '_' {
            let start = i;
            while i < bytes.len() {
                let ch = s[i..].chars().next().unwrap();
                if ch.is_alphanumeric() || ch == '_' {
                    i += ch.len_utf8();
                } else if ch == '.'
                    && s[i + 1..].chars().next().is_some_and(|n| n.is_alphanumeric() || n == '_')
                {
                    i += 1; // f.a / sf.name：点号并入标识符
                } else {
                    break;
                }
            }
            out.push(Tok::Ident(s[start..i].to_string()));
            continue;
        }
        let two = s.get(i..i + 2).unwrap_or("");
        let two_op = match two {
            "??" => Some("??"),
            "&&" => Some("&&"),
            "||" => Some("||"),
            "==" => Some("=="),
            "!=" => Some("!="),
            ">=" => Some(">="),
            "<=" => Some("<="),
            _ => None,
        };
        if let Some(op) = two_op {
            out.push(Tok::Op(op));
            i += 2;
            continue;
        }
        let one = match c {
            '[' => Some("["),
            ']' => Some("]"),
            ',' => Some(","),
            ':' => Some(":"),
            '?' => Some("?"),
            '=' => Some("="),
            '+' => Some("+"),
            '-' => Some("-"),
            '*' => Some("*"),
            '/' => Some("/"),
            '%' => Some("%"),
            '!' => Some("!"),
            '>' => Some(">"),
            '<' => Some("<"),
            '(' => Some("("),
            ')' => Some(")"),
            _ => None,
        };
        match one {
            Some(op) => {
                out.push(Tok::Op(op));
                i += 1;
            }
            None => return Err(format!("无法识别的字符「{c}」：{s}")),
        }
    }
    Ok(out)
}

// ── 语法树 + 求值（短路在求值阶段，解析一次完成）─────────────────────────

#[derive(Debug, Clone)]
enum Node {
    Val(Value),
    Ident(String),
    Un(&'static str, Box<Node>),
    Bin(&'static str, Box<Node>, Box<Node>),
    /// v0.3.4 三元：cond ? a : b
    Ter(Box<Node>, Box<Node>, Box<Node>),
    /// v0.3.4 数组/字典下标：base[idx]（字典可用字符串下标）
    Index(Box<Node>, Box<Node>),
    /// v0.3.4 成员访问：base.key / base["k"]（字典；数组用 .length）
    Member(Box<Node>, String),
    /// v0.3.4 数组/字典字面量（TJS `[a, b]` / `%[ k: v ]`）；
    /// 布尔 = 是否字典（空字面量 `[]` vs `%[]` 也靠它区分）
    Lit(Vec<Node>, bool),
    /// v0.3.4 字典键值对（Lit 内部：key 必须 literal）
    Pair(Box<Node>, Box<Node>),
}

impl Node {
    fn eval(&self, vars: &Vars) -> Result<Value, String> {
        match self {
            Node::Val(v) => Ok(v.clone()),
            Node::Ident(name) => Ok(lookup(name, vars)),
            Node::Un(op, e) => {
                let v = e.eval(vars)?;
                Ok(match *op {
                    "-" => match &v {
                        Value::Str(s) if s.is_empty() => Value::Int(0),
                        Value::Str(s) => return Err(format!("一元负号不能用于字符串：{s}")),
                        _ if matches!(v, Value::Float(_)) => Value::Float(-v.as_f64()),
                        _ => Value::Int(-v.as_i64()),
                    },
                    "!" => Value::Int(!truthy(&v) as i64),
                    _ => return Err(format!("未知一元运算符 {op}")),
                })
            }
            Node::Bin(op, a, b) => match *op {
                // 短路：右侧不求值
                "&&" => {
                    let l = a.eval(vars)?;
                    if !truthy(&l) {
                        return Ok(Value::Int(0));
                    }
                    Ok(Value::Int(truthy(&b.eval(vars)?) as i64))
                }
                "||" => {
                    let l = a.eval(vars)?;
                    if truthy(&l) {
                        return Ok(Value::Int(1));
                    }
                    Ok(Value::Int(truthy(&b.eval(vars)?) as i64))
                }
                "??" => {
                    let l = a.eval(vars)?;
                    if is_blank(&l) {
                        b.eval(vars)
                    } else {
                        Ok(l)
                    }
                }
                _ => {
                    let (x, y) = (a.eval(vars)?, b.eval(vars)?);
                    if matches!(*op, "==" | "!=" | ">" | ">=" | "<" | "<=") {
                        let r =
                            value_cmp(&x, op, &y).ok_or_else(|| format!("类型不可比较：{op}"))?;
                        return Ok(Value::Int(r as i64));
                    }
                    bin(&x, op, &y)
                }
            },
            Node::Ter(c, a, b) => {
                if truthy(&c.eval(vars)?) {
                    a.eval(vars)
                } else {
                    b.eval(vars)
                }
            }
            Node::Index(base, idx) => {
                let b = base.eval(vars)?;
                let i = idx.eval(vars)?;
                match (b, i) {
                    (Value::Arr(a), i) => {
                        let n = i.as_i64();
                        a.get(n.unsigned_abs() as usize)
                            .cloned()
                            .ok_or_else(|| format!("数组下标越界：[{n}]（长度 {}）", a.len()))
                    }
                    (Value::Dict(d), Value::Str(k)) => d
                        .iter()
                        .find(|(dk, _)| *dk == k)
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| format!("字典无键：「{k}」")),
                    (Value::Dict(_), i) => Err(format!("字典下标必须是字符串：{i:?}")),
                    (b, _) => Err(format!("该类型不支持下标访问：{b:?}")),
                }
            }
            Node::Member(base, key) => {
                let b = base.eval(vars)?;
                match b {
                    Value::Dict(d) => d
                        .iter()
                        .find(|(dk, _)| dk == key)
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| format!("字典无成员：「{key}」")),
                    Value::Arr(_) if key == "length" => {
                        Ok(Value::Int(if let Value::Arr(a) = b { a.len() as i64 } else { 0 }))
                    }
                    _ => Err(format!("该类型不支持成员访问 .{key}")),
                }
            }
            Node::Lit(items, dict) => {
                if *dict {
                    let mut d = Vec::new();
                    for it in items {
                        if let Node::Pair(k, v) = it {
                            let kv = k.eval(vars)?;
                            let Value::Str(key) = kv else {
                                return Err(format!("字典键必须是字符串：{kv:?}"));
                            };
                            d.push((key, v.eval(vars)?));
                        }
                    }
                    Ok(Value::Dict(d))
                } else {
                    let mut a = Vec::new();
                    for it in items {
                        a.push(it.eval(vars)?);
                    }
                    Ok(Value::Arr(a))
                }
            }
            Node::Pair(k, v) => {
                let _ = (k, v);
                Err("键值对只能出现在字典字面量里".into())
            }
        }
    }
}

/// 变量查找：命中返回值；带点的未赋值变量=空串（`??` 语义）；裸词=字符串常量
fn lookup(name: &str, vars: &Vars) -> Value {
    if let Some(v) = vars.get(name) {
        return v.clone();
    }
    match name {
        "true" => Value::Int(1),
        "false" => Value::Int(0),
        _ if name.contains('.') => Value::Str(String::new()),
        _ => Value::Str(name.into()),
    }
}

fn bin(a: &Value, op: &str, b: &Value) -> Result<Value, String> {
    let both_num = !matches!(a, Value::Str(_)) && !matches!(b, Value::Str(_));
    if both_num {
        let (x, y) = (a.as_f64(), b.as_f64());
        let v = match op {
            "+" => x + y,
            "-" => x - y,
            "*" => x * y,
            "/" if y == 0.0 => return Err("除数为零".into()),
            "/" => x / y,
            "%" if y == 0.0 => return Err("取模除数为零".into()),
            "%" => x % y,
            _ => return Err(format!("未知运算符 {op}")),
        };
        return Ok(
            if matches!((a, b), (Value::Float(_), _) | (_, Value::Float(_))) || v.fract() != 0.0 {
                Value::Float(v)
            } else {
                Value::Int(v as i64)
            },
        );
    }
    match op {
        "+" => Ok(Value::Str(format!("{}{}", a.as_str(), b.as_str()))),
        _ => Err(format!("类型不可运算：{op}")),
    }
}

/// ?? 语义：空串/0 视为「未设置」取右值
fn is_blank(v: &Value) -> bool {
    match v {
        Value::Str(s) => s.is_empty(),
        Value::Int(i) => *i == 0,
        Value::Float(f) => *f == 0.0,
        Value::Arr(a) => a.is_empty(),
        Value::Dict(d) => d.is_empty(),
    }
}

// ── 递归下降 ────────────────────────────────────────────────────────────

struct Parser<'a> {
    toks: &'a [Tok],
    i: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i)
    }

    fn eat(&mut self, op: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Op(o)) if *o == op) {
            self.i += 1;
            return true;
        }
        false
    }

    /// `||`（最低）；其上挂三元 `?:`（右结合）
    fn parse_or(&mut self) -> Result<Node, String> {
        let n = self.parse_nullco()?;
        if self.eat("?") {
            let a = self.parse_or()?;
            if !self.eat(":") {
                return Err("三元运算符缺少冒号：cond ? a : b".into());
            }
            return Ok(Node::Ter(Box::new(n), Box::new(a), Box::new(self.parse_or()?)));
        }
        let mut n = n;
        while self.eat("||") {
            n = Node::Bin("||", Box::new(n), Box::new(self.parse_nullco()?));
        }
        Ok(n)
    }

    /// `??`（右结合）
    fn parse_nullco(&mut self) -> Result<Node, String> {
        let n = self.parse_and()?;
        if self.eat("??") {
            return Ok(Node::Bin("??", Box::new(n), Box::new(self.parse_nullco()?)));
        }
        Ok(n)
    }

    fn parse_and(&mut self) -> Result<Node, String> {
        let mut n = self.parse_cmp()?;
        while self.eat("&&") {
            n = Node::Bin("&&", Box::new(n), Box::new(self.parse_cmp()?));
        }
        Ok(n)
    }

    fn parse_cmp(&mut self) -> Result<Node, String> {
        let mut n = self.parse_add()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op(o)) if matches!(*o, "==" | "!=" | ">" | ">=" | "<" | "<=") => *o,
                _ => return Ok(n),
            };
            self.i += 1;
            n = Node::Bin(op, Box::new(n), Box::new(self.parse_add()?));
        }
    }

    fn parse_add(&mut self) -> Result<Node, String> {
        let mut n = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op(o)) if matches!(*o, "+" | "-") => *o,
                _ => return Ok(n),
            };
            self.i += 1;
            n = Node::Bin(op, Box::new(n), Box::new(self.parse_mul()?));
        }
    }

    fn parse_mul(&mut self) -> Result<Node, String> {
        let mut n = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op(o)) if matches!(*o, "*" | "/" | "%") => *o,
                _ => return Ok(n),
            };
            self.i += 1;
            n = Node::Bin(op, Box::new(n), Box::new(self.parse_unary()?));
        }
    }

    fn parse_unary(&mut self) -> Result<Node, String> {
        for op in ["-", "!"] {
            if self.eat(op) {
                let o = if op == "-" { "-" } else { "!" };
                return Ok(Node::Un(o, Box::new(self.parse_unary()?)));
            }
        }
        // v0.3.4 字典字面量 %[ k: v, ... ]（% 与 [ 两个 token）
        if matches!(self.peek(), Some(Tok::Op("%")))
            && matches!(self.toks.get(self.i + 1), Some(Tok::Op("[")))
        {
            self.i += 2;
            let lit = self.parse_composite(true)?;
            return self.parse_postfix(lit);
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Node, String> {
        let Some(tok) = self.peek().cloned() else {
            return Err("表达式意外结束".into());
        };
        self.i += 1;
        let base = match tok {
            Tok::Val(v) => Node::Val(v),
            Tok::Ident(name) => return self.parse_ident_path(name),
            Tok::Op("(") => {
                let n = self.parse_or()?;
                if !self.eat(")") {
                    return Err("括号未闭合".into());
                }
                n
            }
            // v0.3.4 数组字面量 [a, b, c]（可接下标后缀）
            Tok::Op("[") => self.parse_composite(false)?,
            Tok::Op(op) => return Err(format!("此处不能出现运算符「{op}」")),
        };
        self.parse_postfix(base)
    }

    /// v0.3.4 数组/字典字面量：`[a, b, c]` / `%[ k: v, ... ]`（入口见 parse_unary
    /// 前对 `%[` 的特判，字典元素必须 `key : value`）
    fn parse_composite(&mut self, dict: bool) -> Result<Node, String> {
        let mut items = Vec::new();
        if self.eat("]") {
            return Ok(Node::Lit(items, dict));
        }
        loop {
            let k = self.parse_nullco()?;
            if self.eat(":") {
                if !dict {
                    return Err("数组字面量里不能出现冒号（字典字面量请用 %[ ... ]）".into());
                }
                // TJS 惯例：键是裸标识符时按字符串字面量（裸词本可解析成变量）
                let k = match k {
                    Node::Ident(n) => Node::Val(Value::Str(n)),
                    k => k,
                };
                let v = self.parse_nullco()?;
                items.push(Node::Pair(Box::new(k), Box::new(v)));
            } else if dict {
                return Err("字典字面量元素必须是 key: value".into());
            } else {
                items.push(k);
            }
            if self.eat(",") {
                continue;
            }
            if self.eat("]") {
                break;
            }
            return Err("数组/字典字面量应以 , 或 ] 分隔".into());
        }
        Ok(Node::Lit(items, dict))
    }

    /// v0.3.4 后缀链：base[i]（下标；成员 base.key 在 lexer 已并入 Ident，
    /// f.a.b 形态在 parse_ident_path 拆）
    fn parse_postfix(&mut self, base: Node) -> Result<Node, String> {
        let mut base = base;
        loop {
            if self.eat("[") {
                let idx = self.parse_or()?;
                if !self.eat("]") {
                    return Err("下标括号未闭合".into());
                }
                base = Node::Index(Box::new(base), Box::new(idx));
                continue;
            }
            break;
        }
        Ok(base)
    }

    /// v0.3.4 标识符路径：`f.party.hp` / `sf.cfg.x`——f./sf. 前缀与首段合起来
    /// 是变量名（Vars::get 原生认它），其余点段逐级转 Member
    fn parse_ident_path(&mut self, name: String) -> Result<Node, String> {
        if name == "true" {
            return self.parse_postfix(Node::Val(Value::Int(1)));
        }
        if name == "false" {
            return self.parse_postfix(Node::Val(Value::Int(0)));
        }
        let segs: Vec<&str> = name.split('.').collect();
        // f.a / sf.x：前两段是变量名；其余点段是成员
        let (var, rest) = match segs.split_first() {
            Some((first, rest)) if *first == "f" || *first == "sf" => match rest.split_first() {
                Some((second, tail)) => (format!("{first}.{second}"), tail),
                None => (name.clone(), [].as_slice()),
            },
            _ => (segs[0].to_string(), &segs[1..]),
        };
        let mut node = Node::Ident(var);
        for k in rest {
            node = Node::Member(Box::new(node), (*k).to_string());
        }
        self.parse_postfix(node)
    }
}
