//! SDL2_mixer 封装：bgm 循环 / se 一次；缺文件静默；无声卡降级静音。
//! 音量 0-100 存 sf.*（volBgm/volSe），变更即套用。

use sdl2::mixer::{Channel, Chunk, Music, AUDIO_S16LSB};
use std::collections::HashMap;

use crate::config;

pub struct Audio {
    /// 无声卡/初始化失败 → 全部操作静默
    ok: bool,
    music: Option<Music<'static>>,
    chunks: HashMap<String, Chunk>,
    pub bgm_vol: i32,
    pub se_vol: i32,
}

fn first_path(dir: &str, name: &str) -> Option<String> {
    ["ogg", "mp3", "wav"]
        .iter()
        .map(|ext| format!("{}/{}/{}.{}", config::data_dir(), dir, name, ext))
        .find(|p| std::path::Path::new(p).exists())
}

impl Audio {
    pub fn new(bgm_vol: i32, se_vol: i32) -> Self {
        let ok = sdl2::mixer::open_audio(44100, AUDIO_S16LSB, 2, 1024).is_ok();
        sdl2::mixer::allocate_channels(16);
        let mut a = Self { ok, music: None, chunks: HashMap::new(), bgm_vol, se_vol };
        a.apply_volumes();
        a
    }

    pub fn apply_volumes(&mut self) {
        Music::set_volume(self.bgm_vol.clamp(0, 100) * 128 / 100);
        let se = self.se_vol.clamp(0, 100) * 128 / 100;
        for c in self.chunks.values_mut() {
            c.set_volume(se);
        }
    }

    pub fn play_bgm(&mut self, name: &str) {
        if !self.ok {
            return;
        }
        if let Some(p) = first_path("bgm", name) {
            match Music::from_file(&p) {
                Ok(m) => {
                    self.music = Some(m);
                    let _ = self.music.as_ref().unwrap().play(-1);
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
    }

    pub fn play_se(&mut self, name: &str) {
        if !self.ok {
            return;
        }
        if let Some(p) = first_path("sound", name) {
            let chunk = match self.chunks.get(name) {
                Some(_) => None, // 已缓存
                None => Chunk::from_file(&p).ok(),
            };
            if let Some(c) = chunk {
                self.chunks.insert(name.to_string(), c);
            }
            if let Some(c) = self.chunks.get_mut(name) {
                c.set_volume(self.se_vol.clamp(0, 100) * 128 / 100);
                let _ = Channel::all().play(c, 0);
            }
        }
    }
}
