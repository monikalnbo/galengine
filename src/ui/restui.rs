//! 晚安关机倒计时 UI（引擎原语；台词由剧本在 shutdown 指令前用 n/name 演出）。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::config::LOGICAL_W;
use crate::text::font::FontBook;
use crate::ui::theme;
use crate::ui::widgets;

pub fn cancel_rect() -> Rect {
    crate::ui::geom::centered(280, 64, 600)
}

pub fn hit_cancel(x: f32, y: f32) -> bool {
    cancel_rect().contains_point((x as i32, y as i32))
}

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    left_ms: f32,
    total_secs: u32,
    cancelled: bool,
    finished: bool,
) -> Result<(), String> {
    canvas.set_draw_color(Color::RGB(2, 3, 8));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 720))?;

    if cancelled {
        widgets::text_center(canvas, fonts, theme::FS_ITEM, theme::C_TEXT_DIM, "……那，再玩一小会儿哦。", 320)?;
        return Ok(());
    }
    if finished {
        widgets::text_center(canvas, fonts, theme::FS_BIG, theme::C_TEXT, "晚安。", 300)?;
        return Ok(());
    }

    widgets::text_center(canvas, fonts, theme::FS_ITEM, theme::C_TEXT_DIM, "已为你设定关机", 210)?;
    let left = (left_ms / 1000.0).ceil().max(0.0) as u32;
    let m = left / 60;
    let s = left % 60;
    widgets::text_center(
        canvas,
        fonts,
        theme::FS_HUGE,
        theme::C_TEXT,
        &format!("{m:02}:{s:02}"),
        300,
    )?;
    widgets::text_center(
        canvas,
        fonts,
        theme::FS_TINY,
        theme::C_TEXT_FAINT,
        &format!("（共 {total_secs} 秒）"),
        470,
    )?;
    widgets::button(canvas, fonts, cancel_rect(), "取消关机", true)?;
    Ok(())
}
