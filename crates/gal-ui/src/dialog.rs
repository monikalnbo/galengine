//! 底部对话框：半透明盒 + 名字牌（角色色查 config.game.name_colors）+ 打字机正文。
//! DialogStyle 已纯数据化迁至 gal-config::style（Rect→[i32;4]），此处仅 re-export。
//! 正文支持 ruby 注音（基底上方小字）与已读文字色。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use gal_text::font::FontBook;
use gal_text::layout::Unit;
use gal_text::writer::Typewriter;

/// 对话框样式（数据层在 gal-config::style，断行与绘制共用）
pub use gal_config::DialogStyle;

/// 揭示前缀裁剪：按「已显字符数」取可见单元；最后一单元可能只露一部分
fn visible_prefix(units: &[Unit], shown: usize) -> Vec<&Unit> {
    let mut out = Vec::new();
    let mut left = shown;
    for u in units {
        if left == 0 {
            break;
        }
        out.push(u);
        left = left.saturating_sub(u.chars());
    }
    out
}

/// 绘制一段单元序列（含 ruby 与阴影）。返回已画到的 x 坐标。
#[allow(clippy::too_many_arguments)]
fn draw_units(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    units: &[Unit],
    x: i32,
    y: i32,
    style: &DialogStyle,
    color: Color,
) -> Result<i32, String> {
    let o = style.outline as i32;
    let shadow = (style.shadow_offset != [0, 0]).then(|| {
        let [r, g, b] = style.shadow_color;
        (Color::RGB(r, g, b), style.shadow_offset)
    });
    let mut cx = x;
    for u in units {
        match u {
            Unit::Ch(c) => {
                let s = c.to_string();
                draw_piece(canvas, fonts, &s, cx, y, o, style.font_size, color, shadow)?;
                cx += fonts.text_width(style.font_size, &s) as i32;
            }
            Unit::Ruby(base, ruby) => {
                let bw = fonts.text_width(style.font_size, base) as i32;
                let rw = fonts.text_width(style.ruby_size, ruby) as i32;
                let w = bw.max(rw);
                // 注音在基底上方居中（只在有注音时画）
                if !ruby.is_empty() {
                    let rx = cx + (w - rw) / 2;
                    let ry = y - (style.ruby_size as i32) + 2;
                    draw_piece(canvas, fonts, ruby, rx, ry, o, style.ruby_size, color, shadow)?;
                }
                let bx = cx + (w - bw) / 2;
                draw_piece(canvas, fonts, base, bx, y, o, style.font_size, color, shadow)?;
                cx += w;
            }
        }
    }
    Ok(cx)
}

#[allow(clippy::too_many_arguments)]
fn draw_piece(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    s: &str,
    x: i32,
    y: i32,
    o: i32,
    size: u16,
    color: Color,
    shadow: Option<(Color, [i32; 2])>,
) -> Result<(), String> {
    if let Some((sc, [sx, sy])) = shadow {
        let tex = fonts.render_outlined(size, sc, s, style_outline(o))?;
        let q = tex.query();
        canvas.copy(tex, None, Some(Rect::new(x + sx, y + sy, q.width, q.height)))?;
    }
    let tex = fonts.render_outlined(size, color, s, style_outline(o))?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new(x, y, q.width, q.height)))?;
    Ok(())
}

fn style_outline(o: i32) -> u8 {
    o.clamp(0, 255) as u8
}

#[allow(clippy::too_many_arguments)]
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

    // 正文：逐行单元绘制（纹理缓存按整行/整单元命中）
    let (cur_line, cur_chars) = tw.reveal_pos();
    // 只有本页含 ruby 时才在行高上预留注音空间（旧剧本排版零变化）
    let has_ruby = tw.current_units().iter().any(|l| l.iter().any(|u| matches!(u, Unit::Ruby(..))));
    let line_h = fonts.line_h_spaced(style.font_size, style.line_spacing)
        + if has_ruby { style.ruby_size as i32 + 2 } else { 0 };
    let [tr, tg, tb] = style.text_color;
    let text_color = Color::RGB(tr, tg, tb);
    let read_color = style.read_color.map(|[r, g, b]| Color::RGB(r, g, b));
    let o = style.outline as i32;
    for (i, line) in tw.current_units().iter().enumerate() {
        if i > cur_line || line.is_empty() {
            break;
        }
        let shown =
            if i < cur_line { line.iter().map(Unit::chars).sum::<usize>() } else { cur_chars };
        if shown == 0 {
            continue;
        }
        let visible = visible_prefix(line, shown);
        let x = style.text_x - o;
        let y = style.text_y + i as i32 * line_h - o;
        // 已读行：整行换色（当前正在打字的行始终用正文字色）
        let color = match read_color {
            Some(c) if i < cur_line => c,
            _ => text_color,
        };
        let visible: Vec<Unit> = visible.into_iter().cloned().collect();
        draw_units(canvas, fonts, &visible, x, y, style, color)?;
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
