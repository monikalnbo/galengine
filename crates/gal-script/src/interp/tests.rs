//! interp 单元测试

use super::Interp;
use crate::command::Command;
use crate::vars::Vars;
use gal_config::sprites::SpritePos;

fn interp() -> Interp {
    Interp::new(Vars::default(), 30.0, "阿岚", "你")
}

#[test]
fn q版名单标记层() {
    let mut it = interp();
    it.chibi.insert("chibi_normal".into());
    it.exec(Command::Char { layer: 1, storage: "chibi_normal".into(), x: None, y: None }, "t.ks:1")
        .unwrap();
    assert!(it.stage.chars[1].chibi);
    it.exec(Command::Char { layer: 1, storage: "hide".into(), x: None, y: None }, "t.ks:2")
        .unwrap();
    assert!(!it.stage.chars[1].chibi);
}

#[test]
fn 预设位置从注入的配置查表() {
    let mut it = interp();
    it.positions.insert("far_left".into(), SpritePos { x: -140, y: 20 });
    it.exec(
        Command::CharPos { layer: 1, storage: "hide".into(), pos_name: "far_left 0".into() },
        "t.ks:1",
    )
    .unwrap();
    assert_eq!(it.stage.chars[1].offset, (-140, 20));
}

#[test]
fn 默认含_left_center_right() {
    let it = interp();
    assert!(it.positions.contains_key("left"));
    assert!(it.positions.contains_key("center"));
    assert!(it.positions.contains_key("right"));
}

#[test]
fn charsway产生系统事件且偏移随时间衰减() {
    let mut it = interp();
    it.exec(Command::CharSway { layer: 0, count: 2, period_ms: 200 }, "t.ks:1").unwrap();
    // 解释器不直接改 Stage，事件交给引擎层消费
    assert!(matches!(it.events.as_slice(), [crate::interp::SysEvent::CharSway { layer: 0, .. }]));
    it.events.clear();
    // 引擎侧消费后开始摇晃，偏移非 0 且随时间结束
    it.stage.start_sway(0, 2, 200);
    assert_ne!(it.stage.sway_offset(0), 0);
    it.stage.tick(400.0);
    assert_eq!(it.stage.sway_offset(0), 0);
}

#[test]
fn 立绘切换使用注入的淡入时长() {
    let mut it = interp();
    it.char_fade_ms = 1200;
    it.exec(Command::Char { layer: 0, storage: "alice".into(), x: None, y: None }, "t.ks:1")
        .unwrap();
    assert_eq!(it.stage.chars[0].dur, 1200.0);
    it.exec(Command::Char { layer: 0, storage: "hide".into(), x: None, y: None }, "t.ks:2")
        .unwrap();
    assert_eq!(it.stage.chars[0].dur, 1200.0);
}

#[test]
fn 剧本错误不panic而是记录并可继续() {
    let mut it = interp();
    // 第一句是台词（等待点），错误发生在玩家点击推进之后
    it.scripts.insert(
        "bad.ks".into(),
        crate::lexer::parse_script("n 第一句\nnope 未知\nn 第二句\n").unwrap(),
    );
    it.start("bad.ks", None).unwrap();
    assert_eq!(it.state, crate::interp::RunState::WaitClick);
    // 推进到 nope：错误以 Err 返回并同时记录，调用方可选择忽略继续
    let _ = it.click();
    let e = it.error.clone().expect("应记录剧本错误");
    assert!(e.contains("未知指令「nope」"), "{e}");
    // 解释器只收敛到 Ended 并挂 error；L1 收到后转回 WaitClick 提示，不退出整局
    assert_eq!(it.state, crate::interp::RunState::Ended);
}

#[test]
fn 未知预设位置中文报错带行号() {
    let mut it = interp();
    let e = it
        .exec(
            Command::CharPos { layer: 1, storage: "hide".into(), pos_name: "nope 0".into() },
            "00.ks:7",
        )
        .unwrap_err();
    assert!(e.contains("00.ks:7") && e.contains("nope") && e.contains("预设位置"), "{e}");
}
