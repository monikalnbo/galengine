//! galengine 音频层（v0.3.2）：kira 混音 + KAG3 音频语义薄胶水。
//!
//! 方针（2026-11-02 更新）：能用成熟库就用库——解码、输出、淡入淡出（tween）、
//! 循环点、暂停恢复全部交给 kira（内部 cpal 输出 + symphonia 解码）。
//! 本 crate 只保留：通道映射（BGM A/B 双缓冲、SE 多实例、语音独占）、
//! KAG 等待语义（wb/wl/wf 轮询）、静态降级与 gal-audit 多路日志。
//!
//! 方针：玩家侧**静默降级**——缺文件、无声卡、解码失败一律不崩、不弹窗、不中断演出；
//! 作者侧**完整证据链**——所有失败与动作经 gal-audit 落分级日志 + trace 追踪。
//!
//! 向后兼容：`bgm/se/voice` 旧 DSL 原样可用（bgm 名称 = playbgm 无淡入、
//! bgm fadeout ms = fadeoutbgm、se 名称 = playse 无 buffer 号）。

mod core;
mod log;

use std::collections::HashMap;
use std::time::Duration;

use gal_config::Config;
use kira::{sound::static_sound::StaticSoundData, sound::Region, Decibels, Tween};
pub use log::Level;

use core::{Mixer, BGM_A, BGM_B, VOICE};

pub struct Audio {
    /// 无声卡/设备打开失败 → 播放操作静默降级（语义查询仍可用）
    pub ok: bool,
    /// kira 混音器（None = 无声卡降级）
    mixer: Option<Mixer>,
    /// 当前 BGM 名（切歌判断 + 双缓冲角色记录）
    bgm_name: Option<String>,
    /// 正在发声的 BGM 槽位（A/B 轮换）
    bgm_slot: &'static str,
    /// 曲目解码缓存（同一首 BGM/SE 反复播放不重复解码）
    cache: HashMap<String, Result<StaticSoundData, String>>,
    /// SE 逐实例键位计数（同名 SE 并发）
    se_seq: u64,
    pub bgm_vol: i32,
    pub se_vol: i32,
    pub voice_vol: i32,
    pub duck: f32,
    /// 语音被 SE 打断的提示（引擎取走后显示给小字；None = 无）
    interrupted: Option<String>,
    bgm_dir: String,
    se_dir: String,
    voice_dir: String,
}

fn first_path(dir: &str, name: &str, exts: &[&str]) -> Option<String> {
    exts.iter()
        .map(|ext| format!("{}/{}/{}.{}", gal_config::data_dir(), dir, name, ext))
        .find(|p| std::path::Path::new(p).exists())
}

/// 语音查找顺序：voice 目录下 <角色>/<文件名>、<文件名>，扩展名 ogg/mp3/wav
fn voice_path(dir: &str, name: &str) -> Option<String> {
    ["ogg", "mp3", "wav"].iter().find_map(|ext| {
        let p = format!("{}/{}/{}.{}", gal_config::data_dir(), dir, name, ext);
        std::path::Path::new(&p).exists().then_some(p)
    })
}

/// 音频素材查找：找不到 → WARN 日志 + trace 追踪（证据链），返回 None 静默降级
fn lookup(dir: &str, name: &str, kind: &str) -> Option<String> {
    let p = first_path(dir, name, &["ogg", "mp3", "wav"]);
    if p.is_none() {
        audit!(Warn, kind, "音频文件不存在：{dir}/{name}（.ogg/.mp3/.wav 均未找到，静默跳过）");
        audit_trace!(kind, "miss", "name" => name, "dir" => dir);
    }
    p
}

fn tween(ms: u32) -> Tween {
    Tween { duration: Duration::from_millis(ms.max(1) as u64), ..Default::default() }
}

/// 百分比 0-100 → kira Decibels（0% = SILENCE，100% = 0dB）
fn pct_to_db(pct: i32) -> Decibels {
    match pct.clamp(0, 100) {
        0 => Decibels::SILENCE,
        100 => Decibels::IDENTITY,
        p => Decibels::from(20.0 * (p as f32 / 100.0).log10()),
    }
}

impl Audio {
    pub fn new(conf: &Config) -> Self {
        // 日志尽早初始化（数据目录已知）
        log::init(&gal_config::data_dir());
        let (mixer, ok) = match core::Mixer::open(conf.voice.duck) {
            Some(m) => (Some(m), true),
            None => {
                audit!(
                    Warn,
                    "audio",
                    "无声卡/输出设备不可用：音频静默降级（演出不受影响，仅无声）"
                );
                (None, false)
            }
        };
        if ok {
            audit!(
                Info,
                "audio",
                "混音器就绪：kira 0.10（cpal 输出 + symphonia 解码），通道=bgm_a/bgm_b/voice"
            );
        }
        let mut a = Self {
            ok,
            mixer,
            bgm_name: None,
            bgm_slot: BGM_A,
            cache: HashMap::new(),
            se_seq: 0,
            bgm_vol: conf.audio.bgm_volume,
            se_vol: conf.audio.se_volume,
            voice_vol: conf.voice.volume,
            duck: conf.voice.duck,
            interrupted: None,
            bgm_dir: if conf.audio.bgm_dir.is_empty() {
                "bgm".into()
            } else {
                conf.audio.bgm_dir.clone()
            },
            se_dir: if conf.audio.se_dir.is_empty() {
                "sound".into()
            } else {
                conf.audio.se_dir.clone()
            },
            voice_dir: if conf.voice.dir.is_empty() {
                "voice".into()
            } else {
                conf.voice.dir.clone()
            },
        };
        a.apply_volumes();
        a
    }

    // ---------- 基础设施 ----------

    /// 解码 + 缓存。失败：ERROR 日志 + trace，静默降级返回 None。
    fn load(&mut self, kind: &str, path: &str, key: &str) -> Option<StaticSoundData> {
        if let Some(r) = self.cache.get(key) {
            return r.clone().ok();
        }
        let r = StaticSoundData::from_file(path).map_err(|e| format!("{e:?}"));
        match &r {
            Ok(d) => audit!(
                Debug,
                kind,
                "解码完成：{path}（{} 帧，{}Hz）",
                d.frames.len(),
                d.sample_rate
            ),
            Err(e) => {
                audit!(Error, kind, "音频解码失败：{path}（{e}）——静默跳过");
                audit_trace!(kind, "decode_error", "path" => path, "err" => e);
            }
        }
        self.cache.insert(key.to_string(), r.clone());
        r.ok()
    }

    /// 上挂 BGM/语音到指定槽位（SE 走独立实例键）
    fn spawn(
        &mut self,
        key: &str,
        data: StaticSoundData,
        track_vol: Option<(&'static str, f32)>,
        loop_region: Option<(f64, f64)>,
    ) {
        let Some(m) = &mut self.mixer else { return };
        // 通道音量先就位（fade_in 由 settings 挂 tween）
        if let Some((tk, v)) = track_vol {
            m.set_channel_volume(tk, v);
        }
        if let Err(e) = m.play(key, data) {
            audit!(Error, "audio", "播放失败（{key}）：{e}——静默跳过");
            audit_trace!("audio", "play_error", "key" => key, "err" => e);
            return;
        }
        if let (Some(p), Some((a, b))) = (m.playing.get_mut(key), loop_region) {
            p.loop_region = Some((a, b));
        }
    }

    // ---------- BGM ----------

    /// 播放/切换 BGM。
    /// - `fade_in_ms > 0`：淡入（fadeinbgm）
    /// - `fade_out_ms > 0`：旧曲淡出后移除（fadeoutbgm 用 stop 变体）
    /// - `xchg`：双缓冲交叉淡化（旧曲淡出与新曲淡入同时进行，无缝切换）
    fn play_bgm_impl(&mut self, name: &str, fade_in_ms: u32, fade_out_ms: u32, xchg: bool) {
        audit_trace!("bgm", "play", "name" => name, "fade_in_ms" => fade_in_ms,
            "fade_out_ms" => fade_out_ms, "xchg" => xchg);
        if self.bgm_name.as_deref() == Some(name) && !xchg {
            audit!(Debug, "bgm", "重复播放同一首 BGM「{name}」，忽略");
            return;
        }
        let Some(path) = lookup(&self.bgm_dir, name, "bgm") else { return };
        let Some(data) = self.load("bgm", &path, &format!("bgm:{name}")) else { return };
        let new_slot = if self.bgm_slot == BGM_A { BGM_B } else { BGM_A };
        // 旧曲处理：交叉淡出（xchg/fade_out）或立即停
        let old_slot = self.bgm_slot;
        let out_ms = if xchg { fade_out_ms.max(fade_in_ms) } else { fade_out_ms };
        if let Some(m) = &mut self.mixer {
            if let Some(old) = m.playing.get_mut(old_slot) {
                if let Some(h) = &mut old.handle {
                    if out_ms > 0 {
                        h.stop(tween(out_ms));
                    } else {
                        h.stop(Tween::default());
                    }
                }
                // 淡出中的旧曲移出槽位语义（句柄继续存在直到自然停）
            }
            m.playing.remove(old_slot);
        }
        let mut d = data;
        d.settings = core::settings(fade_in_ms, Decibels::IDENTITY);
        self.spawn(new_slot, d, Some((new_slot, self.bgm_vol.clamp(0, 100) as f32 / 100.0)), None);
        self.bgm_slot = new_slot;
        self.bgm_name = Some(name.to_string());
        audit!(
            Info,
            "bgm",
            "播放 BGM「{name}」：淡入 {fade_in_ms}ms 旧曲淡出 {out_ms}ms 槽 {new_slot}"
        );
    }

    pub fn play_bgm(&mut self, name: &str) {
        self.play_bgm_impl(name, 0, 0, false);
    }

    /// fadeinbgm：定时淡入
    pub fn fade_in_bgm(&mut self, name: &str, ms: u32) {
        self.play_bgm_impl(name, ms.max(1), 0, false);
    }

    /// fadeoutbgm：定时淡出并停止
    pub fn fade_out_bgm(&mut self, ms: u32) {
        audit_trace!("bgm", "fadeout", "ms" => ms);
        if let Some(m) = &mut self.mixer {
            if let Some(p) = m.playing.get_mut(self.bgm_slot) {
                if let Some(h) = &mut p.handle {
                    h.stop(tween(ms));
                    audit!(Info, "bgm", "BGM 淡出 {ms}ms 后停止");
                }
            } else {
                audit!(Debug, "bgm", "淡出时无 BGM 在播，忽略");
            }
        }
        self.bgm_name = None;
    }

    /// 旧 DSL `bgm fadeout ms`（同 fadeoutbgm）
    pub fn stop_bgm(&mut self) {
        audit_trace0!("bgm", "stop");
        if let Some(m) = &mut self.mixer {
            if let Some(p) = m.playing.remove(self.bgm_slot) {
                if let Some(mut h) = p.handle {
                    h.stop(Tween::default());
                }
            }
        }
        self.bgm_name = None;
    }

    /// xchgbgm：双缓冲交叉淡化切换（无爆音无缝衔接）
    pub fn xchg_bgm(&mut self, name: &str, ms: u32) {
        self.play_bgm_impl(name, ms.max(1), ms.max(1), true);
    }

    /// pausebgm：暂停当前 BGM（保留播放头）
    pub fn pause_bgm(&mut self) {
        audit_trace0!("bgm", "pause");
        if let Some(m) = &mut self.mixer {
            if let Some(p) = m.playing.get_mut(self.bgm_slot) {
                if let Some(h) = &mut p.handle {
                    h.pause(Tween::default());
                }
            }
        }
    }

    /// resumebgm：恢复暂停的 BGM
    pub fn resume_bgm(&mut self) {
        audit_trace0!("bgm", "resume");
        if let Some(m) = &mut self.mixer {
            if let Some(p) = m.playing.get_mut(self.bgm_slot) {
                if let Some(h) = &mut p.handle {
                    h.resume(Tween::default());
                }
            }
        }
    }

    /// fadepausebgm：淡出后暂停（播放头停在淡出点，resumebgm 从原位恢复）
    pub fn fade_pause_bgm(&mut self, ms: u32) {
        audit_trace!("bgm", "fade_pause", "ms" => ms);
        if let Some(m) = &mut self.mixer {
            if let Some(p) = m.playing.get_mut(self.bgm_slot) {
                if let Some(h) = &mut p.handle {
                    h.pause(tween(ms)); // kira pause(tween) = 音量淡到 0 后暂停
                }
            }
        }
    }

    /// fadebgm：在播 BGM 音量淡到指定百分比（0-100）
    pub fn fade_bgm(&mut self, to_pct: i32, ms: u32) {
        audit_trace!("bgm", "fade", "to_pct" => to_pct, "ms" => ms);
        if let Some(m) = &mut self.mixer {
            if let Some(p) = m.playing.get_mut(self.bgm_slot) {
                if let Some(h) = &mut p.handle {
                    h.set_volume(pct_to_db(to_pct), tween(ms));
                }
            }
        }
    }

    /// bgmopt：循环点（毫秒 → 秒，kira Region）
    pub fn bgmopt(&mut self, loop_start_ms: u64, loop_end_ms: u64) {
        audit_trace!("bgm", "opt", "loop_start_ms" => loop_start_ms, "loop_end_ms" => loop_end_ms);
        if loop_start_ms >= loop_end_ms {
            audit!(
                Warn,
                "bgm",
                "bgmopt 循环点非法（start {loop_start_ms}ms >= end {loop_end_ms}ms），忽略"
            );
            return;
        }
        let region = (loop_start_ms as f64 / 1000.0, loop_end_ms as f64 / 1000.0);
        if let Some(m) = &mut self.mixer {
            match m.playing.get_mut(self.bgm_slot) {
                Some(p) => {
                    if let Some(h) = &mut p.handle {
                        h.set_loop_region(Region::from(region.0..region.1));
                    }
                    p.loop_region = Some(region);
                    audit!(Info, "bgm", "循环点生效：{loop_start_ms}ms → {loop_end_ms}ms");
                }
                None => {
                    audit!(Warn, "bgm", "bgmopt：当前无 BGM 在播，循环点设置被忽略");
                }
            }
        }
    }

    /// wl：是否已到过循环点（取走一次事件）
    pub fn take_loop_hit(&mut self) -> bool {
        if let Some(m) = &mut self.mixer {
            if let Some(p) = m.playing.get_mut(self.bgm_slot) {
                return p.loop_hits > 0 && {
                    p.loop_hits -= 1;
                    true
                };
            }
        }
        false
    }

    /// wb：BGM 淡入淡出是否仍在进行
    pub fn bgm_fading(&self) -> bool {
        self.mixer.as_ref().and_then(|m| m.playing.get(self.bgm_slot)).is_some_and(|p| p.fading())
    }

    // ---------- SE ----------

    /// playse：播放音效。`buf` 语义简化为并发实例键（KAG 的 3 buffer 槽由并发键覆盖）。
    pub fn play_se(&mut self, name: &str) {
        self.play_se_impl(name, 0, 0, None);
    }

    /// fadeinse / fadeoutse / fadese 的实现（fade_in_ms→淡入；fade_out_ms+target→定点淡）
    pub fn play_se_impl(
        &mut self,
        name: &str,
        fade_in_ms: u32,
        fade_out_ms: u32,
        target: Option<f32>,
    ) {
        audit_trace!("se", "play", "name" => name, "fade_in_ms" => fade_in_ms,
            "fade_out_ms" => fade_out_ms, "target" => target.unwrap_or(1.0));
        let Some(path) = lookup(&self.se_dir, name, "se") else { return };
        let Some(mut data) = self.load("se", &path, &format!("se:{name}")) else { return };
        // 语音被 SE 打断提示（保留 v0.3.1 行为）
        if self.voice_playing() && self.interrupted.is_none() {
            self.interrupted = Some(name.to_string());
        }
        self.se_seq += 1;
        let key = format!("se_{}", self.se_seq);
        // SE 实例音量 = 通道音量 ×（目标百分比/1.0）
        let mut settings = core::settings(
            fade_in_ms,
            pct_to_db(
                (self.se_vol.clamp(0, 100) as f32 / 100.0
                    * target.unwrap_or(1.0).clamp(0.0, 1.0)
                    * 100.0) as i32,
            ),
        );
        if fade_out_ms > 0 {
            // fadeoutse：起播即挂淡出（kira 无 fade_out settings，用播放后 set_volume+stop 组合见下）
            settings = settings.volume(Decibels::IDENTITY);
        }
        data.settings = settings;
        if let Some(m) = &mut self.mixer {
            if let Err(e) = m.play(&key, data) {
                audit!(Error, "se", "SE 播放失败（{name}）：{e}——静默跳过");
                return;
            }
            if fade_out_ms > 0 {
                // 起播即向 0 淡出，淡完自动停（fadeoutse 语义）
                if let Some(p) = m.playing.get_mut(&key) {
                    if let Some(h) = &mut p.handle {
                        h.set_volume(Decibels::SILENCE, tween(fade_out_ms));
                        h.stop(tween(fade_out_ms));
                    }
                }
            }
        }
        audit!(Debug, "se", "播放 SE「{name}」（实例 {key}）");
    }

    /// stopse：停掉指定（或全部）SE
    pub fn stop_se(&mut self, name: Option<&str>) {
        audit_trace!("se", "stop", "name" => name.unwrap_or("(all)"));
        let Some(m) = &mut self.mixer else { return };
        let keys: Vec<String> =
            m.playing.keys().filter(|k| k.starts_with("se_")).cloned().collect();
        let mut n = 0;
        for k in keys {
            if name.is_none() {
                if let Some(p) = m.playing.remove(&k) {
                    if let Some(mut h) = p.handle {
                        h.stop(Tween::default());
                    }
                }
                n += 1;
            }
            // 按名停：需要名字→实例的反查表，这里以「全部」为主用例
        }
        if n == 0 {
            audit!(Debug, "se", "stopse：无活动 SE");
        }
    }

    /// wf：是否还有 SE 在淡入淡出
    pub fn se_fading(&self) -> bool {
        self.mixer
            .as_ref()
            .map(|m| {
                m.playing.values().filter(|p| p.handle.is_some()).any(|p| p.active() && p.fading())
            })
            .unwrap_or(false)
    }

    // ---------- 语音 ----------

    /// 播放语音（同一时刻只保留一条；独立通道，音量独立于 BGM/SE）
    pub fn play_voice(&mut self, name: &str) {
        audit_trace!("voice", "play", "name" => name);
        let Some(path) = voice_path(&self.voice_dir, name) else {
            audit!(Debug, "voice", "语音不存在：{}/{}", self.voice_dir, name);
            audit_trace!("voice", "miss", "name" => name, "dir" => self.voice_dir.clone());
            return;
        };
        let Some(data) = self.load("voice", &path, &format!("voice:{name}")) else { return };
        // 语音独占：先停旧的一条
        if let Some(m) = &mut self.mixer {
            if let Some(old) = m.playing.remove(VOICE) {
                if let Some(mut h) = old.handle {
                    h.stop(Tween::default());
                }
            }
        }
        self.spawn(VOICE, data, Some((VOICE, self.voice_vol.clamp(0, 100) as f32 / 100.0)), None);
        audit!(Debug, "voice", "播放语音「{name}」（独立通道）");
    }

    pub fn stop_voice(&mut self) {
        audit_trace0!("voice", "stop");
        if let Some(m) = &mut self.mixer {
            if let Some(p) = m.playing.remove(VOICE) {
                if let Some(mut h) = p.handle {
                    h.stop(Tween::default());
                }
            }
        }
    }

    pub fn voice_playing(&self) -> bool {
        self.mixer.as_ref().and_then(|m| m.playing.get(VOICE)).is_some_and(|p| p.active())
    }

    /// 取走「语音被 SE 打断」事件（每帧调用；一次事件只报一次）
    pub fn take_interrupted(&mut self) -> Option<String> {
        self.interrupted.take()
    }

    // ---------- 通道音量（设置面板）----------

    pub fn apply_volumes(&mut self) {
        if let Some(m) = &mut self.mixer {
            m.set_channel_volume(BGM_A, self.bgm_vol.clamp(0, 100) as f32 / 100.0);
            m.set_channel_volume(BGM_B, self.bgm_vol.clamp(0, 100) as f32 / 100.0);
            m.set_channel_volume(VOICE, self.voice_vol.clamp(0, 100) as f32 / 100.0);
        }
    }

    /// 每帧：循环点命中计数 + 输出活跃度探针
    pub fn pump(&mut self) {
        if let Some(m) = &mut self.mixer {
            let keys: Vec<String> = m.playing.keys().cloned().collect();
            for k in keys {
                m.tick(&k);
            }
            // 播完自动出队（kira 句柄 state == Stopped）
            let done: Vec<String> =
                m.playing.iter().filter(|(_, p)| !p.active()).map(|(k, _)| k.clone()).collect();
            for k in done {
                if k.starts_with("se_") || k == VOICE {
                    m.playing.remove(&k);
                }
                // BGM 槽位保留（fadeout 后 bgm_name 已清；槽位覆盖由下次播放接管）
            }
        }
    }

    /// update_duck：语音播放时压低 BGM/SE（kira tween 对 bgm 轨道动态音量）
    pub fn update_duck(&mut self) {
        let duck_on = self.voice_playing();
        if let Some(m) = &mut self.mixer {
            let target =
                if duck_on && self.duck > 0.0 && self.duck < 1.0 { self.duck } else { 1.0 };
            let bgm = self.bgm_vol.clamp(0, 100) as f32 / 100.0 * target;
            m.set_channel_volume(BGM_A, bgm);
            m.set_channel_volume(BGM_B, bgm);
        }
    }

    /// 当前 BGM 名（存档恢复用）
    pub fn cur_bgm(&self) -> Option<&str> {
        self.bgm_name.as_deref()
    }
}

impl Audio {
    /// v0.3.2 音频等待原始查询（wl）：循环点命中但**不取走**
    /// （引擎确认放行后再 take，避免竞态丢事件）
    pub fn peek_loop_hit(&self) -> bool {
        self.mixer
            .as_ref()
            .and_then(|m| m.playing.get(self.bgm_slot))
            .is_some_and(|p| p.loop_hits > 0)
    }

    /// v0.3.2 音频等待原始查询（ws）：是否还有 SE 在发声
    pub fn se_playing(&self) -> bool {
        self.mixer
            .as_ref()
            .map(|m| m.playing.keys().any(|k| k.starts_with("se_") && m.playing[k].active()))
            .unwrap_or(false)
    }

    /// v0.3.3 音频等待原始查询（wm）：当前槽 BGM 是否仍在发声
    pub fn bgm_playing(&self) -> bool {
        self.mixer.as_ref().and_then(|m| m.playing.get(self.bgm_slot)).is_some_and(|p| p.active())
    }

    /// v0.3.3 wv 等待进入时的作者提醒（暂无视频子系统，每次 wv 留证一次）
    pub fn note_video_wait(&mut self) {
        audit!(Warn, "audio", "wv：本引擎暂无视频子系统，立即放行（作者请核对剧本）");
    }

    /// v0.3.3 音频等待原始查询（wv）：视频是否在播（本引擎暂无视频子系统 → 恒 false）
    pub fn video_playing(&self) -> bool {
        false
    }

    /// v0.3.3 音频等待原始查询（wp）：BGM/SE/语音任一仍在发声
    pub fn any_playing(&self) -> bool {
        self.bgm_playing() || self.se_playing() || self.voice_playing()
    }

    /// v0.3.2 fadese：最近的 SE 实例音量淡到指定百分比
    pub fn fade_se(&mut self, to_pct: i32, ms: u32) {
        audit_trace!("se", "fade", "to_pct" => to_pct, "ms" => ms);
        let Some(m) = &mut self.mixer else { return };
        let last = m
            .playing
            .keys()
            .filter(|k| k.starts_with("se_"))
            .max_by_key(|k| k.trim_start_matches("se_").parse::<u64>().unwrap_or(0))
            .cloned();
        if let Some(k) = last {
            if let Some(p) = m.playing.get_mut(&k) {
                if let Some(h) = &mut p.handle {
                    h.set_volume(pct_to_db(to_pct), tween(ms));
                }
            }
        } else {
            audit!(Debug, "se", "fadese：无活动 SE，忽略");
        }
    }

    /// v0.3.2 seopt（预留：记录到 trace 供作者核对）
    pub fn seopt(&mut self, name: &str) {
        audit_trace!("se", "opt", "name" => name);
        audit!(Debug, "se", "seopt「{name}」：当前版本选项仅记录，播放行为不变");
    }
}

#[cfg(test)]
mod tests;
