//! 设置菜单（绘制层）：分组 + 分页的行渲染、滑条/开关/枚举/键位 + 命中检测。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use gal_config::LOGICAL_W;
use gal_text::font::FontBook;

use super::{SettingKind, SettingRow, SettingsCtx};

pub const FONT: u16 = 24;
const SMALL: u16 = 18;
const TITLE: u16 = 30;
const ROW_H: i32 = 46;
const GAP: i32 = 4;
const TOP_Y: i32 = 96;
const ROW_X: i32 = 100;
const LABEL_X: i32 = 140;
const BAR_X: i32 = 470;
const BAR_W: u32 = 460;
const BAR_H: u32 = 10;
const SLIDER_R: i32 = 12;
const VALUE_X: i32 = 990;

/// 每页行数（按 720 高排版：96 + 11×(46+4) = 646，留底部呼吸）
pub const PER_PAGE: usize = 11;

fn row_w() -> u32 {
    LOGICAL_W - 200
}

/// 屏幕内第 i 行的 y 坐标
pub fn row_y(i: usize) -> i32 {
    TOP_Y + i as i32 * (ROW_H + GAP)
}

/// 第 page 页第 i 行 → 全局行号
pub fn global_index(page: usize, i: usize) -> usize {
    page * PER_PAGE + i
}

/// 页首行（选择变化时用于跟页）
pub fn page_of(index: usize) -> usize {
    index / PER_PAGE
}

/// 命中检测（屏幕内行号；调用方加页偏移）
pub fn hit_test(n: usize, x: f32, y: f32) -> Option<usize> {
    (0..n.min(PER_PAGE)).find(|&i| {
        Rect::new(ROW_X, row_y(i), row_w(), ROW_H as u32).contains_point((x as i32, y as i32))
    })
}

/// 第 row 行滑条命中（返回 0.0-1.0 位置）
pub fn slider_hit(row: usize, x: f32, y: f32) -> Option<f32> {
    let r = Rect::new(BAR_X - SLIDER_R, row_y(row), BAR_W + SLIDER_R as u32 * 2, ROW_H as u32);
    r.contains_point((x as i32, y as i32))
        .then(|| ((x as i32 - BAR_X) as f32 / BAR_W as f32).clamp(0.0, 1.0))
}

fn text(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    size: u16,
    color: Color,
    s: &str,
    x: i32,
    y: i32,
) -> Result<(), String> {
    let tex = fonts.render_text(size, color, s)?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new(x, y, q.width, q.height)))?;
    Ok(())
}

/// 绘制设置界面；`page` 由选择行推出（调用方保证）
pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    items: &[SettingRow],
    ctx: &SettingsCtx,
    sel: usize,
    rebinding: bool,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Color::RGBA(4, 6, 16, 235));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 720))?;

    text(canvas, fonts, TITLE, Color::RGB(230, 234, 244), "设置", 76, 46)?;
    let page = page_of(sel);
    let pages = items.len().div_ceil(PER_PAGE).max(1);
    let page_s = format!("{}/{}", page + 1, pages);
    text(canvas, fonts, SMALL, Color::RGB(130, 140, 165), &page_s, 1100, 56)?;

    let start = page * PER_PAGE;
    let mut last_group = "";
    for (i, item) in items.iter().skip(start).take(PER_PAGE).enumerate() {
        let idx = start + i;
        let y = row_y(i);
        // 分组标题：本页首行或换组时画在行尾右侧
        if item.group != last_group {
            last_group = item.group;
            text(
                canvas,
                fonts,
                SMALL,
                Color::RGB(110, 122, 150),
                item.group,
                ROW_X + 6,
                y + ROW_H - 20,
            )?;
        }
        let hot = idx == sel;

        canvas.set_draw_color(if hot {
            Color::RGBA(30, 40, 70, 200)
        } else {
            Color::RGBA(16, 20, 36, 180)
        });
        canvas.fill_rect(Rect::new(ROW_X, y, row_w(), ROW_H as u32))?;

        let label_color = if hot { Color::RGB(255, 255, 255) } else { Color::RGB(200, 208, 224) };
        text(canvas, fonts, FONT, label_color, &item.label, LABEL_X, y + (ROW_H - 28) / 2)?;

        match &item.kind {
            SettingKind::Slider { min, max, get, fmt, .. } => {
                let val = get(ctx);
                let pct = ((val - min) as f32 / (max - min).max(1) as f32).clamp(0.0, 1.0);
                let bar_y = y + (ROW_H - BAR_H as i32) / 2;

                canvas.set_draw_color(Color::RGBA(50, 56, 80, 200));
                canvas.fill_rect(Rect::new(BAR_X, bar_y, BAR_W, BAR_H))?;

                let fill_w = (BAR_W as f32 * pct) as u32;
                canvas.set_draw_color(Color::RGBA(100, 160, 255, 220));
                canvas.fill_rect(Rect::new(BAR_X, bar_y, fill_w, BAR_H))?;

                let sx = BAR_X + (BAR_W as f32 * pct) as i32;
                canvas.set_draw_color(Color::RGB(220, 230, 250));
                canvas.fill_rect(Rect::new(sx - 4, bar_y - 6, 8, BAR_H + 12))?;

                text(canvas, fonts, SMALL, Color::RGB(160, 170, 190), &fmt(val), VALUE_X, y + 16)?;
            }
            SettingKind::Toggle { get, .. } => {
                let on = get(ctx);
                let txt = if on { "开" } else { "关" };
                let color = if on { Color::RGB(120, 220, 140) } else { Color::RGB(120, 130, 150) };
                text(canvas, fonts, FONT, color, txt, BAR_X + 20, y + 12)?;
            }
            SettingKind::Enum { labels, get, .. } => {
                let v = get(ctx).max(0) as usize;
                let label = labels.get(v).cloned().unwrap_or_default();
                let txt = if labels.len() > 1 { format!("‹ {label} ›") } else { label };
                text(canvas, fonts, FONT, Color::RGB(180, 200, 240), &txt, BAR_X, y + 12)?;
            }
            SettingKind::Rebind { action, .. } => {
                let keys = if rebinding && hot {
                    "按任意键…".to_string()
                } else {
                    let shown = ctx.key_of(action);
                    if shown.is_empty() {
                        "未设置".into()
                    } else {
                        shown
                    }
                };
                let color = if rebinding && hot {
                    Color::RGB(255, 200, 120)
                } else {
                    Color::RGB(180, 190, 215)
                };
                text(canvas, fonts, FONT, color, &keys, BAR_X, y + 12)?;
            }
            SettingKind::Action { label, .. } => {
                text(canvas, fonts, FONT, Color::RGB(255, 180, 120), label, BAR_X, y + 12)?;
            }
        }
    }
    let hint = if rebinding {
        "按要绑定的键 · Esc 取消"
    } else {
        "↑↓ 选择 · ←→ 调整 · 回车 切换/重绑 · Esc 关闭"
    };
    text(canvas, fonts, SMALL, Color::RGB(140, 148, 170), hint, 200, 664)?;
    Ok(())
}
