//! config/save.rs —— 存档配置

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SaveCfg {
    pub dir: String,
    pub slots: usize,
    /// 自动存档间隔（0=关闭，ms）
    pub auto_save_ms: u32,
}
impl Default for SaveCfg {
    fn default() -> Self {
        Self { dir: "savedata".into(), slots: 9, auto_save_ms: 0 }
    }
}
