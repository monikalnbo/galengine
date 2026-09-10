//! 激活逻辑：确认键分发 + 菜单/标题/底栏按钮 + 鉴赏/仪式/输入确认。

use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::systems::Game;
use gal_render::renderer::Renderer;
use gal_script::interp::RunState;
use gal_ui::overlay::Overlay;
use gal_ui::savemenu::ThumbCache;

use super::{saves, settings_io};

/// 回车/空格：按覆盖层 → 台词三态分发
pub fn confirm_key(
    g: &mut Game,
    canvas: &mut Canvas<Window>,
    renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    if g.sys.rest.is_some() {
        g.cancel_rest();
        return Ok(true);
    }
    match g.overlay.clone() {
        Overlay::Menu { sel } => menu_activate(g, sel, canvas, renderer, thumbs),
        Overlay::Save { mode_save, sel } => {
            saves::save_activate(g, canvas, renderer, thumbs, mode_save, sel)?;
            Ok(true)
        }
        Overlay::Ritual { .. } => {
            ritual_confirm(g)?;
            Ok(true)
        }
        Overlay::Backlog { .. } => {
            g.overlay = Overlay::None;
            Ok(true)
        }
        Overlay::Title { sel } => title_activate(g, sel, thumbs),
        Overlay::Gallery { .. } => {
            gallery_confirm(g);
            Ok(true)
        }
        Overlay::Settings { .. } => settings_io::enter(g, canvas),
        Overlay::None => match &g.interp.state {
            RunState::WaitChoice { .. } => g.interp.choose(g.choice_sel).map(|_| true),
            RunState::WaitInput(_) => confirm_input(g).map(|_| true),
            RunState::WaitClick => g.interp.click().map(|_| true),
            _ => Ok(true),
        },
    }
}

/// 底部功能条按钮：0自动 1快进 2存档 3读档 4设置
pub fn bottombar_activate(
    g: &mut Game,
    idx: usize,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    match idx {
        0 => {
            g.auto = !g.auto;
            g.auto_acc = 0.0;
            g.msg(if g.auto { "自动：开" } else { "自动：关" });
        }
        1 => {
            g.ctrl_hold = !g.ctrl_hold;
            g.msg(if g.ctrl_hold {
                "快进：开（再点一次停止）"
            } else {
                "快进：关"
            });
        }
        2 => saves::open_save_menu(g, thumbs, true),
        3 => saves::open_save_menu(g, thumbs, false),
        4 => g.overlay = Overlay::Settings { sel: 0 },
        _ => {}
    }
    Ok(true)
}

/// 系统菜单项激活；返回 false=退出游戏（走正常收尾，sf.* 落盘）
pub fn menu_activate(
    g: &mut Game,
    idx: usize,
    _canvas: &mut Canvas<Window>,
    _renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    match idx {
        0 => g.overlay = Overlay::None,               // 继续
        1 => saves::open_save_menu(g, thumbs, true),  // 存档
        2 => saves::open_save_menu(g, thumbs, false), // 读档
        3 => g.overlay = Overlay::Gallery { sel: 0, page: 0, full: None }, // 鉴赏
        4 => g.overlay = Overlay::Settings { sel: 0 }, // 设置
        5 => g.back_to_title(),                       // 回标题
        6 => return Ok(false),                        // 退出游戏
        _ => {}
    }
    Ok(true)
}

/// 标题菜单项激活
pub fn title_activate(g: &mut Game, idx: usize, thumbs: &mut ThumbCache) -> Result<bool, String> {
    match idx {
        0 => g.start_game()?, // 开始游戏
        1 => {
            // 继续游戏：最新档；无档提示
            let (dir, n) = (g.sys.save_dir.clone(), g.sys.slots);
            match gal_save::slots::latest_slot(&dir, n) {
                Some(slot) => saves::do_load(g, slot),
                None => g.msg("还没有任何存档。"),
            }
        }
        2 => saves::open_save_menu(g, thumbs, false), // 读档
        3 => g.overlay = Overlay::Gallery { sel: 0, page: 0, full: None },
        4 => g.overlay = Overlay::Settings { sel: 0 },
        5 => return Ok(false), // 退出游戏
        _ => {}
    }
    Ok(true)
}

/// 鉴赏确认：网格→全屏 / 全屏→网格；锁定不可开
pub fn gallery_confirm(g: &mut Game) {
    if let Overlay::Gallery { sel, page, full } = &mut g.overlay {
        match full {
            None => {
                let idx = *page * gal_ui::gallery::per_page() + *sel;
                let items = gal_ui::gallery::catalog();
                if let Some(name) = items.get(idx) {
                    let unlocked = g.interp.vars.get_or("sf.cgs").as_str();
                    if unlocked.split(',').any(|x| x == name) {
                        *full = Some(idx);
                    } else {
                        g.msg("这张 CG 还没有解锁。");
                    }
                }
            }
            Some(_) => *full = None,
        }
    }
}

/// 仪式删档：确认推进；第三击后渐白，白满真删
pub fn ritual_confirm(g: &mut Game) -> Result<(), String> {
    if let Overlay::Ritual { step, .. } = &mut g.overlay {
        *step += 1;
    }
    Ok(())
}

/// 输入框确认：取输入值推进解释器
pub fn confirm_input(g: &mut Game) -> Result<(), String> {
    if let Some(ui) = g.input_ui.take() {
        g.interp.input_result(ui.value())?;
    }
    Ok(())
}
