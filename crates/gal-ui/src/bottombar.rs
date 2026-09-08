//! 对话框下方功能条：自动 / 快进 / 存档 / 读档 / 设置。
//! 鼠标党直达（键盘 A/Ctrl/F9/F10/Esc 依旧全有效）；自动/快进高亮指示当前状态。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use gal_text::font::FontBook;

pub const LABELS: [&str; 5] = ["自动", "快进", "存档", "读档", "设置"];
const BW: u32 = 92;
const BH: u32 = 28;
const GAP: u32 = 10;
/// 对话框底边（514+172=686）之下的细条
const Y: i32 = 689;
/// 与对话框右缘对齐（56+1168=1224）
const RIGHT: i32 = 1224;

pub fn item_rect(i: usize) -> Rect {
    let total = LABELS.len() as i32 * BW as i32 + (LABELS.len() - 1) as i32 * GAP as i32;
    Rect::new(RIGHT - total + i as i32 * (BW + GAP) as i32, Y, BW, BH)
}

pub fn hit(x: i32, y: i32) -> Option<usize> {
    (0..LABELS.len()).find(|&i| item_rect(i).contains_point((x, y)))
}

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    auto_on: bool,
    ff_on: bool,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    for (i, label) in LABELS.iter().enumerate() {
        let r = item_rect(i);
        let on = (i == 0 && auto_on) || (i == 1 && ff_on);
        canvas.set_draw_color(if on {
            Color::RGBA(52, 74, 128, 235)
        } else {
            Color::RGBA(28, 34, 58, 215)
        });
        canvas.fill_rect(r)?;
        canvas.set_draw_color(if on {
            Color::RGBA(160, 200, 255, 220)
        } else {
            Color::RGBA(120, 138, 185, 150)
        });
        canvas.draw_rect(r)?;
        let tex = fonts.render_text(
            19,
            if on { Color::RGB(255, 255, 255) } else { Color::RGB(205, 212, 230) },
            label,
        )?;
        let q = tex.query();
        canvas.copy(
            tex,
            None,
            Some(Rect::new(
                r.x + (BW as i32 - q.width as i32) / 2,
                r.y + (BH as i32 - q.height as i32) / 2,
                q.width,
                q.height,
            )),
        )?;
    }
    Ok(())
}
