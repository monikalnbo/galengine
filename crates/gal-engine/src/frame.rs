//! L1 一帧渲染组装：演出层 → 对话/选项/输入 → 覆盖层 → letterbox 贴窗
//! → 越界层（画进黑边）→ 提示 toast → present。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::systems::Game;
use gal_render::assets::TextureBank;
use gal_render::renderer::Renderer;
use gal_script::interp::RunState;
use gal_text::font::FontBook;
use gal_ui::overlay::{esc_items, Overlay};
use gal_ui::{bottombar, choice, dialog, gallery, inputbox, menu, ritual, savemenu, title};

/// 帧附加输入：视频层所需（`video` feature 关闭时仅作透传占位）
pub struct FrameExt<'a> {
    pub dt_ms: f32,
    pub creator: &'a sdl2::render::TextureCreator<sdl2::video::WindowContext>,
}

pub fn frame(
    g: &mut Game,
    fonts: &mut FontBook,
    bank: &mut TextureBank,
    renderer: &mut Renderer,
    canvas: &mut Canvas<Window>,
    thumbs: &mut savemenu::ThumbCache,
    _ext: FrameExt<'_>,
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
    let skin = g.skin.clone();
    let name = g.interp.cur_name.clone();
    let sel = g.choice_sel.min(g.choice_len().saturating_sub(1));
    // 选项/提示文本过语言包（{@键}）；旧剧本无宏 → 原样
    let choice_loc: Option<(String, Vec<(String, String)>)> = match &g.interp.state {
        RunState::WaitChoice { prompt, items } => {
            let lp = g.interp.localize(prompt);
            let li: Vec<(String, String)> =
                items.iter().map(|(t, l)| (g.interp.localize(t), l.clone())).collect();
            if gal_script::interp::dbg()
                && li.first().map(|(t, _)| t != &items[0].0).unwrap_or(false)
            {
                // 同一组选项只打一次（等待选择时每帧都会走到这里）
                let mut h: u64 = 0xcbf2_9ce4_8422_2325;
                for (t, _) in &li {
                    for b in t.as_bytes() {
                        h ^= *b as u64;
                        h = h.wrapping_mul(0x100_0000_01b3);
                    }
                }
                if g.lang_logged != h {
                    g.lang_logged = h;
                    eprintln!(
                        "[dbg lang] 选项本地化：{}",
                        li.first().map(|(t, _)| t.as_str()).unwrap_or("")
                    );
                }
            }
            Some((lp, li))
        }
        _ => None,
    };
    let now = g.now_ms;
    let auto = g.auto;

    // 视频层（`video` feature）：闭包外 tick + 取帧拷贝，避免与 g 只读借用冲突。
    // 拷贝一帧 RGBA（~2.7MB @1280x720，30fps 下开销可接受）换借用解耦
    #[cfg(feature = "video")]
    let video_frame: Option<(Vec<u8>, usize, usize)> = {
        let v = &mut g.sys.video;
        if v.playing() && v.tick(_ext.dt_ms) {
            v.frame_rgba().map(|(d, w, h)| (d.to_vec(), w, h))
        } else {
            None
        }
    };

    renderer.compose(canvas, dx, dy, |tc| {
        // 有新帧 → 满帧绘制，盖在演出层之上
        #[cfg(feature = "video")]
        if let Some((data, w, h)) = &video_frame {
            gal_render::video::blit_frame(data, *w, *h, _ext.creator, tc)?;
        }
        if g.started {
            gal_render::draw_stage(&g.interp.stage, tc, bank)?;
            if !g.hide_ui {
                match &g.interp.state {
                    RunState::WaitChoice { prompt, items } => {
                        let (lprompt, litems) = choice_loc
                            .as_ref()
                            .map(|(p, i)| (p.as_str(), i.as_slice()))
                            .unwrap_or((prompt, items.as_slice()));
                        choice::draw(
                            tc,
                            fonts,
                            &g.conf.ui.choice,
                            &g.conf.ui.button,
                            lprompt,
                            litems,
                            sel,
                        )?;
                        for e in &g.exts {
                            e.draw(tc, fonts)?; // 拓展叠加（倒计时条等）
                        }
                        bottombar::draw(tc, fonts, &g.conf.ui.bottombar, &skin, auto, g.ctrl_hold)?;
                    }
                    RunState::WaitInput(_) => {
                        if let Some(ui) = &g.input_ui {
                            inputbox::draw(tc, fonts, ui, &g.conf)?;
                        }
                    }
                    _ => {
                        dialog::draw(tc, fonts, style, name.as_deref(), &g.interp.tw, now)?;
                        bottombar::draw(tc, fonts, &g.conf.ui.bottombar, &skin, auto, g.ctrl_hold)?;
                    }
                }
            }
        }
        // 覆盖层（标题/菜单/存档/鉴赏/设置/仪式）
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

    // 中文提示 toast（底部居中，按窗口实际分辨率对齐）
    if let Some(m) = &g.sys.msg {
        canvas.set_blend_mode(BlendMode::Blend);
        let tex = fonts.render_text(26, Color::RGB(255, 224, 160), &m.text)?;
        let q = tex.query();
        let w = q.width + 48;
        let toast_h = q.height + 20;
        let (ww, wh) =
            canvas.output_size().unwrap_or((gal_config::LOGICAL_W, gal_config::LOGICAL_H));
        let r = Rect::new((ww as i32 - w as i32) / 2, wh as i32 - toast_h as i32 - 24, w, toast_h);
        canvas.set_draw_color(Color::RGBA(10, 12, 26, 220));
        canvas.fill_rect(r)?;
        canvas.copy(tex, None, Some(Rect::new(r.x + 24, r.y + 10, q.width, q.height)))?;
    }

    // 左下角小字提示（语音被打断等；不遮挡对话框）
    if let Some(m) = &g.sys.voice_hint {
        canvas.set_blend_mode(BlendMode::Blend);
        let a = ((m.left_ms / 1500.0).clamp(0.0, 1.0) * 220.0) as u8;
        let tex = fonts.render_text(20, Color::RGBA(210, 222, 255, a), &m.text)?;
        let q = tex.query();
        let (_, wh) =
            canvas.output_size().unwrap_or((gal_config::LOGICAL_W, gal_config::LOGICAL_H));
        canvas.copy(
            tex,
            None,
            Some(Rect::new(18, wh as i32 - q.height as i32 - 18, q.width, q.height)),
        )?;
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
            // 系统菜单文字过语言包
            let items: Vec<String> =
                esc_items(&g.conf).iter().map(|t| g.interp.localize(t)).collect();
            menu::draw(
                tc,
                fonts,
                &g.conf.ui.menu,
                &g.conf.ui.button,
                &items,
                *sel,
                g.conf.ui.menu.top_y,
                true,
            )?;
        }
        Overlay::Title { sel, t } => {
            title::draw(
                tc,
                fonts,
                bank,
                &g.conf,
                &g.interp.vars,
                &g.sys.meta.title_evolve,
                *sel,
                g.now_ms,
                *t,
                &g.lang.strings,
            )?;
        }
        Overlay::Save { mode_save, sel } => {
            savemenu::draw(
                tc,
                fonts,
                thumbs,
                &g.conf,
                &g.save_entries,
                &g.sys.meta.fake_saves,
                &g.sys.meta.corrupt,
                *mode_save,
                *sel,
                g.interp.snapshot_lock.as_deref(),
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
            let env = crate::input::settings_io::env(g);
            let items = gal_ui::settings::rows(&g.conf, &env);
            let fs = tc.window().fullscreen_state() != sdl2::video::FullscreenType::Off;
            let ctx =
                gal_ui::settings::SettingsCtx::from_vars(&g.interp.vars, &g.conf, &g.keymap(), fs);
            gal_ui::settings::draw(tc, fonts, &items, &ctx, *sel, g.rebind.is_some())?;
        }
        Overlay::Ritual { step, fade, .. } => {
            ritual::draw(tc, fonts, &g.conf, *step, *fade)?;
        }
        Overlay::Backlog { scroll } => {
            gal_ui::backlog::draw(tc, fonts, &g.style, &g.interp.backlog, *scroll)?;
        }
    }
    if let Some(rest) = &g.sys.rest {
        gal_ui::restui::draw(tc, fonts, &g.conf, rest.left_ms)?;
    }
    Ok(())
}
