//! 键盘按键事件总分发：Esc菜单/快存读/回车/Ctrl/自动/历史/方向/退格

use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::systems::Game;
use gal_render::renderer::Renderer;
use gal_ui::savemenu::ThumbCache;

use super::{activate, keys, saves};

pub fn key_down(
    g: &mut Game,
    k: Keycode,
    canvas: &mut Canvas<Window>,
    renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    if g.hide_ui {
        g.hide_ui = false;
        return Ok(true);
    }
    if let gal_ui::overlay::Overlay::Backlog { scroll } = &mut g.overlay {
        match k {
            Keycode::Escape => g.overlay = gal_ui::overlay::Overlay::None,
            Keycode::Up | Keycode::PageUp => *scroll += 1,
            Keycode::Down | Keycode::PageDown => *scroll = scroll.saturating_sub(1),
            _ => g.overlay = gal_ui::overlay::Overlay::None,
        }
        return Ok(true);
    }
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
                // 标题必须选一项进入，Esc 不动作
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
        Keycode::PageUp
            if g.interp.state == gal_script::interp::RunState::WaitClick && !g.overlay.active() =>
        {
            g.overlay = gal_ui::overlay::Overlay::Backlog { scroll: 0 };
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
