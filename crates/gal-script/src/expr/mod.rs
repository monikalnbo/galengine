//! 表达式求值：字面量（数字/引号串/裸词=字符串）、f.*/sf.* 变量、
//! ?? 空值合并（右结合）、+ - * /（数字；+ 亦拼接字符串）。左结合折叠。

use crate::vars::{Value, Vars};

#[cfg(test)]
mod tests;

pub fn eval(expr: &str, vars: &Vars) -> Result<Value, String> {
    let e = expr.trim();
    if e.is_empty() {
        return Err("表达式为空".into());
    }
    if let Some(v) = atom(e, vars) {
        return Ok(v); // 单字面量/单变量快速路径（含负数）
    }
    // ?? 空值合并（右结合，取最右分割）
    if let Some((i, _)) = splits(e, &["??"]).pop() {
        let left = eval(&e[..i], vars)?;
        return if is_blank(&left) { eval(&e[i + 2..], vars) } else { Ok(left) };
    }
    for seps in [&["+", "-"][..], &["*", "/"][..]] {
        let ops = splits(e, seps);
        // 过滤一元符号：前段为空（运算符紧跟运算符）说明是 +/- 一元号，并入操作数段
        let kept: Vec<(usize, &'static str)> = ops
            .iter()
            .copied()
            .scan(0usize, |seg_start, (i, op)| {
                let keep = !e[*seg_start..i].trim().is_empty();
                if keep {
                    *seg_start = i + 1;
                }
                Some((keep, (i, op)))
            })
            .filter(|(keep, _)| *keep)
            .map(|(_, op)| op)
            .collect();
        if !kept.is_empty() {
            // 左结合折叠：seg0 (op seg1) (op seg2) …
            let mut v = eval(&e[..kept[0].0], vars)?;
            let mut prev = kept[0];
            for &(i, op) in &kept[1..] {
                let rhs = eval(&e[prev.0 + 1..i], vars)?;
                v = bin(&v, prev.1, &rhs)?;
                prev = (i, op);
            }
            let rhs = eval(&e[prev.0 + 1..], vars)?;
            return bin(&v, prev.1, &rhs);
        }
    }
    Err(format!("无法求值：{e}"))
}

/// if 条件求值（指令已拆为 var / op / val）
pub fn eval_cond(var: &str, op: &str, val: &str, vars: &Vars) -> Result<bool, String> {
    // 左操作数为「未赋值的变量名」（裸名或 f./sf. 前缀的标识符，查无值）→ 按 0
    // （裸名未赋值在 eval 里会被当作字符串字面量，这里显式纠正为数字 0）
    let name = var.trim();
    let is_ident = !name.is_empty()
        && !name.contains(' ')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    let a = match vars.get(name) {
        Some(v) => v.clone(),
        None if is_ident => Value::Int(0),
        None => eval(var, vars)?,
    };
    let b = eval(val, vars)?;
    let bad = || format!("条件类型不可比较：{var} {op} {val}");
    // 未赋值变量（空串）与数字比较：按 0 处理（剧本可省初始化；排练/直入路线不再崩溃）
    let blank = |v: &Value| matches!(v, Value::Str(s) if s.is_empty());
    if (blank(&a) || blank(&b))
        && (!matches!(&a, Value::Str(_)) || blank(&a))
        && (!matches!(&b, Value::Str(_)) || blank(&b))
    {
        let (x, y) =
            (if blank(&a) { 0.0 } else { a.as_f64() }, if blank(&b) { 0.0 } else { b.as_f64() });
        return Ok(match op {
            "==" => x == y,
            "!=" => x != y,
            ">=" => x >= y,
            "<=" => x <= y,
            ">" => x > y,
            "<" => x < y,
            _ => return Err(bad()),
        });
    }
    if matches!(&a, Value::Str(_)) || matches!(&b, Value::Str(_)) {
        return match op {
            "==" => Ok(a == b),
            "!=" => Ok(a != b),
            _ => Err(bad()),
        };
    }
    let (x, y) = (a.as_f64(), b.as_f64());
    Ok(match op {
        "==" => x == y,
        "!=" => x != y,
        ">=" => x >= y,
        "<=" => x <= y,
        ">" => x > y,
        "<" => x < y,
        _ => return Err(bad()),
    })
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

/// 整体解析为单值；引号串/数字/变量命中；含点未命中变量=空值（?? 语义）
fn atom(e: &str, vars: &Vars) -> Option<Value> {
    let b = e.as_bytes();
    if b.len() >= 2
        && ((b[0] == b'\'' && b[b.len() - 1] == b'\'') || (b[0] == b'"' && b[b.len() - 1] == b'"'))
    {
        return Some(Value::Str(e[1..e.len() - 1].into()));
    }
    if let Ok(i) = e.parse::<i64>() {
        return Some(Value::Int(i));
    }
    if let Ok(f) = e.parse::<f64>() {
        return Some(Value::Float(f));
    }
    if !e.contains(' ') {
        if let Some(v) = vars.get(e) {
            return Some(v.clone());
        }
        if e.contains('.') {
            return Some(Value::Str(String::new())); // 未赋值变量=空（?? 视为 blank）
        }
        return Some(Value::Str(e.into())); // 裸词=字符串常量
    }
    None
}

/// ?? 语义：空串/0 视为「未设置」取右值
fn is_blank(v: &Value) -> bool {
    match v {
        Value::Str(s) => s.is_empty(),
        Value::Int(i) => *i == 0,
        Value::Float(f) => *f == 0.0,
    }
}

/// 引号外分隔符位置（按出现顺序）
fn splits(s: &str, seps: &[&'static str]) -> Vec<(usize, &'static str)> {
    let mut out = Vec::new();
    let mut quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            None if c == '\'' || c == '"' => quote = Some(c),
            None => {
                for &sep in seps {
                    if s[i..].starts_with(sep) {
                        out.push((i, sep));
                    }
                }
            }
            _ => {}
        }
    }
    out
}
