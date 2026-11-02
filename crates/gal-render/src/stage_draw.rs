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
    draw_layer_offset(l, canvas, bank, 0, 0)
}

/// 带额外绘制偏移的层绘制（charsway 的瞬时抖动，不写回 Layer.offset）
fn draw_layer_offset(
    l: &Layer,
    canvas: &mut Canvas<Window>,
    bank: &mut TextureBank,
    add_x: i32,
    add_y: i32,
) -> Result<(), String> {
    let p = l.alpha();
    let (dx, dy) = (l.offset.0 + add_x, l.offset.1 + add_y);
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
    for (i, c) in s.chars.iter().enumerate() {
        let sway = s.sway_offset(i);
        if sway != 0 {
            // 摇晃是纯绘制偏移：只叠加到本次绘制，不污染 Layer.offset（位置/存档语义）
            draw_layer_offset(c, canvas, bank, sway, 0)?;
        } else {
            draw_layer(c, canvas, bank)?;
        }
    }
    draw_layer(&s.cg, canvas, bank)?;
    Ok(())
}
