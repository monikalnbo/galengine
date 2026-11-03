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
    // v0.3.7：首参不是合法层号 → 无前缀形态（64 当 ms、x 当 method）
    assert!(matches!(cmd("trans", "64 x").unwrap(), Command::ForeLay { n: None, .. }));
    // v0.3.7：无前缀形态（无层号无素材）= back 页翻上，不再是错误
    assert!(matches!(cmd("trans", "").unwrap(), Command::ForeLay { n: None, .. }));
    // 无前缀双参：trans 500 crossfade（第二参纯数字 → ForeLay）；素材名非数字走旧语法
    match cmd("trans", "500 crossfade").unwrap() {
        Command::ForeLay { n, dur_ms, method } => {
            assert_eq!((n, dur_ms, method.as_str()), (None, 500, "crossfade"));
        }
        _ => panic!(),
    }
    match cmd("trans", "600").unwrap() {
        Command::ForeLay { n, dur_ms, method } => {
            assert_eq!((n, dur_ms, method.as_str()), (None, 600, "universal"));
        }
        _ => panic!(),
    }
    assert!(matches!(cmd("trans", "0 bg/next").unwrap(), Command::Trans { .. }));
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

// ===== v0.3.2 KAG3 音频标签组解析 =====

#[test]
fn v032音频标签组_解析() {
    assert!(matches!(
        cmd("playbgm", "theme").unwrap(),
        Command::PlayBgm { ref name, fade_in_ms: 0 } if name == "theme"
    ));
    assert!(matches!(
        cmd("playbgm", "theme 800").unwrap(),
        Command::PlayBgm { fade_in_ms: 800, .. }
    ));
    assert!(matches!(cmd("pausebgm", "").unwrap(), Command::PauseBgm));
    assert!(matches!(cmd("resumebgm", "").unwrap(), Command::ResumeBgm));
    assert!(matches!(
        cmd("fadeinbgm", "theme 1200").unwrap(),
        Command::FadeInBgm { ref name, ms: 1200 } if name == "theme"
    ));
    assert!(matches!(cmd("fadeoutbgm", "500").unwrap(), Command::FadeOutBgm { ms: 500 }));
    assert!(matches!(cmd("fadepausebgm", "300").unwrap(), Command::FadePauseBgm { ms: 300 }));
    assert!(matches!(cmd("fadebgm", "40 600").unwrap(), Command::FadeBgm { to_pct: 40, ms: 600 }));
    assert!(matches!(
        cmd("xchgbgm", "next 900").unwrap(),
        Command::XchgBgm { ref name, ms: 900 } if name == "next"
    ));
    assert!(matches!(
        cmd("bgmopt", "10000 30000").unwrap(),
        Command::BgmOpt { loop_start_ms: 10000, loop_end_ms: 30000 }
    ));
    assert!(matches!(
        cmd("playse", "click").unwrap(),
        Command::PlaySe { ref name } if name == "click"
    ));
    assert!(matches!(cmd("stopse", "").unwrap(), Command::StopSe));
    assert!(matches!(cmd("fadeinse", "click 200").unwrap(), Command::FadeInSe { ms: 200, .. }));
    assert!(matches!(cmd("fadeoutse", "click 400").unwrap(), Command::FadeOutSe { ms: 400, .. }));
    assert!(matches!(cmd("fadese", "30 500").unwrap(), Command::FadeSe { to_pct: 30, ms: 500 }));
    assert!(matches!(cmd("wb", "").unwrap(), Command::WaitBgm));
    assert!(matches!(cmd("wl", "").unwrap(), Command::WaitLoop));
    assert!(matches!(cmd("ws", "").unwrap(), Command::WaitSe));
    assert!(matches!(cmd("wf", "").unwrap(), Command::WaitSeFade));
    // 旧语法零改动：bgm/se/voice 仍然照旧解析
    assert!(matches!(cmd("bgm", "stop").unwrap(), Command::BgmStop));
    assert!(matches!(cmd("bgm", "fadeout 400").unwrap(), Command::BgmFadeOut { ms: 400 }));
    assert!(matches!(cmd("bgm", "theme").unwrap(), Command::Bgm(ref n) if n == "theme"));
    // 非法参数给中文报错
    assert!(cmd("bgmopt", "9999").is_err());
    assert!(cmd("fadeoutbgm", "").is_err());
    assert!(cmd("fadebgm", "40").is_err());
}

#[test]
fn v033等待标签_解析() {
    // wm/wv/wp：与 wb/wl/ws/wf 同款无参等待
    assert!(matches!(cmd("wm", "").unwrap(), Command::WaitMusic));
    assert!(matches!(cmd("wv", "").unwrap(), Command::WaitVideo));
    assert!(matches!(cmd("wp", "").unwrap(), Command::WaitAllSound));
    // 带参数视为剧本写错，中文报错
    assert!(cmd("wm", "extra").is_err());
    assert!(cmd("wv", "1").is_err());
    assert!(cmd("wp", "x").is_err());
}

#[test]
fn v038视频标签组_解析() {
    assert!(matches!(
        cmd("video", "mode=1 seek=500").unwrap(),
        Command::Video { ref params } if params == "mode=1 seek=500"
    ));
    assert!(matches!(
        cmd("playvideo", "op").unwrap(),
        Command::PlayVideo { ref name, ms: 0, skip: false } if name == "op"
    ));
    assert!(matches!(
        cmd("playvideo", "op 3000").unwrap(),
        Command::PlayVideo { ms: 3000, skip: false, .. }
    ));
    assert!(matches!(
        cmd("playvideo", "op skip").unwrap(),
        Command::PlayVideo { ms: 0, skip: true, .. }
    ));
    assert!(matches!(
        cmd("playvideo", "op 3000 skip").unwrap(),
        Command::PlayVideo { ms: 3000, skip: true, .. }
    ));
    assert!(cmd("playvideo", "").is_err());
    assert!(matches!(cmd("stopvideo", "").unwrap(), Command::StopVideo));
    assert!(matches!(
        cmd("openvideo", "op").unwrap(),
        Command::OpenVideo { ref name } if name == "op"
    ));
    assert!(cmd("openvideo", "").is_err());
    assert!(matches!(cmd("pausevideo", "").unwrap(), Command::PauseVideo));
    assert!(matches!(cmd("resumevideo", "").unwrap(), Command::ResumeVideo));
    assert!(matches!(
        cmd("preparevideo", "op").unwrap(),
        Command::PrepareVideo { ref name } if name == "op"
    ));
    assert!(matches!(cmd("rewindvideo", "").unwrap(), Command::RewindVideo));
    // 带参数视为剧本写错，中文报错
    assert!(cmd("stopvideo", "x").is_err());
    assert!(cmd("pausevideo", "1").is_err());
}

#[test]
fn v033转场方法_解析() {
    use crate::stage::{Direction, TransMethod};
    assert_eq!(TransMethod::parse("crossfade"), TransMethod::Crossfade);
    assert_eq!(TransMethod::parse("universal"), TransMethod::Universal);
    assert_eq!(TransMethod::parse("slidel"), TransMethod::Slide(Direction::Left));
    assert_eq!(TransMethod::parse("slide r"), TransMethod::Slide(Direction::Right));
    assert_eq!(TransMethod::parse("scrollu"), TransMethod::Scroll(Direction::Up));
    assert_eq!(TransMethod::parse("scroll down"), TransMethod::Scroll(Direction::Down));
    // 未知方法退化交叉淡化（不报错，静默留证口径）
    assert_eq!(TransMethod::parse("mosaicscroll"), TransMethod::Unsupported);
    assert_eq!(TransMethod::parse("slide x"), TransMethod::Unsupported);
}

#[test]
fn v033转场偏移_slide_scroll() {
    use crate::stage::{Direction, TransClip, TransMethod};
    let clip = |m| TransClip {
        method: m,
        layer: 0,
        to: String::new(),
        from: None,
        offset: (0, 0),
        dur_ms: 1000.0,
        left_ms: 500.0,
    };
    // slide l：胶片式——旧内容左移半屏、新内容从右侧半屏处跟进
    let (old, new) = clip(TransMethod::Slide(Direction::Left)).slide_offsets(0.5);
    assert_eq!(old, (-640, 0));
    assert_eq!(new, (640, 0));
    // scroll u：擦除式——旧内容原位、新内容从上方推入半屏
    let (old, new) = clip(TransMethod::Scroll(Direction::Up)).slide_offsets(0.5);
    assert_eq!(old, (0, 0));
    assert_eq!(new, (0, -360));
    // crossfade：全零偏移
    let (old, new) = clip(TransMethod::Crossfade).slide_offsets(0.5);
    assert_eq!((old, new), ((0, 0), (0, 0)));
}

#[test]
fn v039条件标签收尾_解析() {
    // layopt：KAG3 属性写法
    assert!(matches!(
        cmd("layopt", "1 visible=false").unwrap(),
        Command::LayOpt { n: 1, visible: Some(false), opacity: None }
    ));
    assert!(matches!(
        cmd("layopt", "2 opacity=128").unwrap(),
        Command::LayOpt { n: 2, visible: None, opacity: Some(128) }
    ));
    assert!(matches!(
        cmd("layopt", "1 visible=true opacity=0").unwrap(),
        Command::LayOpt { visible: Some(true), opacity: Some(0), .. }
    ));
    assert!(cmd("layopt", "").is_err());
    assert!(cmd("layopt", "1 bogus=x").is_err());
    assert!(cmd("layopt", "1 opacity=999").is_err());
    // laycount / stoptrans
    assert!(matches!(cmd("laycount", "5").unwrap(), Command::LayCount { n: 5 }));
    assert!(cmd("laycount", "").is_err());
    assert!(matches!(cmd("stoptrans", "").unwrap(), Command::StopTrans));
    assert!(cmd("stoptrans", "1").is_err());
    // move 缓动尾参：识别方法名 → MoveEase，否则原 Move
    assert!(matches!(
        cmd("move", "5 100,0@500 easeinout").unwrap(),
        Command::MoveEase { n: 5, ease, .. } if ease == "easeinout"
    ));
    assert!(matches!(
        cmd("move", "5 0,0 200,40@600 linear").unwrap(),
        Command::MoveEase { ease, .. } if ease == "linear"
    ));
    // 方法名不是帧写法，不会误当 x,y 解析；无尾参保持旧 Move（v0.3.6 剧本零改动）
    assert!(matches!(cmd("move", "5 100,0@500").unwrap(), Command::Move { n: 5, .. }));
    // 层号同名帧写法不受影响（move 1 0,0 无尾参）
    assert!(matches!(cmd("move", "1 0,0").unwrap(), Command::Move { n: 1, .. }));
    // animstart 层 档名（v0.3.10）
    assert!(matches!(
        cmd("animstart", "5 swing").unwrap(),
        Command::AnimStart { n: 5, name } if name == "swing"
    ));
    assert!(cmd("animstart", "").is_err());
    assert!(cmd("animstart", "5").is_err());
    assert!(cmd("animstart", "5 a b").is_err());
}

#[test]
fn v0311演出等待收尾_解析() {
    // quake [ms] [maxamp]（KAG3 口径：缺省 300ms）
    assert!(matches!(cmd("quake", "").unwrap(), Command::Quake { ms: 300, max_amp: 14 }));
    assert!(matches!(cmd("quake", "600").unwrap(), Command::Quake { ms: 600, max_amp: 14 }));
    assert!(matches!(cmd("quake", "500 20").unwrap(), Command::Quake { ms: 500, max_amp: 20 }));
    assert!(cmd("quake", "x").is_err());
    // stopquake / wq / wc / resetwait（无参指令）
    assert!(matches!(cmd("stopquake", "").unwrap(), Command::StopQuake));
    assert!(matches!(cmd("wq", "").unwrap(), Command::WaitQuake));
    assert!(matches!(cmd("wc", "").unwrap(), Command::WaitClickCount));
    assert!(matches!(cmd("resetwait", "").unwrap(), Command::ResetWait));
    assert!(cmd("wq", "1").is_err());
    // locksnapshot [名] / unlocksnapshot
    assert!(matches!(
        cmd("locksnapshot", "封印中").unwrap(),
        Command::LockSnapshot { name } if name == "封印中"
    ));
    assert!(matches!(
        cmd("locksnapshot", "").unwrap(),
        Command::LockSnapshot { name } if name.is_empty()
    ));
    assert!(matches!(cmd("unlocksnapshot", "").unwrap(), Command::UnlockSnapshot));
}

#[test]
fn v0312存档控制标签_解析() {
    // s（无参）
    assert!(matches!(cmd("s", "").unwrap(), Command::StopSkip));
    // disablestore [t/f]（缺省 true）
    assert!(matches!(cmd("disablestore", "").unwrap(), Command::DisableStore { on: true }));
    assert!(matches!(cmd("disablestore", "false").unwrap(), Command::DisableStore { on: false }));
    assert!(matches!(cmd("disablestore", "f").unwrap(), Command::DisableStore { on: false }));
    assert!(cmd("disablestore", "maybe").is_err());
    // tempsave / tempload / record / goback（无参）
    assert!(matches!(cmd("tempsave", "").unwrap(), Command::TempSave));
    assert!(matches!(cmd("tempload", "").unwrap(), Command::TempLoad));
    assert!(matches!(cmd("record", "").unwrap(), Command::Record));
    assert!(matches!(cmd("goback", "").unwrap(), Command::GoBack));
    // copybookmark 源 目标
    assert!(matches!(
        cmd("copybookmark", "1 3").unwrap(),
        Command::CopyBookmark { from: 1, to: 3 }
    ));
    assert!(cmd("copybookmark", "1").is_err());
    assert!(cmd("copybookmark", "1 x").is_err());
    // erasebookmark 槽号
    assert!(matches!(cmd("erasebookmark", "2").unwrap(), Command::EraseBookmark { slot: 2 }));
    assert!(cmd("erasebookmark", "").is_err());
    // waittrig 名
    assert!(matches!(
        cmd("waittrig", "open").unwrap(),
        Command::WaitTrig { name } if name == "open"
    ));
    assert!(cmd("waittrig", "").is_err());
}

#[test]
fn v0313文本标记标签_解析() {
    // graph 素材名
    assert!(matches!(
        cmd("graph", "face_a").unwrap(),
        Command::Graph { storage } if storage == "face_a"
    ));
    assert!(cmd("graph", "").is_err());
    // ruby text=基底 alt=注音
    assert!(matches!(
        cmd("ruby", "text=夏 alt=なつ").unwrap(),
        Command::RubyTag { text, alt } if text == "夏" && alt == "なつ"
    ));
    assert!(matches!(
        cmd("ruby", "text=夏 alt=\"な つ\"").unwrap(),
        Command::RubyTag { alt, .. } if alt == "な つ"
    ));
    assert!(cmd("ruby", "").is_err());
    assert!(cmd("ruby", "text=夏").is_err());
    // hch / font / deffont / resetfont / style / defstyle / resetstyle
    assert!(matches!(cmd("hch", "重要").unwrap(), Command::Hch { text } if text == "重要"));
    assert!(matches!(
        cmd("font", "size=24 color=0xFF0000").unwrap(),
        Command::Font { size: Some(24), color: Some(_) }
    ));
    assert!(matches!(cmd("font", "").unwrap(), Command::Font { size: None, color: None }));
    assert!(matches!(
        cmd("deffont", "size=20").unwrap(),
        Command::Deffont { size: Some(20), color: None }
    ));
    assert!(matches!(cmd("resetfont", "").unwrap(), Command::Resetfont));
    assert!(matches!(
        cmd("style", "align=center").unwrap(),
        Command::Style { align: Some(ref a) } if a == "center"
    ));
    assert!(matches!(cmd("defstyle", "").unwrap(), Command::Defstyle { align: None }));
    assert!(matches!(cmd("resetstyle", "").unwrap(), Command::Resetstyle));
}
