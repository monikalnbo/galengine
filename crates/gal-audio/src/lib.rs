//! SDL2_mixer 封装：bgm 循环 / se 一次 / voice 一次（播放时压低 BGM）；缺文件静默；无声卡降级静音。
//! 音量 0-100 存 sf.*（volBgm/volSe/volVoice），变更即套用。

use sdl2::mixer::{Channel, Chunk, Music, AUDIO_S16LSB};
use std::collections::HashMap;

use gal_config::{Config, VoiceCfg};

/// 语音独占通道（其余通道给 SE）
const VOICE_CHANNEL: Channel = Channel(0);

pub struct Audio {
    /// 无声卡/初始化失败 → 全部操作静默
    ok: bool,
    music: Option<Music<'static>>,
    fading_music: Option<Music<'static>>,
    chunks: HashMap<String, Chunk>,
    /// 当前语音（键=查找名，解码后缓存一次）
    voice: Option<(String, Chunk)>,
    pub bgm_vol: i32,
    pub se_vol: i32,
    pub voice_vol: i32,
    /// 语音播放时 BGM/SE 压低比例（>0 生效）
    pub duck: f32,
    /// 当前是否处于压低状态（语音自然播完后要恢复一次）
    ducked: bool,
    /// BGM 切换淡入时长（作者 audio.fade_ms）
    fade_ms: u32,
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
fn voice_path(cfg: &VoiceCfg, name: &str) -> Option<String> {
    ["ogg", "mp3", "wav"].iter().find_map(|ext| {
        let p = format!("{}/{}/{}.{}", gal_config::data_dir(), cfg.dir, name, ext);
        std::path::Path::new(&p).exists().then_some(p)
    })
}

impl Audio {
    pub fn new(conf: &Config) -> Self {
        let ok = sdl2::mixer::open_audio(44100, AUDIO_S16LSB, 2, 1024).is_ok();
        sdl2::mixer::allocate_channels(16);
        let _ = sdl2::mixer::reserve_channels(1); // 0 号留语音
        let mut a = Self {
            ok,
            music: None,
            fading_music: None,
            chunks: HashMap::new(),
            voice: None,
            interrupted: None,
            bgm_vol: conf.audio.bgm_volume,
            se_vol: conf.audio.se_volume,
            voice_vol: conf.voice.volume,
            duck: conf.voice.duck,
            ducked: false,
            fade_ms: conf.audio.fade_ms,
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

    pub fn pump(&mut self) {
        if self.fading_music.is_some() && !sdl2::mixer::Music::is_playing() {
            self.fading_music = None;
        }
    }

    /// 语音是否在播（duck 判断用）
    fn voice_playing(&self) -> bool {
        self.ok && VOICE_CHANNEL.is_playing()
    }

    pub fn apply_volumes(&mut self) {
        let duck = if self.voice_playing() && self.duck > 0.0 && self.duck < 1.0 {
            self.duck
        } else {
            1.0
        };
        let bgm = (self.bgm_vol.clamp(0, 100) as f32 * duck) as i32;
        Music::set_volume(bgm * 128 / 100);
        let se = (self.se_vol.clamp(0, 100) as f32 * duck) as i32;
        let se = (se * 128 / 100).clamp(0, 128);
        for c in self.chunks.values_mut() {
            c.set_volume(se);
        }
        VOICE_CHANNEL.set_volume(self.voice_vol.clamp(0, 100) * 128 / 100);
    }

    pub fn play_bgm(&mut self, name: &str) {
        if !self.ok {
            return;
        }
        self.fading_music = None;
        let exts = ["ogg", "mp3", "wav"];
        if let Some(p) = first_path(&self.bgm_dir, name, &exts) {
            match Music::from_file(&p) {
                Ok(m) => {
                    self.music = Some(m);
                    let m = self.music.as_ref().unwrap();
                    // 作者 audio.fade_ms：切歌淡入而非硬切
                    let r = if self.fade_ms > 0 {
                        m.fade_in(-1, self.fade_ms as i32)
                    } else {
                        m.play(-1)
                    };
                    let _ = r;
                    self.apply_volumes();
                }
                Err(e) => eprintln!("[audio] bgm 加载失败：{p}（{e}）"),
            }
        } // 缺文件静默（规格）
    }

    pub fn stop_bgm(&mut self) {
        if self.ok {
            sdl2::mixer::Music::halt();
        }
        self.music = None;
        self.fading_music = None;
    }

    /// 淡出后停止（保留句柄由 pump/切换时释放，避免立即 Drop 掐断声音）
    pub fn fade_out_bgm(&mut self, ms: u32) {
        if self.ok {
            let _ = sdl2::mixer::Music::fade_out(ms.max(100) as i32);
        }
        self.fading_music = self.music.take();
    }

    pub fn play_se(&mut self, name: &str) {
        if !self.ok {
            return;
        }
        if let Some(p) = first_path(&self.se_dir, name, &["ogg", "mp3", "wav"]) {
            let chunk = match self.chunks.get(name) {
                Some(_) => None, // 已缓存
                None => Chunk::from_file(&p).ok(),
            };
            if let Some(c) = chunk {
                self.chunks.insert(name.to_string(), c);
            }
            let duck = if self.voice_playing() && self.duck > 0.0 && self.duck < 1.0 {
                self.duck
            } else {
                1.0
            };
            // SE 占用全部通道 → 正在播的语音被顶掉，玩家容易困惑；记一笔提示
            if self.voice_playing() && self.voice.as_ref().is_some_and(|(k, _)| k != name) {
                self.interrupted = Some(name.to_string());
            }
            if let Some(c) = self.chunks.get_mut(name) {
                c.set_volume((self.se_vol.clamp(0, 100) as f32 * duck) as i32 * 128 / 100);
                let _ = Channel::all().play(c, 0);
            }
        }
    }

    /// 播放语音（同一时刻只保留一条；播完由 mixer 自然结束）
    pub fn play_voice(&mut self, name: &str) {
        if !self.ok {
            return;
        }
        let cfg = VoiceCfg { dir: self.voice_dir.clone(), volume: self.voice_vol, duck: self.duck };
        let Some(p) = voice_path(&cfg, name) else { return };
        let hit = self.voice.as_ref().is_some_and(|(k, _)| k == name);
        if !hit {
            match Chunk::from_file(&p) {
                Ok(c) => self.voice = Some((name.to_string(), c)),
                Err(e) => {
                    eprintln!("[audio] voice 加载失败：{p}（{e}）");
                    return;
                }
            }
        }
        if let Some((_, c)) = &self.voice {
            VOICE_CHANNEL.set_volume(self.voice_vol.clamp(0, 100) * 128 / 100);
            let _ = VOICE_CHANNEL.play(c, 0);
            self.apply_volumes(); // 立即压低其它通道
        }
    }

    /// 取走「语音被 SE 打断」事件（每帧调用；一次事件只报一次）
    pub fn take_interrupted(&mut self) -> Option<String> {
        self.interrupted.take()
    }

    pub fn stop_voice(&mut self) {
        if self.ok {
            VOICE_CHANNEL.halt();
            self.apply_volumes();
        }
    }

    /// 语音自然播完后恢复 BGM/SE 音量（每帧调用；只在状态切换时动音量）
    pub fn update_duck(&mut self) {
        let want = self.voice_playing() && self.duck > 0.0 && self.duck < 1.0;
        if self.ducked != want {
            self.ducked = want;
            self.apply_volumes();
        }
    }
}
