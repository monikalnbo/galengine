//! L1 游戏输入路由：事件按「覆盖层 → 关机 → 台词/选项/输入三态」分发。
//! L2 不碰 SDL 事件泵，本层是唯一的 SDL 事件翻译点。

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::gfx::renderer::Renderer;
use crate::script::interp::RunState;
use crate::script::vars::Value;
use crate::systems::Game;
use crate::text::font::FontBook;
use crate::ui::inputbox::{self, Hit};
use crate::ui::{choice, overlay::Overlay};

/// 处理一条事件；返回 false=请求退出
pub fn dispatch(
    g: &mut Game,
    ev: Event,
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
) -> Result<bool, String> {
    match ev {
        Event::Quit { .. } => return Ok(false),

        Event::KeyDown { keycode: Some(k), .. } => {
            match k {
                Keycode::Escape => {
                    if g.sys.rest.is_some() {
                        g.cancel_rest();
                    } else if g.overlay.active() {
                        g.overlay = Overlay::None;
                    } else {
                        return Ok(false); // M4：Esc 唤出系统菜单
                    }
                }
                Keycode::LCtrl | Keycode::RCtrl => g.ctrl_hold = true,
                Keycode::A if g.interp.state == RunState::WaitClick => {
                    g.auto = !g.auto;
                    g.auto_acc = 0.0;
                }
                k if volume_hotkey(g, k) => {}
                Keycode::Up if g.choice_len() > 0 => g.choice_sel = g.choice_sel.saturating_sub(1),
                Keycode::Down if g.choice_len() > 0 => {
                    g.choice_sel = (g.choice_sel + 1).min(g.choice_len() - 1)
                }
                Keycode::Return | Keycode::KpEnter | Keycode::Space => match &g.interp.state {
                    RunState::WaitChoice { .. } => g.interp.choose(g.choice_sel)?,
                    RunState::WaitInput(_) => confirm_input(g)?,
                    RunState::WaitClick => g.interp.click()?,
                    _ => {}
                },
                Keycode::Backspace if g.input_ui.is_some() => {
                    if let Some(ui) = g.input_ui.as_mut() {
                        ui.backspace();
                    }
                }
                _ => {}
            }
        }

        Event::KeyUp { keycode: Some(Keycode::LCtrl | Keycode::RCtrl), .. } => g.ctrl_hold = false,

        Event::TextInput { ref text, .. } if g.input_ui.is_some() => {
            if let Some(ui) = g.input_ui.as_mut() {
                ui.push_char(text);
            }
        }

        Event::MouseMotion { x, y, .. } if g.choice_len() > 0 => {
            let (lx, ly) = Renderer::to_logical(canvas, x, y);
            if let Some(i) = choice::hit_test(g.choice_len(), lx, ly) {
                g.choice_sel = i;
            }
        }

        Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => {
            let (lx, ly) = Renderer::to_logical(canvas, x, y);
            match &g.interp.state {
                RunState::WaitChoice { .. } => {
                    if let Some(i) = choice::hit_test(g.choice_len(), lx, ly) {
                        g.interp.choose(i)?;
                    }
                }
                RunState::WaitInput(_) => match g.input_ui.as_ref().and_then(|ui| inputbox::hit_test(ui, lx, ly)) {
                    Some(Hit::Preset(i)) => {
                        if let Some(ui) = g.input_ui.as_mut() {
                            if let Some(p) = ui.presets.get(i) {
                                ui.buf = p.clone();
                            }
                        }
                    }
                    Some(Hit::Confirm) => confirm_input(g)?,
                    None => {}
                },
                RunState::WaitClick => g.interp.click()?,
                _ => {} // M4/M5 覆盖层点击
            }
        }
        _ => {}
    }
    Ok(true)
}

/// 输入框确认（写回变量并继续）
fn confirm_input(g: &mut Game) -> Result<(), String> {
    if let Some(ui) = g.input_ui.take() {
        g.interp.input_result(ui.value())?;
    }
    Ok(())
}

/// 验收自动驱动（ES_DEBUG_AUTOCLICK_MS）：台词=点击，选项=选0，输入=默认值
pub fn auto_step(g: &mut Game) -> Result<(), String> {
    if g.sys.rest.is_some() {
        g.cancel_rest(); // 验收环境自动取消关机，跑完剧本
        return Ok(());
    }
    match &g.interp.state {
        RunState::WaitClick => g.interp.click(),
        RunState::WaitChoice { .. } => g.interp.choose(0),
        RunState::WaitInput(_) => {
            let v = g.input_ui.as_ref().map(|ui| ui.value()).unwrap_or_default();
            g.interp.input_result(v)
        }
        _ => Ok(()),
    }
}

impl Game {
    pub fn choice_len(&self) -> usize {
        match &self.interp.state {
            RunState::WaitChoice { items, .. } => items.len(),
            _ => 0,
        }
    }
}

/// F5/F6 BGM 音量 −/+，F7/F8 SE 音量 −/+（存 sf.*，规格 §2.4）
fn volume_hotkey(g: &mut Game, k: Keycode) -> bool {
    let (delta, is_bgm) = match k {
        Keycode::F5 => (-10, true),
        Keycode::F6 => (10, true),
        Keycode::F7 => (-10, false),
        Keycode::F8 => (10, false),
        _ => return false,
    };
    let (name, v) = if is_bgm {
        g.sys.audio.bgm_vol = (g.sys.audio.bgm_vol + delta).clamp(0, 100);
        g.interp.vars.sf.insert("volBgm".into(), Value::Int(g.sys.audio.bgm_vol as i64));
        ("BGM", g.sys.audio.bgm_vol)
    } else {
        g.sys.audio.se_vol = (g.sys.audio.se_vol + delta).clamp(0, 100);
        g.interp.vars.sf.insert("volSe".into(), Value::Int(g.sys.audio.se_vol as i64));
        ("SE", g.sys.audio.se_vol)
    };
    g.sys.audio.apply_volumes();
    g.msg(format!("{name} 音量 {v}"));
    true
}
