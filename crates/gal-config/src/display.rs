//! config/display.rs —— 显示设置

use serde::Deserialize;

pub const LOGICAL_W: u32 = 1280;
pub const LOGICAL_H: u32 = 720;

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct DisplayCfg {
    pub width: u32,
    pub height: u32,
    pub letterbox: bool,
    /// 窗口是否可缩放
    pub resizable: bool,
    /// 背景饱和度缩放（1.0=原图；<1 降饱和护眼，仅作用于 bgimage 素材）
    pub bg_saturation: f32,
}
impl Default for DisplayCfg {
    fn default() -> Self {
        Self {
            width: LOGICAL_W,
            height: LOGICAL_H,
            letterbox: true,
            resizable: true,
            bg_saturation: 1.0,
        }
    }
}
