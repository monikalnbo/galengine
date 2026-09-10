//! 鼠标点击分发：覆盖层命中 → 底栏 → 台词/选项/输入三态。

use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::systems::Game;
use gal_render::renderer::Renderer;
use gal_ui::overlay::{esc_items, title_items, Overlay};
use gal_ui::savemenu::{self, ThumbCache};
use gal_ui::{choice, menu};

use super::{activate, saves, settings_io};

pub fn click(
    g: &mut Game,
    lx: f32,
    ly: f32,
    canvas: &mut Canvas<Window>,
    renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    if g.hide_ui {
        g.hide_ui = false;
        return Ok(true);
    }
    if g.sys.rest.is_some() {
        if gal_ui::restui::cancel_rect().contains_point((lx as i32, ly as i32)) {
            g.cancel_rest();
        }
        return Ok(true);
    }
    if g.overlay.active() {
        match g.overlay.clone() {
            Overlay::Backlog { .. } => {
                g.overlay = Overlay::None;
                return Ok(true);
            }
            Overlay::Menu { .. } => {
                if let Some(i) = menu::hit_test(esc_items().len(), g.conf.ui.menu.top_y, lx, ly) {
                    g.overlay = Overlay::Menu { sel: i };
                    if !activate::menu_activate(g, i, canvas, renderer, thumbs)? {
                        return Ok(false);
                    }
                }
            }
            Overlay::Save { mode_save, .. } => {
                if savemenu::close_rect().contains_point((lx as i32, ly as i32)) {
                    g.overlay = Overlay::None;
                    return Ok(true);
                }
                let n = g.save_entries.len() + g.sys.meta.fake_saves.len();
                if let Some(i) = savemenu::hit_test(n, lx, ly) {
                    g.overlay = Overlay::Save { mode_save, sel: i };
                    saves::save_activate(g, canvas, renderer, thumbs, mode_save, i)?;
                }
            }
            Overlay::Ritual { .. } => {
                if gal_ui::ritual::del_rect().contains_point((lx as i32, ly as i32)) {
                    activate::ritual_confirm(g)?;
                } else if gal_ui::ritual::back_rect().contains_point((lx as i32, ly as i32)) {
                    g.overlay = Overlay::None; // 「回头」路径
                }
            }
            Overlay::Title { .. } => {
                if let Some(i) =
                    menu::hit_test(title_items().len(), g.conf.ui.title.menu_top_y, lx, ly)
                {
                    g.overlay = Overlay::Title { sel: i };
                    if !activate::title_activate(g, i, thumbs)? {
                        return Ok(false);
                    }
                }
            }
            Overlay::Gallery { page, full, .. } => match full {
                Some(_) => g.overlay = Overlay::Gallery { sel: 0, page, full: None },
                None => {
                    if let Some(i) = gal_ui::gallery::hit_test(gal_ui::gallery::per_page(), lx, ly)
                    {
                        g.overlay = Overlay::Gallery { sel: i, page, full: None };
                        activate::gallery_confirm(g);
                    }
                }
            },
            Overlay::Settings { .. } => settings_io::click(g, canvas, lx, ly),
            Overlay::None => {}
        }
        return Ok(true);
    }
    // 底部功能条：自动/快进/存档/读档/设置（输入界面不显示，避免与确认键拥挤）
    if g.started && !matches!(g.interp.state, gal_script::interp::RunState::WaitInput(_)) {
        if let Some(i) = gal_ui::bottombar::hit(lx as i32, ly as i32) {
            return activate::bottombar_activate(g, i, thumbs);
        }
    }
    match &g.interp.state {
        gal_script::interp::RunState::WaitChoice { .. } => {
            // 拓展鼠标事件转发（可拦截）
            let mut handled = false;
            for e in g.exts.iter_mut() {
                let mut ectx = gal_ext::ExtContext::new(&mut g.interp);
                if e.on_click(lx, ly, &mut ectx) == gal_ext::ExtResult::Handled {
                    handled = true;
                }
            }
            if !handled {
                if let Some(i) = choice::hit_test(g.choice_len(), lx, ly) {
                    g.interp.choose(i)?;
                }
            }
        }
        gal_script::interp::RunState::WaitInput(_) => {
            use gal_ui::inputbox::{self, Hit};
            match g.input_ui.as_ref().and_then(|ui| inputbox::hit_test(ui, lx, ly)) {
                Some(Hit::Preset(i)) => {
                    if let Some(ui) = g.input_ui.as_mut() {
                        if let Some(p) = ui.presets.get(i) {
                            ui.buf = p.clone();
                        }
                    }
                }
                Some(Hit::Confirm) => activate::confirm_input(g)?,
                None => {}
            }
        }
        gal_script::interp::RunState::WaitClick => g.interp.click()?,
        _ => {}
    }
    Ok(true)
}
