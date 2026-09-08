//! 存读档端到端测试（SDL dummy 驱动，无窗口）：缩略图/指纹/演出层快照/变量/BGM 恢复。

use super::saves::{do_load, do_save};
use crate::systems::Game;
use gal_config as config;
use gal_render::renderer::Renderer;
use gal_script::interp::{Interp, RunState};
use gal_script::vars::{Value, Vars};
use gal_ui::savemenu::ThumbCache;

#[test]
fn 存读档端到端() {
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    // 副本数据 + 带 bgm 的剧本：BGM 随档恢复验证（不碰正式 testdata）
    let data = std::env::temp_dir().join("galengine_e2e_data");
    let _ = std::fs::remove_dir_all(&data);
    copy_dir("testdata/game/data", &data);
    std::fs::write(
        data.join("scenario/bgmtest.ks"),
        "bg bg_black 100\nbgm theme_test\nn BGM 场景测试。\nn 第二句。\nend\n",
    )
    .unwrap();
    std::env::set_var("ES_DATA_DIR", data.to_str().unwrap());

    let sdl = sdl2::init().unwrap();
    let video = sdl.video().unwrap();
    let window = video.window("t", 1280, 720).build().unwrap();
    let mut canvas = window.into_canvas().software().build().unwrap();
    let creator = canvas.texture_creator();
    let mut renderer = Renderer::new(&creator).unwrap();
    let mut thumbs = ThumbCache::new(&creator);

    let conf = config::load();
    let mut interp =
        Interp::new(Vars::default(), 30.0, &conf.game.hero_default, &conf.game.you_default);
    interp.start("bgmtest.ks", None).unwrap();
    // 首句等待点即可存档（bgm 已随场景指令生效）
    assert_eq!(interp.state, RunState::WaitClick);
    assert_eq!(interp.cur_bgm.as_deref(), Some("theme_test"));
    interp.vars.set("f.probe", Value::Int(7));
    let mut g = Game::new(conf, interp);

    let dir = std::env::temp_dir().join("galengine_e2e_save");
    let _ = std::fs::remove_dir_all(&dir);
    g.sys.save_dir = dir.to_str().unwrap().to_string();

    renderer.compose(&mut canvas, 0, 0, |_| Ok(())).unwrap();
    do_save(&mut g, &mut renderer, &mut canvas, &mut thumbs, 1).unwrap();
    let saved = gal_save::slots::load(&g.sys.save_dir, 1).expect("do_save 后应有档");
    assert_eq!(saved.bgm.as_deref(), Some("theme_test"));

    // 推进烧完剧本（dummy 环境无 apply_pending，点击即快进）+ 污染 f.*
    for _ in 0..8 {
        let _ = g.interp.click();
    }
    assert_eq!(g.interp.state, RunState::Ended);
    g.interp.vars.set("f.probe", Value::Int(99));
    g.interp.cur_bgm = None;
    do_load(&mut g, 1);
    assert_eq!(g.interp.vars.get_or("f.probe"), Value::Int(7));
    assert_eq!(g.interp.state, RunState::WaitClick);
    // BGM 随档恢复（dummy 声卡静音降级，但状态必须回位）
    assert_eq!(g.interp.cur_bgm.as_deref(), Some("theme_test"));

    // 缩略图缓存构建 + meta 不明档
    g.sys.meta.fake_saves.push(gal_save::meta::FakeSave {
        date: "7月7日".into(),
        time: "23:59".into(),
        image: "bgimage/bg_black.jpg".into(),
    });
    g.refresh_saves();
    thumbs.build(&g.save_entries, &g.sys.meta.fake_saves);
    assert!(thumbs.get("s1").is_some());
    assert!(thumbs.get("f0").is_some());
}

/// 递归拷贝目录（测试用，避免引依赖）
fn copy_dir(src: &str, dst: &std::path::Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap().flatten() {
        let from = e.path();
        let to = dst.join(e.file_name());
        if from.is_dir() {
            copy_dir(from.to_str().unwrap(), &to);
        } else {
            std::fs::copy(from, to).unwrap();
        }
    }
}
