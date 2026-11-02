//! config/style.rs —— 对话/文字运行时样式（纯数据，断行与绘制共用）
//!
//! 由 `ui.dialog` + `text.*` + 玩家 `sf.textSize` / `sf.lineSpacing` 合成；
//! 设置面板改字号/行距后调用 [`DialogStyle::with_player`] 重新合成，下一帧生效。

use std::collections::HashMap;

use super::{DialogCfg, SkinCfg, TextCfg};

#[derive(Clone)]
pub struct DialogStyle {
    /// [x, y, w, h]
    pub box_rect: [i32; 4],
    /// 对话框填充 / 边框 RGBA（来自当前皮肤）
    pub fill: [u8; 4],
    pub border: [u8; 4],
    pub text_x: i32,
    pub text_y: i32,
    /// 实际字号（作者 text.size / ui.dialog.font_size / 玩家 sf.textSize 合成后）
    pub font_size: u16,
    pub name_font_size: u16,
    pub lines_per_page: usize,
    pub name_colors: HashMap<String, [u8; 3]>,
    /// 正文字色
    pub text_color: [u8; 3],
    /// 描边宽度（像素，0 = 无）
    pub outline: u8,
    /// 阴影偏移（0,0 = 无）
    pub shadow_offset: [i32; 2],
    pub shadow_color: [u8; 3],
    /// 行距（像素，叠加在字体推荐行高之上）
    pub line_spacing: i32,
    /// 页满「▼」标记
    pub wait_marker: bool,
    /// ruby 注音字号（px）
    pub ruby_size: u16,
    /// 已读文字色（None = 不区分已读）
    pub read_color: Option<[u8; 3]>,
}

impl DialogStyle {
    pub fn from_cfg(c: &DialogCfg, name_colors: HashMap<String, [u8; 3]>) -> Self {
        Self::from_cfg_text(c, &TextCfg::default(), name_colors, 0, 0)
    }

    /// 作者配置 + 玩家覆盖（0 = 不覆盖）合成
    pub fn from_cfg_text(
        c: &DialogCfg,
        text: &TextCfg,
        name_colors: HashMap<String, [u8; 3]>,
        player_size: u16,
        player_spacing: i32,
    ) -> Self {
        let base = text.base_size(c.font_size);
        let font_size = if player_size > 0 { player_size } else { base };
        let line_spacing = if player_spacing > 0 { player_spacing } else { text.line_spacing };
        let skin = SkinCfg::default();
        Self {
            box_rect: c.box_rect,
            fill: skin.dialog_fill,
            border: skin.dialog_border,
            text_x: c.text_x,
            text_y: c.text_y,
            font_size,
            name_font_size: c.name_font_size,
            lines_per_page: c.lines_per_page,
            name_colors,
            text_color: skin.dialog_text.unwrap_or(text.color),
            outline: text.outline,
            shadow_offset: text.shadow_offset,
            shadow_color: text.shadow_color,
            line_spacing,
            wait_marker: text.wait_marker,
            ruby_size: text.ruby_size(font_size),
            read_color: text.read_color,
        }
    }

    /// 玩家改字号/行距后重新合成（保留作者其它字段）
    pub fn with_player(&self, player_size: u16, player_spacing: i32) -> Self {
        let mut s = self.clone();
        if player_size > 0 {
            s.font_size = player_size;
        }
        if player_spacing > 0 {
            s.line_spacing = player_spacing;
        }
        s.ruby_size = if s.ruby_size > 0 { s.ruby_size } else { (s.font_size / 2).max(10) };
        s
    }

    /// 套用皮肤（对话框填充/边框/正文字色）
    pub fn with_skin(&self, skin: &SkinCfg) -> Self {
        let mut s = self.clone();
        s.fill = skin.dialog_fill;
        s.border = skin.dialog_border;
        if let Some(c) = skin.dialog_text {
            s.text_color = c;
        }
        s
    }

    /// 角色名颜色（缺省白）
    pub fn name_color(&self, name: &str) -> [u8; 3] {
        self.name_colors.get(name).copied().unwrap_or([255, 255, 255])
    }
}
