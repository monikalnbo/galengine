//! 表达式求值单测（纯逻辑，TDD 基线）

use crate::script::expr::{eval, eval_cond};
use crate::script::vars::{Value, Vars};

fn vars() -> Vars {
    let mut v = Vars::new();
    v.set("f.aff", Value::Int(3));
    v.set("sf.name", Value::Str("拓海".into()));
    Vars { f: v.f, sf: v.sf }
}

#[test]
fn literals() {
    let v = Vars::new();
    assert_eq!(eval("42", &v).unwrap(), Value::Int(42));
    assert_eq!(eval("-5", &v).unwrap(), Value::Int(-5));
    assert_eq!(eval("3.5", &v).unwrap(), Value::Float(3.5));
    assert_eq!(eval("'拓海'", &v).unwrap(), Value::Str("拓海".into()));
    assert_eq!(eval("拓海", &v).unwrap(), Value::Str("拓海".into())); // 裸词=字符串
}

#[test]
fn variables_and_ops() {
    let v = vars();
    assert_eq!(eval("f.aff", &v).unwrap(), Value::Int(3));
    assert_eq!(eval("f.aff + 2", &v).unwrap(), Value::Int(5));
    assert_eq!(eval("f.aff * 4 - 10", &v).unwrap(), Value::Int(2));
    assert_eq!(eval("f.aff / 2", &v).unwrap(), Value::Float(1.5));
    assert!(eval("f.aff / 0", &v).is_err(), "除零必须报错");
}

#[test]
fn coalesce() {
    let v = vars();
    assert_eq!(eval("sf.missing ?? 7", &v).unwrap(), Value::Int(7));
    assert_eq!(eval("f.aff ?? 9", &v).unwrap(), Value::Int(3)); // 非空取左
    assert_eq!(
        eval("sf.missing ?? 拓海", &v).unwrap(),
        Value::Str("拓海".into())
    );
}

#[test]
fn string_concat_and_quotes() {
    let v = vars();
    assert_eq!(
        eval("'a' + 'b'", &v).unwrap(),
        Value::Str("ab".into())
    );
    // 引号内的 + 不分割
    assert_eq!(
        eval("'a+b'", &v).unwrap(),
        Value::Str("a+b".into())
    );
}

#[test]
fn conditions() {
    let v = vars();
    assert!(eval_cond("f.aff", "==", "3", &v).unwrap());
    assert!(eval_cond("f.aff", ">=", "3", &v).unwrap());
    assert!(!eval_cond("f.aff", "<", "3", &v).unwrap());
    assert!(eval_cond("sf.name", "==", "拓海", &v).unwrap());
    assert!(eval_cond("sf.name", "!=", "澪", &v).unwrap());
    // 字符串不支持大小比较
    assert!(eval_cond("sf.name", ">", "a", &v).is_err());
}

#[test]
fn errors_are_chinese() {
    let v = Vars::new();
    let e = eval("", &v).unwrap_err();
    assert!(e.contains('空'), "错误必须中文：{e}");
}
