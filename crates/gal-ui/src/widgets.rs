//! 通用 UI 小部件：半透明按钮与几何居中对齐工具

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use gal_config::{ButtonCfg, LOGICAL_W};
use gal_text::font::FontBook;

/// 半透明通用按钮（choice/预设/确认/菜单/存档通用原语）
/// `style` = 作者 ui.button 配色（tint/accent 两态）
pub fn button(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    r: Rect,
    label: &str,
    size: u16,
    accent: bool,
    style: &ButtonCfg,
) -> Result<(), String> {
    let (fill, border, tc) = if accent {
        (style.fill_accent, style.border_accent, style.text_accent)
    } else {
        (style.fill_normal, style.border_normal, style.text_normal)
    };
    canvas.set_draw_color(Color::RGBA(fill[0], fill[1], fill[2], fill[3]));
    canvas.fill_rect(r)?;
    canvas.set_draw_color(Color::RGBA(border[0], border[1], border[2], border[3]));
    for i in 0..style.border_width.max(1) as i32 {
        let rr = Rect::new(r.x + i, r.y + i, r.width() - 2 * i as u32, r.height() - 2 * i as u32);
        canvas.draw_rect(rr)?;
    }
    let tex = fonts.render_text(size, Color::RGB(tc[0], tc[1], tc[2]), label)?;
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
