//! 选项列表：居中竖排半透明条；↑↓+回车 和 鼠标悬停+点击 双通道。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use gal_config::LOGICAL_W;
use gal_text::font::FontBook;

const ITEM_W: u32 = 760;
const ITEM_H: u32 = 66;
const GAP: i32 = 18;
const TOP_Y: i32 = 236;
const FONT_SIZE: u16 = 28;

pub fn item_rect(idx: usize) -> Rect {
    Rect::new(
        (LOGICAL_W as i32 - ITEM_W as i32) / 2,
        TOP_Y + idx as i32 * (ITEM_H as i32 + GAP),
        ITEM_W,
        ITEM_H,
    )
}

/// 鼠标命中（n=选项总数）
pub fn hit_test(n: usize, x: f32, y: f32) -> Option<usize> {
    (0..n).find(|&i| item_rect(i).contains_point((x as i32, y as i32)))
}

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    prompt: &str,
    items: &[(String, String)],
    selected: usize,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    let tex = fonts.render_text(FONT_SIZE, Color::RGB(220, 224, 236), prompt)?;
    let q = tex.query();
    canvas.copy(
        tex,
        None,
        Some(Rect::new((LOGICAL_W as i32 - q.width as i32) / 2, TOP_Y - 56, q.width, q.height)),
    )?;
    for (i, (text, _)) in items.iter().enumerate() {
        let r = item_rect(i);
        let hot = i == selected;
        canvas.set_draw_color(if hot {
            Color::RGBA(64, 84, 140, 225)
        } else {
            Color::RGBA(12, 16, 32, 205)
        });
        canvas.fill_rect(r)?;
        canvas.set_draw_color(if hot {
            Color::RGBA(150, 200, 255, 220)
        } else {
            Color::RGBA(120, 130, 160, 110)
        });
        canvas.draw_rect(r)?;
        let color = if hot { Color::RGB(255, 255, 255) } else { Color::RGB(205, 210, 225) };
        let tex = fonts.render_text(FONT_SIZE, color, text)?;
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
    }
    Ok(())
}
