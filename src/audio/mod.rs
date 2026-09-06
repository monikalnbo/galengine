//! 音频系统：BGM（循环）/ SE（一次）/ 音量调节（0-100）。
//! 无声卡环境（xvfb 验收）自动降级静音模式；文件缺失静默跳过（docs/20 §3.3）。

use std::path::PathBuf;

use sdl2::mixer::{self, InitFlag};

pub struct AudioSys {
    _init: mixer::Sdl2MixerContext,
    bgm_volume: i32,
    se_volume: i32,
    current: Option<String>,
}

impl AudioSys {
    /// 初始化失败（无声卡）返回 None，调用方静默降级
    pub fn new(bgm_volume: i32, se_volume: i32) -> Option<Self> {
        let init = mixer::init(InitFlag::OGG | InitFlag::MP3).ok()?;
        mixer::open_audio(44100, mixer::AUDIO_S16LSB, 2, 1024).ok()?;
        mixer::allocate_channels(8);
        let s = Self {
            _init: init,
            bgm_volume,
            se_volume,
            current: None,
        };
        s.apply_bgm_volume();
        Some(s)
    }

    fn apply_bgm_volume(&self) {
        mixer::Music::set_volume(self.bgm_volume);
    }

    /// 播放 BGM（循环）；同曲不重启；缺失静默
    pub fn bgm(&mut self, name: &str) {
        if self.current.as_deref() == Some(name) {
            return;
        }
        match find_audio("bgm", name).and_then(|p| mixer::Music::from_file(&p).ok()) {
            Some(m) => {
                let _ = m.play(-1);
                self.current = Some(name.to_string());
            }
            None => eprintln!("[audio] BGM 缺失，静默跳过：{name}"),
        }
    }

    #[allow(dead_code)]
    pub fn stop_bgm(&mut self) {
        let _ = mixer::Music::halt();
        self.current = None;
    }

    /// 播放 SE（一次）
    pub fn se(&self, name: &str) {
        if let Some(mut c) = find_audio("sound", name).and_then(|p| mixer::Chunk::from_file(&p).ok()) {
            c.set_volume(self.se_volume);
            let _ = mixer::Channel::all().play(&c, 0);
        } else {
            eprintln!("[audio] SE 缺失，静默跳过：{name}");
        }
    }

    pub fn set_bgm_volume(&mut self, v: i32) {
        self.bgm_volume = v.clamp(0, 100);
        self.apply_bgm_volume();
    }

    pub fn set_se_volume(&mut self, v: i32) {
        self.se_volume = v.clamp(0, 100);
    }

    pub fn volumes(&self) -> (i32, i32) {
        (self.bgm_volume, self.se_volume)
    }
}

/// 在 data 下找音频文件：{sub}/{name}.{ogg|mp3|wav}
fn find_audio(sub: &str, name: &str) -> Option<PathBuf> {
    let data = crate::config::data_dir();
    ["ogg", "mp3", "wav"]
        .iter()
        .map(|ext| format!("{data}/{sub}/{name}.{ext}"))
        .find(|p| std::path::Path::new(p).exists())
        .map(PathBuf::from)
}
