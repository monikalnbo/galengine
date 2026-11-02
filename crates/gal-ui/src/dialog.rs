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

    // 盒子：当前皮肤的半透明底 + 细边
    let f = style.fill;
    canvas.set_draw_color(Color::RGBA(f[0], f[1], f[2], f[3]));
    canvas.fill_rect(br)?;
    let bd = style.border;
    canvas.set_draw_color(Color::RGBA(bd[0], bd[1], bd[2], bd[3]));
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
    let line_h = fonts.line_h_spaced(style.font_size, style.line_spacing);
    let [tr, tg, tb] = style.text_color;
    let color = Color::RGB(tr, tg, tb);
    // 描边模式下字形纹理四周各扩 outline 像素，绘制时反向外移对齐
    let o = style.outline as i32;
    let [sx, sy] = style.shadow_offset;
    let shadow = if sx != 0 || sy != 0 {
        let [sr, sg, sb] = style.shadow_color;
        Some(Color::RGB(sr, sg, sb))
    } else {
        None
    };
    for (i, line) in tw.current_page().iter().enumerate() {
        if i > cur_line || line.is_empty() {
            break;
        }
        let total_chars = line.chars().count();
        let shown = if i < cur_line { total_chars } else { cur_chars };
        if shown == 0 {
            continue;
        }
        let x = style.text_x - o;
        let y = style.text_y + i as i32 * line_h - o;
        // 打字机裁剪宽度（含描边外扩）
        let clip_w = (shown < total_chars)
            .then(|| fonts.line_prefix_w(style.font_size, line, shown) + o as u32);
        // 先铺阴影（同字形另色纹理），再盖正文
        if let Some(sc) = shadow {
            let tex = fonts.render_outlined(style.font_size, sc, line, style.outline)?;
            let q = tex.query();
            let w = clip_w.map_or(q.width, |w| w.min(q.width));
            let src = clip_w.map(|_| Rect::new(0, 0, w, q.height));
            canvas.copy(tex, src, Some(Rect::new(x + sx, y + sy, w, q.height)))?;
        }
        let tex = fonts.render_outlined(style.font_size, color, line, style.outline)?;
        let q = tex.query();
        let dst = match clip_w {
            Some(w) => {
                let w = w.min(q.width);
                (Some(Rect::new(0, 0, w, q.height)), Rect::new(x, y, w, q.height))
            }
            None => (None, Rect::new(x, y, q.width, q.height)),
        };
        canvas.copy(tex, dst.0, Some(dst.1))?;
    }

    // 页满指示：右下角「▼」呼吸（作者可关）
    if tw.page_full() && style.wait_marker {
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
