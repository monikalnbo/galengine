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
    /// 玩家当前皮肤（sf.skinIndex → ui.skin 列表）
    pub skin: gal_config::SkinCfg,
    pub now_ms: f32,
    /// 字色/字号/行距变化后需重建排版（下一帧生效）
    pub style_dirty: bool,
    /// 自动存档计时（save.auto_save_ms；0=关闭）
    pub autosave_acc: f32,
    /// 可用字体族名（FontBook 实况；设置面板枚举用）
    pub font_families: Vec<String>,
    /// 设置面板「按键重绑」进行中：待绑定的动作名
    pub rebind: Option<String>,
    /// 最近一次存/读的槽位（自动存档写回这里）
    pub last_slot: usize,
    /// 实际皮肤列表（ui.skin.schemes + skins/*.yaml 主题包合并）
    pub skins: Vec<gal_config::SkinCfg>,
    /// 可用语言包（lang/*.yaml；空 = 无多语言）
    pub langs: Vec<gal_config::LangPack>,
    /// 当前语言包（空表 = 作者原文）
    pub lang: gal_config::LangPack,
    /// 朗读状态（后端/开关/当前子进程）
    pub tts: TtsState,
    /// 「载入系统字体」已请求（主循环下一帧执行）
    pub scan_sys_fonts: bool,
    /// 已打印过选项本地化日志的状态指纹（避免每帧刷屏）
    pub lang_logged: u64,
    /// 验收设置脚本已执行步数（ES_DEBUG_SETTINGS_STEPS）
    pub auto_steps: usize,
    /// 脚本是否已跑完（跑完把界面复位，避免卡在设置面板）
    pub auto_steps_done: bool,
}

/// 无障碍朗读状态（不新增依赖：系统命令后台朗读）
pub struct TtsState {
    pub backend: Option<gal_platform::tts::Backend>,
    pub enabled: bool,
    pub rate: i32,
    pub child: Option<std::process::Child>,
    /// 最近一句（同句不重复朗读）
    last: String,
}

impl TtsState {
    pub fn new(conf: &gal_config::Config) -> Self {
        Self {
            backend: gal_platform::tts::detect(),
            enabled: conf.tts.on,
            rate: conf.tts.rate,
            child: None,
            last: String::new(),
        }
    }

    /// 朗读一句（去重；后端不可用/关闭时静默）
    pub fn speak(&mut self, text: &str) {
        if !self.enabled {
            return;
        }
        let Some(b) = self.backend else { return };
        let text = gal_text::layout::plain(text);
        if text.trim().is_empty() || text == self.last {
            return;
        }
        self.stop();
        self.last = text.clone();
        self.child = gal_platform::tts::speak(b, &text, self.rate);
    }

    pub fn stop(&mut self) {
        gal_platform::tts::stop(&mut self.child);
    }

    /// 每帧：子进程自然结束就清句柄
    pub fn tick(&mut self) {
        gal_platform::tts::alive(&mut self.child);
    }
}

/// sf.language 选中的语言包（0/越界 = 作者原文；n = langs[n-1]）
pub fn lang_for(
    langs: &[gal_config::LangPack],
    vars: &gal_script::vars::Vars,
) -> gal_config::LangPack {
    let idx = vars.sf.get("language").map(|v| v.as_i64().max(0) as usize).unwrap_or(0);
    match idx.checked_sub(1).and_then(|i| langs.get(i)) {
        Some(p) => p.clone(),
        None => gal_config::LangPack::default(),
    }
}

/// 合成生效样式：作者 ui.dialog/text + 玩家 sf.*（字号/行距/皮肤）。
/// `skins` 为「ui.skin.schemes + skins/*.yaml 主题包」合并后的实况列表。
pub fn compose_style(
    conf: &Config,
    skins: &[gal_config::SkinCfg],
    skin_index: Option<usize>,
    vars: &gal_script::vars::Vars,
) -> DialogStyle {
    let get = |k: &str| vars.sf.get(k).map(|v| v.as_i64()).unwrap_or(0);
    let psize = get("textSize").clamp(0, 255) as u16;
    let pspace = get("lineSpacing").clamp(0, 1_000) as i32;
    let style = DialogStyle::from_cfg_text(
        &conf.ui.dialog,
        &conf.text,
        conf.game.name_colors.clone(),
        psize,
        pspace,
    );
    let idx = skin_index.unwrap_or_else(|| get("skinIndex").max(0) as usize);
    let skin = skins.get(idx).cloned().unwrap_or_else(|| skins[0].clone());
    let mut style = style.with_skin(&skin);
    // 玩家开关页满等待标记（0/1；未设置 = 跟随作者 text.wait_marker）
    if let Some(v) = vars.sf.get("waitMarker") {
        style.wait_marker = v.as_i64() == 1;
    }
    // 玩家描边宽度（0 = 跟随作者 text.outline）
    if let Some(v) = vars.sf.get("outline") {
        let w = v.as_i64().clamp(0, gal_config::FX_MAX as i64) as u8;
        style.outline = w;
    }
    style
}

impl Game {
    pub fn new(conf: Config, mut interp: Interp) -> Self {
        let meta_path = format!("{}/meta.json", conf.save.dir);
        let meta = MetaState::load(&meta_path);
        let audio = Audio::new(&conf);
        // 主题包：skins/*.yaml 与作者 ui.skin.schemes 合并（同名覆盖）
        let skins = conf.ui.skin.with_packs(gal_config::skin::scan_packs(&conf.ui.skin.dir));
        let langs = gal_config::lang::scan(&conf.lang.dir);
        let lang = lang_for(&langs, &interp.vars);
        interp.lang = lang.strings.clone();
        let style = compose_style(&conf, &skins, None, &gal_script::vars::Vars::default());
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
            voice_hint: None,
        };
        Self {
            interp,
            sys,
            lang,
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
            style: style.clone(),
            skin: skins[0].clone(),
            style_dirty: false,
            autosave_acc: 0.0,
            font_families: Vec::new(),
            rebind: None,
            last_slot: 1,
            skins,
            tts: TtsState::new(&conf),
            scan_sys_fonts: false,
            lang_logged: 0,
            auto_steps: 0,
            auto_steps_done: false,
            langs,
            conf,
            now_ms: 0.0,
        }
    }

    /// 生效键位表：作者默认（game.yaml keymap）→ 玩家 sf.keymap.<action> 覆盖
    pub fn keymap(&self) -> std::collections::HashMap<String, Vec<String>> {
        let mut m = self.conf.keymap.effective();
        for (action, _) in gal_config::keymap::ACTIONS {
            let Some(v) = self.interp.vars.sf.get(&format!("keymap.{action}")) else {
                continue;
            };
            let keys: Vec<String> = v
                .as_str()
                .split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect();
            if !keys.is_empty() {
                m.insert((*action).to_string(), keys);
            }
        }
        m
    }

    /// 重算样式（玩家 sf.textSize/lineSpacing/waitMarker/skinIndex 生效）
    pub fn refresh_style(&mut self) {
        self.style = compose_style(&self.conf, &self.skins, None, &self.interp.vars);
        let idx =
            self.interp.vars.sf.get("skinIndex").map(|v| v.as_i64().max(0) as usize).unwrap_or(0);
        self.skin = self.skins.get(idx).cloned().unwrap_or_else(|| self.skins[0].clone());
        self.style_dirty = true;
    }

    /// 应用持久化的玩家设置（启动时一次：音频三通道/转场开关/字体族/样式）
    pub fn apply_saved_settings(&mut self, fonts: &mut gal_text::font::FontBook) {
        let int = |k: &str| self.interp.vars.sf.get(k).map(|v| v.as_i64());
        if let Some(v) = int("volBgm") {
            self.sys.audio.bgm_vol = v.clamp(0, 100) as i32;
        }
        if let Some(v) = int("volSe") {
            self.sys.audio.se_vol = v.clamp(0, 100) as i32;
        }
        if let Some(v) = int("volVoice") {
            self.sys.audio.voice_vol = v.clamp(0, 100) as i32;
        }
        self.sys.audio.apply_volumes();
        if let Some(v) = int("transOn") {
            self.interp.trans_enabled = v != 0;
        }
        if let Some(v) = int("textSpeed") {
            if v > 0 {
                self.interp.tw.interval_ms = v as f32;
            }
        }
        // 朗读：玩家开关/语速（先取值，避免与下面的可变借用打架）
        let tts_on = int("ttsOn");
        let tts_rate = int("ttsRate");
        if let Some(v) = tts_on {
            self.tts.enabled = v == 1 && self.tts.backend.is_some();
        }
        if let Some(v) = tts_rate {
            self.tts.rate = v.clamp(-10, 10) as i32;
        }
        self.apply_font_family(fonts);
        self.apply_lang();
        self.refresh_style();
        if gal_script::interp::dbg() {
            eprintln!(
                "[dbg settings] 启动套用：字号 {} 行距 {} 描边 {} 标记 {} 填充 {:?} 字色 {:?} 字体 {} 转场 {} 音量 {}/{}/{} 语言 {} 朗读 {}（后端 {:?}）语速 {}",
                self.style.font_size,
                self.style.line_spacing,
                self.style.outline,
                self.style.wait_marker,
                self.style.fill,
                self.style.text_color,
                fonts.family_index(),
                self.interp.trans_enabled,
                self.sys.audio.bgm_vol,
                self.sys.audio.se_vol,
                self.sys.audio.voice_vol,
                if self.lang.name.is_empty() { "原文" } else { &self.lang.name },
                self.tts.enabled,
                self.tts.backend,
                self.tts.rate,
            );
        }
    }

    /// 套用 sf.language 选中的语言包（0 = 作者原文）
    pub fn apply_lang(&mut self) {
        let want = lang_for(&self.langs, &self.interp.vars);
        let changed = want.code != self.lang.code || want.strings != self.lang.strings;
        self.lang = want;
        self.interp.lang = self.lang.strings.clone();
        if changed {
            self.style_dirty = true;
            eprintln!(
                "[lang] 切换语言：{}",
                if self.lang.name.is_empty() { "作者原文" } else { &self.lang.name }
            );
        }
    }

    /// 切换字体族（sf.fontFamily）；越界/不可用回落默认
    pub fn apply_font_family(&mut self, fonts: &mut gal_text::font::FontBook) {
        self.font_families = fonts.family_names().to_vec();
        let want =
            self.interp.vars.sf.get("fontFamily").map(|v| v.as_i64().max(0) as usize).unwrap_or(0);
        let want = want.min(self.font_families.len().saturating_sub(1));
        if fonts.family_index() != want {
            fonts.set_family(want);
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
        self.sys.audio.update_duck();
        // 语音被 SE 顶掉 → 左下角轻提示（不抢主 toast）
        if let Some(se) = self.sys.audio.take_interrupted() {
            eprintln!("[audio] 语音被 SE「{se}」打断");
            self.sys.voice_hint =
                Some(Msg { text: format!("♪ 语音被音效「{se}」打断"), left_ms: 1500.0 });
        }
        if let Some(m) = &mut self.sys.voice_hint {
            m.left_ms -= dt_ms;
            if m.left_ms <= 0.0 {
                self.sys.voice_hint = None;
            }
        }
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
        if let Overlay::Title { t, .. } = &mut self.overlay {
            *t += dt_ms;
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
