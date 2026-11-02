//! config/text.rs —— 文字排版默认值（字号/行距/颜色/描边/阴影/是否显示等待标记）
//!
//! 这里是**作者侧默认值**：玩家在设置面板改的是 `savedata/global.json` 里的
//! `sf.*`，运行时以玩家值为准（见 gal_ui::settings / gal_engine::input::settings_io）。

use serde::Deserialize;

/// 字号可调范围（玩家设置滑条同用，避免两处漂移）
pub const FONT_SIZE_MIN: u16 = 20;
pub const FONT_SIZE_MAX: u16 = 48;
/// 行距可调范围（像素，叠加在字体推荐行高之上）
pub const LINE_SPACING_MAX: i32 = 16;
/// 描边/阴影最大像素
pub const FX_MAX: i32 = 4;

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct TextCfg {
    /// 字号（0 = 跟随 ui.dialog.font_size）
    pub size: u16,
    /// 行距（像素；0 = 字体推荐行高）
    pub line_spacing: i32,
    /// 正文字色 RGB
    pub color: [u8; 3],
    /// 描边宽度（0 = 无描边；SDL_ttf 描边与字同色，故无独立描边色）
    pub outline: u8,
    /// 阴影偏移（[dx, dy]；0,0 = 无阴影）
    pub shadow_offset: [i32; 2],
    /// 阴影颜色 RGB
    pub shadow_color: [u8; 3],
    /// 页满时是否绘制「▼」等待标记
    pub wait_marker: bool,
    /// ruby 注音字号（px；0 = 正文一半）
    pub ruby_size: u16,
    /// 已读文字色 RGB（null = 不区分已读）
    pub read_color: Option<[u8; 3]>,
    /// 可切换的字体族列表（空 = 只内置候选链那一种）。
    /// 条目形如 `{"name": "Noto Sans CJK SC", "path": "/usr/share/fonts/..."}`
    pub families: Vec<FontFamilyCfg>,
}
impl Default for TextCfg {
    fn default() -> Self {
        Self {
            size: 0,
            line_spacing: 0,
            color: [242, 242, 246],
            outline: 0,
            shadow_offset: [0, 0],
            shadow_color: [0, 0, 0],
            wait_marker: true,
            ruby_size: 0,
            read_color: None,
            families: Vec::new(),
        }
    }
}

/// 一个可切换的字体族（family 名 + 文件路径；路径相对数据目录或绝对路径）
#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct FontFamilyCfg {
    pub name: String,
    pub path: String,
}

impl TextCfg {
    /// ruby 实际字号（0 = 正文一半，最小 10）
    pub fn ruby_size(&self, base: u16) -> u16 {
        if self.ruby_size > 0 {
            self.ruby_size
        } else {
            (base / 2).max(10)
        }
    }

    /// 实际基准字号：text.size 优先，否则回落对话框字号
    pub fn base_size(&self, dialog_font_size: u16) -> u16 {
        if self.size > 0 {
            self.size
        } else {
            dialog_font_size
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 字号优先作者配置否则跟随对话框() {
        let mut t = TextCfg::default();
        assert_eq!(t.base_size(30), 30);
        t.size = 34;
        assert_eq!(t.base_size(30), 34);
    }
}
