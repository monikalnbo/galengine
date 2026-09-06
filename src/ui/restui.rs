//! 晚安关机 UI：大字倒计时 + 取消按钮 + config.shutdown_message。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::config::{Config, LOGICAL_H, LOGICAL_W};
use crate::text::font::FontBook;
use crate::ui::inputbox::button;

pub fn cancel_rect() -> Rect {
    Rect::new((LOGICAL_W - 240) as i32 / 2, 500, 240, 62)
}

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    conf: &Config,
    left_ms: f32,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Color::RGBA(4, 6, 16, 235));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, LOGICAL_H))?;

    let msg = if conf.game.shutdown_message.is_empty() { "晚安。" } else { &conf.game.shutdown_message };
    let tex = fonts.render_text(40, Color::RGB(200, 214, 240), msg)?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new((LOGICAL_W - q.width) as i32 / 2, 240, q.width, q.height)))?;

    let secs = (left_ms / 1000.0).ceil().max(0.0);
    let tex = fonts.render_text(96, Color::RGB(255, 255, 255), &format!("{secs:3.0}"))?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new((LOGICAL_W - q.width) as i32 / 2, 320, q.width, q.height)))?;

    button(canvas, fonts, cancel_rect(), "「再待一会儿」", false)?;
    Ok(())
}
