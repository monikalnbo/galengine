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

    /// `||`（最低）
    fn parse_or(&mut self) -> Result<Node, String> {
        let mut n = self.parse_nullco()?;
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
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Node, String> {
        let Some(tok) = self.peek().cloned() else {
            return Err("表达式意外结束".into());
        };
        self.i += 1;
        match tok {
            Tok::Val(v) => Ok(Node::Val(v)),
            Tok::Ident(name) => Ok(Node::Ident(name)),
            Tok::Op("(") => {
                let n = self.parse_or()?;
                if !self.eat(")") {
                    return Err("括号未闭合".into());
                }
                Ok(n)
            }
            Tok::Op(op) => Err(format!("此处不能出现运算符「{op}」")),
        }
    }
}
