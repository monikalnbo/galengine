//! UI 控件原语：面板/文本/按钮/滑条（只画不记忆状态，状态归各界面模块）。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::text::font::FontBook;
use crate::ui::theme;

/// 面板：半透明底+边框（hot=选中态）
pub fn panel(canvas: &mut Canvas<Window>, r: Rect, hot: bool) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(if hot { theme::C_PANEL_HOT } else { theme::C_PANEL });
    canvas.fill_rect(r)?;
    canvas.set_draw_color(if hot { theme::C_BORDER_HOT } else { theme::C_BORDER });
    canvas.draw_rect(r)?;
    Ok(())
}

/// 居中文本（水平居中，y 为顶）
pub fn text_center(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    size: u16,
    color: Color,
    text: &str,
    y: i32,
) -> Result<(), String> {
    let tex = fonts.render_text(size, color, text)?;
    let q = tex.query();
    canvas.copy(tex, None, Some(crate::ui::geom::centered(q.width, q.height, y)))?;
    Ok(())
}

/// 左对齐文本
pub fn text_left(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    size: u16,
    color: Color,
    text: &str,
    x: i32,
    y: i32,
) -> Result<(), String> {
    let tex = fonts.render_text(size, color, text)?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new(x, y, q.width, q.height)))?;
    Ok(())
}

/// 按钮矩形（面板+居中文字）
pub fn button(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    r: Rect,
    label: &str,
    hot: bool,
) -> Result<(), String> {
    panel(canvas, r, hot)?;
    let color = if hot { theme::C_TEXT } else { theme::C_TEXT_DIM };
    let tex = fonts.render_text(theme::FS_ITEM, color, label)?;
    let q = tex.query();
    canvas.copy(
        tex,
        None,
        Some(Rect::new(
            r.x + (r.width() as i32 - q.width as i32) / 2,
            r.y + (r.height() as i32 - q.height as i32) / 2,
            q.width,
            q.height,
        )),
    )?;
    Ok(())
}

/// 滑条：标签 + 轨道 + 填充（value 0..1）
pub fn slider(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    r: Rect,
    label: &str,
    value01: f32,
    hot: bool,
) -> Result<(), String> {
    panel(canvas, r, hot)?;
    text_left(
        canvas,
        fonts,
        theme::FS_SMALL,
        if hot { theme::C_TEXT } else { theme::C_TEXT_DIM },
        label,
        r.x + 24,
        r.y + 16,
    )?;
    // 轨道
    let track = Rect::new(r.x + 280, r.y + r.height() as i32 - 34, r.width() - 320, 10);
    canvas.set_draw_color(theme::C_BORDER);
    canvas.fill_rect(track)?;
    let fill = Rect::new(track.x, track.y, ((track.width() as f32 * value01) as u32).max(2), 10);
    canvas.set_draw_color(if hot { theme::C_ACCENT } else { theme::C_TEXT_DIM });
    canvas.fill_rect(fill)?;
    Ok(())
}
