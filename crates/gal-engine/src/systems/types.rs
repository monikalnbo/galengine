//! 引擎系统子状态定义：抖动、越界演出、关机倒计时、Toast 消息

use gal_audio::Audio;
use gal_save::meta::MetaState;

pub struct ShakeSt {
    pub left_ms: f32,
    pub dur_ms: f32,
}

pub struct ReachSt {
    pub path: String,
    pub t: f32,
    pub dur_ms: f32,
    pub scale: f32,
    /// reach hide：淡出中
    pub hiding: bool,
}

pub struct RestSt {
    pub left_ms: f32,
}

pub struct Msg {
    pub text: String,
    pub left_ms: f32,
}

pub struct Systems {
    pub audio: Audio,
    /// v0.3.8 视频播放器（`video` feature 关闭时恒 None，wv 走无声降级路径）
    #[cfg(feature = "video")]
    pub video: gal_render::video::VideoPlayer,
    pub meta: MetaState,
    pub meta_path: String,
    pub save_dir: String,
    pub slots: usize,
    pub base_title: String,
    pub shake: Option<ShakeSt>,
    pub reach: Option<ReachSt>,
    pub rest: Option<RestSt>,
    /// 中文提示 toast（拒读指纹/平台错误等）
    pub msg: Option<Msg>,
    /// 左下角小字提示（语音被 SE 打断等，1.5s 自动消失）
    pub voice_hint: Option<Msg>,
}
