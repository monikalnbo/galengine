//! 设置菜单（绘制层）：行/滑条/开关渲染 + 命中检测。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use gal_config::LOGICAL_W;
use gal_text::font::FontBook;

use super::{SettingKind, SettingRow, SettingsCtx};

pub const FONT: u16 = 26;
const SMALL: u16 = 20;
const ROW_H: i32 = 64;
const GAP: i32 = 16;
const TOP_Y: i32 = 120;
const LABEL_X: i32 = 140;
const BAR_X: i32 = 480;
const BAR_W: u32 = 500;
const BAR_H: u32 = 12;
const SLIDER_R: i32 = 10;

fn row_w() -> u32 {
    LOGICAL_W - 200
}

fn row_y(i: usize) -> i32 {
    TOP_Y + i as i32 * (ROW_H + GAP)
}

/// 命中检测（返回行号）
pub fn hit_test(n: usize, x: f32, y: f32) -> Option<usize> {
    (0..n).find(|&i| {
        Rect::new(100, row_y(i), row_w(), ROW_H as u32).contains_point((x as i32, y as i32))
    })
}

/// 第 row 行滑条命中（返回 0.0-1.0 位置）
pub fn slider_hit(row: usize, x: f32, y: f32) -> Option<f32> {
    let r = Rect::new(BAR_X - SLIDER_R, row_y(row), BAR_W + SLIDER_R as u32 * 2, ROW_H as u32);
    r.contains_point((x as i32, y as i32))
        .then(|| ((x as i32 - BAR_X) as f32 / BAR_W as f32).clamp(0.0, 1.0))
}

/// 绘制设置界面
pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    items: &[SettingRow],
    ctx: &SettingsCtx,
    sel: usize,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Color::RGBA(4, 6, 16, 235));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 720))?;

    let tex = fonts.render_text(32, Color::RGB(230, 234, 244), "设置")?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new(76, 50, q.width, q.height)))?;

    for (i, item) in items.iter().enumerate() {
        let y = row_y(i);
        let hot = i == sel;

        canvas.set_draw_color(if hot {
            Color::RGBA(30, 40, 70, 200)
        } else {
            Color::RGBA(16, 20, 36, 180)
        });
        canvas.fill_rect(Rect::new(100, y, row_w(), ROW_H as u32))?;

        let tex = fonts.render_text(
            FONT,
            if hot { Color::RGB(255, 255, 255) } else { Color::RGB(200, 208, 224) },
            &item.label,
        )?;
        let q = tex.query();
        canvas.copy(
            tex,
            None,
            Some(Rect::new(LABEL_X, y + (ROW_H - q.height as i32) / 2, q.width, q.height)),
        )?;

        match &item.kind {
            SettingKind::Slider { min, max, get, fmt, .. } => {
                let val = get(ctx);
                let pct = (val - min) as f32 / (max - min) as f32;
                let bar_y = y + (ROW_H - BAR_H as i32) / 2;

                canvas.set_draw_color(Color::RGBA(50, 56, 80, 200));
                canvas.fill_rect(Rect::new(BAR_X, bar_y, BAR_W, BAR_H))?;

                let fill_w = (BAR_W as f32 * pct) as u32;
                canvas.set_draw_color(Color::RGBA(100, 160, 255, 220));
                canvas.fill_rect(Rect::new(BAR_X, bar_y, fill_w, BAR_H))?;

                let sx = BAR_X + (BAR_W as f32 * pct) as i32;
                canvas.set_draw_color(Color::RGB(220, 230, 250));
                canvas.fill_rect(Rect::new(sx - 4, bar_y - 6, 8, BAR_H + 12))?;

                let txt = fmt(val);
                let tex = fonts.render_text(SMALL, Color::RGB(160, 170, 190), &txt)?;
                let q = tex.query();
                canvas.copy(
                    tex,
                    None,
                    Some(Rect::new(
                        BAR_X + BAR_W as i32 + 24,
                        y + (ROW_H - q.height as i32) / 2,
                        q.width,
                        q.height,
                    )),
                )?;
            }
            SettingKind::Toggle { get, .. } => {
                let on = get(ctx);
                let txt = if on { "开" } else { "关" };
                let color = if on { Color::RGB(120, 220, 140) } else { Color::RGB(120, 130, 150) };
                let tex = fonts.render_text(FONT, color, txt)?;
                let q = tex.query();
                canvas.copy(
                    tex,
                    None,
                    Some(Rect::new(
                        BAR_X + 40,
                        y + (ROW_H - q.height as i32) / 2,
                        q.width,
                        q.height,
                    )),
                )?;
            }
            SettingKind::Action { label, .. } => {
                let tex = fonts.render_text(FONT, Color::RGB(255, 180, 120), label)?;
                let q = tex.query();
                canvas.copy(
                    tex,
                    None,
                    Some(Rect::new(BAR_X, y + (ROW_H - q.height as i32) / 2, q.width, q.height)),
                )?;
            }
        }
    }
    let tex = fonts.render_text(
        SMALL,
        Color::RGB(140, 148, 170),
        "↑↓ 选择 · ←→ 调整 · 回车 切换 · Esc 关闭",
    )?;
    let q = tex.query();
    canvas.copy(
        tex,
        None,
        Some(Rect::new((LOGICAL_W - q.width) as i32 / 2, 664, q.width, q.height)),
    )?;
    Ok(())
}
