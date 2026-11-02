//! command 单元测试

use super::types::Command;
use crate::lexer::LineKind;

fn cmd(name: &str, rest: &str) -> Result<Command, String> {
    Command::parse(&LineKind::Command { name: name.into(), rest: rest.into() }, "t.ks:1")
}

#[test]
fn 基本指令解析() {
    assert!(
        matches!(cmd("bg", "a 800").unwrap(), Command::Bg { storage, fade_ms: 800 } if storage == "a")
    );
    assert!(
        matches!(cmd("char", "0 hide").unwrap(), Command::Char { layer: 0, ref storage, x: None, y: None } if storage == "hide")
    );
    assert!(cmd("char", "3 x").is_err());
    assert!(
        matches!(cmd("flag", "a +2").unwrap(), Command::Flag { var, op: '+', val: 2 } if var == "a")
    );
    assert!(matches!(cmd("flag", "a -1").unwrap(), Command::Flag { op: '-', val: 1, .. }));
    assert!(
        matches!(cmd("reach", "hide").unwrap(), Command::Reach { ref storage, .. } if storage == "hide")
    );
    assert!(matches!(cmd("window_fx", "title restore").unwrap(), Command::WindowFxTitleRestore));
    assert!(matches!(cmd("shutdown", "").unwrap(), Command::Shutdown(300)));
}

#[test]
fn set表达式与desktop内容含空格() {
    match cmd("set", "a = 1 + 2").unwrap() {
        Command::Set { var, expr } => {
            assert_eq!(var, "a");
            assert_eq!(expr, "1 + 2");
        }
        _ => panic!(),
    }
    match cmd("desktop_write", "letter.txt|还记得吗。{hero}").unwrap() {
        Command::DesktopWrite { file, content } => {
            assert_eq!(file, "letter.txt");
            assert_eq!(content, "还记得吗。{hero}");
        }
        _ => panic!(),
    }
}

#[test]
fn 未知指令中文报错() {
    let e = cmd("foo", "").unwrap_err();
    assert!(e.starts_with("t.ks:1：未知指令「foo」"), "{e}");
    assert!(e.contains("已停止在上一句台词"), "{e}");
}

#[test]
fn charsway与顶层shake() {
    match cmd("charsway", "0 15 400").unwrap() {
        Command::CharSway { layer, count, period_ms } => {
            assert_eq!((layer, count, period_ms), (0, 15, 400));
        }
        _ => panic!(),
    }
    // 省略周期走默认 300ms
    assert!(matches!(
        cmd("charsway", "2 3").unwrap(),
        Command::CharSway { layer: 2, count: 3, period_ms: 300 }
    ));
    assert!(cmd("charsway", "0 15 400 extra").is_err());
    assert!(cmd("charsway", "3 5").is_err()); // 层越界
    assert!(matches!(cmd("shake", "500").unwrap(), Command::WindowFxShake(500)));
    assert!(cmd("shake", "").is_err());
}

#[test]
fn cg_hide的ms参数不再被吞() {
    // 回归：`cg hide 800` 曾被 [s] | [s, _] 臂吞成默认 700
    assert!(matches!(
        cmd("cg", "hide 800").unwrap(),
        Command::Cg { ref storage, fade_ms: 800 } if storage == "hide"
    ));
    assert!(matches!(
        cmd("cg", "hide").unwrap(),
        Command::Cg { ref storage, fade_ms: 700 } if storage == "hide"
    ));
    assert!(matches!(cmd("cg", "a 1200").unwrap(), Command::Cg { fade_ms: 1200, .. }));
}

#[test]
fn clear与end容忍多余参数() {
    assert!(matches!(cmd("clear", "").unwrap(), Command::Clear));
    assert!(matches!(cmd("clear", "all").unwrap(), Command::Clear));
    assert!(matches!(cmd("end", "").unwrap(), Command::End));
    assert!(matches!(cmd("end", "now").unwrap(), Command::End));
}

#[test]
fn char位置名解析() {
    match cmd("char", "0 alice_normal center").unwrap() {
        Command::CharPos { layer, storage, pos_name } => {
            assert_eq!(layer, 0);
            assert_eq!(storage, "alice_normal");
            assert_eq!(pos_name, "center");
        }
        _ => panic!("应解析为 CharPos"),
    }
}

#[test]
fn 演示剧本start_ks全量解析通过() {
    let script = include_str!("../../../../template/game/data/scenario/start.ks");
    let lines = crate::lexer::parse_script(script).expect("词法解析通过");
    assert!(!lines.is_empty());
    for line in lines {
        if let crate::lexer::LineKind::Command { .. } | crate::lexer::LineKind::Choice { .. } =
            &line.kind
        {
            Command::parse(&line.kind, &format!("start.ks:{}", line.no)).expect("指令解析通过");
        }
    }
}
