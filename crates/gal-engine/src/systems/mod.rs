//! L1 系统动作与状态枢纽：消费 interp 的 SysEvent 队列，
//! 分发音频/窗口/平台/存档 meta/越界/抖动/关机；Game 为各层共享的非纹理状态。

mod events;
mod lifecycle;
mod types;

pub use types::{Msg, ReachSt, RestSt, ShakeSt, Systems};

use gal_audio::Audio;
use gal_config::Config;
use gal_save::meta::MetaState;
use gal_save::slots::{self, SaveEntry};
use gal_script::interp::Interp;
use gal_ui::dialog::DialogStyle;
use gal_ui::inputbox::InputUi;
use gal_ui::overlay::Overlay;

/// L1 共享状态（无 SDL 纹理借用，纹理对象由 app 持有按参传递）
pub struct Game {
    pub interp: Interp,
    pub sys: Systems,
    pub overlay: Overlay,
    /// 自定义鼠标两态（未配置=系统光标）
    pub cursor: gal_ui::cursor::CursorMgr,
    /// 已挂载拓展（yaml 声明）
    pub exts: Vec<Box<dyn gal_ext::Extension>>,
    pub choice_sel: usize,
    pub input_ui: Option<InputUi>,
    /// 存档界面展示用槽位快照（开界面时刷新）
    pub save_entries: Vec<(usize, Option<SaveEntry>)>,
    /// 标题入口：并未进入剧本（Ended 但不退出）
    pub started: bool,
    pub start_script: String,
    pub start_label: Option<String>,
    pub auto: bool,
    pub auto_acc: f32,
    pub ctrl_hold: bool,
    /// 右键隐藏 UI 纯赏图标记
    pub hide_ui: bool,
    pub conf: Config,
    pub style: DialogStyle,
    pub now_ms: f32,
}

impl Game {
    pub fn new(conf: Config, interp: Interp) -> Self {
        let meta_path = format!("{}/meta.json", conf.save.dir);
        let meta = MetaState::load(&meta_path);
        let audio = Audio::new(conf.audio.bgm_volume, conf.audio.se_volume);
        let style = DialogStyle::from_cfg(&conf.ui.dialog, conf.game.name_colors.clone());
        let sys = Systems {
            audio,
            meta,
            meta_path,
            save_dir: conf.save.dir.clone(),
            slots: conf.save.slots,
            base_title: conf.meta.title.clone(),
            shake: None,
            reach: None,
            rest: None,
            msg: None,
        };
        Self {
            interp,
            sys,
            overlay: Overlay::None,
            cursor: gal_ui::cursor::CursorMgr::system(),
            exts: Vec::new(),
            choice_sel: 0,
            input_ui: None,
            save_entries: Vec::new(),
            started: false,
            start_script: String::new(),
            start_label: None,
            auto: false,
            auto_acc: 0.0,
            ctrl_hold: false,
            hide_ui: false,
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
        self.save_entries =
            (1..=self.sys.slots).map(|s| (s, slots::load(&self.sys.save_dir, s))).collect();
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
                self.sys.rest = None;
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
}
