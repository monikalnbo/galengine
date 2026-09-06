//! L1 系统动作：存/读档执行、缩略图、CG 清单、演出事件（音频/meta/越界/抖动/关机）。
//! app.rs 主循环调用；不接触事件泵。

use std::io::Cursor;

use sdl2::pixels::PixelFormatEnum;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::audio::AudioSys;
use crate::config::Config;
use crate::gfx::assets::resolve;
use crate::gfx::renderer::Renderer;
use crate::save::meta::MetaState;
use crate::save::slots::{now_str, SaveData, SlotStore};
use crate::script::interp::{GameEvent, Interp};
use crate::ui::overlay::OverlaySys;

/// 越界演出状态（reach）
pub struct ReachState {
    pub path: String,
    pub t: f32,
    pub dur: f32,
    pub scale: f32,
}

/// 窗口抖动状态
pub struct ShakeState {
    pub left: f32,
    pub total: f32,
}

pub struct Systems {
    /// 窗口原标题（window_fx title 恢复用）
    pub base_title: String,
    pub store: SlotStore,
    pub meta: MetaState,
    pub audio: Option<AudioSys>,
    /// 数据目录中的全部 CG storage
    pub all_cgs: Vec<String>,
    pub reach: Option<ReachState>,
    pub shake: Option<ShakeState>,
    /// 一次性状态消息（错误/提示，主循环显示）
    pub flash_msg: Option<String>,
    save_dir: String,
}

impl Systems {
    pub fn new(conf: &Config) -> Self {
        let store = SlotStore::new(&conf.save_dir, conf.save_slots);
        let meta = MetaState::load(&conf.save_dir);
        let audio = AudioSys::new(conf.audio.bgm_volume, conf.audio.se_volume);
        if audio.is_none() {
            eprintln!("[audio] 无音频设备，静音模式");
        }
        let mut s = Self {
            base_title: conf.title.clone(),
            store,
            meta,
            audio,
            all_cgs: Vec::new(),
            reach: None,
            shake: None,
            flash_msg: None,
            save_dir: conf.save_dir.clone(),
        };
        s.scan_cgs();
        s
    }

    /// 扫描数据目录 cg_*.jpg/png → 鉴赏清单（引擎能力，内容游戏侧）
    pub fn scan_cgs(&mut self) {
        let dir = format!("{}/bgimage", crate::config::data_dir());
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                let stem = name.split('.').next().unwrap_or("").to_string();
                if stem.starts_with("cg_") {
                    out.push(stem);
                }
            }
        }
        out.sort();
        self.all_cgs = out;
    }

    pub fn volumes(&self) -> (i32, i32) {
        self.audio.as_ref().map(|a| a.volumes()).unwrap_or((0, 0))
    }

    // —— 存读档 ——

    /// 执行存档（slot 1..=9；0=快速存档）
    pub fn do_save(
        &mut self,
        interp: &Interp,
        canvas: &Canvas<Window>,
        slot: usize,
    ) -> Result<(), String> {
        if interp.state == crate::script::interp::RunState::Ended {
            return Err("当前无法存档".into());
        }
        let snap = interp.stage.snapshot();
        let data = SaveData {
            file: interp.file.clone(),
            pc: interp.pc_at_wait(),
            f_vars: interp.vars.f.clone(),
            stage: snap,
            thumb: format!("thumbs/slot{slot}.png"),
            saved_at: now_str(),
            digest: interp.current_digest(),
            chapter: interp.file.clone(),
        };
        let png = thumbnail(canvas);
        if slot == 0 {
            self.store.save(0, &data, &png).map_err(|e| e.to_string())?;
        } else {
            self.store.save(slot, &data, &png)?;
        }
        Ok(())
    }

    /// 执行读档；破損/假档在此拦截（meta 演出数据在 UI 层呈现）
    pub fn do_load(&self, interp: &mut Interp, slot: usize) -> Result<(), String> {
        if self.meta.is_corrupted(slot) {
            return Err("这格存档已经破損，无法读取。".into());
        }
        if slot == self.store.slots && self.meta.fake_save.is_some() {
            return Err("无法读取。".into());
        }
        let d = self.store.load(slot)?;
        interp.restore(&d.file, d.pc, d.f_vars, d.digest, &d.stage)
    }

    // —— 剧本演出事件 ——

    pub fn handle_events(
        &mut self,
        events: &[GameEvent],
        overlay: &mut OverlaySys,
        canvas: &mut Canvas<Window>,
        conf: &Config,
    ) {
        for ev in events {
            match ev {
                GameEvent::Bgm(n) => {
                    if let Some(a) = self.audio.as_mut() {
                        a.bgm(n);
                    }
                }
                GameEvent::Se(n) => {
                    if let Some(a) = self.audio.as_ref() {
                        a.se(n);
                    }
                }
                GameEvent::FakeSave { date, time, image } => {
                    self.meta.fake_save = Some(crate::save::meta::FakeSave {
                        date: date.clone(),
                        time: time.clone(),
                        thumb: image.clone(),
                    });
                    let _ = self.meta.save(&self.save_dir);
                }
                GameEvent::Corrupt(n) => {
                    let targets: Vec<usize> = if n == "all" {
                        (1..=self.store.slots).collect()
                    } else {
                        n.parse::<usize>().into_iter().collect()
                    };
                    for t in targets {
                        if !self.meta.corrupted.contains(&t) {
                            self.meta.corrupted.push(t);
                        }
                    }
                    let _ = self.meta.save(&self.save_dir);
                }
                GameEvent::DeleteLast => {
                    let slot = self.store.last_used().unwrap_or(0);
                    if slot > 0 {
                        overlay.open_ritual(slot);
                    }
                }
                GameEvent::TitleEvolve(mode) => {
                    if let Ok(st) = mode.parse::<u32>() {
                        self.meta.title_stage = st;
                        let _ = self.meta.save(&self.save_dir);
                    }
                }
                GameEvent::Reach { storage, dur_ms, scale } => {
                    self.reach = Some(ReachState {
                        path: resolve("img", storage),
                        t: 0.0,
                        dur: (*dur_ms as f32).max(1.0),
                        scale: *scale,
                    });
                }
                GameEvent::Shake(ms) => {
                    self.shake = Some(ShakeState {
                        left: *ms as f32,
                        total: *ms as f32,
                    });
                }
                GameEvent::TitleFx(t) => {
                    let _ = canvas.window_mut().set_title(t);
                }
                GameEvent::Shutdown(secs) => {
                    let _ = crate::platform::schedule_shutdown(*secs, &conf.game.shutdown_message);
                    overlay.open_rest(*secs);
                }
                GameEvent::DesktopWrite { file, content } => {
                    match crate::platform::desktop_write(file, content) {
                        Ok(p) => self.flash_msg = Some(format!("已写到桌面：{p}")),
                        Err(e) => eprintln!("[platform] {e}"),
                    }
                }
                GameEvent::ReachHide => {
                    self.reach = None;
                }
                GameEvent::TitleRestore => {
                    let _ = canvas.window_mut().set_title(&self.base_title);
                }
                GameEvent::DesktopOpen(file) => {
                    if let Some(dir) = crate::platform::desktop_dir() {
                        let p = dir.join(&file);
                        if let Err(e) = crate::platform::open_file(&p.to_string_lossy()) {
                            eprintln!("[platform] {e}");
                        }
                    }
                }
            }
        }
    }

    /// 帧推进：越界/抖动动画
    pub fn tick(&mut self, dt_ms: f32, canvas: &mut Canvas<Window>) {
        if let Some(r) = self.reach.as_mut() {
            r.t += dt_ms;
            if r.t >= r.dur {
                // 完成：保持满幅直到剧本收（简单起见动画完成后停驻）
                r.t = r.dur;
            }
        }
        if let Some(s) = self.shake.as_mut() {
            s.left -= dt_ms;
            let p = (s.left / s.total).clamp(0.0, 1.0);
            let dx = (s.left * 0.08).sin() * 7.0 * p;
            let pos = canvas.window().position();
            let _ = canvas.window_mut().set_position(
                sdl2::video::WindowPos::Positioned(pos.0 as i32 + dx as i32),
                sdl2::video::WindowPos::Positioned(pos.1),
            );
            if s.left <= 0.0 {
                self.shake = None;
            }
        }

    }
}

/// 窗口游戏区 → 320x180 PNG（存档缩略图）
pub fn thumbnail(canvas: &Canvas<Window>) -> Vec<u8> {
    let r = Renderer::letterbox(canvas);
    let Ok(px) = canvas.read_pixels(r, PixelFormatEnum::RGBA8888) else {
        return Vec::new();
    };
    let Some(img) = image::RgbaImage::from_raw(r.width(), r.height(), px) else {
        return Vec::new();
    };
    let small = image::imageops::resize(&img, 320, 180, image::imageops::FilterType::Triangle);
    let mut buf = Cursor::new(Vec::new());
    if small
        .write_to(&mut buf, image::ImageFormat::Png)
        .is_ok()
    {
        buf.into_inner()
    } else {
        Vec::new()
    }
}
