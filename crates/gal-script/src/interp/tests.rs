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
fn if块与else() {
    let mut it = interp();
    it.scripts.insert(
        "if.ks".into(),
        crate::lexer::parse_script(
            "*start\nset f.a = 2\nif f.a > 5\nn 大\nelse\nn 小\nendif\nn 之后\n",
        )
        .unwrap(),
    );
    // 块配对表由 load 建立；测试直接插入脚本时手动建立
    let (blocks, elses) = crate::interp::blocks::pair(&it.scripts["if.ks"]).unwrap();
    it.blocks.insert("if.ks".into(), blocks);
    it.else_targets.insert("if.ks".into(), elses);
    it.labels.insert(("if.ks".into(), "start".into()), 0);
    it.start("if.ks", None).unwrap();
    assert_eq!(it.state, crate::interp::RunState::WaitClick);
    assert_eq!(it.pending_text.as_deref(), Some("小"));
    it.click().unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("之后"));
}

#[test]
fn if块无else且条件成立() {
    let mut it = interp();
    it.scripts.insert(
        "if.ks".into(),
        crate::lexer::parse_script("*start\nset f.a = 9\nif f.a == 9\nn 命中\nendif\nn 之后\n")
            .unwrap(),
    );
    let (blocks, elses) = crate::interp::blocks::pair(&it.scripts["if.ks"]).unwrap();
    it.blocks.insert("if.ks".into(), blocks);
    it.else_targets.insert("if.ks".into(), elses);
    it.labels.insert(("if.ks".into(), "start".into()), 0);
    it.start("if.ks", None).unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("命中"));
    it.click().unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("之后"));
}

#[test]
fn if块嵌套() {
    let mut it = interp();
    it.scripts.insert(
        "if.ks".into(),
        crate::lexer::parse_script(
            "*start\nset f.a = 1\nset f.b = 0\nif f.a == 1\nif f.b == 1\nn 内真\nelse\nn 内假\nendif\nn 外真\nelse\nn 外假\nendif\nn 之后\n",
        )
        .unwrap(),
    );
    let (blocks, elses) = crate::interp::blocks::pair(&it.scripts["if.ks"]).unwrap();
    it.blocks.insert("if.ks".into(), blocks);
    it.else_targets.insert("if.ks".into(), elses);
    it.labels.insert(("if.ks".into(), "start".into()), 0);
    it.start("if.ks", None).unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("内假"));
    it.click().unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("外真"));
    it.click().unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("之后"));
}

#[test]
fn if单行表达式条件() {
    let mut it = interp();
    it.scripts.insert(
        "one.ks".into(),
        crate::lexer::parse_script("*start\nset f.a = 3\nif f.a * 2 > 5 *hit\nn 未命中\njump *end\n*hit\nn 命中\n*end\nn 之后\n")
            .unwrap(),
    );
    let (blocks, elses) = crate::interp::blocks::pair(&it.scripts["one.ks"]).unwrap();
    it.blocks.insert("one.ks".into(), blocks);
    it.else_targets.insert("one.ks".into(), elses);
    it.labels.insert(("one.ks".into(), "start".into()), 0);
    it.labels.insert(("one.ks".into(), "hit".into()), 5);
    it.labels.insert(("one.ks".into(), "end".into()), 7);
    it.start("one.ks", None).unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("命中"));
}

#[test]
fn call与return回到下一行() {
    let mut it = interp();
    it.scripts.insert(
        "call.ks".into(),
        crate::lexer::parse_script(
            "*start\nset f.a = 0\ncall *sub\nn 返回后\nend\n*sub\nflag f.a +1\nreturn\n",
        )
        .unwrap(),
    );
    let (blocks, elses) = crate::interp::blocks::pair(&it.scripts["call.ks"]).unwrap();
    it.blocks.insert("call.ks".into(), blocks);
    it.else_targets.insert("call.ks".into(), elses);
    it.labels.insert(("call.ks".into(), "start".into()), 0);
    it.labels.insert(("call.ks".into(), "sub".into()), 5);
    it.start("call.ks", None).unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("返回后"));
    assert_eq!(it.vars.get("f.a"), Some(&crate::vars::Value::Int(1)));
    it.click().unwrap();
    assert_eq!(it.state, crate::interp::RunState::Ended);
}

#[test]
fn return无call与call递归上限() {
    let mut it = interp();
    it.scripts.insert("bad.ks".into(), crate::lexer::parse_script("*start\nreturn\n").unwrap());
    it.labels.insert(("bad.ks".into(), "start".into()), 0);
    it.start("bad.ks", None).unwrap_err();
    assert!(it.error.as_deref().unwrap_or("").contains("return 没有对应的 call"));

    let mut it = interp();
    it.scripts.insert(
        "rec.ks".into(),
        crate::lexer::parse_script("*start\ncall *start\nreturn\n").unwrap(),
    );
    it.labels.insert(("rec.ks".into(), "start".into()), 0);
    let _ = it.start("rec.ks", None);
    let e = it.error.clone().unwrap_or_default();
    assert!(e.contains("call 嵌套超过"), "{e}");
}

#[test]
fn if块配对错误带行号() {
    let mut it = interp();
    it.scripts.insert("bad.ks".into(), crate::lexer::parse_script("if f.a == 1\nn x\n").unwrap());
    let e = crate::interp::blocks::pair(&it.scripts["bad.ks"]).unwrap_err();
    assert!(e.contains("缺少 endif"), "{e}");
    let e =
        crate::interp::blocks::pair(&crate::lexer::parse_script("else\n").unwrap()).unwrap_err();
    assert!(e.contains("没有配对的 if"), "{e}");
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
