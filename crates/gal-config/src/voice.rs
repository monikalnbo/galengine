//! config/voice.rs —— 语音通道默认值（目录/音量/语音时压低其它通道）

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct VoiceCfg {
    /// 语音素材目录（相对数据目录；目录可含子目录，按 `角色/文件名` 查找）
    pub dir: String,
    /// 语音默认音量 0-100（玩家 `sf.volVoice` 覆盖）
    pub volume: i32,
    /// 语音播放时把 BGM/SE 压到的比例（0.0-1.0；1.0 = 不压低）
    pub duck: f32,
}
impl Default for VoiceCfg {
    fn default() -> Self {
        Self { dir: "voice".into(), volume: 100, duck: 0.45 }
    }
}
