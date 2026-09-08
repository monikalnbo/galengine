//! 覆盖层方向键导航 + 音量快捷键。

use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::systems::Game;
use gal_script::vars::Value;
use gal_ui::overlay::{esc_items, title_items, Overlay};

use super::settings_io;

/// F5/F6 BGM 音量 −/+，F7/F8 SE 音量 −/+（存 sf.*，规格 §2.4）
pub fn hotkey_volume(g: &mut Game, k: Keycode) -> bool {
    let (bgm, delta, name) = match k {
        Keycode::F5 => (true, -10, "BGM"),
        Keycode::F6 => (true, 10, "BGM"),
        Keycode::F7 => (false, -10, "SE"),
        Keycode::F8 => (false, 10, "SE"),
        _ => return false,
    };
    let v = if bgm {
        g.sys.audio.bgm_vol = (g.sys.audio.bgm_vol as i64 + delta).clamp(0, 100) as i32;
        g.interp.vars.sf.insert("volBgm".into(), Value::Int(g.sys.audio.bgm_vol as i64));
        g.sys.audio.bgm_vol
    } else {
        g.sys.audio.se_vol = (g.sys.audio.se_vol as i64 + delta).clamp(0, 100) as i32;
        g.interp.vars.sf.insert("volSe".into(), Value::Int(g.sys.audio.se_vol as i64));
        g.sys.audio.se_vol
    };
    g.sys.audio.apply_volumes();
    g.msg(format!("{name} 音量 {v}"));
    true
}

/// 覆盖层方向键移动（设置行内调值；存档/鉴赏网格；菜单/标题竖排）
pub fn overlay_move(g: &mut Game, k: Keycode, canvas: &mut Canvas<Window>) {
    // 设置：↑↓选行，←→调值
    if matches!(g.overlay, Overlay::Settings { .. }) {
        match k {
            Keycode::Left => settings_io::arrow(g, canvas, -1),
            Keycode::Right => settings_io::arrow(g, canvas, 1),
            _ => settings_io::move_sel(g, k),
        }
        return;
    }
    if let Overlay::Gallery { sel, page, full } = &mut g.overlay {
        gallery_move(&g.interp.vars, sel, page, full, k);
        return;
    }
    let n = match &g.overlay {
        Overlay::Menu { .. } => esc_items().len(),
        Overlay::Title { .. } => title_items().len(),
        Overlay::Save { .. } => g.save_entries.len() + g.sys.meta.fake_saves.len(),
        _ => return,
    };
    if n == 0 {
        return;
    }
    let cur = match &g.overlay {
        Overlay::Menu { sel } | Overlay::Save { sel, .. } | Overlay::Title { sel } => *sel,
        _ => 0,
    };
    let cols = match g.overlay {
        Overlay::Save { .. } => 3,
        _ => 1,
    };
    let next = match k {
        Keycode::Up => cur.saturating_sub(cols),
        Keycode::Down => (cur + cols).min(n - 1),
        Keycode::Left => cur.saturating_sub(1),
        Keycode::Right => (cur + 1).min(n - 1),
        _ => cur,
    };
    match &mut g.overlay {
        Overlay::Menu { sel } | Overlay::Save { sel, .. } | Overlay::Title { sel } => *sel = next,
        _ => {}
    }
}

/// 鉴赏导航：网格 4 列移动 / 全屏在已解锁 CG 间循环
fn gallery_move(
    vars: &gal_script::vars::Vars,
    sel: &mut usize,
    page: &mut usize,
    full: &mut Option<usize>,
    k: Keycode,
) {
    if full.is_some() {
        let unlocked: Vec<usize> = {
            let list = gal_ui::gallery::catalog();
            let opened = vars.get_or("sf.cgs").as_str();
            list.iter()
                .enumerate()
                .filter(|(_, n)| opened.split(',').any(|x| x == n.as_str()))
                .map(|(i, _)| i)
                .collect()
        };
        if unlocked.is_empty() {
            return;
        }
        let cur = (*full).unwrap_or(0);
        let pos = unlocked.iter().position(|&i| i == cur).unwrap_or(0);
        let next = match k {
            Keycode::Left | Keycode::Up => (pos + unlocked.len() - 1) % unlocked.len(),
            Keycode::Right | Keycode::Down => (pos + 1) % unlocked.len(),
            _ => pos,
        };
        let idx = unlocked[next];
        *full = Some(idx);
        *page = idx / gal_ui::gallery::per_page();
        *sel = idx % gal_ui::gallery::per_page();
        return;
    }
    let n = gal_ui::gallery::catalog().len();
    if n == 0 {
        return;
    }
    let per = gal_ui::gallery::per_page();
    let mut idx = *page * per + *sel;
    idx = match k {
        Keycode::Up => idx.saturating_sub(4),
        Keycode::Down => (idx + 4).min(n - 1),
        Keycode::Left => idx.saturating_sub(1),
        Keycode::Right => (idx + 1).min(n - 1),
        _ => idx,
    };
    *page = idx / per;
    *sel = idx % per;
}
