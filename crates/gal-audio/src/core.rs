//! 混音核心：kira 后端薄胶水。
//!
//! 方针（2026-11-02 用户更新）：能用成熟库就用库——解码/淡入淡出/循环点/暂停恢复
//! 全部交给 kira（内部 cpal 输出 + symphonia 解码），本 crate 只保留：
//! - 通道映射：BGM 双缓冲（A/B 槽）、SE 多实例、语音独占通道 → kira 子轨道
//! - KAG 语义层：等待标签（wb/wl/wf）轮询查询、循环点命中计数
//! - 静默降级：无声卡（AudioManager 构造失败）= None，控制面照常工作
//!
//! 多路日志（gal-audit）见 log.rs。

use std::collections::HashMap;

use kira::{
    sound::static_sound::{StaticSoundData, StaticSoundSettings},
    sound::PlaybackState,
    track::{TrackBuilder, TrackHandle},
    AudioManager, AudioManagerSettings, DefaultBackend, Tween,
};

/// BGM 双缓冲键位：A/B 两条源交叉淡入淡出
pub const BGM_A: &str = "bgm_a";
pub const BGM_B: &str = "bgm_b";
/// 语音独占键位
pub const VOICE: &str = "voice";

/// 一个正在播放的声音的句柄 + KAG 语义状态。
#[derive(Default)]
pub struct Playing {
    pub handle: Option<kira::sound::static_sound::StaticSoundHandle>,
    /// 淡出后自动暂停（fadepausebgm）：kira pause 自带 tween，无需额外状态
    /// 循环点命中次数（wl 等待语义）：按 position() 跨过 loop_end 判定
    pub loop_hits: u64,
    /// 上次轮询到的位置（秒）：检测回卷/跨点
    pub last_pos: f64,
    /// 循环点（秒）；None = 无
    pub loop_region: Option<(f64, f64)>,
}

impl Playing {
    /// KAG 语义上的「仍在发声」：Playing/Pausing/Stopping 都算（淡出还没落停）
    pub fn active(&self) -> bool {
        self.handle.as_ref().is_some_and(|h| h.state() != PlaybackState::Stopped)
    }

    /// 是否正在淡（wb/wf：kira tween 期间 state 处于 Pausing/Stopping 或音量在途）。
    /// kira 不直接暴露 tween 进度，用状态近似：Pausing/Stopping = 淡出在途。
    pub fn fading(&self) -> bool {
        matches!(
            self.handle.as_ref().map(|h| h.state()),
            Some(PlaybackState::Pausing | PlaybackState::Stopping)
        )
    }
}

/// 通道（kira 子轨道）句柄表：通道音量 = 轨道音量
pub struct Channels {
    pub bgm_a: TrackHandle,
    pub bgm_b: TrackHandle,
    pub voice: TrackHandle,
}

/// 混音核心：kira 管理器 + 三条通道子轨道 + 每键位播放状态。
pub struct Mixer {
    pub manager: AudioManager<DefaultBackend>,
    pub channels: Channels,
    /// 播放状态表：键位 → 句柄与语义状态
    pub playing: HashMap<String, Playing>,
}

impl Mixer {
    /// 打开输出设备并建通道。无声卡返回 None（调用方静默降级）。
    pub fn open(duck: f32) -> Option<Mixer> {
        let mut manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).ok()?;
        let channels = Channels {
            bgm_a: manager.add_sub_track(TrackBuilder::new()).ok()?,
            bgm_b: manager.add_sub_track(TrackBuilder::new()).ok()?,
            voice: manager.add_sub_track(TrackBuilder::new()).ok()?,
        };
        let _ = duck; // voice duck 由上层在 tick 里对 bgm/se 轨道 set_volume 实现
        Some(Mixer { manager, channels, playing: HashMap::new() })
    }

    /// 通道音量（0-1 线性 → kira Decibels）
    pub fn set_channel_volume(&mut self, key: &str, v01: f32) {
        let track = match key {
            BGM_A => &mut self.channels.bgm_a,
            BGM_B => &mut self.channels.bgm_b,
            VOICE => &mut self.channels.voice,
            _ => return,
        };
        track.set_volume(kira::Decibels::from(v01.clamp(0.0, 1.0)), Tween::default());
    }

    /// 从解码缓存数据播放（key = 通道键位；settings 已带循环点等）。
    pub fn play(&mut self, key: &str, data: StaticSoundData) -> Result<(), String> {
        let track = match key {
            BGM_A => &mut self.channels.bgm_a,
            BGM_B => &mut self.channels.bgm_b,
            VOICE => &mut self.channels.voice,
            k if k.starts_with("se_") => {
                // SE 实例直接挂主轨（通道音量在 settings.volume 里合成）
                return self
                    .manager
                    .play(data)
                    .map(|h| {
                        self.playing.insert(
                            k.to_string(),
                            Playing { handle: Some(h), ..Default::default() },
                        );
                    })
                    .map_err(|e| format!("kira play 失败：{e:?}"));
            }
            _ => return Err(format!("未知通道：{key}")),
        };
        let handle = track.play(data).map_err(|e| format!("kira play 失败：{e:?}"))?;
        self.playing
            .insert(key.to_string(), Playing { handle: Some(handle), ..Default::default() });
        Ok(())
    }

    /// 每帧轮询：更新循环点命中计数（wl 用）。
    /// 检测方式：位置从 loop_end 附近跳回 loop_start（kira 回卷）= 一次命中。
    pub fn tick(&mut self, key: &str) {
        let Some(p) = self.playing.get_mut(key) else { return };
        let (Some(handle), Some((start, end))) = (&p.handle, p.loop_region) else { return };
        let pos = handle.position();
        // 播放头在循环区内且突然跳到回卷点 → 命中
        if p.last_pos <= end
            && pos < start
            && pos <= start + 0.5
            && handle.state() == PlaybackState::Playing
        {
            p.loop_hits += 1;
        }
        p.last_pos = pos;
    }
}

/// 静态音设置辅助：起始音量 + 可选淡入（fadeinbgm/fadeinse）
pub fn settings(fade_in_ms: u32, volume: kira::Decibels) -> StaticSoundSettings {
    let s = StaticSoundSettings::new().volume(volume);
    if fade_in_ms > 0 {
        s.fade_in_tween(Tween {
            duration: std::time::Duration::from_millis(fade_in_ms as u64),
            ..Default::default()
        })
    } else {
        s
    }
}
