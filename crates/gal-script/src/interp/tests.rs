//! interp 单元测试

use super::{Interp, SysEvent};
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
    assert!(it.stage.char(1).chibi);
    it.exec(Command::Char { layer: 1, storage: "hide".into(), x: None, y: None }, "t.ks:2")
        .unwrap();
    assert!(!it.stage.char(1).chibi);
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
    assert_eq!(it.stage.char(1).offset, (-140, 20));
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
    assert_eq!(it.stage.char(0).dur, 1200.0);
    it.exec(Command::Char { layer: 0, storage: "hide".into(), x: None, y: None }, "t.ks:2")
        .unwrap();
    assert_eq!(it.stage.char(0).dur, 1200.0);
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
    let (blocks, elses, _) = crate::interp::blocks::pair(&it.scripts["if.ks"]).unwrap();
    it.blocks.insert("if.ks".into(), blocks);
    it.elsif_chains.insert("if.ks".into(), Default::default());
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
    let (blocks, elses, _) = crate::interp::blocks::pair(&it.scripts["if.ks"]).unwrap();
    it.blocks.insert("if.ks".into(), blocks);
    it.elsif_chains.insert("if.ks".into(), Default::default());
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
    let (blocks, elses, _) = crate::interp::blocks::pair(&it.scripts["if.ks"]).unwrap();
    it.blocks.insert("if.ks".into(), blocks);
    it.elsif_chains.insert("if.ks".into(), Default::default());
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
    let (blocks, elses, _) = crate::interp::blocks::pair(&it.scripts["one.ks"]).unwrap();
    it.blocks.insert("one.ks".into(), blocks);
    it.elsif_chains.insert("one.ks".into(), Default::default());
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
    let (blocks, elses, _) = crate::interp::blocks::pair(&it.scripts["call.ks"]).unwrap();
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
fn 自由层写进统一层表并可存档恢复() {
    let mut it = interp();
    it.exec(Command::Layer { n: 7, storage: "overlay".into(), fade_ms: 500 }, "t.ks:1").unwrap();
    let p7 = it.stage.layer(7).unwrap().cur.clone().expect("自由层已设置");
    assert!(p7.contains("overlay"), "{p7}");
    // 合成顺序：自由层排在 cg(4) 之后
    let order: Vec<usize> =
        it.stage.ordered_layers().filter(|(_, _, l)| l.visible()).map(|(n, ..)| n).collect();
    assert_eq!(order, vec![7]);

    let snap = it.stage.snapshot();
    assert_eq!(snap.layers, vec![(7, p7.clone())]);
    let mut back = interp();
    back.stage.restore(snap);
    assert_eq!(back.stage.layer(7).unwrap().cur.as_deref(), Some(p7.as_str()));

    // hide 走同一层表，不报错
    it.exec(Command::Layer { n: 7, storage: "hide".into(), fade_ms: 0 }, "t.ks:2").unwrap();
    assert!(it.stage.layer(7).unwrap().cur.is_none());
}

#[test]
fn 转场沿用目标层内容且按时长收敛() {
    let mut it = interp();
    it.stage.set_layer(0, Some("old/bg".into()), 0).unwrap();
    it.exec(
        Command::Trans { n: 0, storage: "bg/next".into(), dur_ms: 400, method: "crossfade".into() },
        "t.ks:1",
    )
    .unwrap();
    // 目标层内容立即切到新素材（存档语义 = 转场后状态）
    let cur = it.stage.bg().cur.clone().unwrap();
    assert!(cur.contains("bg/next"), "{cur}");
    let t = it.stage.trans.as_ref().expect("应有在途转场");
    assert_eq!(t.from.as_deref(), Some("old/bg"));
    assert_eq!(t.method, crate::stage::TransMethod::Crossfade);
    assert_eq!(t.alpha(), 0.0);
    // wt 等到转场结束
    it.exec(Command::WaitTrans, "t.ks:2").unwrap();
    assert!(matches!(it.state, crate::interp::RunState::WaitTimer { .. }));
    it.stage.tick(400.0);
    assert!(it.stage.trans.is_none());
}

#[test]
fn 转场hide淡出到空且不换图() {
    let mut it = interp();
    it.stage.set_layer(4, Some("cg/a".into()), 0).unwrap();
    it.exec(
        Command::Trans { n: 4, storage: "hide".into(), dur_ms: 300, method: "universal".into() },
        "t.ks:1",
    )
    .unwrap();
    assert!(it.stage.cg().cur.is_none());
    let t = it.stage.trans.as_ref().expect("应有淡出片段");
    assert_eq!(t.from.as_deref(), Some("cg/a"));
    assert!(t.to.is_empty());
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

// ── v0.3.4：elsif 链 / emb / eval（赋值表达式）/ trace ───────────────────

use crate::vars::Value;

/// 装载剧本 + 建块配对表（测试快捷方式）
fn load_with_blocks(it: &mut Interp, file: &str, src: &str) {
    it.scripts.insert(file.into(), crate::lexer::parse_script(src).unwrap());
    let (blocks, elses, chains) = crate::interp::blocks::pair(&it.scripts[file]).unwrap();
    it.blocks.insert(file.into(), blocks);
    it.elsif_chains.insert(file.into(), chains);
    it.else_targets.insert(file.into(), elses);
    it.labels.insert((file.into(), "start".into()), 0);
    it.start(file, None).unwrap();
}

#[test]
fn elsif链按序命中() {
    let mut it = interp();
    load_with_blocks(
        &mut it,
        "el.ks",
        "*start\nset f.a = 5\nif f.a > 100\nn 巨大\nelsif f.a > 3\nn 中等\nelsif f.a > 4\nn 中等2\nelse\nn 小\nendif\nn 之后\n",
    );
    assert_eq!(it.pending_text.as_deref(), Some("中等"));
    // 点击后走到下一个 elsif：已有命中分支 → 直接跳到 endif 之后
    it.click().unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("之后"));
}

#[test]
fn elsif全假落到else() {
    let mut it = interp();
    load_with_blocks(
        &mut it,
        "el2.ks",
        "*start\nset f.a = 1\nif f.a > 100\nn 甲\nelsif f.a > 3\nn 乙\nelse\nn 丙\nendif\nn 之后\n",
    );
    assert_eq!(it.pending_text.as_deref(), Some("丙"));
    it.click().unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("之后"));
}

#[test]
fn elsif无else全假直落endif() {
    let mut it = interp();
    load_with_blocks(
        &mut it,
        "el3.ks",
        "*start\nset f.a = 0\nif f.a > 1\nn 甲\nelsif f.a > 2\nn 乙\nendif\nn 落点\n",
    );
    assert_eq!(it.pending_text.as_deref(), Some("落点"));
}

#[test]
fn elsif链嵌套内外层互不干扰() {
    let mut it = interp();
    load_with_blocks(
        &mut it,
        "el4.ks",
        "*start\nset f.a = 1\nset f.b = 9\nif f.a > 0\nif f.b > 100\nn 内甲\nelsif f.b > 5\nn 内乙\nelse\nn 内丙\nendif\nn 外尾\nendif\nn 结束\n",
    );
    assert_eq!(it.pending_text.as_deref(), Some("内乙"));
    it.click().unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("外尾"));
}

#[test]
fn emb拼接表达式结果进台词() {
    let mut it = interp();
    it.vars.set("f.hp", Value::Int(42));
    it.exec(Command::Emb { expr: "'HP：' + f.hp".into() }, "t.ks:1").unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("HP：42"));
}

#[test]
fn eval赋值表达式直接量与复合() {
    let mut it = interp();
    it.exec(Command::Eval { expr: "f.a = 5".into() }, "t:1").unwrap();
    it.exec(Command::Eval { expr: "f.a += 3".into() }, "t:2").unwrap();
    it.exec(Command::Eval { expr: "f.s = '夏' + '天'".into() }, "t:3").unwrap();
    it.exec(Command::Eval { expr: "f.arr = [1, 2, 3]".into() }, "t:4").unwrap();
    it.exec(Command::Eval { expr: "f.arr[1] = 20".into() }, "t:5").unwrap();
    it.exec(Command::Eval { expr: "f.d = %[ name: '岚', hp: 9 ]".into() }, "t:6").unwrap();
    it.exec(Command::Eval { expr: "f.d.hp += 1".into() }, "t:7").unwrap();
    assert_eq!(it.vars.get("f.a"), Some(&Value::Int(8)));
    assert_eq!(it.vars.get("f.s"), Some(&Value::Str("夏天".into())));
    assert_eq!(
        it.vars.get("f.arr"),
        Some(&Value::Arr(vec![Value::Int(1), Value::Int(20), Value::Int(3)]))
    );
    assert_eq!(
        it.vars.get("f.d"),
        Some(&Value::Dict(vec![
            ("name".into(), Value::Str("岚".into())),
            ("hp".into(), Value::Int(10))
        ]))
    );
    // 纯表达式 eval 也允许（不求值结果丢弃）
    it.exec(Command::Eval { expr: "1 + 2".into() }, "t:8").unwrap();
    // 赋值目标缺失报错
    assert!(it.exec(Command::Eval { expr: "= 5".into() }, "t:9").is_err());
}

#[test]
fn trace求值内联且不支持时原样保留() {
    let mut it = interp();
    it.vars.set("f.n", Value::Int(7));
    // 正常路径不报错（日志落盘由 engine.log 承载，这里只验证求值不炸）
    it.exec(Command::Trace { text: "当前 n=%f.n，双倍=%f.n * 2".into() }, "t:1").unwrap();
    // 求值失败的表达式：不报错（trace 是诊断通道，不中断演出）
    it.exec(Command::Trace { text: "坏字段 %f.n ?? x".into() }, "t:2").unwrap();
}

// ── v0.3.5：宏系统（定义/展开/删除）+ ignore + clearvar/clearsysvar ────────

use crate::lexer::LineKind;
use crate::RunState;

/// 装载剧本（宏注册走真实 load 路径：写临时剧本文件不可行，这里手动建表）
fn load_full(it: &mut Interp, file: &str, src: &str) {
    let lines = crate::lexer::parse_script(src).unwrap();
    // 摘宏定义（与 Interp::load 同口径）
    let mut macros = Vec::new();
    let lines: Vec<_> = lines
        .into_iter()
        .filter_map(|l| match l.kind {
            LineKind::MacroDef { name, body } => {
                macros.push((name, body));
                None
            }
            _ => Some(l),
        })
        .collect();
    for (n, b) in macros {
        // 宏体块配对挂「macro:宏名」键（与 Interp::load 同口径）
        let key = format!("macro:{n}");
        let (mb, me, mc) = crate::interp::blocks::pair(&b).unwrap();
        it.blocks.insert(key.clone(), mb);
        it.elsif_chains.insert(key.clone(), mc);
        it.else_targets.insert(key, me);
        it.macros.insert(n, b);
    }
    it.scripts.insert(file.into(), lines.clone());
    let (blocks, elses, chains) = crate::interp::blocks::pair(&lines).unwrap();
    it.blocks.insert(file.into(), blocks);
    it.elsif_chains.insert(file.into(), chains);
    it.else_targets.insert(file.into(), elses);
    it.labels.insert((file.into(), "start".into()), 0);
    // 启动错误（如递归宏防护）不 unwrap：由用例自行断言 error/Ended
    let _ = it.start(file, None);
}

#[test]
fn v035_宏定义与调用展开() {
    let mut it = interp();
    load_full(
        &mut it,
        "m.ks",
        "*start\nmacro greet\neval f.greeted = f.greeted + 1\nn 问候 %who 次数 %n\nendmacro\neval f.greeted = 0\ngreet who=夏 n=2\ngreet\nn 结束\n",
    );
    // 第一次调用：属性代入（who=夏 n=2）
    assert_eq!(it.vars.get("f.greeted"), Some(&Value::Int(1)));
    assert_eq!(it.pending_text.as_deref(), Some("问候 夏 次数 2"));
    // 第二次调用：未传属性按空串代入（KAG3 缺省口径，不崩演出）
    it.click().unwrap();
    assert_eq!(it.vars.get("f.greeted"), Some(&Value::Int(2)));
    assert_eq!(it.pending_text.as_deref(), Some("问候  次数 "));
    it.click().unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("结束"));
}

#[test]
fn v035_宏属性代入与缺省() {
    let mut it = interp();
    load_full(
        &mut it,
        "m2.ks",
        "*start\nmacro hit\neval f.total = f.total + %dmg\nendmacro\neval f.total = 0\nhit dmg=7\nhit dmg=5\n",
    );
    // 7 + 5
    assert_eq!(it.vars.get("f.total"), Some(&Value::Int(12)));
}

#[test]
fn v035_宏内if块与嵌套宏调用() {
    let mut it = interp();
    load_full(
        &mut it,
        "m3.ks",
        "*start\nmacro inner\neval f.log = f.log + \"i\"\nendmacro\nmacro outer\ninner\ninner\nif %flag == 1\neval f.log = f.log + \"T\"\nelse\neval f.log = f.log + \"F\"\nendif\nendmacro\neval f.log = \"\"\nouter flag=0\nn 完成\n",
    );
    // 宏体内 if 块走 else 分支（flag=0）；嵌套宏调用 inner 执行两次

    assert_eq!(it.vars.get("f.log"), Some(&Value::Str("iiF".into())));
    assert_eq!(it.pending_text.as_deref(), Some("完成"));
}

#[test]
fn v035_递归宏防护() {
    let mut it = interp();
    load_full(&mut it, "m4.ks", "*start\nmacro loop_a\nloop_a\nn 不该到这\nendmacro\nloop_a\n");
    // 递归展开在深度上限处报错（不崩进程，error 留证 + Ended）
    assert!(it.error.is_some());
    assert_eq!(it.state, RunState::Ended);
}

#[test]
fn v035_erasemacro删除() {
    let mut it = interp();
    load_full(&mut it, "m5.ks", "*start\nmacro gone\nn 旧宏\nendmacro\nerasemacro gone\nn 结束\n");
    assert!(!it.macros.contains_key("gone"));
    assert_eq!(it.pending_text.as_deref(), Some("结束"));
}

#[test]
fn v035_ignore块与clearvar() {
    let mut it = interp();
    it.vars.set("f.keep", Value::Int(1));
    it.vars.set("sf.game", Value::Str("x".into()));
    load_full(
        &mut it,
        "m6.ks",
        "*start\nignore\nn 注释块里的台词（不执行）\neval f.keep = 99\nendignore\nclearvar\nn 清完\n",
    );
    // ignore 块内 eval 未执行；clearvar 清了 f.*（含脚本外的 keep）
    assert_eq!(it.vars.get("f.keep"), None);
    assert_eq!(it.pending_text.as_deref(), Some("清完"));
    // sf 不受 clearvar 影响
    assert_eq!(it.vars.get("sf.game"), Some(&Value::Str("x".into())));
}

#[test]
fn v035_clearsysvar清sf() {
    let mut it = interp();
    it.vars.set("sf.a", Value::Int(1));
    it.vars.set("sf.b", Value::Str("t".into()));
    it.vars.set("f.c", Value::Int(2));
    it.exec(Command::ClearSysVar, "t:1").unwrap();
    assert_eq!(it.vars.get("sf.a"), None);
    assert_eq!(it.vars.get("sf.b"), None);
    assert_eq!(it.vars.get("f.c"), Some(&Value::Int(2)));
}

// ── v0.3.6：fore/back 双页（backlay/copylay）+ move/wa/animstop ────────────

#[test]
fn v036_backlay与copylay经指令执行() {
    let mut it = interp();
    it.stage.set_layer(0, Some("bg/a".into()), 0).unwrap();
    it.exec(Command::BackLay { n: Some(0), hide: false }, "t:1").unwrap();
    assert_eq!(it.stage.layers[0].back.as_deref(), Some("bg/a"));
    it.stage.set_layer(4, Some("cg/k".into()), 0).unwrap();
    it.exec(Command::CopyLay { from: 4, to: 7, dur_ms: 0 }, "t:2").unwrap();
    assert_eq!(it.stage.layer(7).unwrap().cur.as_deref(), Some("cg/k"));
    // backlay hide 清空
    it.exec(Command::BackLay { n: Some(0), hide: true }, "t:3").unwrap();
    assert!(it.stage.layers[0].back.is_none());
}

#[test]
fn v036_move等待与animstop() {
    let mut it = interp();
    it.stage.set_layer(5, Some("free/a".into()), 0).unwrap();
    it.exec(Command::Move { n: 5, frames: vec![(0, 0, 0), (200, 40, 0)] }, "t:1").unwrap();
    // wa：等动画结束
    it.exec(Command::WaitMove, "t:2").unwrap();
    assert!(matches!(it.state, RunState::WaitTimer { .. }));
    it.stage.tick(200.0);
    assert_eq!(it.stage.layers[5].offset, (40, 0));
    // animstop 指定层：动画取消、offset 停在当前位置（无 t=0 帧 → 从当前位置起步）
    it.exec(Command::Move { n: 5, frames: vec![(500, 99, 99)] }, "t:3").unwrap();
    assert_eq!(it.stage.moves[0].frames[0], (0, 40, 0));
    it.exec(Command::AnimStop { n: Some(5) }, "t:4").unwrap();
    assert!(it.stage.moves.is_empty());
    assert_eq!(it.stage.layers[5].offset, (40, 0));
}

#[test]
fn v036_animstop无参取消全部() {
    let mut it = interp();
    it.stage.start_move(5, vec![(100, 5, 5)]).unwrap();
    it.stage.start_move(6, vec![(100, 6, 6)]).unwrap();
    it.exec(Command::AnimStop { n: None }, "t:1").unwrap();
    assert!(it.stage.moves.is_empty());
}

#[test]
fn v036_move无在途时wa直通() {
    let mut it = interp();
    // 无剧本可停在：wa 无在途动画时立即放行（不进 WaitTimer）
    it.exec(Command::WaitMove, "t:1").unwrap();
    assert!(!matches!(it.state, crate::interp::RunState::WaitTimer { .. }));
}

// ── v0.3.7：forelay / 无前缀 trans（双页原子转场）──────────────────────────

#[test]
fn v037_forelay指令翻上back页() {
    let mut it = interp();
    it.stage.set_layer(0, Some("bg/a".into()), 0).unwrap();
    it.exec(Command::BackLay { n: Some(0), hide: false }, "t:1").unwrap();
    it.stage.set_layer(0, Some("bg/b".into()), 0).unwrap();
    it.exec(Command::ForeLay { n: None, dur_ms: 400, method: "universal".into() }, "t:2").unwrap();
    assert_eq!(it.stage.layers[0].cur.as_deref(), Some("bg/a"));
    assert!(it.stage.layers[0].back.is_none());
    assert!(it.stage.trans.is_some());
    // wt 可等在途转场
    it.exec(Command::WaitTrans, "t:3").unwrap();
    assert!(matches!(it.state, RunState::WaitTimer { .. }));
}

#[test]
fn v037_无前缀trans翻上back页() {
    let mut it = interp();
    it.stage.set_layer(0, Some("bg/a".into()), 0).unwrap();
    it.exec(Command::BackLay { n: Some(0), hide: false }, "t:1").unwrap();
    it.stage.set_layer(0, Some("bg/c".into()), 0).unwrap();
    // 无前缀形态：trans 500 crossfade
    it.exec(Command::ForeLay { n: None, dur_ms: 500, method: "crossfade".into() }, "t:2").unwrap();
    assert_eq!(it.stage.layers[0].cur.as_deref(), Some("bg/a"));
    assert_eq!(it.stage.trans.as_ref().unwrap().method, crate::stage::TransMethod::Crossfade);
}

#[test]
fn v037_transon关闭时forelay直切() {
    let mut it = interp();
    it.trans_enabled = false;
    it.stage.set_layer(0, Some("bg/a".into()), 0).unwrap();
    it.exec(Command::BackLay { n: Some(0), hide: false }, "t:1").unwrap();
    it.exec(Command::ForeLay { n: None, dur_ms: 500, method: "universal".into() }, "t:2").unwrap();
    assert_eq!(it.stage.layers[0].cur.as_deref(), Some("bg/a"));
    // 直切：转场片段时长为 0（与 Trans 指令同构，下一帧即完成）
    assert_eq!(it.stage.trans.as_ref().map(|t| t.dur_ms), Some(0.0));
}

#[test]
fn v038视频标签组_推系统事件() {
    let mut it = interp();
    it.exec(Command::Video { params: "mode=1".into() }, "t:1").unwrap();
    it.exec(Command::PlayVideo { name: "op".into(), ms: 2000, skip: true }, "t:2").unwrap();
    it.exec(Command::StopVideo, "t:3").unwrap();
    it.exec(Command::OpenVideo { name: "ed".into() }, "t:4").unwrap();
    it.exec(Command::PauseVideo, "t:5").unwrap();
    it.exec(Command::ResumeVideo, "t:6").unwrap();
    it.exec(Command::PrepareVideo { name: "ed".into() }, "t:7").unwrap();
    it.exec(Command::RewindVideo, "t:8").unwrap();
    assert!(matches!(
        it.events.as_slice(),
        [
            crate::interp::SysEvent::VideoSet { params },
            crate::interp::SysEvent::PlayVideo { name, ms: 2000, skip: true },
            crate::interp::SysEvent::StopVideo,
            crate::interp::SysEvent::OpenVideo { name: n2 },
            crate::interp::SysEvent::PauseVideo,
            crate::interp::SysEvent::ResumeVideo,
            crate::interp::SysEvent::PrepareVideo { name: n3 },
            crate::interp::SysEvent::RewindVideo,
        ] if params == "mode=1" && name == "op" && n2 == "ed" && n3 == "ed"
    ));
}

#[test]
fn v038_wv等待视频播完() {
    let mut it = interp();
    it.exec(Command::WaitVideo, "t:1").unwrap();
    assert!(matches!(it.state, RunState::WaitAudio(crate::interp::AudioWait::VideoDone)));
}

#[test]
fn v039_layopt与stoptrans执行() {
    let mut it = interp();
    it.stage.set_layer(1, Some("char/a".into()), 0).unwrap();
    // layopt 隐藏：visible() 为假（渲染层据此跳过）
    it.exec(Command::LayOpt { n: 1, visible: Some(false), opacity: None }, "t:1").unwrap();
    assert!(!it.stage.layer(1).unwrap().visible());
    // 不透明度落层
    it.exec(Command::LayOpt { n: 1, visible: Some(true), opacity: Some(128) }, "t:2").unwrap();
    assert!(it.stage.layer(1).unwrap().visible());
    assert_eq!(it.stage.layer(1).unwrap().opacity, 128);
    // stoptrans：有在途转场 → 丢弃并停在目标态；无转场 → 静默
    it.stage.set_layer(0, Some("bg/a".into()), 0).unwrap();
    it.stage.start_trans(0, "bg/b".into(), 5000, crate::stage::TransMethod::Crossfade);
    it.exec(Command::StopTrans, "t:3").unwrap();
    assert!(it.stage.trans.is_none());
    assert_eq!(it.stage.layers[0].cur.as_deref(), Some("bg/b"));
    it.exec(Command::StopTrans, "t:4").unwrap();
}

#[test]
fn v039_move缓动尾参执行() {
    let mut it = interp();
    it.exec(Command::MoveEase { n: 5, frames: vec![(500, 100, 0)], ease: "easein".into() }, "t:1")
        .unwrap();
    let m = it.stage.moves.first().unwrap();
    assert_eq!(m.ease, crate::stage::Ease::EaseIn);
    // 缓动映射：easein 在 k=0.5 时进度 0.25 → x=25
    assert!((m.offset_at(0.5).0 as f32 - 25.0).abs() < 1.0);
    // linear 对照：k=0.5 → 50
    it.stage.stop_move(5);
    it.exec(Command::Move { n: 5, frames: vec![(500, 100, 0)] }, "t:2").unwrap();
    let m = it.stage.moves.first().unwrap();
    assert_eq!(m.ease, crate::stage::Ease::Linear);
    assert_eq!(m.offset_at(0.5).0, 50);
}

#[test]
fn v0310_animstart执行与等待() {
    let mut it = interp();
    // 合法档（内联文本走 stage 直接口径；文件读取在无头回归 animstart-demo 覆盖）
    let _ = it.exec(Command::AnimStart { n: 5, name: "swing".into() }, "t:1");
    // data 目录无 asd/swing.asd → 中文报错（不静默崩）
    let err = it.exec(Command::AnimStart { n: 5, name: "nosuch".into() }, "t:2").unwrap_err();
    assert!(err.contains("animstart"));
    // 直接走 stage：start_anim 后 wa 等待口径
    it.stage.start_anim(5, vec![(0, 0, 0), (300, 30, 0)], vec![]).unwrap();
    assert_eq!(it.stage.move_left_ms(), Some(300));
    it.stage.tick(300.0);
    assert!(it.stage.anims.is_empty());
    assert_eq!(it.stage.layers[5].offset, (30, 0));
}

#[test]
fn v0311_quake族与wc_resetwait执行() {
    let mut it = interp();
    // quake → Quake 事件
    it.exec(Command::Quake { ms: 500, max_amp: 20 }, "t:1").unwrap();
    assert!(matches!(it.events.as_slice(), [SysEvent::Quake { ms: 500, max_amp: 20 }]));
    // wq → 等 QuakeDone（引擎按 shake 态放行）
    it.exec(Command::WaitQuake, "t:2").unwrap();
    assert!(matches!(it.state, RunState::WaitAudio(crate::interp::AudioWait::QuakeDone)));
    // stopquake → StopQuake 事件
    it.events.clear();
    it.exec(Command::StopQuake, "t:3").unwrap();
    assert!(matches!(it.events.as_slice(), [SysEvent::StopQuake]));
    // wc → 进入点击等待态
    it.exec(Command::WaitClickCount, "t:4").unwrap();
    assert_eq!(it.state, RunState::WaitClick);
    // resetwait：无等待态时静默
    it.exec(Command::ResetWait, "t:5").unwrap();
    assert_eq!(it.state, RunState::WaitClick);
    // locksnapshot / unlocksnapshot
    it.exec(Command::LockSnapshot { name: "封印".into() }, "t:6").unwrap();
    assert_eq!(it.snapshot_lock.as_deref(), Some("封印"));
    it.exec(Command::UnlockSnapshot, "t:7").unwrap();
    assert!(it.snapshot_lock.is_none());
}

#[test]
fn v0311_resetwait取消强制等待() {
    let mut it = interp();
    it.scripts.insert("rw.ks".into(), crate::lexer::parse_script("wait 5000\nn 之后\n").unwrap());
    it.start("rw.ks", None).unwrap();
    assert!(matches!(it.state, RunState::WaitTimer { .. }));
    assert!(matches!(it.state, RunState::WaitTimer { .. }));
    it.exec(Command::ResetWait, "t:2").unwrap();
    // 强制等待被取消，解释器继续推进到下一个等待点（台词行挂起）
    assert_eq!(it.state, RunState::WaitClick);
    assert_eq!(it.pending_text.as_deref(), Some("之后"));
}

#[test]
fn v0312_存档控制标签执行() {
    let mut it = interp();
    // s → StopSkip 事件
    it.exec(Command::StopSkip, "t:1").unwrap();
    assert!(matches!(it.events.as_slice(), [SysEvent::StopSkip]));
    // disablestore 翻转
    it.exec(Command::DisableStore { on: true }, "t:2").unwrap();
    assert!(it.store_disabled);
    it.exec(Command::DisableStore { on: false }, "t:3").unwrap();
    assert!(!it.store_disabled);
    // tempsave/tempload/copybookmark/erasebookmark/record/goback → 对应事件
    it.events.clear();
    it.exec(Command::TempSave, "t:4").unwrap();
    assert!(matches!(it.events.as_slice(), [SysEvent::TempSave]));
    it.events.clear();
    it.exec(Command::TempLoad, "t:5").unwrap();
    assert!(matches!(it.events.as_slice(), [SysEvent::TempLoad]));
    it.events.clear();
    it.exec(Command::CopyBookmark { from: 1, to: 3 }, "t:6").unwrap();
    assert!(matches!(it.events.as_slice(), [SysEvent::CopyBookmark { from: 1, to: 3 }]));
    it.events.clear();
    it.exec(Command::EraseBookmark { slot: 2 }, "t:7").unwrap();
    assert!(matches!(it.events.as_slice(), [SysEvent::EraseBookmark { slot: 2 }]));
    it.events.clear();
    it.exec(Command::Record, "t:8").unwrap();
    assert!(matches!(it.events.as_slice(), [SysEvent::OpenGallery]));
    it.events.clear();
    it.exec(Command::GoBack, "t:9").unwrap();
    assert!(matches!(it.events.as_slice(), [SysEvent::GoBack]));
}

#[test]
fn v0312_waittrig等待与放行() {
    let mut it = interp();
    it.scripts.insert("wt.ks".into(), crate::lexer::parse_script("n 结束\n").unwrap());
    it.start("wt.ks", None).unwrap();
    // 未置位 → 进入 Trig 等待
    it.exec(Command::WaitTrig { name: "open2".into() }, "t:1").unwrap();
    assert!(
        matches!(it.state, RunState::WaitAudio(crate::interp::AudioWait::Trig(ref k)) if k == "trig_open2")
    );
    // 已置位 → 引擎轮询放行（truthy 为真即 audio_wait_release），推进到台词行
    it.vars.set("f.trig_open2", crate::vars::Value::Int(1));
    it.audio_wait_release().unwrap();
    assert_eq!(it.state, RunState::Ended);
    // 台词已被消费（剧本走完）
    // 已置位再调 → 立即通过不等待（状态不再变化，无二次挂起）
    it.exec(Command::WaitTrig { name: "open2".into() }, "t:2").unwrap();
    assert_eq!(it.state, RunState::Ended);
}

#[test]
fn v0313_graph与ruby拼进当前台词() {
    let mut it = interp();
    // graph：\u{b}素材名\u{b} 占位符
    it.pending_text = Some("看她".into());
    it.exec(Command::Graph { storage: "face_a".into() }, "t:1").unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("看她\u{b}face_a\u{b}"));
    // 无台词时 graph 拼在已有 pending 后（同一行继续追加，emb 同款语义）
    it.exec(Command::Graph { storage: "solo".into() }, "t:2").unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("看她\u{b}face_a\u{b}\u{b}solo\u{b}"));
    // ruby：转成 [基底]^(注音) 文法
    it.pending_text = Some("名字是".into());
    it.exec(Command::RubyTag { text: "夏".into(), alt: "なつ".into() }, "t:3").unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("名字是[夏]^(なつ)"));
    // hch：按普通文本拼进（简化口径）
    it.pending_text = Some("他说".into());
    it.exec(Command::Hch { text: "重要".into() }, "t:4").unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("他说重要"));
    // font/style 族：无输出无状态（演出零变化）
    it.exec(Command::Font { size: Some(24), color: Some("0xFF0000".into()) }, "t:5").unwrap();
    it.exec(Command::Resetfont, "t:6").unwrap();
    it.exec(Command::Style { align: Some("center".into()) }, "t:7").unwrap();
    it.exec(Command::Resetstyle, "t:8").unwrap();
    assert_eq!(it.pending_text.as_deref(), Some("他说重要"));
}
