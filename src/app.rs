//! L1 应用层：主循环 + 系统键/覆盖层路由（游戏输入见 input.rs）。

use std::env;
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;

use crate::config::{self, Config};
use crate::gfx::assets::{self, TextureBank};
use crate::gfx::renderer::Renderer;
use crate::input::InputRouter;
use crate::platform;
use crate::script::interp::{Interp, RunState};
use crate::script::vars::{Value, Vars};
use crate::systems::Systems;
use crate::text::font::FontBook;
use crate::ui::dialog::{self, DialogStyle};
use crate::ui::overlay::{Action, Ctx, Overlay, OverlaySys};

const GLOBAL_SAVE: &str = "savedata/global.json";

pub fn run() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let _image = sdl2::image::init(sdl2::image::InitFlag::PNG | sdl2::image::InitFlag::JPG)?;
    let ttf = sdl2::ttf::init().map_err(|e| format!("字体系统初始化失败：{e}"))?;

    let conf = Config::load();
    let mut canvas = make_canvas(&video, &conf.title)?;
    let texture_creator = canvas.texture_creator();
    let mut renderer = Renderer::new(&texture_creator)?;
    let mut bank = TextureBank::new(&texture_creator);
    let mut fonts = FontBook::new(&ttf, &texture_creator, &conf.fonts)?;
    let style = DialogStyle::from_cfg(&conf.dialog, conf.game.name_colors.clone());

    let mut interp = Interp::new(Vars::load_global(GLOBAL_SAVE), conf.typewriter_ms);
    interp.name_defaults = (conf.game.hero_default.clone(), conf.game.you_default.clone());
    let mut systems = Systems::new(&conf);
    let mut overlay = OverlaySys::default();
    let (title_main, title_sub) = split_title(&conf.title);
    let presets = (
        conf.game.input_presets.get("hero").cloned().unwrap_or_default(),
        conf.game.input_presets.get("player").cloned().unwrap_or_default(),
    );
    let mut ir = InputRouter::new(presets);
    let mut started = false;
    let mut last_bg: Option<String> = None;
    let mut msg: Option<(String, f32)> = None;

    let autoclick_ms: f32 = env::var("ES_DEBUG_AUTOCLICK_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let mut click_acc = 0.0f32;

    let mut events = sdl.event_pump()?;
    let budget = Duration::from_millis(config::FRAME_BUDGET_MS);
    let mut last = Instant::now();
    let mut now_ms = 0.0f32;

    'running: loop {
        let t0 = Instant::now();
        let dt = t0.duration_since(last).as_secs_f32() * 1000.0;
        last = t0;
        now_ms += dt;
        if let Some((_, left)) = msg.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                msg = None;
            }
        }

        ir.maintain_input(&interp);
        let ctx = build_ctx(&interp, &mut systems, &conf, &title_main, &title_sub);

        // ================= 事件路由 =================
        for ev in events.poll_iter() {
            match system_route(&ev, &mut overlay, &mut systems, &mut interp, &mut canvas, &mut ir, &ctx, &conf, &mut started, &mut msg, started_ok(&interp, started)) {
                Ok(RouteResult::Handled) => {}
                Ok(RouteResult::Pass) => {
                    // 游戏态输入
                    if !overlay.active() {
                        ir.on_event(&ev, &mut interp, &canvas)?;
                    }
                }
                Ok(RouteResult::Quit) => break 'running,
                Err(e) => return Err(e),
            }
        }

        let act = overlay.tick(dt, &ctx);
        if matches!(act, Action::Quit) {
            break 'running;
        }
        apply_action(act, &mut interp, &mut overlay, &mut systems, &mut canvas, &conf, &mut started, &mut msg);

        if !interp.events.is_empty() {
            let evs = std::mem::take(&mut interp.events);
            systems.handle_events(&evs, &mut overlay, &mut canvas, &conf);
        }
        systems.tick(dt, &mut canvas);

        // 游戏推进
        if started && !overlay.active() {
            ir.tick(&mut interp, conf.auto_delay_ms, dt)?;
            interp.tick(dt);
            interp.apply_pending(&mut fonts, &style);
        }

        // 验收自动驱动（标题/仪式/游戏三态）
        if autoclick_ms > 0.0 {
            click_acc += dt;
            if click_acc >= autoclick_ms {
                click_acc = 0.0;
                let act = autoclick_next(&mut overlay, &mut ir, &mut interp, started, &ctx);
                apply_action(act, &mut interp, &mut overlay, &mut systems, &mut canvas, &conf, &mut started, &mut msg);
            }
        }

        // 场景级资源管理
        bank.prefetch.pump();
        if started {
            for (kind, storage) in interp.lookahead_storages(40) {
                bank.prefetch.request(&assets::resolve(kind, &storage));
            }
            let cur_bg = interp.stage.bg.cur().cloned();
            if cur_bg != last_bg {
                last_bg = cur_bg;
                let mut keep = std::collections::HashSet::new();
                interp.stage.collect_paths(&mut keep);
                bank.retain(&keep);
                fonts.clear_text_cache();
            }
        }

        // 剧本结束：验收直接退；正常回标题
        if interp.state == RunState::Ended
            && started
            && !matches!(overlay.cur, Overlay::Rest { .. })
        {
            if crate::script::interp::dbg() {
                eprintln!("[dbg] ended");
                break 'running;
            }
            started = false;
            overlay.cur = Overlay::Title { sel: 0 };
            systems.reach = None;
            last_bg = None;
        }

        // ================= 渲染 =================
        crate::render::frame(
            &mut renderer, &mut canvas, &mut bank, &mut fonts, &style, &interp, &ir,
            &overlay, &ctx, &mut systems, started, now_ms, msg.as_ref().map(|m| m.0.as_str()),
        )?;
        let spent = t0.elapsed();
        if spent < budget {
            std::thread::sleep(budget - spent);
        }
    }

    interp.vars.save_global(GLOBAL_SAVE)?;
    systems.meta.save(&conf.save_dir)?;
    Ok(())
}

/// 系统级事件（Quit/Esc/Ctrl/F键/Overlay）——被处理返回 Handled，游戏态返回 Pass
#[allow(clippy::too_many_arguments)]
fn system_route(
    ev: &Event,
    overlay: &mut OverlaySys,
    systems: &mut Systems,
    interp: &mut Interp,
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    ir: &mut InputRouter,
    ctx: &Ctx,
    conf: &Config,
    started: &mut bool,
    msg: &mut Option<(String, f32)>,
    can_menu: bool,
) -> Result<RouteResult, String> {
    match ev {
        Event::Quit { .. } => return Ok(RouteResult::Quit),
        Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
        } => {
            if overlay.active() {
                let a = overlay.key(Keycode::Escape, ctx);
                if matches!(a, Action::Quit) {
                    return Ok(RouteResult::Quit);
                }
                apply_action(a, interp, overlay, systems, canvas, conf, started, msg);
            } else if can_menu {
                overlay.open_menu();
            } else {
                return Ok(RouteResult::Quit);
            }
            Ok(RouteResult::Handled)
        }
        Event::KeyDown {
            keycode: Some(Keycode::LCtrl | Keycode::RCtrl),
            ..
        } => {
            ir.ctrl = true;
            Ok(RouteResult::Handled)
        }
        Event::KeyUp {
            keycode: Some(Keycode::LCtrl | Keycode::RCtrl),
            ..
        } => {
            ir.ctrl = false;
            Ok(RouteResult::Handled)
        }
        Event::KeyDown {
            keycode: Some(Keycode::F9),
            ..
        } => {
            let _ = systems.do_save(interp, canvas, 0);
            Ok(RouteResult::Handled)
        }
        Event::KeyDown {
            keycode: Some(Keycode::F10),
            ..
        } => {
            if let Err(e) = systems.do_load(interp, 0) {
                eprintln!("[load] {e}");
            }
            Ok(RouteResult::Handled)
        }
        Event::KeyDown {
            keycode: Some(k @ (Keycode::F5 | Keycode::F6 | Keycode::F7 | Keycode::F8)),
            ..
        } => {
            let (bgm, se) = systems.volumes();
            let is_bgm = matches!(*k, Keycode::F5 | Keycode::F6);
            let v = if is_bgm {
                let d = if *k == Keycode::F5 { -5 } else { 5 };
                (bgm + d).clamp(0, 100)
            } else {
                let d = if *k == Keycode::F7 { -5 } else { 5 };
                (se + d).clamp(0, 100)
            };
            if let Some(a) = systems.audio.as_mut() {
                if is_bgm {
                    a.set_bgm_volume(v);
                } else {
                    a.set_se_volume(v);
                }
            }
            let key = if is_bgm { "sf.bgm_vol" } else { "sf.se_vol" };
            interp.vars.set(key, Value::Int(v as i64));
            Ok(RouteResult::Handled)
        }
        Event::KeyDown {
            keycode: Some(k), ..
        } if overlay.active() => {
            let a = overlay.key(*k, ctx);
            if matches!(a, Action::Quit) {
                return Ok(RouteResult::Quit);
            }
            apply_action(a, interp, overlay, systems, canvas, conf, started, msg);
            Ok(RouteResult::Handled)
        }
        Event::MouseButtonDown {
            mouse_btn: MouseButton::Left,
            x,
            y,
            ..
        } if overlay.active() => {
            let (lx, ly) = Renderer::to_logical(canvas, *x, *y);
            let a = overlay.click(lx, ly, ctx);
            if matches!(a, Action::Quit) {
                return Ok(RouteResult::Quit);
            }
            apply_action(a, interp, overlay, systems, canvas, conf, started, msg);
            Ok(RouteResult::Handled)
        }
        Event::MouseButtonDown {
            mouse_btn: MouseButton::Right,
            ..
        } if can_menu && !overlay.active() => {
            overlay.open_menu();
            Ok(RouteResult::Handled)
        }
        _ => Ok(RouteResult::Pass),
    }
}

enum RouteResult {
    Handled,
    Pass,
    Quit,
}

fn started_ok(interp: &Interp, started: bool) -> bool {
    started && interp.state != RunState::Ended
}

fn autoclick_next(
    overlay: &mut OverlaySys,
    ir: &mut InputRouter,
    interp: &mut Interp,
    started: bool,
    ctx: &Ctx,
) -> Action {
    use crate::script::interp::RunState as R;
    if !started && matches!(overlay.cur, Overlay::Title { .. }) {
        return overlay.key(Keycode::Return, ctx);
    }
    if let Overlay::Ritual { .. } = overlay.cur {
        return overlay.key(Keycode::Return, ctx);
    }
    if started && !overlay.active() {
        match &interp.state {
            R::WaitClick => {
                let _ = interp.click();
            }
            R::WaitChoice { .. } => {
                let _ = interp.choose(ir.choice_sel);
            }
            R::WaitInput(_) => {
                let v = ir.input_ui.as_ref().map(|ui| ui.value()).unwrap_or_default();
                let _ = interp.input_result(v);
            }
            _ => {}
        }
    }
    Action::None
}

fn build_ctx(
    interp: &Interp,
    systems: &mut Systems,
    conf: &Config,
    title_main: &str,
    title_sub: &str,
) -> Ctx {
    let unlocked: Vec<String> = match interp.vars.get("sf.cgs") {
        Some(Value::Str(s)) => s.split(',').filter(|x| !x.is_empty()).map(String::from).collect(),
        _ => Vec::new(),
    };
    let you = match interp.vars.get("sf.playerName") {
        Some(Value::Str(s)) if !s.is_empty() => s.clone(),
        _ => conf.game.you_default.clone(),
    };
    Ctx {
        meta: systems.meta.clone(),
        slots: systems.store.list(),
        save_dir: conf.save_dir.clone(),
        all_cgs: systems.all_cgs.clone(),
        unlocked_cgs: unlocked,
        you,
        volumes: systems.volumes(),
        tw_ms: interp.tw.interval_ms,
        title_stage: systems.meta.title_stage,
        title_main: title_main.to_string(),
        title_sub: title_sub.to_string(),
        ritual_texts: conf.game.ritual_texts.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_action(
    action: Action,
    interp: &mut Interp,
    overlay: &mut OverlaySys,
    systems: &mut Systems,
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    conf: &Config,
    started: &mut bool,
    msg: &mut Option<(String, f32)>,
) {
    match action {
        Action::None => {}
        Action::Close => overlay.cur = Overlay::None,
        Action::Quit => {}
        Action::StartNew => {
            interp.vars.f.clear();
            let script = env::var("ES_SCRIPT").unwrap_or_else(|_| conf.script_start.clone());
            if let Err(e) = interp.start(&script) {
                *msg = Some((e, 3000.0));
                return;
            }
            *started = true;
            overlay.cur = Overlay::None;
        }
        Action::ContinueRun => match systems.do_load(interp, 0) {
            Ok(_) => {
                *started = true;
                overlay.cur = Overlay::None;
            }
            Err(e) => *msg = Some((e, 2500.0)),
        },
        Action::SaveTo(n) => {
            if let Err(e) = systems.do_save(interp, canvas, n) {
                *msg = Some((e, 2500.0));
            }
            overlay.cur = Overlay::None;
        }
        Action::LoadFrom(n) => match systems.do_load(interp, n) {
            Ok(_) => overlay.cur = Overlay::None,
            Err(e) => *msg = Some((e, 2500.0)),
        },
        Action::DeleteSlot(n) => {
            if let Err(e) = systems.store.delete(n) {
                *msg = Some((e, 2500.0));
            }
        }
        Action::RitualDone => overlay.cur = Overlay::None,
        Action::ApplyVolumes { bgm, se, tw_ms } => {
            if let Some(a) = systems.audio.as_mut() {
                a.set_bgm_volume(bgm);
                a.set_se_volume(se);
            }
            interp.tw.interval_ms = tw_ms;
            interp.vars.set("sf.bgm_vol", Value::Int(bgm as i64));
            interp.vars.set("sf.se_vol", Value::Int(se as i64));
            interp.vars.set("sf.typewriter_ms", Value::Float(tw_ms as f64));
            overlay.cur = Overlay::None;
        }
        Action::CancelShutdown => {
            let _ = platform::cancel_shutdown();
            if let Overlay::Rest { cancelled, .. } = &mut overlay.cur {
                *cancelled = true;
            }
        }
    }
}

fn split_title(t: &str) -> (String, String) {
    match t.split_once(' ') {
        Some((a, b)) => (a.to_string(), b.to_string()),
        None => (t.to_string(), String::new()),
    }
}

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
        Err(_) => build()?
            .into_canvas()
            .software()
            .build()
            .map_err(|e| e.to_string()),
    }
}
