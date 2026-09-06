//! 音量/速度设置：BGM / SE / 文字速度三滑条（存 sf.volBgm/volSe/textSpeed）。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::config::{LOGICAL_H, LOGICAL_W};
use crate::text::font::FontBook;

const ROWS: i32 = 3;
const FONT: u16 = 26;

fn row_rect(i: i32) -> Rect {
    Rect::new(LOGICAL_W as i32 / 2 - 330, 200 + i * 110, 660, 70)
}

fn hit_test(y: f32) -> Option<usize> {
    (0..ROWS).find(|&i| row_rect(i).contains_point((0, y as i32))).map(|i| i as usize)
}

pub fn hit_row(_lx: f32, ly: f32) -> Option<usize> {
    hit_test(ly)
}

/// values: [bgm 0-100, se 0-100, 文字速度 ms 5-200]
pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    values: [i64; 3],
    sel: usize,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Color::RGBA(4, 6, 16, 235));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, LOGICAL_H))?;

    let tex = fonts.render_text(30, Color::RGB(230, 234, 244), "环境设置")?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new(76, 70, q.width, q.height)))?;

    let labels = [("BGM 音量", 0, 100), ("SE 音量", 0, 100), ("文字速度", 5, 200)];
    for (i, (label, lo, hi)) in labels.iter().enumerate() {
        let r = row_rect(i as i32);
        let hot = i == sel;
        canvas.set_draw_color(if hot { Color::RGBA(60, 80, 140, 230) } else { Color::RGBA(18, 22, 40, 230) });
        canvas.fill_rect(r)?;
        let tex = fonts.render_text(FONT, Color::RGB(220, 224, 236), label)?;
        let q = tex.query();
        canvas.copy(tex, None, Some(Rect::new(r.x + 20, r.y + (r.height() as i32 - q.height as i32) / 2, q.width, q.height)))?;
        // 滑条
        let track = Rect::new(r.x + 230, r.y + (r.height() as i32 - 10) / 2, 300, 10);
        canvas.set_draw_color(Color::RGB(40, 46, 70));
        canvas.fill_rect(track)?;
        let ratio = (values[i] - lo) as f32 / (hi - lo) as f32;
        let fill_w = (ratio * track.width() as f32).clamp(0.0, track.width() as f32) as u32;
        canvas.set_draw_color(if hot { Color::RGB(150, 200, 255) } else { Color::RGB(110, 130, 180) });
        canvas.fill_rect(Rect::new(track.x, track.y, fill_w, track.height()))?;
        let unit = if i == 2 { format!("{}ms", values[i]) } else { values[i].to_string() };
        let tex = fonts.render_text(FONT, Color::RGB(200, 206, 220), &unit)?;
        let q = tex.query();
        canvas.copy(
            tex,
            None,
            Some(Rect::new(track.right() as i32 + 24, r.y + (r.height() as i32 - q.height as i32) / 2, q.width, q.height)),
        )?;
    }
    let tex = fonts.render_text(20, Color::RGB(140, 148, 170), "← → 调整 · Esc 关闭")?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new((LOGICAL_W - q.width) as i32 / 2, 580, q.width, q.height)))
}
