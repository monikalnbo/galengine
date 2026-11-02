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
fn if的单行与块形态() {
    // 单行：条件表达式原样保存，目标去星号
    match cmd("if", "f.a + 1 > 3 *next").unwrap() {
        Command::If { cond, target } => {
            assert_eq!(cond, "f.a + 1 > 3");
            assert_eq!(target, "next");
        }
        _ => panic!("应解析为单行 if"),
    }
    // 旧写法 `if f.a == 1 *next` 走同一条路径
    match cmd("if", "f.a == 1 *next").unwrap() {
        Command::If { cond, target } => {
            assert_eq!((cond.as_str(), target.as_str()), ("f.a == 1", "next"));
        }
        _ => panic!(),
    }
    // 无 *目标 = 块形态
    assert!(
        matches!(cmd("if", "f.a == 1").unwrap(), Command::IfBlock { ref cond } if cond == "f.a == 1")
    );
    assert!(matches!(cmd("else", "").unwrap(), Command::Else));
    assert!(matches!(cmd("endif", "").unwrap(), Command::EndIf));
    assert!(cmd("if", "").is_err());
    assert!(cmd("else", "x").is_err());
}

#[test]
fn call与return解析() {
    match cmd("call", "*sub").unwrap() {
        Command::Call { file: None, label } => assert_eq!(label, "sub"),
        _ => panic!(),
    }
    match cmd("call", "other.ks *sub").unwrap() {
        Command::Call { file: Some(f), label } => {
            assert_eq!((f.as_str(), label.as_str()), ("other.ks", "sub"));
        }
        _ => panic!(),
    }
    assert!(matches!(cmd("return", "").unwrap(), Command::Return));
    assert!(cmd("call", "").is_err());
    assert!(cmd("return", "x").is_err());
}

#[test]
fn 自由层layer解析与层号校验() {
    match cmd("layer", "7 free/a").unwrap() {
        Command::Layer { n, storage, fade_ms } => {
            assert_eq!((n, storage.as_str(), fade_ms), (7, "free/a", 700));
        }
        _ => panic!(),
    }
    match cmd("layer", "5 free/a 300").unwrap() {
        Command::Layer { n, fade_ms, .. } => assert_eq!((n, fade_ms), (5, 300)),
        _ => panic!(),
    }
    assert!(matches!(cmd("layer", "9 hide").unwrap(), Command::Layer { n: 9, .. }));
    // 0-4 是保留层号；64 起超上限
    assert!(cmd("layer", "0 x").is_err());
    assert!(cmd("layer", "4 x").is_err());
    assert!(cmd("layer", "64 x").is_err());
    assert!(cmd("layer", "7").is_err());
}

#[test]
fn 转场trans与wt解析() {
    match cmd("trans", "0 bg/next").unwrap() {
        Command::Trans { n, storage, dur_ms, method } => {
            assert_eq!(
                (n, storage.as_str(), dur_ms, method.as_str()),
                (0, "bg/next", 700, "universal")
            );
        }
        _ => panic!(),
    }
    match cmd("trans", "4 cg/a 900 crossfade").unwrap() {
        Command::Trans { n, dur_ms, method, .. } => {
            assert_eq!((n, dur_ms, method.as_str()), (4, 900, "crossfade"));
        }
        _ => panic!(),
    }
    match cmd("trans", "9 free/a 300").unwrap() {
        Command::Trans { n, dur_ms, method, .. } => {
            assert_eq!((n, dur_ms, method.as_str()), (9, 300, "universal"));
        }
        _ => panic!(),
    }
    // 三参数且第三参不是数字 → 当 method（宽容写法）
    match cmd("trans", "5 free/a crossfade").unwrap() {
        Command::Trans { dur_ms, method, .. } => {
            assert_eq!((dur_ms, method.as_str()), (700, "crossfade"));
        }
        _ => panic!(),
    }
    assert!(cmd("trans", "64 x").is_err());
    assert!(cmd("trans", "0").is_err());
    assert!(matches!(cmd("wt", "").unwrap(), Command::WaitTrans));
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
