//! config/ui.rs —— UI 部件配置（对话框/选项/底栏/标题/菜单/输入框/鼠标/按钮/图层叠加）

use serde::Deserialize;
use std::collections::HashMap;

// ============================================================
// UI 总配置
// ============================================================

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct UiCfg {
    pub dialog: DialogCfg,
    pub choice: ChoiceCfg,
    pub bottombar: BottombarCfg,
    pub title: TitleCfg,
    pub menu: MenuCfg,
    pub input: InputCfg,
    pub cursor: CursorCfg,
    pub button: ButtonCfg,
    /// 图层叠加定义（标题画面/自定义界面用）
    pub layers: HashMap<String, Vec<LayerDef>>,
}

// ============================================================
// 对话框
// ============================================================

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct DialogCfg {
    /// 自定义对话框底图（null=引擎默认半透明黑）
    pub box_image: Option<String>,
    pub box_rect: [i32; 4],
    pub text_x: i32,
    pub text_y: i32,
    pub font_size: u16,
    pub name_font_size: u16,
    pub lines_per_page: usize,
    pub typewriter_ms: f32,
    /// 对话框透明度（0=全透明 255=不透明）
    pub opacity: u8,
}
impl Default for DialogCfg {
    fn default() -> Self {
        Self {
            box_image: None,
            box_rect: [56, 514, 1168, 172],
            text_x: 104,
            text_y: 542,
            font_size: 30,
            name_font_size: 30,
            lines_per_page: 3,
            typewriter_ms: 30.0,
            opacity: 170,
        }
    }
}

// ============================================================
// 选项
// ============================================================

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ChoiceCfg {
    pub item_image: Option<String>,
    pub item_hover_image: Option<String>,
    pub item_width: u32,
    pub item_height: u32,
    pub gap: i32,
    pub top_y: i32,
    pub font_size: u16,
}
impl Default for ChoiceCfg {
    fn default() -> Self {
        Self {
            item_image: None,
            item_hover_image: None,
            item_width: 760,
            item_height: 66,
            gap: 18,
            top_y: 236,
            font_size: 28,
        }
    }
}

// ============================================================
// 底栏
// ============================================================

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct BottombarCfg {
    pub enabled: bool,
    pub buttons: Vec<String>,
    pub button_width: u32,
    pub button_height: u32,
    pub gap: u32,
    pub y: i32,
    pub right_align: i32,
    pub font_size: u16,
}
impl Default for BottombarCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            buttons: Vec::new(),
            button_width: 92,
            button_height: 28,
            gap: 10,
            y: 689,
            right_align: 1224,
            font_size: 19,
        }
    }
}

// ============================================================
// 标题画面
// ============================================================

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct TitleCfg {
    pub background: Option<String>,
    pub title_text_y: i32,
    pub title_font_size: u16,
    pub hint_text: String,
    pub hint_y: i32,
    pub menu_top_y: i32,
    pub menu_gap: i32,
}
impl Default for TitleCfg {
    fn default() -> Self {
        Self {
            background: None,
            title_text_y: 100,
            title_font_size: 96,
            hint_text: "— select —".into(),
            hint_y: 238,
            menu_top_y: 270,
            menu_gap: 76,
        }
    }
}

// ============================================================
// 系统菜单（Esc）
// ============================================================

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct MenuCfg {
    pub item_image: Option<String>,
    pub item_width: u32,
    pub item_height: u32,
    pub gap: i32,
    pub top_y: i32,
    pub font_size: u16,
}
impl Default for MenuCfg {
    fn default() -> Self {
        Self {
            item_image: None,
            item_width: 380,
            item_height: 62,
            gap: 14,
            top_y: 100,
            font_size: 30,
        }
    }
}

// ============================================================
// 名字输入框
// ============================================================

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct InputCfg {
    pub prompt_y: i32,
    pub box_y: i32,
    pub preset_y: i32,
    pub confirm_y: i32,
    pub hint_y: i32,
    pub veil_alpha: u8,
}
impl Default for InputCfg {
    fn default() -> Self {
        Self {
            prompt_y: 180,
            box_y: 270,
            preset_y: 400,
            confirm_y: 480,
            hint_y: 575,
            veil_alpha: 160,
        }
    }
}

// ============================================================
// 鼠标样式
// ============================================================

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct CursorCfg {
    /// 默认鼠标图（32×32 透明PNG，null=系统默认）
    pub default: Option<String>,
    /// 点击时鼠标图
    pub click: Option<String>,
    /// 热点偏移 [x, y]
    pub hotspot: [i32; 2],
}

// ============================================================
// 通用按钮样式
// ============================================================

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ButtonCfg {
    pub normal_image: Option<String>,
    pub accent_image: Option<String>,
    pub fill_normal: [u8; 4],
    pub fill_accent: [u8; 4],
    pub border_normal: [u8; 4],
    pub border_accent: [u8; 4],
    pub text_normal: [u8; 3],
    pub text_accent: [u8; 3],
    pub border_width: u8,
    pub radius: Option<u8>,
}
impl Default for ButtonCfg {
    fn default() -> Self {
        Self {
            normal_image: None,
            accent_image: None,
            fill_normal: [28, 34, 58, 215],
            fill_accent: [52, 74, 128, 235],
            border_normal: [130, 150, 200, 150],
            border_accent: [160, 200, 255, 220],
            text_normal: [210, 216, 232],
            text_accent: [255, 255, 255],
            border_width: 1,
            radius: None,
        }
    }
}

// ============================================================
// 图层叠加定义（核心：用图片组合出精美界面）
// ============================================================

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct LayerDef {
    /// 图片路径（相对于数据目录）；null = 纯色/纯文字层
    pub image: Option<String>,
    /// 纯色填充 RGBA（image 为空时使用）
    pub fill: Option<[u8; 4]>,
    /// 文字内容（支持 {meta.title} 等变量引用）
    pub text: Option<String>,
    /// 绘制区域 [x, y, w, h]；省略=全屏
    pub rect: Option<[i32; 4]>,
    /// 文字大小（有 text 时生效）
    pub font_size: Option<u16>,
    /// 文字颜色 RGB
    pub color: Option<[u8; 3]>,
    /// 动画（可选：fade_in_ms / slide_in 等，后续拓展）
    pub anim: Option<String>,
}
