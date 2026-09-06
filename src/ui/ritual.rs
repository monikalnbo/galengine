//! 删档仪式 UI 原语：三重确认框 + 光页渐白（文案由游戏侧配置，引擎只提供演出件）。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::config::LOGICAL_W;
use crate::text::font::FontBook;
use crate::ui::theme;
use crate::ui::widgets;

pub fn box_rect() -> Rect {
    crate::ui::geom::centered(760, 300, 200)
}

pub fn btn_rects() -> (Rect, Rect) {
    let b = box_rect();
    (
        Rect::new(b.x + 60, b.bottom() - 90, 280, 62),
        Rect::new(b.right() - 340, b.bottom() - 90, 280, 62),
    )
}

pub fn hit_btns(x: f32, y: f32) -> Option<bool> {
    let (yes, no) = btn_rects();
    if yes.contains_point((x as i32, y as i32)) {
        Some(true)
    } else if no.contains_point((x as i32, y as i32)) {
        Some(false)
    } else {
        None
    }
}

/// 三重确认框：文案（含 {you} 已替换）+ 步数指示 + 确认/回头
pub fn draw_confirm(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    text: &str,
    step: u8,
    you: &str,
    sel_yes: bool,
) -> Result<(), String> {
    // 压暗
    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Color::RGBA(0, 0, 0, 170));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 720))?;
    let b = box_rect();
    widgets::panel(canvas, b, false)?;
    let text = text.replace("{you}", you);
    widgets::text_center(canvas, fonts, theme::FS_ITEM, theme::C_TEXT, &text, b.y + 56)?;
    widgets::text_center(
        canvas,
        fonts,
        theme::FS_TINY,
        theme::C_TEXT_FAINT,
        &format!("确认 {}/3", step + 1),
        b.y + 130,
    )?;
    let (yes, no) = btn_rects();
    widgets::button(canvas, fonts, yes, "按下，删除", sel_yes)?;
    widgets::button(canvas, fonts, no, "回 头", !sel_yes)?;
    Ok(())
}

/// 光页渐白（progress 0..1）
pub fn draw_flash(canvas: &mut Canvas<Window>, progress: f32) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    let a = (progress.clamp(0.0, 1.0) * 255.0) as u8;
    canvas.set_draw_color(Color::RGBA(250, 250, 255, a));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 720))?;
    Ok(())
}
