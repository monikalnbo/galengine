//! config/sprites.rs —— 立绘配置（层数/淡入/预设位置）

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SpritesCfg {
    pub layers: u8,
    pub default_fade_ms: u32,
    /// 预设位置（char 指令可按名引用）
    pub positions: HashMap<String, SpritePos>,
    /// Q版立绘名单（出场时背景虚化突出角色）
    pub chibi: Vec<String>,
}
impl Default for SpritesCfg {
    fn default() -> Self {
        let mut positions = HashMap::new();
        positions.insert("left".into(), SpritePos { x: 0, y: 0 });
        positions.insert("center".into(), SpritePos { x: 320, y: 0 });
        positions.insert("right".into(), SpritePos { x: 640, y: 0 });
        Self { layers: 3, default_fade_ms: 500, positions, chibi: Vec::new() }
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct SpritePos {
    pub x: i32,
    pub y: i32,
}
