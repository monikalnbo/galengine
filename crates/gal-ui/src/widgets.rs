//! 通用 UI 小部件：半透明按钮与几何居中对齐工具

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use gal_config::LOGICAL_W;
use gal_text::font::FontBook;

const FONT: u16 = 30;

/// 半透明通用按钮（choice/预设/确认/菜单/存档通用原语）
pub fn button(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    r: Rect,
    label: &str,
    accent: bool,
) -> Result<(), String> {
    canvas.set_draw_color(if accent {
        Color::RGBA(52, 74, 128, 235)
    } else {
        Color::RGBA(28, 34, 58, 220)
    });
    canvas.fill_rect(r)?;
    canvas.set_draw_color(if accent {
        Color::RGBA(160, 200, 255, 220)
    } else {
        Color::RGBA(130, 150, 200, 150)
    });
    canvas.draw_rect(r)?;
    let tex = fonts.render_text(
        FONT,
        if accent { Color::RGB(255, 255, 255) } else { Color::RGB(210, 216, 232) },
        label,
    )?;
    let q = tex.query();
    canvas.copy(tex, None, Some(centered_in(r, q.width, q.height)))
}

pub fn centered_in(r: Rect, w: u32, h: u32) -> Rect {
    Rect::new(
        r.x + (r.width() as i32 - w as i32) / 2,
        r.y + (r.height() as i32 - h as i32) / 2,
        w,
        h,
    )
}

pub fn centered_h(w: u32, h: u32, y: i32) -> Rect {
    Rect::new((LOGICAL_W as i32 - w as i32) / 2, y, w, h)
}
