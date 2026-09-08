//! L1 输入路由：SDL 事件 → 关机 → 覆盖层 → 台词/选项/输入三态。
//! 唯一的 SDL 事件翻译点；覆盖层交互在子模块（keys/mouse/settings_io/saves/activate）。

mod activate;
mod keys;
mod mouse;
mod saves;
mod settings_io;

#[cfg(test)]
mod saves_test;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::systems::Game;
use gal_render::renderer::Renderer;
use gal_text::font::FontBook;
use gal_ui::savemenu::ThumbCache;

/// 处理一条事件；返回 false=请求退出
pub fn dispatch(
    g: &mut Game,
    ev: Event,
    canvas: &mut Canvas<Window>,
    _fonts: &mut FontBook,
    renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    match ev {
        Event::Quit { .. } => Ok(false),

        Event::KeyDown { keycode: Some(k), .. } => key_down(g, k, canvas, renderer, thumbs),

        Event::KeyUp { keycode: Some(Keycode::LCtrl | Keycode::RCtrl), .. } => {
            g.ctrl_hold = false;
            Ok(true)
        }

        Event::TextInput { ref text, .. } if g.input_ui.is_some() => {
            if let Some(ui) = g.input_ui.as_mut() {
                ui.push_char(text);
            }
            Ok(true)
        }

        Event::MouseMotion { x, y, .. } if !g.overlay.active() && g.choice_len() > 0 => {
            let (lx, ly) = Renderer::to_logical(canvas, x, y);
            if let Some(i) = gal_ui::choice::hit_test(g.choice_len(), lx, ly) {
                g.choice_sel = i;
            }
            Ok(true)
        }

        Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => {
            g.cursor.set_click();
            let (lx, ly) = Renderer::to_logical(canvas, x, y);
            mouse::click(g, lx, ly, canvas, renderer, thumbs)
        }
        Event::MouseButtonUp { mouse_btn: MouseButton::Left, .. } => {
            g.cursor.set_default();
            Ok(true)
        }
        _ => Ok(true),
    }
}

fn key_down(
    g: &mut Game,
    k: Keycode,
    canvas: &mut Canvas<Window>,
    renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    if keys::hotkey_volume(g, k) {
        return Ok(true);
    }
    // 拓展键盘事件转发（选项态；任一拓展消化则不再默认处理）
    if matches!(g.interp.state, gal_script::interp::RunState::WaitChoice { .. })
        && !g.overlay.active()
    {
        let mut handled = false;
        for e in g.exts.iter_mut() {
            let mut ectx = gal_ext::ExtContext::new(&mut g.interp);
            if e.on_key(k, &mut ectx) == gal_ext::ExtResult::Handled {
                handled = true;
            }
        }
        if handled {
            return Ok(true);
        }
    }
    let mut quit = false;
    match k {
        Keycode::Escape => {
            if g.sys.rest.is_some() {
                g.cancel_rest();
            } else if matches!(g.overlay, gal_ui::overlay::Overlay::Title { .. }) {
                // 标题必须选一项进入，Esc 不动作（否则未开局黑屏）
            } else if g.overlay.active() {
                g.overlay = gal_ui::overlay::Overlay::None;
            } else {
                g.overlay = gal_ui::overlay::Overlay::Menu { sel: 0 };
            }
        }
        Keycode::F9 => {
            saves::do_save(g, renderer, canvas, thumbs, g.sys.slots)?;
        }
        Keycode::F10 => saves::do_load(g, g.sys.slots),
        Keycode::Return | Keycode::KpEnter | Keycode::Space => {
            if !activate::confirm_key(g, canvas, renderer, thumbs)? {
                quit = true;
            }
        }
        Keycode::LCtrl | Keycode::RCtrl => g.ctrl_hold = true,
        Keycode::A
            if g.interp.state == gal_script::interp::RunState::WaitClick && !g.overlay.active() =>
        {
            g.auto = !g.auto;
            g.auto_acc = 0.0;
        }
        Keycode::Up | Keycode::Down | Keycode::Left | Keycode::Right if g.overlay.active() => {
            keys::overlay_move(g, k, canvas);
        }
        Keycode::Up if g.choice_len() > 0 => g.choice_sel = g.choice_sel.saturating_sub(1),
        Keycode::Down if g.choice_len() > 0 => {
            g.choice_sel = (g.choice_sel + 1).min(g.choice_len() - 1)
        }
        Keycode::Backspace if g.input_ui.is_some() => {
            if let Some(ui) = g.input_ui.as_mut() {
                ui.backspace();
            }
        }
        _ => {}
    }
    Ok(!quit)
}

/// 验收自动驱动（ES_DEBUG_AUTOCLICK_MS）：标题=开始，台词=点击，选项=选0，输入=默认值
pub fn auto_step(g: &mut Game, thumbs: &mut ThumbCache) -> Result<(), String> {
    if g.sys.rest.is_some() {
        g.cancel_rest(); // 验收环境自动取消关机，跑完剧本
        return Ok(());
    }
    if g.overlay.active() {
        // 验收驱动：覆盖层默认首项/确认推进（标题开始/菜单继续/仪式确认）
        match g.overlay.clone() {
            gal_ui::overlay::Overlay::Title { sel, .. } => {
                activate::title_activate(g, sel, thumbs)?;
            }
            gal_ui::overlay::Overlay::Menu { .. }
            | gal_ui::overlay::Overlay::Save { .. }
            | gal_ui::overlay::Overlay::Gallery { .. }
            | gal_ui::overlay::Overlay::Settings { .. } => {
                g.overlay = gal_ui::overlay::Overlay::None;
            }
            gal_ui::overlay::Overlay::Ritual { step, fade, .. } => {
                g.overlay = gal_ui::overlay::Overlay::Ritual { step: step + 1, fade, done: false };
            }
            _ => {}
        }
        return Ok(());
    }
    use gal_script::interp::RunState;
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
