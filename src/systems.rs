//! L1 系统动作与状态枢纽：消费 interp 的 SysEvent 队列，
//! 分发音频/窗口/平台/存档 meta/越界/抖动/关机；Game 为各层共享的非纹理状态。

use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::audio::Audio;
use crate::config::Config;
use crate::save::meta::MetaState;
use crate::save::slots::{self, SaveEntry};
use crate::script::interp::{Interp, RunState, SysEvent};
use crate::ui::dialog::DialogStyle;
use crate::ui::inputbox::InputUi;
use crate::ui::overlay::Overlay;

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
}

/// L1 共享状态（无 SDL 纹理借用，纹理对象由 app 持有按参传递）
pub struct Game {
    pub interp: Interp,
    pub sys: Systems,
    pub overlay: Overlay,
    pub choice_sel: usize,
    pub input_ui: Option<InputUi>,
    /// 存档界面展示用槽位快照（开界面时刷新）
    pub save_entries: Vec<(usize, Option<SaveEntry>)>,
    pub auto: bool,
    pub auto_acc: f32,
    pub ctrl_hold: bool,
    pub conf: Config,
    pub style: DialogStyle,
    pub now_ms: f32,
}

impl Game {
    pub fn new(conf: Config, interp: Interp) -> Self {
        let meta_path = format!("{}/meta.json", conf.save_dir);
        let meta = MetaState::load(&meta_path);
        let audio = Audio::new(conf.audio.bgm_volume, conf.audio.se_volume);
        let style = DialogStyle::from_cfg(&conf.dialog, conf.game.name_colors.clone());
        let sys = Systems {
            audio,
            meta,
            meta_path,
            save_dir: conf.save_dir.clone(),
            slots: conf.save_slots,
            base_title: conf.title.clone(),
            shake: None,
            reach: None,
            rest: None,
            msg: None,
        };
        Self {
            interp,
            sys,
            overlay: Overlay::None,
            choice_sel: 0,
            input_ui: None,
            save_entries: Vec::new(),
            auto: false,
            auto_acc: 0.0,
            ctrl_hold: false,
            style,
            conf,
            now_ms: 0.0,
        }
    }

    /// 中文提示 toast（3s）
    pub fn msg(&mut self, text: impl Into<String>) {
        self.sys.msg = Some(Msg { text: text.into(), left_ms: 3000.0 });
    }

    /// 刷新存档界面快照
    pub fn refresh_saves(&mut self) {
        self.save_entries = (1..=self.sys.slots)
            .map(|s| (s, slots::load(&self.sys.save_dir, s)))
            .collect();
    }

    /// 帧计时：抖动/越界淡入淡出/关机倒计时/提示/仪式渐白
    pub fn tick(&mut self, dt_ms: f32) {
        self.now_ms += dt_ms;
        if let Some(s) = &mut self.sys.shake {
            s.left_ms -= dt_ms;
            if s.left_ms <= 0.0 {
                self.sys.shake = None;
            }
        }
        if let Some(r) = &mut self.sys.reach {
            r.t += dt_ms;
            if r.hiding && r.t >= 600.0 {
                self.sys.reach = None;
            }
        }
        if let Some(m) = &mut self.sys.msg {
            m.left_ms -= dt_ms;
            if m.left_ms <= 0.0 {
                self.sys.msg = None;
            }
        }
        if let Some(r) = &mut self.sys.rest {
            r.left_ms -= dt_ms;
            if r.left_ms <= 0.0 {
                self.sys.rest = None; // Windows：真关机由系统接管；验收环境仅日志
                eprintln!("[platform] 关机倒计时结束（验收环境不执行）");
            }
        }
        if let Some(ui) = &mut self.input_ui {
            ui.blink += dt_ms;
        }
        // 仪式删档：三击后渐白，白满真删（备份 backup/）
        let mut ritual_done = false;
        if let Overlay::Ritual { step, fade, done } = &mut self.overlay {
            if *step >= 3 && !*done {
                *fade += dt_ms / 1200.0;
                if *fade >= 1.0 {
                    *done = true;
                    ritual_done = true;
                }
            }
        }
        if ritual_done {
            let n = self.sys.slots;
            let dir = self.sys.save_dir.clone();
            if let Some(slot) = slots::delete_latest(&dir, n) {
                self.msg(format!("……删掉了。（槽 {slot} 已备份）"));
            }
            self.overlay = Overlay::None;
        }
    }

    /// 消费解释器系统事件（每帧）
    pub fn drain_events(&mut self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let events = std::mem::take(&mut self.interp.events);
        for ev in events {
            self.apply_event(ev, canvas)?;
        }
        Ok(())
    }

    fn apply_event(&mut self, ev: SysEvent, canvas: &mut Canvas<Window>) -> Result<(), String> {
        match ev {
            SysEvent::Bgm(n) => self.sys.audio.play_bgm(&n),
            SysEvent::Se(n) => self.sys.audio.play_se(&n),
            SysEvent::MetaFake { date, time, image } => {
                self.sys.meta.fake_saves.push(crate::save::meta::FakeSave { date, time, image });
                self.sys.meta.save(&self.sys.meta_path);
            }
            SysEvent::MetaCorrupt(n) => {
                let all: Vec<usize> = (1..=self.sys.slots).collect();
                let target: Vec<usize> = if n == "all" {
                    all
                } else {
                    vec![n.parse().map_err(|_| format!("meta_corrupt 参数无效：{n}"))?]
                };
                for s in target {
                    if !self.sys.meta.corrupt.contains(&s) {
                        self.sys.meta.corrupt.push(s);
                    }
                }
                self.sys.meta.save(&self.sys.meta_path);
            }
            SysEvent::MetaDeleteLast => {
                // 有可删的存档才进入仪式；否则提示并跳过（M3 slots 接入真删）
                if crate::save::slots::latest_slot(&self.sys.save_dir, self.sys.slots).is_some() {
                    self.overlay = Overlay::Ritual { step: 0, fade: 0.0, done: false };
                } else {
                    self.msg("这里没有可以删除的存档。");
                }
            }
            SysEvent::TitleEvolve(m) => {
                self.sys.meta.title_evolve = m;
                self.sys.meta.save(&self.sys.meta_path);
            }
            SysEvent::Reach { path, dur_ms, scale } => {
                self.sys.reach = Some(ReachSt { path, t: 0.0, dur_ms: dur_ms as f32, scale, hiding: false });
            }
            SysEvent::ReachHide => {
                if let Some(r) = &mut self.sys.reach {
                    r.hiding = true;
                    r.t = 0.0;
                }
            }
            SysEvent::Shake(ms) => {
                self.sys.shake = Some(ShakeSt { left_ms: ms as f32, dur_ms: ms as f32 });
            }
            SysEvent::SetTitle(t) => {
                let _ = canvas.window_mut().set_title(&t);
            }
            SysEvent::RestoreTitle => {
                let base = self.sys.base_title.clone();
                let _ = canvas.window_mut().set_title(&base);
            }
            SysEvent::Shutdown(sec) => {
                crate::platform::arm_shutdown(sec);
                self.sys.rest = Some(RestSt { left_ms: sec as f32 * 1000.0 });
            }
            SysEvent::DesktopWrite { file, content } => {
                if let Err(e) = crate::platform::desktop_write(&file, &content) {
                    self.msg(e);
                }
            }
            SysEvent::DesktopOpen(file) => {
                if let Err(e) = crate::platform::desktop_open(&file) {
                    self.msg(e);
                }
            }
        }
        Ok(())
    }

    /// 关机取消（Rest 覆盖层按钮）
    pub fn cancel_rest(&mut self) {
        crate::platform::cancel_shutdown();
        self.sys.rest = None;
    }

    /// 台词态是否可存档（等待点=对话行）
    pub fn can_save(&self) -> bool {
        self.interp.state == RunState::WaitClick && self.interp.resume_point().is_some()
    }
}
