//! 存档/读档界面：3×3 槽位网格（缩略图+时间戳+场景标题），
//! meta 不明存档（不可读）与破損标记同屏展示。

mod cache;
mod draw;

pub use cache::ThumbCache;
pub use draw::draw;

use sdl2::rect::Rect;

use gal_config::LOGICAL_W;

const CELL_W: u32 = 360;
const CELL_H: u32 = 150;
const GAP: i32 = 24;
const COLS: usize = 3;
const TOP_Y: i32 = 130;

/// 槽位/不明档 布局矩形（idx ≥ slots 为不明档，追加一行展示）
pub fn cell_rect(idx: usize) -> Rect {
    let row = idx / COLS;
    let col = idx % COLS;
    let total_w = COLS as i32 * CELL_W as i32 + (COLS as i32 - 1) * GAP;
    let x0 = (LOGICAL_W as i32 - total_w) / 2;
    Rect::new(
        x0 + col as i32 * (CELL_W as i32 + GAP),
        TOP_Y + row as i32 * (CELL_H as i32 + GAP),
        CELL_W,
        CELL_H,
    )
}

pub fn hit_test(n: usize, x: f32, y: f32) -> Option<usize> {
    (0..n).find(|&i| cell_rect(i).contains_point((x as i32, y as i32)))
}

/// 右上角关闭按钮矩形（共享几何源，供 draw 与 mouse click 统一使用）
pub fn close_rect() -> Rect {
    Rect::new(LOGICAL_W as i32 - 216, 54, 140, 52)
}
