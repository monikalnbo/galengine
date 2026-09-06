//! 应用组装与主循环：事件按解释器状态分发（台词/选项/输入），
//! 含自动模式（A）、快进（按住 Ctrl）、验收自动驱动（ES_DEBUG_AUTOCLICK_MS）。

use std::env;
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

use crate::config;
use crate::gfx::assets::{self, TextureBank};
use crate::gfx::renderer::Renderer;
use crate::script::interp::{InputSpec, Interp, RunState};
use crate::script::vars::Vars;
use crate::text::font::FontBook;
use crate::ui::dialog::{self, DialogStyle};
use crate::ui::{choice, inputbox};

/// sf.* 全局变量持久化位置
const GLOBAL_SAVE: &str = "savedata/global.json";

// rust-sdl2 未暴露的 SDL API（键盘文本输入，IME 支持）
extern "C" {
    fn SDL_StartTextInput();
}

pub fn run() -> Result<(), String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let _image = sdl2::image::init(sdl2::image::InitFlag::PNG | sdl2::image::InitFlag::JPG)?;
    let ttf = sdl2::ttf::init().map_err(|e| format!("字体系统初始化失败：{e}"))?;

    let conf = config::Config::load();
    let mut canvas = make_canvas(&video, &conf.title)?;
    let texture_creator = canvas.texture_creator();
    let mut renderer = Renderer::new(&texture_creator)?;
    let mut bank = TextureBank::new(&texture_creator);

    // ===== 顶层总配置（data/config.json，分层架构唯一配置源）=====
    let mut fonts = FontBook::new(&ttf, &texture_creator, &conf.fonts)?;
    let style = DialogStyle::from_cfg(&conf.dialog);

    let mut interp = Interp::new(Vars::load_global(GLOBAL_SAVE), conf.typewriter_ms);
    let script = env::var("ES_SCRIPT").unwrap_or_else(|_| conf.script_start.clone());
    interp.start(&script)?;
    let auto_delay = conf.auto_delay_ms;
    // 场景级资源管理：背景变更 = 场景切换信号
    let mut last_bg: Option<String> = None;

    // 交互状态
    let mut choice_sel = 0usize;
    let mut input_ui: Option<inputbox::InputUi> = None;
    let mut auto = false;
    let mut auto_acc = 0.0f32;
    let mut ctrl = false;

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

        // 输入态生命周期维护（进入时启动文本输入）
        if input_ui.is_none() {
            if let RunState::WaitInput(spec) = &interp.state {
                input_ui = Some(make_input_ui(spec));
                unsafe { SDL_StartTextInput() };
            }
        } else if !matches!(interp.state, RunState::WaitInput(_)) {
            input_ui = None;
        }
        if let Some(ui) = input_ui.as_mut() {
            ui.blink += dt;
        }

        let in_choice = matches!(interp.state, RunState::WaitChoice { .. });
        let in_input = matches!(interp.state, RunState::WaitInput(_));
        let in_click = interp.state == RunState::WaitClick;
        let choice_n = match &interp.state {
            RunState::WaitChoice { items } => items.len(),
            _ => 0,
        };

        for ev in events.poll_iter() {
            match ev {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,

                // Ctrl 快进（按住）
                Event::KeyDown {
                    keycode: Some(Keycode::LCtrl | Keycode::RCtrl),
                    ..
                } => ctrl = true,
                Event::KeyUp {
                    keycode: Some(Keycode::LCtrl | Keycode::RCtrl),
                    ..
                } => ctrl = false,

                // A 键：自动模式（台词状态）
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } if in_click => {
                    auto = !auto;
                    auto_acc = 0.0;
                }

                // —— 选项态：键盘 ——
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } if in_choice => choice_sel = choice_sel.saturating_sub(1),
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } if in_choice => choice_sel = (choice_sel + 1).min(choice_n - 1),
                Event::KeyDown {
                    keycode: Some(Keycode::Return | Keycode::KpEnter | Keycode::Space),
                    ..
                } if in_choice => interp.choose(choice_sel)?,

                // —— 选项态：鼠标 ——
                Event::MouseMotion { x, y, .. } if in_choice => {
                    let (lx, ly) = Renderer::to_logical(&canvas, x, y);
                    let idxs: Vec<usize> = (0..choice_n).collect();
                    if let Some(i) = choice::hit_test(&idxs, lx, ly) {
                        choice_sel = i;
                    }
                }
                Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    x,
                    y,
                    ..
                } if in_choice => {
                    let (lx, ly) = Renderer::to_logical(&canvas, x, y);
                    let idxs: Vec<usize> = (0..choice_n).collect();
                    if let Some(i) = choice::hit_test(&idxs, lx, ly) {
                        interp.choose(i)?;
                    }
                }

                // —— 输入态 ——
                Event::TextInput { ref text, .. } if in_input => {
                    if let Some(ui) = input_ui.as_mut() {
                        ui.push_char(text);
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Backspace),
                    ..
                } if in_input => {
                    if let Some(ui) = input_ui.as_mut() {
                        ui.backspace();
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Return | Keycode::KpEnter),
                    ..
                } if in_input => confirm_input(&mut interp, &mut input_ui)?,
                Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    x,
                    y,
                    ..
                } if in_input => {
                    let (lx, ly) = Renderer::to_logical(&canvas, x, y);
                    let hit = input_ui.as_ref().and_then(|ui| inputbox::hit_test(ui, lx, ly));
                    match hit {
                        Some(inputbox::Hit::Preset(i)) => {
                            if let Some(ui) = input_ui.as_mut() {
                                if let Some(p) = ui.presets.get(i) {
                                    ui.buf = p.clone();
                                }
                            }
                        }
                        Some(inputbox::Hit::Confirm) => confirm_input(&mut interp, &mut input_ui)?,
                        None => {}
                    }
                }

                // —— 台词态：推进 ——
                Event::KeyDown {
                    keycode: Some(Keycode::Space | Keycode::Return | Keycode::KpEnter),
                    ..
                } if in_click => interp.click()?,
                Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    ..
                } if in_click => interp.click()?,

                _ => {}
            }
        }

        // 快进（wait/meta 段不作用，规格 §3.4）
        if ctrl && interp.state == RunState::WaitClick {
            interp.click()?;
        }

        // 自动模式：行完延迟推进
        if auto && interp.state == RunState::WaitClick {
            if interp.tw.line_finished() {
                auto_acc += dt;
                if auto_acc >= auto_delay {
                    auto_acc = 0.0;
                    interp.click()?;
                }
            } else {
                auto_acc = 0.0;
            }
        }

        // 验收自动驱动：台词=点击，选项=选0，输入=默认值
        if autoclick_ms > 0.0 {
            click_acc += dt;
            let due = click_acc >= autoclick_ms;
            if due {
                click_acc = 0.0;
                match &interp.state {
                    RunState::WaitClick => interp.click()?,
                    RunState::WaitChoice { .. } => interp.choose(0)?,
                    RunState::WaitInput(_) => {
                        let v = input_ui.as_ref().map(|ui| ui.value()).unwrap_or_default();
                        interp.input_result(v)?;
                    }
                    _ => {}
                }
            }
        }

        interp.tick(dt);
        interp.apply_pending(&mut fonts, &style);

        // ===== 场景级资源管理（分层：stage 为场景真相源）=====
        // 1) 缓存层收货 + 预读喂料
        bank.prefetch.pump();
        for (kind, storage) in interp.lookahead_storages(40) {
            bank.prefetch.request(&assets::resolve(kind, &storage));
        }
        // 2) 背景变更 = 切场景：回收不用的图片/文字纹理
        let cur_bg = interp.stage.bg.cur().cloned();
        if cur_bg != last_bg {
            last_bg = cur_bg;
            let mut keep = std::collections::HashSet::new();
            interp.stage.collect_paths(&mut keep);
            bank.retain(&keep);
            fonts.clear_text_cache();
            if crate::script::interp::dbg() {
                eprintln!(
                    "[dbg] 场景切换：保留 {} 张图，预解码完成 {} 张",
                    keep.len(),
                    bank.prefetch.stats_done
                );
            }
        }

        if interp.state == RunState::Ended {
            if crate::script::interp::dbg() {
                eprintln!("[dbg t={now_ms:.2}] ended");
            }
            break 'running;
        }

        let name_opt = interp.cur_name.clone();
        let sel = choice_sel;
        let is_auto = auto;
        let show_choice = matches!(interp.state, RunState::WaitChoice { .. });
        let show_input = matches!(interp.state, RunState::WaitInput(_));
        renderer.present_frame(&mut canvas, Color::BLACK, |tc| {
            interp.stage.draw(tc, &mut bank)?;
            if show_choice {
                if let RunState::WaitChoice { items } = &interp.state {
                    choice::draw(tc, &mut fonts, items, sel)?;
                }
            } else if show_input {
                if let Some(ui) = &input_ui {
                    inputbox::draw(tc, &mut fonts, ui)?;
                }
            } else {
                dialog::draw(tc, &mut fonts, &style, name_opt.as_deref(), &interp.tw, now_ms)?;
            }
            // AUTO 角标
            if is_auto && !show_choice && !show_input {
                let tex = fonts.render_text(22, Color::RGB(110, 220, 255), "AUTO")?;
                tc.copy(tex, None, Some(Rect::new(1170, 486, 60, 26)))?;
            }
            Ok(())
        })?;

        let spent = t0.elapsed();
        if spent < budget {
            std::thread::sleep(budget - spent);
        }
    }

    interp.vars.save_global(GLOBAL_SAVE)?;
    Ok(())
}

fn make_input_ui(spec: &InputSpec) -> inputbox::InputUi {
    inputbox::InputUi::new(&spec.var, &spec.prompt, spec.width, &spec.default)
}

fn confirm_input(
    interp: &mut Interp,
    input_ui: &mut Option<inputbox::InputUi>,
) -> Result<(), String> {
    if let Some(ui) = input_ui.take() {
        interp.input_result(ui.value())?;
    }
    Ok(())
}

/// 创建窗口与画布：可自由缩放；首选硬件加速+vsync，
/// 无 GPU 环境（xvfb 验收）自动回退软件渲染。
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
