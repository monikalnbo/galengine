//! v0.3.15 图形按钮（KAG3 button 子集）：image/ 素材整幅可点，
//! 居中绘制；悬停微放大提示可点。缺文件不绘制不崩。

use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use gal_config::{LOGICAL_H, LOGICAL_W};
use gal_render::assets::TextureBank;

/// 按钮落位：以画面水平居中、垂直 3/4 处为锚，按素材原尺寸（不放大）
pub fn button_rect(bank: &mut TextureBank, graphic: &str) -> Option<Rect> {
    let path = gal_config::paths::resolve("image", graphic);
    let (w, h) = bank.size_of(&path)?;
    let x = (LOGICAL_W as i32 - w as i32) / 2;
    let y = (LOGICAL_H as i32 * 3 - h as i32) / 4;
    Some(Rect::new(x, y, w, h))
}

pub fn draw(
    canvas: &mut Canvas<Window>,
    bank: &mut TextureBank,
    graphic: &str,
    hover: bool,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    let Some(mut r) = button_rect(bank, graphic) else { return Ok(()) };
    if hover {
        // 悬停：向外扩 4px（不变形，按原比例）
        let k = 1.06f32;
        let nw = (r.width() as f32 * k) as u32;
        let nh = (r.height() as f32 * k) as u32;
        r = Rect::new(
            r.x - (nw as i32 - r.width() as i32) / 2,
            r.y - (nh as i32 - r.height() as i32) / 2,
            nw,
            nh,
        );
    }
    let path = gal_config::paths::resolve("image", graphic);
    bank.draw_scaled(canvas, &path, r, 1.0)
}

/// 命中（逻辑坐标）
pub fn hit(bank: &mut TextureBank, graphic: &str, x: f32, y: f32) -> bool {
    button_rect(bank, graphic).map(|r| r.contains_point((x as i32, y as i32))).unwrap_or(false)
}
