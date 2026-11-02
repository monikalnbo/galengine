//! 键盘按键事件总分发：键位表（game.yaml keymap + 玩家 sf.keymap.*）→ 动作分发。
//!
//! 方向键/退格是界面导航键，不走可重绑动作表（否则设置面板自身无法操作）。

use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::video::{FullscreenType, Window};

use crate::systems::Game;
use gal_render::renderer::Renderer;
use gal_ui::savemenu::ThumbCache;

use super::{activate, keys, saves, settings_io};

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
    // 设置面板按键重绑进行中：这一次按键只用于绑键
    if g.rebind.is_some() {
        settings_io::rebind_key(g, k);
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
    // 输入框：退格/回车等功能键优先（TextInput 走 dispatch）
    if g.input_ui.is_some() && k == Keycode::Backspace {
        if let Some(ui) = g.input_ui.as_mut() {
            ui.backspace();
        }
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
    // 方向键：界面导航 / 选项移动
    match k {
        Keycode::Up | Keycode::Down | Keycode::Left | Keycode::Right if g.overlay.active() => {
            keys::overlay_move(g, k, canvas);
            return Ok(true);
        }
        Keycode::Up if g.choice_len() > 0 => {
            g.choice_sel = g.choice_sel.saturating_sub(1);
            return Ok(true);
        }
        Keycode::Down if g.choice_len() > 0 => {
            g.choice_sel = (g.choice_sel + 1).min(g.choice_len() - 1);
            return Ok(true);
        }
        _ => {}
    }

    let action = resolve_action(g, k);
    let mut quit = false;
    match action.as_deref() {
        Some("menu") => {
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
        Some("save") => {
            saves::do_save(g, renderer, canvas, thumbs, g.sys.slots)?;
        }
        Some("load") => saves::do_load(g, g.sys.slots),
        Some("next") => {
            quit = !activate::confirm_key(g, canvas, renderer, thumbs)?;
        }
        Some("skip") => g.ctrl_hold = true,
        Some("auto")
            if g.interp.state == gal_script::interp::RunState::WaitClick && !g.overlay.active() =>
        {
            g.auto = !g.auto;
            g.auto_acc = 0.0;
        }
        Some("backlog")
            if g.interp.state == gal_script::interp::RunState::WaitClick && !g.overlay.active() =>
        {
            g.overlay = gal_ui::overlay::Overlay::Backlog { scroll: 0 };
        }
        Some("fullscreen") => toggle_fullscreen(g, canvas),
        _ => {}
    }
    Ok(!quit)
}

/// SDL 按键 → 动作名（键位表逐键匹配；无匹配 = None）
fn resolve_action(g: &Game, k: Keycode) -> Option<String> {
    let map = g.keymap();
    map.into_iter()
        .find(|(_, names)| names.iter().any(|n| Keycode::from_name(n) == Some(k)))
        .map(|(action, _)| action)
}

fn toggle_fullscreen(g: &mut Game, canvas: &mut Canvas<Window>) {
    let want = if canvas.window().fullscreen_state() == FullscreenType::Off {
        FullscreenType::Desktop
    } else {
        FullscreenType::Off
    };
    if let Err(e) = canvas.window_mut().set_fullscreen(want) {
        g.msg(format!("全屏切换失败：{e}"));
    }
}
