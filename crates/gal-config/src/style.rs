//! config/style.rs —— 对话框样式（纯数据，断行与绘制共用）

use std::collections::HashMap;

use super::DialogCfg;

#[derive(Clone)]
pub struct DialogStyle {
    /// [x, y, w, h]
    pub box_rect: [i32; 4],
    pub text_x: i32,
    pub text_y: i32,
    pub font_size: u16,
    pub name_font_size: u16,
    pub lines_per_page: usize,
    pub name_colors: HashMap<String, [u8; 3]>,
}

impl DialogStyle {
    pub fn from_cfg(c: &DialogCfg, name_colors: HashMap<String, [u8; 3]>) -> Self {
        Self {
            box_rect: c.box_rect,
            text_x: c.text_x,
            text_y: c.text_y,
            font_size: c.font_size,
            name_font_size: c.name_font_size,
            lines_per_page: c.lines_per_page,
            name_colors,
        }
    }

    /// 角色名颜色（缺省白）
    pub fn name_color(&self, name: &str) -> [u8; 3] {
        self.name_colors.get(name).copied().unwrap_or([255, 255, 255])
    }
}
