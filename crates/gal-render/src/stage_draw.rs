//! 演出层绘制：gal-script 的 Stage/Layer 数据 → gal-render 合成（底到顶）。

use sdl2::render::Canvas;
use sdl2::video::Window;

use gal_script::stage::{Layer, Stage};

use crate::assets::TextureBank;

fn draw_layer(
    l: &Layer,
    canvas: &mut Canvas<Window>,
    bank: &mut TextureBank,
) -> Result<(), String> {
    let p = l.alpha();
    let (dx, dy) = l.offset;
    if let Some(prev) = &l.prev {
        bank.draw_full_alpha_offset(canvas, prev, 1.0 - p, dx, dy)?;
    }
    if let Some(cur) = &l.cur {
        bank.draw_full_alpha_offset(canvas, cur, p, dx, dy)?;
    }
    Ok(())
}

/// 演出层绘制：Q版在场→背景虚化突出角色；否则背景正常交叉淡化。
/// 虚化分支不参与 bg 交叉淡化（ponytail：Q版在场时切 bg 少见，直切可接受）。
pub fn draw_stage(
    s: &Stage,
    canvas: &mut Canvas<Window>,
    bank: &mut TextureBank,
) -> Result<(), String> {
    let chibi_on = s.chars.iter().any(|c| c.chibi && c.cur.is_some());
    if chibi_on {
        if let Some(path) = s.bg.cur.clone() {
            bank.draw_blur(canvas, &path)?;
        }
    } else {
        draw_layer(&s.bg, canvas, bank)?;
    }
    for c in &s.chars {
        draw_layer(c, canvas, bank)?;
    }
    draw_layer(&s.cg, canvas, bank)?;
    Ok(())
}
