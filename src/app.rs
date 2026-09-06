//! L1 主循环：初始化 → 事件泵 → 输入路由 → 解释器推进 → 系统事件 → 渲染。
//! 保持薄：具体逻辑在 input/render/systems。

use std::env;
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::pixels::Color;

use crate::config::{self, Config};
use crate::gfx::assets::{self, TextureBank};
use crate::gfx::renderer::Renderer;
use crate::script::interp::{Interp, RunState};
use crate::script::vars::{Value, Vars};
use crate::systems::Game;
use crate::text::font::FontBook;

// rust-sdl2 未暴露的 SDL API（键盘文本输入，IME 支持）
extern "C" {
    fn SDL_StartTextInput();
    fn SDL_StopTextInput();
}

const FRAME_BUDGET_MS: u64 = 16;

pub fn run() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let _image = sdl2::image::init(sdl2::image::InitFlag::PNG | sdl2::image::InitFlag::JPG)?;
    let ttf = sdl2::ttf::init().map_err(|e| format!("字体系统初始化失败：{e}"))?;

    let conf = config::load();
    let mut canvas = make_canvas(&video, &conf.title)?;
    let creator = canvas.texture_creator();
    let mut renderer = Renderer::new(&creator)?;
    let mut bank = TextureBank::new(&creator);
    let mut fonts = FontBook::new(&ttf, &creator, &conf.fonts)?;

    // 全局变量（sf.*）加载 + 首次启动种子默认名
    let global_path = format!("{}/global.json", conf.save_dir);
    let mut vars = Vars::load_global(&global_path);
    if vars.sf.get("playerName").is_none() {
        vars.sf.insert("playerName".into(), Value::Str(conf.game.you_default.clone()));
    }

    let script = env::var("ES_SCRIPT").unwrap_or_else(|_| conf.script_start.clone());
    let label = env::var("ES_START_LABEL").ok();
    let mut interp = Interp::new(vars, conf.typewriter_ms, &conf.game.hero_default, &conf.game.you_default);
    interp.start(&script, label.as_deref())?;

    let mut g = Game::new(conf, interp);
    let autoclick_ms: f32 = env::var("ES_DEBUG_AUTOCLICK_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let mut click_acc = 0.0f32;

    // 场景级资源管理：背景变更 = 切场景（回收图片/文字纹理）
    let mut last_bg: Option<String> = None;

    let mut events = sdl.event_pump()?;
    let budget = Duration::from_millis(FRAME_BUDGET_MS);
    let mut last = Instant::now();

    'running: loop {
        let t0 = Instant::now();
        let dt = t0.duration_since(last).as_secs_f32() * 1000.0;
        last = t0;

        // 输入态生命周期（进入/离开 WaitInput）
        maintain_input_ui(&mut g);
        for ev in events.poll_iter() {
            if !crate::input::dispatch(&mut g, ev, &mut canvas, &mut fonts)? {
                break 'running;
            }
        }

        // 快进（按住 Ctrl；wait/meta 段不可跳——仅台词态生效）
        if g.ctrl_hold && g.interp.state == RunState::WaitClick {
            g.interp.click()?;
        }
        // 自动模式：行完延迟推进
        if g.auto && g.interp.state == RunState::WaitClick && g.interp.tw.line_finished() {
            g.auto_acc += dt;
            if g.auto_acc >= g.conf.auto_delay_ms {
                g.auto_acc = 0.0;
                g.interp.click()?;
            }
        } else {
            g.auto_acc = 0.0;
        }
        // 验收自动驱动
        if autoclick_ms > 0.0 {
            click_acc += dt;
            if click_acc >= autoclick_ms {
                click_acc = 0.0;
                crate::input::auto_step(&mut g)?;
            }
        }

        g.tick(dt);
        g.interp.tick(dt);
        g.interp.apply_pending(&mut fonts, &g.style);

        // 文字速度可调（sf.textSpeed 覆盖配置）
        if let Some(Value::Int(v)) = g.interp.vars.sf.get("textSpeed") {
            if *v > 0 {
                g.interp.tw.interval_ms = *v as f32;
            }
        }

        // 缓存层收货 + 预读喂料 + 场景切换回收
        bank.prefetch.pump();
        for (kind, storage) in g.interp.lookahead_storages(40) {
            bank.prefetch.request(&assets::resolve(kind, &storage));
        }
        let cur_bg = g.interp.stage.bg.cur().cloned();
        if cur_bg != last_bg {
            last_bg = cur_bg;
            let mut keep = std::collections::HashSet::new();
            g.interp.stage.collect_paths(&mut keep);
            if let Some(r) = &g.sys.reach {
                keep.insert(r.path.clone());
            }
            bank.retain(&keep);
            fonts.clear_text_cache();
            if crate::script::interp::dbg() {
                eprintln!("[dbg] 场景切换：保留 {} 张图，预解码 {} 张", keep.len(), bank.prefetch.stats_done);
            }
        }

        g.drain_events(&mut canvas)?;

        if g.interp.state == RunState::Ended {
            if crate::script::interp::dbg() {
                eprintln!("[dbg t={:.2}] ended", g.now_ms);
            }
            break 'running;
        }

        crate::render::frame(&mut g, &mut fonts, &mut bank, &mut renderer, &mut canvas)?;

        let spent = t0.elapsed();
        if spent < budget {
            std::thread::sleep(budget - spent);
        }
    }

    g.interp.vars.save_global(&global_path)?;
    Ok(())
}

/// 输入态进入/离开：建 UI + 开/关 IME 文本输入
fn maintain_input_ui(g: &mut Game) {
    match (&g.interp.state, g.input_ui.is_some()) {
        (RunState::WaitInput(spec), false) => {
            let presets = crate::ui::inputbox::presets_for(&g.conf, &spec.var);
            g.input_ui = Some(crate::ui::inputbox::InputUi::new((**spec).clone(), presets));
            unsafe { SDL_StartTextInput() };
        }
        (RunState::WaitInput(_), true) => {}
        _ => {
            if g.input_ui.take().is_some() {
                unsafe { SDL_StopTextInput() };
            }
        }
    }
}

/// 可自由缩放窗口；首选加速+vsync，无 GPU（xvfb 验收）回退软件渲染
fn make_canvas(
    video: &sdl2::VideoSubsystem,
    title: &str,
) -> Result<sdl2::render::Canvas<sdl2::video::Window>, String> {
    let build = || {
        video
            .window(title, config::LOGICAL_W, config::LOGICAL_H)
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())
    };
    match build()?.into_canvas().accelerated().present_vsync().build() {
        Ok(c) => Ok(c),
        Err(_) => build()?.into_canvas().software().build().map_err(|e| e.to_string()),
    }
}
