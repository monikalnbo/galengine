//! 选项列表：居中竖排半透明条；↑↓+回车 和 鼠标悬停+点击 双通道。
//! 几何/字号/配色全部来自 `ui.choice` + `ui.button` + 当前皮肤。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use gal_config::{ButtonCfg, ChoiceCfg, LOGICAL_W};
use gal_text::font::FontBook;

pub fn item_rect(idx: usize, cfg: &ChoiceCfg) -> Rect {
    Rect::new(
        (LOGICAL_W as i32 - cfg.item_width as i32) / 2,
        cfg.top_y + idx as i32 * (cfg.item_height as i32 + cfg.gap),
        cfg.item_width,
        cfg.item_height,
    )
}

/// 鼠标命中（n=选项总数）
pub fn hit_test(n: usize, cfg: &ChoiceCfg, x: f32, y: f32) -> Option<usize> {
    (0..n).find(|&i| item_rect(i, cfg).contains_point((x as i32, y as i32)))
}

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    cfg: &ChoiceCfg,
    button: &ButtonCfg,
    prompt: &str,
    items: &[(String, String)],
    selected: usize,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    let tex = fonts.render_text(cfg.font_size, Color::RGB(220, 224, 236), prompt)?;
    let q = tex.query();
    canvas.copy(
        tex,
        None,
        Some(Rect::new((LOGICAL_W as i32 - q.width as i32) / 2, cfg.top_y - 56, q.width, q.height)),
    )?;
    for (i, (text, _)) in items.iter().enumerate() {
        let r = item_rect(i, cfg);
        let hot = i == selected;
        let (fill, border, tc) = if hot {
            (button.fill_accent, button.border_accent, button.text_accent)
        } else {
            (button.fill_normal, button.border_normal, button.text_normal)
        };
        canvas.set_draw_color(Color::RGBA(fill[0], fill[1], fill[2], fill[3]));
        canvas.fill_rect(r)?;
        canvas.set_draw_color(Color::RGBA(border[0], border[1], border[2], border[3]));
        canvas.draw_rect(r)?;
        let tex = fonts.render_text(cfg.font_size, Color::RGB(tc[0], tc[1], tc[2]), text)?;
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
