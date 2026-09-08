//! L1 一帧渲染组装：演出层 → 对话/选项/输入 → 覆盖层 → letterbox 贴窗
//! → 越界层（画进黑边）→ 提示 toast → present。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::systems::Game;
use gal_config::LOGICAL_H;
use gal_render::assets::TextureBank;
use gal_render::renderer::Renderer;
use gal_script::interp::RunState;
use gal_text::font::FontBook;
use gal_ui::overlay::{esc_items, Overlay};
use gal_ui::{bottombar, choice, dialog, gallery, inputbox, menu, ritual, savemenu, title};

pub fn frame(
    g: &mut Game,
    fonts: &mut FontBook,
    bank: &mut TextureBank,
    renderer: &mut Renderer,
    canvas: &mut Canvas<Window>,
    thumbs: &mut savemenu::ThumbCache,
) -> Result<(), String> {
    // 抖动偏移：幅度随剩余时间衰减
    let (dx, dy) = match &g.sys.shake {
        Some(s) if s.dur_ms > 0.0 => {
            let amp = 14.0 * (s.left_ms / s.dur_ms);
            let ox = (g.now_ms / 26.0).sin() * amp;
            let oy = (g.now_ms / 31.0).cos() * amp * 0.6;
            (ox as i32, oy as i32)
        }
        _ => (0, 0),
    };

    // 先取走越界层参数（避免与闭包双重借用）
    let reach = g.sys.reach.as_ref().map(|r| {
        let alpha = if r.hiding {
            1.0 - (r.t / 600.0).min(1.0)
        } else {
            (r.t / r.dur_ms.max(1.0)).min(1.0)
        };
        (r.path.clone(), r.scale, alpha)
    });

    let style = &g.style;
    let name = g.interp.cur_name.clone();
    let sel = g.choice_sel;
    let now = g.now_ms;
    let auto = g.auto;

    renderer.compose(canvas, dx, dy, |tc| {
        if g.started {
            gal_render::draw_stage(&g.interp.stage, tc, bank)?;
            match &g.interp.state {
                RunState::WaitChoice { prompt, items } => {
                    choice::draw(tc, fonts, prompt, items, sel)?;
                    for e in &g.exts {
                        e.draw(tc, fonts)?; // 拓展叠加（倒计时条等）
                    }
                    bottombar::draw(tc, fonts, auto, g.ctrl_hold)?;
                }
                RunState::WaitInput(_) => {
                    if let Some(ui) = &g.input_ui {
                        inputbox::draw(tc, fonts, ui)?;
                    }
                }
                _ => {
                    dialog::draw(tc, fonts, style, name.as_deref(), &g.interp.tw, now)?;
                    bottombar::draw(tc, fonts, auto, g.ctrl_hold)?;
                }
            }
        }
        // 覆盖层（标题/菜单/存档/鉴赏/音量/仪式）
        draw_overlay(tc, g, fonts, thumbs, bank)?;
        Ok(())
    })?;

    // 越界层：以游戏区中心放大伸出黑边（窗口坐标）
    if let Some((path, scale, alpha)) = reach {
        if alpha > 0.01 {
            let r = Renderer::letterbox(canvas, 0, 0);
            let w = (r.width() as f32 * scale) as u32;
            let h = (r.height() as f32 * scale) as u32;
            let dst = Rect::new(
                r.x + r.width() as i32 / 2 - w as i32 / 2,
                r.y + r.height() as i32 / 2 - h as i32 / 2,
                w,
                h,
            );
            bank.draw_scaled(canvas, &path, dst, alpha)?;
        }
    }

    // 中文提示 toast（底部居中）
    if let Some(m) = &g.sys.msg {
        canvas.set_blend_mode(BlendMode::Blend);
        let tex = fonts.render_text(26, Color::RGB(255, 224, 160), &m.text)?;
        let q = tex.query();
        let w = q.width + 48;
        let r = Rect::new(
            (canvas.output_size().unwrap().0 as i32 - w as i32) / 2,
            LOGICAL_H as i32 - 60,
            w,
            q.height + 20,
        );
        canvas.set_draw_color(Color::RGBA(10, 12, 26, 220));
        canvas.fill_rect(r)?;
        canvas.copy(tex, None, Some(Rect::new(r.x + 24, r.y + 10, q.width, q.height)))?;
    }

    canvas.present();
    Ok(())
}

/// 覆盖层绘制
#[allow(clippy::too_many_arguments)]
fn draw_overlay(
    tc: &mut Canvas<Window>,
    g: &Game,
    fonts: &mut FontBook,
    thumbs: &savemenu::ThumbCache,
    bank: &mut TextureBank,
) -> Result<(), String> {
    match &g.overlay {
        Overlay::None => {}
        Overlay::Menu { sel } => {
            menu::draw(tc, fonts, &esc_items(), *sel, g.conf.ui.menu.top_y, true)?;
        }
        Overlay::Title { sel, .. } => {
            title::draw(
                tc,
                fonts,
                bank,
                &g.conf,
                &g.interp.vars,
                &g.sys.meta.title_evolve,
                *sel,
                g.now_ms,
            )?;
        }
        Overlay::Save { mode_save, sel } => {
            savemenu::draw(
                tc,
                fonts,
                thumbs,
                &g.save_entries,
                &g.sys.meta.fake_saves,
                &g.sys.meta.corrupt,
                *mode_save,
                *sel,
            )?;
        }
        Overlay::Gallery { sel, page, full } => {
            let items = gallery::catalog();
            let opened = g.interp.vars.get_or("sf.cgs").as_str();
            let unlocked: Vec<String> =
                opened.split(',').filter(|s| !s.is_empty()).map(String::from).collect();
            match full {
                Some(idx) => {
                    if let Some(name) = items.get(*idx) {
                        gallery::draw_full(tc, fonts, bank, name, *idx, unlocked.len())?;
                    }
                }
                None => {
                    gallery::draw_grid(tc, fonts, bank, &items, &unlocked, *page, *sel)?;
                }
            }
        }
        Overlay::Settings { sel } => {
            let items = gal_ui::settings::rows(&g.conf);
            let fs = tc.window().fullscreen_state() != sdl2::video::FullscreenType::Off;
            let ctx = gal_ui::settings::SettingsCtx::from_vars(
                &g.interp.vars,
                g.sys.audio.bgm_vol,
                g.sys.audio.se_vol,
                fs,
            );
            gal_ui::settings::draw(tc, fonts, &items, &ctx, *sel)?;
        }
        Overlay::Ritual { step, fade, .. } => {
            ritual::draw(tc, fonts, &g.conf, *step, *fade)?;
        }
    }
    if let Some(rest) = &g.sys.rest {
        gal_ui::restui::draw(tc, fonts, &g.conf, rest.left_ms)?;
    }
    Ok(())
}
