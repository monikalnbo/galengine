//! 音量与速度设置：BGM / SE / 文字速度 三滑条。

use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::config::LOGICAL_W;
use crate::text::font::FontBook;
use crate::ui::theme;
use crate::ui::widgets;

pub const N: usize = 3;

pub fn slider_rects() -> Vec<Rect> {
    crate::ui::geom::menu_column(240, 800, 200, N, 96, 26)
}

#[allow(dead_code)]
pub fn hit(x: f32, y: f32) -> Option<usize> {
    crate::ui::geom::hit(&slider_rects(), x, y)
}

/// 值域：bgm/se 0-100；文字速度显示 0-100（映射 10-90ms，越大越快）
pub fn tw_from_speed(speed: i32) -> f32 {
    90.0 - (speed as f32 / 100.0) * 80.0
}
pub fn speed_from_tw(tw_ms: f32) -> i32 {
    ((90.0 - tw_ms) / 80.0 * 100.0).round() as i32
}

/// 调整：item 0=bgm 1=se 2=文字速度；delta ±5
pub fn adjust(_item: usize, value: i32, delta: i32) -> i32 {
    (value + delta).clamp(0, 100)
}

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    selected: usize,
    bgm: i32,
    se: i32,
    tw_ms: f32,
) -> Result<(), String> {
    canvas.set_draw_color(theme::C_BG);
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 720))?;
    widgets::text_center(canvas, fonts, theme::FS_TITLE, theme::C_TEXT, "音量 · 速度", 66)?;
    let rows = [
        ("BGM 音量", bgm as f32 / 100.0),
        ("SE 音量", se as f32 / 100.0),
        ("文字速度", speed_from_tw(tw_ms) as f32 / 100.0),
    ];
    for (i, (label, v01)) in rows.iter().enumerate() {
        widgets::slider(canvas, fonts, slider_rects()[i], label, *v01, i == selected)?;
        // 数值
        widgets::text_center(
            canvas,
            fonts,
            theme::FS_TINY,
            theme::C_TEXT_FAINT,
            &format!("{}", (v01 * 100.0).round() as i32),
            slider_rects()[i].y + 48,
        )?;
    }
    widgets::text_center(
        canvas,
        fonts,
        theme::FS_SMALL,
        theme::C_TEXT_FAINT,
        "←→ 调整 ｜ ↑↓ 切换 ｜ Esc 返回",
        620,
    )?;
    Ok(())
}
