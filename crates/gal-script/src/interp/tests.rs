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
