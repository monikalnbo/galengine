//! expr 单元测试

use super::*;

fn v() -> Vars {
    let mut v = Vars::default();
    v.set("f.a", Value::Int(2));
    v.set("sf.name", Value::Str("夏".into()));
    v.set("f.blank", Value::Str(String::new()));
    v
}

#[test]
fn 四则与左结合() {
    let vars = v();
    assert_eq!(eval("1 + 2 * 3", &vars).unwrap(), Value::Int(7));
    assert_eq!(eval("f.a * f.a + 1", &vars).unwrap(), Value::Int(5));
    assert_eq!(eval("10 - 4 - 1", &vars).unwrap(), Value::Int(5));
    assert_eq!(eval("10 / 4", &vars).unwrap(), Value::Float(2.5));
    assert_eq!(eval("10 / 5", &vars).unwrap(), Value::Int(2));
    assert!(eval("1 / 0", &vars).is_err());
}

#[test]
fn 负数字面量与字符串拼接() {
    let vars = v();
    assert_eq!(eval("-5", &vars).unwrap(), Value::Int(-5));
    assert_eq!(eval("\"夏\" + '天'", &vars).unwrap(), Value::Str("夏天".into()));
    assert_eq!(eval("sf.name + 1", &vars).unwrap(), Value::Str("夏1".into()));
    assert_eq!(eval("f.a - -1", &vars).unwrap(), Value::Int(3));
}

#[test]
fn 空值合并() {
    let vars = v();
    assert_eq!(eval("f.blank ?? '兜底'", &vars).unwrap(), Value::Str("兜底".into()));
    assert_eq!(eval("f.nothing ?? f.a", &vars).unwrap(), Value::Int(2));
    assert_eq!(eval("sf.name ?? 'x'", &vars).unwrap(), Value::Str("夏".into()));
    assert_eq!(
        eval("f.missing ?? f.blank ?? '深兜底'", &vars).unwrap(),
        Value::Str("深兜底".into())
    );
}

#[test]
fn 条件求值() {
    let vars = v();
    assert!(eval_cond("f.a", "==", "2", &vars).unwrap());
    assert!(eval_cond("f.a", ">=", "2", &vars).unwrap());
    assert!(!eval_cond("f.a", "<", "2", &vars).unwrap());
    assert!(eval_cond("sf.name", "==", "夏", &vars).unwrap());
    assert!(eval_cond("sf.name", "!=", "冬", &vars).unwrap());
    assert!(eval_cond("sf.name", ">", "1", &vars).is_err());
}

#[test]
fn 未赋值变量与数字比较按零() {
    let vars = v();
    // 未赋值（空串）vs 数字：按 0，不再报错（剧本可省初始化）
    assert!(eval_cond("f.nothing", "<", "80", &vars).unwrap());
    assert!(!eval_cond("f.nothing", ">=", "80", &vars).unwrap());
    assert!(eval_cond("f.nothing", "==", "0", &vars).unwrap());
    assert!(eval_cond("f.nothing", "!=", "1", &vars).unwrap());
    // 两侧都是空串（均未赋值）：按 0 与 0 比较
    assert!(eval_cond("f.nothing", "==", "f.blank", &vars).unwrap());
    assert!(!eval_cond("f.nothing", ">", "f.blank", &vars).unwrap());
    // 非空字符串 vs 任意字符串仍禁用大小比较
    assert!(eval_cond("sf.name", ">", "f.blank", &vars).is_err());
    // 裸名未赋值（会被 eval 当字符串字面量）：条件左值按 0
    assert!(eval_cond("nope", "<", "80", &vars).unwrap());
    assert!(!eval_cond("nope", ">=", "1", &vars).unwrap());
    assert!(eval_cond("nope", "==", "0", &vars).unwrap());
}
