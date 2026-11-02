//! 通用菜单列表：居中竖排按钮（Esc 系统菜单 / 标题画面共用）。
//! top_y = 第一项按钮的顶部 y；列表向下排布（v0.1.1 修复：原公式把 top_y 当
//! 中心用导致整列下坠出屏——7 项菜单后 3 项不可见不可点）。
//! 几何/字号来自 `ui.menu`，配色来自 `ui.button` + 当前皮肤。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::widgets::button;
use gal_config::{ButtonCfg, MenuCfg, LOGICAL_W};
use gal_text::font::FontBook;

pub fn item_rect(idx: usize, cfg: &MenuCfg, top_y: i32) -> Rect {
    Rect::new(
        (LOGICAL_W as i32 - cfg.item_width as i32) / 2,
        top_y + idx as i32 * (cfg.item_height as i32 + cfg.gap),
        cfg.item_width,
        cfg.item_height,
    )
}

pub fn hit_test(n: usize, cfg: &MenuCfg, top_y: i32, x: f32, y: f32) -> Option<usize> {
    (0..n).find(|&i| item_rect(i, cfg, top_y).contains_point((x as i32, y as i32)))
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    cfg: &MenuCfg,
    button_style: &ButtonCfg,
    items: &[String],
    sel: usize,
    top_y: i32,
    dim: bool,
) -> Result<(), String> {
    if dim {
        canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 150));
        canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 720))?;
    }
    for (i, label) in items.iter().enumerate() {
        button(
            canvas,
            fonts,
            item_rect(i, cfg, top_y),
            label,
            cfg.font_size,
            i == sel,
            button_style,
        )?;
    }
    Ok(())
}
