//! 表达式求值：galgame 所需子集。
//! 支持：字面量（数字/'单引号'/"双引号"/裸词=字符串）、变量（f.x / sf.x）、
//! 运算 ?? （空值合并，右结合）、+ - * /（数字；+ 亦拼接字符串）。
//! 优先级：?? 最低 → + - → * /。引号内不分割。

use crate::script::vars::{Value, Vars};

pub fn eval(expr: &str, vars: &Vars) -> Result<Value, String> {
    let e = expr.trim();
    if e.is_empty() {
        return Err("表达式为空".into());
    }
    // 整体快速路径：单字面量/单变量（避免 "-5" 被当作减法拆开）
    if let Some(v) = atom(e, vars) {
        return Ok(v);
    }
    // ?? 空值合并（右结合，取最右分割）
    if let Some((i, _)) = splits(e, &["??"]).pop() {
        let left = eval(&e[..i], vars)?;
        return if is_blank(&left) {
            eval(&e[i + 2..], vars)
        } else {
            Ok(left)
        };
    }
    if let Some((i, _)) = splits(e, &["+", "-"]).pop() {
        let (a, b) = (eval(&e[..i], vars)?, eval(&e[i + 1..], vars)?);
        return bin(&a, &e[i..i + 1], &b);
    }
    if let Some((i, _)) = splits(e, &["*", "/"]).pop() {
        let (a, b) = (eval(&e[..i], vars)?, eval(&e[i + 1..], vars)?);
        return bin(&a, &e[i..i + 1], &b);
    }
    Err(format!("无法求值：{e}"))
}

/// if 条件求值（指令已拆为 var / op / val）
pub fn eval_cond(var: &str, op: &str, val: &str, vars: &Vars) -> Result<bool, String> {
    let a = eval(var, vars)?;
    let b = eval(val, vars)?;
    let bad = || format!("条件类型不可比较：{var} {op} {val}");
    Ok(match (&a, &b) {
        (Value::Str(x), Value::Str(y)) => match op {
            "==" => x == y,
            "!=" => x != y,
            _ => return Err(bad()),
        },
        _ => {
            let (x, y) = (num_f(&a), num_f(&b));
            match op {
                "==" => x == y,
                "!=" => x != y,
                ">=" => x >= y,
                "<=" => x <= y,
                ">" => x > y,
                "<" => x < y,
                _ => return Err(bad()),
            }
        }
    })
}

fn num_f(v: &Value) -> f64 {
    match v {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        Value::Str(s) => s.parse().unwrap_or(0.0),
    }
}

fn bin(a: &Value, op: &str, b: &Value) -> Result<Value, String> {
    match (a, b) {
        // 数字运算（Int 保持 Int，除非出现 Float）
        _ if matches!((a, b), (Value::Int(_), Value::Int(_)) | (Value::Float(_), _) | (_, Value::Float(_))) => {
            let (x, y) = (num_f(a), num_f(b));
            match op {
                "+" => Ok(mk_num(x + y, a, b)),
                "-" => Ok(mk_num(x - y, a, b)),
                "*" => Ok(mk_num(x * y, a, b)),
                "/" => {
                    if y == 0.0 {
                        Err("除数为零".into())
                    } else {
                        Ok(mk_num(x / y, a, b))
                    }
                }
                _ => Err(format!("未知运算符 {op}")),
            }
        }
        // 字符串：+ 拼接
        (Value::Str(x), Value::Str(y)) if op == "+" => Ok(Value::Str(format!("{x}{y}"))),
        (Value::Str(x), _) if op == "+" => Ok(Value::Str(format!("{x}{}", b.as_str()))),
        (_, Value::Str(y)) if op == "+" => Ok(Value::Str(format!("{}{y}", a.as_str()))),
        _ => Err(format!("类型不可运算：{op}")),
    }
}

fn mk_num(v: f64, a: &Value, b: &Value) -> Value {
    if matches!(a, Value::Float(_)) || matches!(b, Value::Float(_)) {
        Value::Float(v)
    } else if v.fract() == 0.0 {
        Value::Int(v as i64)
    } else {
        Value::Float(v)
    }
}

/// 尝试整体解析为单个值；失败返回 None（继续按运算分割）
fn atom(e: &str, vars: &Vars) -> Option<Value> {
    if let Some(inner) = strip_quotes(e) {
        return Some(Value::Str(inner));
    }
    if let Ok(i) = e.parse::<i64>() {
        return Some(Value::Int(i));
    }
    if let Ok(f) = e.parse::<f64>() {
        return Some(Value::Float(f));
    }
    if e.contains('.') && !e.contains(' ') {
        return vars.get(e).cloned();
    }
    // 裸词 = 字符串（中文常量，如 拓海）
    if !e.contains(' ') {
        return Some(Value::Str(e.to_string()));
    }
    None
}

fn strip_quotes(e: &str) -> Option<String> {
    let b = e.as_bytes();
    if b.len() >= 2 && ((b[0] == b'\'' && b[b.len() - 1] == b'\'') || (b[0] == b'"' && b[b.len() - 1] == b'"')) {
        Some(e[1..e.len() - 1].to_string())
    } else {
        None
    }
}

/// ?? 空值语义：空串/数值 0 视为「未设置」（取右值）
fn is_blank(v: &Value) -> bool {
    match v {
        Value::Str(s) => s.is_empty(),
        Value::Int(i) => *i == 0,
        Value::Float(f) => *f == 0.0,
    }
}

/// 找出不在引号内的分隔符位置
fn splits(s: &str, seps: &[&'static str]) -> Vec<(usize, &'static str)> {
    let mut out = Vec::new();
    let mut quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
            }
            None => {
                if c == '\'' || c == '"' {
                    quote = Some(c);
                } else {
                    for &sep in seps {
                        if s[i..].starts_with(sep) {
                            out.push((i, sep));
                            break;
                        }
                    }
                }
            }
        }
    }
    out
}
