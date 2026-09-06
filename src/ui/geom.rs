//! UI 几何：界面布局的矩形计算（纯函数，无绘制）。

use sdl2::rect::Rect;

use crate::config::LOGICAL_W;

/// 通用网格：cols×rows，返回线性顺序的格子矩形
pub fn grid(start: (i32, i32), cols: usize, rows: usize, cell: (u32, u32), gap: i32) -> Vec<Rect> {
    (0..cols * rows)
        .map(|i| {
            let (c, r) = (i % cols, i / cols);
            Rect::new(
                start.0 + c as i32 * (cell.0 as i32 + gap),
                start.1 + r as i32 * (cell.1 as i32 + gap),
                cell.0,
                cell.1,
            )
        })
        .collect()
}

/// 水平居中矩形（逻辑坐标）
pub fn centered(w: u32, h: u32, y: i32) -> Rect {
    Rect::new((LOGICAL_W as i32 - w as i32) / 2, y, w, h)
}

/// 竖排菜单条目矩形（x,w,top 定位）
pub fn menu_column(x: i32, w: u32, top: i32, n: usize, item_h: u32, gap: i32) -> Vec<Rect> {
    (0..n)
        .map(|i| Rect::new(x, top + i as i32 * (item_h as i32 + gap), w, item_h))
        .collect()
}

/// 命中测试：返回下标
pub fn hit(rects: &[Rect], x: f32, y: f32) -> Option<usize> {
    rects
        .iter()
        .position(|r| r.contains_point((x as i32, y as i32)))
}
