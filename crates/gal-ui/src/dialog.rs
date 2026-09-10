//! 底部对话框：半透明盒 + 名字牌（角色色查 config.game.name_colors）+ 打字机正文。
//! DialogStyle 已纯数据化迁至 gal-config::style（Rect→[i32;4]），此处仅 re-export。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use gal_text::font::FontBook;
use gal_text::writer::Typewriter;

/// 对话框样式（数据层在 gal-config::style，断行与绘制共用）
pub use gal_config::DialogStyle;

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    style: &DialogStyle,
    name: Option<&str>,
    tw: &Typewriter,
    now_ms: f32,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);

    // 纯数据 box_rect: [x, y, w, h]
    let b = style.box_rect;
    let br = Rect::new(b[0], b[1], b[2] as u32, b[3] as u32);

    // 盒子：半透明深底 + 细边
    canvas.set_draw_color(Color::RGBA(10, 12, 26, 205));
    canvas.fill_rect(br)?;
    canvas.set_draw_color(Color::RGBA(255, 255, 255, 60));
    canvas.draw_rect(br)?;

    // 名字牌：悬于盒子上沿左侧
    if let Some(name) = name.filter(|n| !n.is_empty()) {
        let [cr, cg, cb] = style.name_color(name);
        let color = Color::RGB(cr, cg, cb);
        let tex = fonts.render_text(style.name_font_size, color, name)?;
        let q = tex.query();
        let plate = Rect::new(br.x + 24, br.y - 24, q.width + 44, 46);
        canvas.set_draw_color(Color::RGBA(cr, cg, cb, 56));
        canvas.fill_rect(plate)?;
        canvas.set_draw_color(Color::RGBA(cr, cg, cb, 200));
        canvas.draw_rect(plate)?;
        let dst = Rect::new(
            plate.x + 22,
            plate.y + (plate.height() as i32 - q.height as i32) / 2,
            q.width,
            q.height,
        );
        canvas.copy(tex, None, Some(dst))?;
    }

    // 正文：单行单次纹理缓存 + 字符宽度裁剪绘制（彻底消灭逐字创建纹理的显存暴涨）
    let (cur_line, cur_chars) = tw.reveal_pos();
    let line_h = fonts.line_h(style.font_size);
    for (i, line) in tw.current_page().iter().enumerate() {
        if i > cur_line || line.is_empty() {
            break;
        }
        let total_chars = line.chars().count();
        let shown = if i < cur_line { total_chars } else { cur_chars };
        if shown == 0 {
            continue;
        }
        let show_w = if shown < total_chars {
            Some(fonts.line_prefix_w(style.font_size, line, shown))
        } else {
            None
        };
        let tex = fonts.render_text(style.font_size, Color::RGB(242, 242, 246), line)?;
        let q = tex.query();
        let (src, dst) = match show_w {
            None => {
                (None, Rect::new(style.text_x, style.text_y + i as i32 * line_h, q.width, q.height))
            }
            Some(w) => {
                let w = w.min(q.width);
                (
                    Some(Rect::new(0, 0, w, q.height)),
                    Rect::new(style.text_x, style.text_y + i as i32 * line_h, w, q.height),
                )
            }
        };
        canvas.copy(tex, src, Some(dst))?;
    }

    // 页满指示：右下角「▼」呼吸
    if tw.page_full() {
        let a = (140.0 + 100.0 * (now_ms / 400.0).sin()) as u8;
        canvas.set_draw_color(Color::RGBA(255, 255, 255, a));
        let bx = br.right() - 34;
        let by = br.bottom() - 26;
        for dy in 0..8i32 {
            let half = (8 - dy) as u32;
            canvas.fill_rect(Rect::new(bx - (half / 2) as i32 + 4, by + dy - 8, half.max(1), 1))?;
        }
    }
    Ok(())
}
