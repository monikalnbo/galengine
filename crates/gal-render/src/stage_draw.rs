//! 演出层绘制：gal-script 的 Stage 统一层表 → gal-render 合成（底到顶）。
//!
//! P0-6 起只有**一条**合成路径：背景/立绘/CG/自由层都按 `Stage::ordered_layers()`
//! 的层号升序绘制；类别只决定「这层怎么画」（背景可虚化、立绘可摇晃），
//! 不再决定「这层何时画」。

use sdl2::render::Canvas;
use sdl2::video::Window;

use gal_script::stage::{Layer, LayerKind, Stage};

use crate::assets::TextureBank;

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

/// 统一层序合成：层号升序，逐层按类别选绘制方式。
///
/// - 背景（0）：Q版立绘在场时改走虚化（不参与交叉淡化，直切可接受）
/// - 立绘（1..=3）：叠加 charsway 的瞬时摇晃偏移
/// - CG（4）/ 自由层（>=5）：直绘，用自身 offset（将来 move 复用）
/// - 转场（P1-1）：目标层绘制时用转场图覆盖；`hide_under` 时隐藏原有内容
pub fn draw_stage(
    s: &Stage,
    canvas: &mut Canvas<Window>,
    bank: &mut TextureBank,
) -> Result<(), String> {
    let chibi_on = s.chibi_on();
    for (n, kind, l) in s.ordered_layers() {
        if !layer_or_trans_visible(s, n, l) {
            continue;
        }
        // 在途转场：旧内容定格 + 新内容交叉淡化，走同一层位（层序不变）
        if let Some(t) = &s.trans {
            if t.layer == n {
                let (dx, dy) = t.offset;
                if let Some(from) = &t.from {
                    let blur = chibi_on && n == 0;
                    draw_trans_side(from, t.old_alpha(), blur, canvas, bank, dx, dy)?;
                }
                if !t.to.is_empty() {
                    let blur = chibi_on && n == 0;
                    draw_trans_side(&t.to, t.alpha(), blur, canvas, bank, dx, dy)?;
                } else {
                    // 淡出到空：目标层内容（已清空）无需绘制
                }
                continue;
            }
        }
        let sway = if kind == LayerKind::Char { s.sway_offset(n - 1) } else { 0 };
        draw_layer_offset(l, canvas, bank, sway, 0)?;
    }
    Ok(())
}

/// 层自身可见，或该层上正有转场（此时层内容由转场接管）
fn layer_or_trans_visible(s: &Stage, n: usize, l: &Layer) -> bool {
    l.visible() || s.trans.as_ref().is_some_and(|t| t.layer == n)
}

/// 转场一侧的绘制：背景沿用 Q 版虚化；其余按普通整图 + 透明度
fn draw_trans_side(
    path: &str,
    alpha: f32,
    blur: bool,
    canvas: &mut Canvas<Window>,
    bank: &mut TextureBank,
    dx: i32,
    dy: i32,
) -> Result<(), String> {
    if blur {
        return bank.draw_blur(canvas, path);
    }
    bank.draw_full_alpha_offset(canvas, path, alpha, dx, dy)
}
