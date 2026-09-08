//! config/audio.rs —— 音频配置

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AudioCfg {
    pub bgm_dir: String,
    pub se_dir: String,
    pub bgm_volume: i32,
    pub se_volume: i32,
    /// BGM 淡入淡出时长 ms
    pub fade_ms: u32,
}
impl Default for AudioCfg {
    fn default() -> Self {
        Self {
            bgm_dir: "bgm".into(),
            se_dir: "sound".into(),
            bgm_volume: 80,
            se_volume: 90,
            fade_ms: 800,
        }
    }
}
