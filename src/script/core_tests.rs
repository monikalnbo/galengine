//! 词法/变量/存档工具单测（纯逻辑）

use crate::script::lexer::{parse_script, LineKind};
use crate::script::vars::{Value, Vars};

// —— lexer ——

#[test]
fn lexer_basic_lines() {
    let src = "# 注释\n*start\nbg bg_port 800\nn 你好，夏天\nname 澪 「台词」里的|竖线\n\n";
    let lines = parse_script(src).unwrap();
    assert_eq!(lines.len(), 4, "注释和空行剔除");
    assert!(matches!(&lines[0].kind, LineKind::Label(l) if l == "start"));
    assert!(matches!(&lines[1].kind, LineKind::Command { name, .. } if name == "bg"));
    assert!(matches!(&lines[2].kind, LineKind::Command { rest, .. } if rest == "你好，夏天"));
    assert_eq!(lines[0].no, 2, "行号保留（错误定位用）");
}

#[test]
fn lexer_choice_block() {
    let src = "choice 要看哪里？\n  帮澪捡讲义|*c1_a\n  看向窗外|*c1_c\nendchoice\n";
    let lines = parse_script(src).unwrap();
    match &lines[0].kind {
        LineKind::Choice { items, .. } => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], ("帮澪捡讲义".into(), "*c1_a".into()));
        }
        _ => panic!("应为 Choice"),
    }
}

#[test]
fn lexer_missing_endchoice() {
    let e = parse_script("choice 提示\n  a|*x\n").unwrap_err();
    assert!(e.contains("endchoice"), "中文错误：{e}");
}

#[test]
fn lexer_bom_stripped() {
    let lines = parse_script("\u{feff}n 台词").unwrap();
    assert!(matches!(&lines[0].kind, LineKind::Command { name, .. } if name == "n"));
}

// —— vars ——

#[test]
fn vars_namespaces() {
    let mut v = Vars::new();
    v.set("f.top", Value::Str("mio".into()));
    v.set("sf.kakusei", Value::Int(2));
    assert_eq!(v.get("f.top"), Some(&Value::Str("mio".into())));
    assert_eq!(v.get("sf.kakusei"), Some(&Value::Int(2)));
    assert_eq!(v.get("kakusei"), None, "裸名默认 f. 命名空间");
    assert_eq!(v.get_or("nope"), Value::Int(0), "缺失默认 0");
}

#[test]
fn vars_global_roundtrip() {
    let mut v = Vars::new();
    v.set("sf.playerName", Value::Str("你".into()));
    v.save_global("/tmp/galengine_test_global.json").unwrap();
    let loaded = Vars::load_global("/tmp/galengine_test_global.json");
    assert_eq!(loaded.get("sf.playerName"), Some(&Value::Str("你".into())));
    std::fs::remove_file("/tmp/galengine_test_global.json").ok();
}

// —— slots ——

#[test]
fn slots_digest_and_date() {
    let a = crate::save::slots::digest_of("同一内容");
    let b = crate::save::slots::digest_of("同一内容");
    let c = crate::save::slots::digest_of("不同内容");
    assert_eq!(a, b, "同内容同指纹");
    assert_ne!(a, c, "内容变指纹变");
    let d = crate::save::slots::now_str();
    assert!(d.contains('-'), "日期格式：{d}");
}
