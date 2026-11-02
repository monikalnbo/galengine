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
fn 条件两侧支持表达式() {
    let mut vars = v();
    vars.set("f.b", Value::Int(3));
    let vars = vars;
    // 左值表达式
    assert!(eval_cond("f.a + 1", "==", "3", &vars).unwrap());
    assert!(eval_cond("f.a * 2", ">", "3", &vars).unwrap());
    // 右值表达式
    assert!(eval_cond("f.a", "<", "f.b + 1", &vars).unwrap());
    // 两侧都是表达式
    assert!(eval_cond("f.a + 1", "==", "f.b - 0", &vars).unwrap());
    // 字符串表达式
    assert!(eval_cond("sf.name + '天'", "==", "'夏天'", &vars).unwrap());
    // 数字字面量左值仍按数字（不被当作未赋值标识符吞成 0）
    assert!(!eval_cond("2", "==", "0", &vars).unwrap());
    assert!(eval_cond("2", "==", "2", &vars).unwrap());
    // 除零等表达式错误照常上抛
    assert!(eval_cond("1 / 0", "==", "1", &vars).is_err());
}

#[test]
fn 括号与优先级() {
    let vars = v();
    assert_eq!(eval("(1 + 2) * 3", &vars).unwrap(), Value::Int(9));
    assert_eq!(eval("2 * (3 + 4) - 1", &vars).unwrap(), Value::Int(13));
    assert_eq!(eval("((f.a))", &vars).unwrap(), Value::Int(2));
    assert!(eval("(1 + 2", &vars).unwrap_err().contains("括号未闭合"));
    assert!(eval("1 + 2)", &vars).unwrap_err().contains("多余内容"));
    assert!(eval("()", &vars).is_err());
}

#[test]
fn 比较与逻辑() {
    let vars = v();
    assert_eq!(eval("f.a > 1", &vars).unwrap(), Value::Int(1));
    assert_eq!(eval("f.a < 1", &vars).unwrap(), Value::Int(0));
    assert_eq!(eval("f.a == 2 && f.a != 3", &vars).unwrap(), Value::Int(1));
    assert_eq!(eval("f.a == 1 || f.a == 2", &vars).unwrap(), Value::Int(1));
    // 与四则的优先级：比较低于算术
    assert_eq!(eval("f.a + 1 == 3", &vars).unwrap(), Value::Int(1));
    assert_eq!(eval("(f.a + 1) * 2 >= 6", &vars).unwrap(), Value::Int(1));
    // 逻辑低于比较
    assert_eq!(eval("f.a > 1 && f.a < 3", &vars).unwrap(), Value::Int(1));
    // ?? 高于 &&，便于 `a ?? 0 && b` 这类写法
    assert_eq!(eval("f.missing ?? 1 && 1", &vars).unwrap(), Value::Int(1));
    // 字符串等值比较允许，大小比较拒绝
    assert_eq!(eval("sf.name == '夏'", &vars).unwrap(), Value::Int(1));
    assert!(eval("sf.name > 'a'", &vars).is_err());
}

#[test]
fn 一元与取模() {
    let vars = v();
    assert_eq!(eval("!0", &vars).unwrap(), Value::Int(1));
    assert_eq!(eval("!f.a", &vars).unwrap(), Value::Int(0));
    assert_eq!(eval("-(1 + 2)", &vars).unwrap(), Value::Int(-3));
    assert_eq!(eval("10 % 3", &vars).unwrap(), Value::Int(1));
    assert_eq!(eval("10 % 5", &vars).unwrap(), Value::Int(0));
    assert!(eval("10 % 0", &vars).is_err());
    assert!(eval("-sf.name", &vars).is_err());
    // 未赋值变量参与算术仍按字符串处理（+ 为拼接，- 报错），与既有语义一致
    assert_eq!(eval("f.blank + 2", &vars).unwrap(), Value::Str("2".into()));
    assert!(eval("f.blank - 2", &vars).is_err());
}

#[test]
fn 短路不求值右侧() {
    let vars = v();
    // 右侧除零：短路后不应触发
    assert_eq!(eval("0 && 1 / 0", &vars).unwrap(), Value::Int(0));
    assert_eq!(eval("1 || 1 / 0", &vars).unwrap(), Value::Int(1));
    // 非短路时照常报错
    assert!(eval("1 && 1 / 0", &vars).is_err());
    assert!(eval("0 || 1 / 0", &vars).is_err());
    // 裸词未赋值 = 非空字符串 = 真
    assert_eq!(eval("nope && 1", &vars).unwrap(), Value::Int(1));
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

// ── v0.3.4：三元 / 数组 / 字典 / 成员访问 ────────────────────────────────

#[test]
fn 三元运算符() {
    let vars = v();
    assert_eq!(eval("f.a > 1 ? '大' : '小'", &vars).unwrap(), Value::Str("大".into()));
    assert_eq!(eval("f.a > 5 ? '大' : '小'", &vars).unwrap(), Value::Str("小".into()));
    // 右结合嵌套
    assert_eq!(
        eval("f.a == 1 ? '一' : f.a == 2 ? '二' : '多'", &vars).unwrap(),
        Value::Str("二".into())
    );
    // 三元惰性：未选分支不报错
    assert_eq!(eval("1 ? 2 : 1 / 0", &vars).unwrap(), Value::Int(2));
    assert!(eval("1 ? 1 / 0 : 2", &vars).is_err());
    // 缺冒号报错
    assert!(eval("1 ? 2", &vars).is_err());
}

#[test]
fn 数组字面量与下标() {
    let vars = v();
    assert_eq!(
        eval("[1, 2, 3]", &vars).unwrap(),
        Value::Arr(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
    assert_eq!(eval("[1, 2, 3][1]", &vars).unwrap(), Value::Int(2));
    assert_eq!(eval("['a', 'b'][f.a - 1]", &vars).unwrap(), Value::Str("b".into()));
    // 越界报错
    assert!(eval("[1][5]", &vars).is_err());
    // 空数组（与空字典按内容区分：[] 归数组、%[] 归字典）
    assert!(matches!(eval("[]", &vars).unwrap(), Value::Arr(a) if a.is_empty()));
}

#[test]
fn 字典字面量与成员访问() {
    let vars = v();
    assert_eq!(
        eval("%[ name: '夏', hp: 100 ]", &vars).unwrap(),
        Value::Dict(vec![("name".into(), Value::Str("夏".into())), ("hp".into(), Value::Int(100))])
    );
    // 成员访问（lexer 已把 f.x.y 并入标识符；此处走字面量后缀）
    assert_eq!(eval("%[ name: '夏' ][ 'name' ]", &vars).unwrap(), Value::Str("夏".into()));
    assert!(eval("%[ a: 1 ].b", &vars).is_err());
    // 字典键必须字符串
    assert!(eval("%[ 1: 'x' ]", &vars).is_err());
    // 数组字典混合判别：有 Pair 即字典
    assert!(matches!(eval("%[ ]", &vars).unwrap(), Value::Dict(_)));
    // 数组里不能有冒号
    assert!(eval("[1: 2]", &vars).is_err());
}

#[test]
fn 变量成员路径与length() {
    let mut vars = v();
    vars.set(
        "f.party",
        Value::Dict(vec![("hp".into(), Value::Int(30)), ("mp".into(), Value::Int(8))]),
    );
    vars.set("f.bag", Value::Arr(vec![Value::Str("药".into()), Value::Int(3)]));
    assert_eq!(eval("f.party.hp + f.party.mp", &vars).unwrap(), Value::Int(38));
    assert_eq!(eval("f.bag.length", &vars).unwrap(), Value::Int(2));
    assert_eq!(eval("f.bag[0]", &vars).unwrap(), Value::Str("药".into()));
    assert_eq!(eval("f.bag.length > 1 ? '够' : '缺'", &vars).unwrap(), Value::Str("够".into()));
    // 不支持的成员
    assert!(eval("f.bag.hp", &vars).is_err());
    assert!(eval("f.a.length", &vars).is_err());
}

#[test]
fn 容器与逻辑真值() {
    let vars = v();
    assert_eq!(eval("[1] && 1", &vars).unwrap(), Value::Int(1));
    assert_eq!(eval("[] || 1", &vars).unwrap(), Value::Int(1));
    assert_eq!(eval("%[ a: 1 ] ? 'y' : 'n'", &vars).unwrap(), Value::Str("y".into()));
    assert_eq!(eval("%[ ] ? 'y' : 'n'", &vars).unwrap(), Value::Str("n".into()));
}
