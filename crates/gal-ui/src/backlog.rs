//! 对白历史（Backlog）：全屏半透明回卷面板，滚轮/上下键滚动，Esc/右键/点击返回。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use gal_config::{DialogStyle, LOGICAL_H, LOGICAL_W};
use gal_script::BacklogItem;
use gal_text::font::FontBook;

const ITEMS_PER_PAGE: usize = 6;
const PANEL_PAD_X: i32 = 80;
const ITEM_H: i32 = 90;

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    style: &DialogStyle,
    backlog: &[BacklogItem],
    scroll: usize,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);

    // 全屏半透明背景
    canvas.set_draw_color(Color::RGBA(12, 14, 24, 235));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, LOGICAL_H))?;

    // 顶栏标题与操作提示
    let header_tex = fonts.render_text(24, Color::RGB(200, 210, 230), "对白历史 (Backlog)")?;
    canvas.copy(
        header_tex,
        None,
        Some(Rect::new(PANEL_PAD_X, 36, header_tex.query().width, header_tex.query().height)),
    )?;

    let hint_tex = fonts.render_text(18, Color::RGB(140, 150, 170), "滚轮 / ↑↓ 滚动 ｜ 点击 / Esc 返回")?;
    canvas.copy(
        hint_tex,
        None,
        Some(Rect::new(
            LOGICAL_W as i32 - PANEL_PAD_X - hint_tex.query().width as i32,
            40,
            hint_tex.query().width,
            hint_tex.query().height,
        )),
    )?;

    // 分割线
    canvas.set_draw_color(Color::RGBA(255, 255, 255, 40));
    canvas.draw_line((PANEL_PAD_X, 72), (LOGICAL_W as i32 - PANEL_PAD_X, 72))?;

    if backlog.is_empty() {
        let empty_tex = fonts.render_text(22, Color::RGB(120, 130, 150), "暂无历史对白")?;
        let q = empty_tex.query();
        canvas.copy(
            empty_tex,
            None,
            Some(Rect::new((LOGICAL_W as i32 - q.width as i32) / 2, 320, q.width, q.height)),
        )?;
        return Ok(());
    }

    // 计算切片：scroll 为距离底部的偏移（0 = 最新）
    let total = backlog.len();
    let max_scroll = total.saturating_sub(ITEMS_PER_PAGE);
    let cur_scroll = scroll.min(max_scroll);
    let start = total.saturating_sub(ITEMS_PER_PAGE + cur_scroll);
    let end = (start + ITEMS_PER_PAGE).min(total);

    let mut y = 96;
    for item in &backlog[start..end] {
        // 角色名
        if let Some(name) = &item.name {
            let [r, g, b] = style.name_color(name);
            let name_tex = fonts.render_text(20, Color::RGB(r, g, b), name)?;
            let q = name_tex.query();
            canvas.copy(name_tex, None, Some(Rect::new(PANEL_PAD_X, y, q.width, q.height)))?;
        }
        // 正文文本
        let text_y = if item.name.is_some() { y + 26 } else { y + 6 };
        let text_tex = fonts.render_text(style.font_size, Color::RGB(230, 230, 235), &item.text)?;
        let tq = text_tex.query();
        canvas.copy(text_tex, None, Some(Rect::new(PANEL_PAD_X + 16, text_y, tq.width, tq.height)))?;

        // 项分隔轻微虚线
        canvas.set_draw_color(Color::RGBA(255, 255, 255, 20));
        canvas.draw_line(
            (PANEL_PAD_X, y + ITEM_H - 8),
            (LOGICAL_W as i32 - PANEL_PAD_X, y + ITEM_H - 8),
        )?;

        y += ITEM_H;
    }

    // 滚动条槽与滑块
    if total > ITEMS_PER_PAGE {
        let bar_x = LOGICAL_W as i32 - PANEL_PAD_X + 24;
        let bar_y = 96;
        let bar_h = (ITEMS_PER_PAGE as i32 * ITEM_H) - 16;
        canvas.set_draw_color(Color::RGBA(40, 45, 65, 200));
        canvas.fill_rect(Rect::new(bar_x, bar_y, 6, bar_h as u32))?;

        let thumb_h =
            ((ITEMS_PER_PAGE as f32 / total as f32) * bar_h as f32).max(24.0) as u32;
        let progress = if max_scroll > 0 {
            (max_scroll - cur_scroll) as f32 / max_scroll as f32
        } else {
            1.0
        };
        let thumb_y = bar_y + ((bar_h as u32 - thumb_h) as f32 * progress) as i32;

        canvas.set_draw_color(Color::RGBA(180, 195, 230, 200));
        canvas.fill_rect(Rect::new(bar_x, thumb_y, 6, thumb_h))?;
    }

    Ok(())
}
