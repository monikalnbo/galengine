//! 解释器：run_until_wait 连续执行指令直到等待点（台词/强制等待/选项/输入/end），
//! 由 app 每帧 tick/click 驱动。演出类指令（音频/meta/越界/窗口/关机/桌面）
//! 不在此执行，进 SysEvent 队列由 L1 systems 层消费——L2 不碰平台。

use std::collections::HashMap;

use crate::command::Command;
use crate::lexer::{self, Line, LineKind};
use crate::stage::{Stage, StageSnap};
use crate::vars::{Value, Vars};
use gal_config as config;
use gal_config::paths::resolve;
use gal_config::sprites::{SpritePos, SpritesCfg};
use gal_config::style::DialogStyle;
use gal_text::font::FontBook;
use gal_text::writer::{ClickResult, Typewriter};

/// 验收调试日志开关（ES_DEBUG）
pub fn dbg() -> bool {
    std::env::var("ES_DEBUG").is_ok()
}

/// L2 → L1 系统事件（systems.rs 消费）
#[derive(Debug, Clone)]
pub enum SysEvent {
    Bgm(String),
    BgmStop,
    BgmFadeOut { ms: u32 },
    Se(String),
    MetaFake { date: String, time: String, image: String },
    MetaCorrupt(String),
    MetaDeleteLast,
    TitleEvolve(String),
    Reach { path: String, dur_ms: u32, scale: f32 },
    ReachHide,
    Shake(u32),
    SetTitle(String),
    RestoreTitle,
    Shutdown(u32),
    DesktopWrite { file: String, content: String },
    DesktopOpen(String),
}

#[derive(Clone, Debug)]
pub struct BacklogItem {
    pub name: Option<String>,
    pub text: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RunState {
    WaitClick,
    WaitTimer { left_ms: f32 },
    WaitChoice { prompt: String, items: Vec<(String, String)> },
    WaitInput(Box<InputSpec>),
    Ended,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputSpec {
    pub var: String,
    pub prompt: String,
    pub width: u32,
    pub default: String,
}

/// 连续执行保护上限（防 label 死循环）
const MAX_STEPS: usize = 10_000;

pub struct Interp {
    scripts: HashMap<String, Vec<Line>>,
    labels: HashMap<(String, String), usize>,
    file: String,
    pc: usize,
    /// 当前等待点所在行（存档恢复用：重执行该行即恢复画面+台词）
    stop_line: Option<usize>,
    pub state: RunState,
    pub stage: Stage,
    pub vars: Vars,
    pub tw: Typewriter,
    pub cur_name: Option<String>,
    /// 当前场景 BGM 名（随存档；读档恢复播放）
    pub cur_bgm: Option<String>,
    /// 存档标题用：最近一句台词截断
    pub last_text: String,
    pub events: Vec<SysEvent>,
    hero_default: String,
    you_default: String,
    pending_text: Option<String>,
    /// 立绘预设位置（game.yaml sprites.positions 注入；默认 left/center/right）
    pub positions: HashMap<String, SpritePos>,
    /// Q版立绘名单（sprites.chibi 注入；在场时背景虚化）
    pub chibi: std::collections::HashSet<String>,
    /// 对白历史记录（回卷查阅用）
    pub backlog: Vec<BacklogItem>,
}

impl Interp {
    pub fn new(vars: Vars, typewriter_ms: f32, hero_default: &str, you_default: &str) -> Self {
        Self {
            scripts: HashMap::new(),
            labels: HashMap::new(),
            file: String::new(),
            pc: 0,
            stop_line: None,
            state: RunState::Ended,
            stage: Stage::default(),
            vars,
            tw: Typewriter::new(typewriter_ms),
            cur_name: None,
            cur_bgm: None,
            last_text: String::new(),
            events: Vec::new(),
            hero_default: hero_default.into(),
            you_default: you_default.into(),
            pending_text: None,
            positions: SpritesCfg::default().positions,
            chibi: std::collections::HashSet::new(),
            backlog: Vec::new(),
        }
    }

    /// 预读扫描：之后 n 条指令将用到的图片（prefetch 喂料）
    pub fn lookahead_storages(&self, n: usize) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        if let Some(lines) = self.scripts.get(&self.file) {
            for line in lines.iter().skip(self.pc).take(n) {
                if let LineKind::Command { name, rest } = &line.kind {
                    let toks: Vec<&str> = rest.split_whitespace().collect();
                    match (name.as_str(), toks.as_slice()) {
                        ("bg", [s, ..]) => out.push(("bg", s.to_string())),
                        ("cg", [s, ..]) if *s != "hide" => out.push(("cg", s.to_string())),
                        ("char", [_, s]) if *s != "hide" => out.push(("char", s.to_string())),
                        ("reach", [s, ..]) if *s != "hide" => out.push(("cg", s.to_string())),
                        _ => {}
                    }
                }
            }
        }
        out
    }

    fn load(&mut self, file: &str) -> Result<(), String> {
        if self.scripts.contains_key(file) {
            return Ok(());
        }
        let path = format!("{}/scenario/{}", config::data_dir(), file);
        let src = std::fs::read_to_string(&path)
            .map_err(|e| format!("剧本文件读取失败：{path}（{e}）"))?;
        let src = src.trim_start_matches('\u{feff}'); // 剥 BOM
        let lines = lexer::parse_script(src).map_err(|e| format!("{file}：{e}"))?;
        for (i, line) in lines.iter().enumerate() {
            if let LineKind::Label(label) = &line.kind {
                self.labels.insert((file.to_string(), label.clone()), i);
            }
        }
        self.scripts.insert(file.to_string(), lines);
        Ok(())
    }

    /// 从头执行剧本（label 可选：ES_START_LABEL / 存档恢复）
    pub fn start(&mut self, file: &str, label: Option<&str>) -> Result<(), String> {
        self.load(file)?;
        self.file = file.to_string();
        self.pc = 0;
        if let Some(label) = label {
            let key = (file.to_string(), label.trim_start_matches('*').to_string());
            self.pc =
                *self.labels.get(&key).ok_or(format!("起始 label 不存在：{file} *{label}"))?;
        }
        self.run_until_wait()
    }

    /// 存档恢复：位置 + f.* + 演出层快照，重执行等待点行
    pub fn restore(
        &mut self,
        file: &str,
        line: usize,
        snap: StageSnap,
        fvars: HashMap<String, Value>,
    ) -> Result<(), String> {
        self.load(file)?;
        self.vars.f = fvars;
        self.stage.restore(snap);
        // ponytail: 读档后Q版标记丢失（快照不含），下次 char 指令即恢复正确
        for c in &mut self.stage.chars {
            c.chibi = false;
        }
        self.tw.clear();
        self.cur_name = None;
        self.pending_text = None;
        self.file = file.to_string();
        self.pc = line;
        self.run_until_wait()
    }

    /// 存档恢复点（当前等待行）
    pub fn resume_point(&self) -> Option<(String, usize)> {
        self.stop_line.map(|l| (self.file.clone(), l))
    }

    /// app 每帧：待显台词交 FontBook 断行；做 {hero}/{you} 替换
    pub fn apply_pending(&mut self, fonts: &mut FontBook, style: &DialogStyle) {
        if let Some(text) = self.pending_text.take() {
            let text = self.replace_names(&text);
            self.last_text = text.chars().take(18).collect();
            // 推入回卷历史（最多保留 100 条）
            if self.backlog.len() >= 100 {
                self.backlog.remove(0);
            }
            self.backlog.push(BacklogItem {
                name: self.cur_name.clone(),
                text: text.clone(),
            });
            self.mark_cur_line_read();
            if dbg() {
                eprintln!(
                    "[dbg] @{}:{} {}",
                    self.file,
                    self.stop_line.map(|l| l + 1).unwrap_or(0),
                    text.chars().take(12).collect::<String>()
                );
            }
            if let Ok(font) = fonts.font(style.font_size) {
                let max_w = (style.box_rect[2] - 96) as u32;
                self.tw.set_text(&text, font, max_w, style.lines_per_page);
            }
        }
    }

    /// 标记当前等待行已读（sf.readLines 持久化）
    pub fn mark_cur_line_read(&mut self) {
        if let Some(line) = self.stop_line {
            let key = format!("{}:{}", self.file, line);
            let mut read_set = self.get_read_lines();
            if read_set.insert(key) {
                let joined = read_set.into_iter().collect::<Vec<_>>().join(",");
                self.vars.sf.insert("readLines".into(), Value::Str(joined));
            }
        }
    }

    /// 当前等待行是否已被阅读过
    pub fn is_cur_line_read(&self) -> bool {
        if let Some(line) = self.stop_line {
            let key = format!("{}:{}", self.file, line);
            self.get_read_lines().contains(&key)
        } else {
            false
        }
    }

    fn get_read_lines(&self) -> std::collections::HashSet<String> {
        self.vars
            .sf
            .get("readLines")
            .map(|v| v.as_str().split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }

    /// {hero}->f.heroName、{you}->sf.playerName（空回退配置默认）
    pub fn replace_names(&self, text: &str) -> String {
        let hero = match self.vars.get("f.heroName") {
            Some(Value::Str(s)) if !s.is_empty() => s.clone(),
            _ => self.hero_default.clone(),
        };
        let you = match self.vars.get("sf.playerName") {
            Some(Value::Str(s)) if !s.is_empty() => s.clone(),
            _ => self.you_default.clone(),
        };
        text.replace("{hero}", &hero).replace("{you}", &you)
    }

    /// 帧驱动：打字机 / 强制等待 / 层渐变
    pub fn tick(&mut self, dt_ms: f32) {
        match &mut self.state {
            RunState::WaitClick => self.tw.tick(dt_ms),
            RunState::WaitTimer { left_ms } => {
                *left_ms -= dt_ms;
                if *left_ms <= 0.0 {
                    self.run_until_wait_ok();
                }
            }
            RunState::WaitChoice { .. } | RunState::WaitInput(_) | RunState::Ended => {}
        }
        self.stage.tick(dt_ms);
    }

    /// 玩家点击推进（仅台词态）
    pub fn click(&mut self) -> Result<(), String> {
        if self.state == RunState::WaitClick && self.tw.click() == ClickResult::LineFinished {
            self.run_until_wait()?;
        }
        Ok(())
    }

    /// 选项选择
    pub fn choose(&mut self, idx: usize) -> Result<(), String> {
        if let RunState::WaitChoice { items, .. } = &self.state {
            let target = items
                .get(idx)
                .map(|(_, t)| t.clone())
                .ok_or_else(|| format!("选项下标无效：{idx}"))?;
            self.jump_label(&target)?;
            self.run_until_wait()
        } else {
            Ok(())
        }
    }

    /// 输入完成：写回变量并继续
    pub fn input_result(&mut self, value: String) -> Result<(), String> {
        if let RunState::WaitInput(spec) = &self.state {
            let var = spec.var.clone();
            self.vars.set(&var, Value::Str(value));
            self.run_until_wait()
        } else {
            Ok(())
        }
    }

    /// cg 指令解锁记录（sf.cgs 累加表，鉴赏用）
    fn unlock_cg(&mut self, storage: &str) {
        let list = self.vars.sf.entry("cgs".into()).or_insert_with(|| Value::Str(String::new()));
        if let Value::Str(s) = list {
            if !s.split(',').any(|x| x == storage) {
                if !s.is_empty() {
                    s.push(',');
                }
                s.push_str(storage);
            }
        }
    }

    fn jump_label(&mut self, label: &str) -> Result<(), String> {
        let label = label.trim_start_matches('*');
        let key = (self.file.clone(), label.to_string());
        let pc = self
            .labels
            .get(&key)
            .ok_or_else(|| format!("跳转目标不存在：{} *{}", self.file, label))?;
        self.pc = *pc;
        Ok(())
    }

    fn run_until_wait_ok(&mut self) {
        let _ = self.run_until_wait();
    }

    fn run_until_wait(&mut self) -> Result<(), String> {
        for _ in 0..MAX_STEPS {
            let lines = match self.scripts.get(&self.file) {
                Some(l) => l,
                None => return Err(format!("剧本未加载：{}", self.file)),
            };
            if self.pc >= lines.len() {
                self.state = RunState::Ended; // 文件尾隐式 end
                return Ok(());
            }
            let line = &lines[self.pc];
            self.pc += 1;
            if matches!(line.kind, LineKind::Label(_)) {
                continue;
            }
            let ctx = format!("{}:{}", self.file, line.no);
            let cmd = Command::parse(&line.kind, &ctx)?;
            if self.exec(cmd, &ctx)? {
                self.stop_line = Some(self.pc - 1); // 等待点=当前行
                return Ok(());
            }
        }
        Err(format!("连续执行超过 {MAX_STEPS} 条（{}）：疑似 label 死循环", self.file))
    }

    /// 执行一条指令；true=遇到等待点
    fn exec(&mut self, cmd: Command, ctx: &str) -> Result<bool, String> {
        match cmd {
            Command::Bg { storage, fade_ms } => {
                self.stage.bg.set(Some(resolve("bg", &storage)), fade_ms);
                Ok(false)
            }
            Command::Bgm(n) => {
                self.cur_bgm = Some(n.clone());
                self.events.push(SysEvent::Bgm(n));
                Ok(false)
            }
            Command::BgmStop => {
                self.cur_bgm = None;
                self.events.push(SysEvent::BgmStop);
                Ok(false)
            }
            Command::BgmFadeOut { ms } => {
                self.cur_bgm = None;
                self.events.push(SysEvent::BgmFadeOut { ms });
                Ok(false)
            }
            Command::Se(n) => {
                self.events.push(SysEvent::Se(n));
                Ok(false)
            }
            Command::Char { layer, storage, x, y } => {
                let next = if storage == "hide" { None } else { Some(resolve("char", &storage)) };
                let l = &mut self.stage.chars[layer as usize];
                l.set(next, 500);
                l.chibi = storage != "hide" && self.chibi.contains(storage.as_str());
                if let (Some(ox), Some(oy)) = (x, y) {
                    l.offset = (ox, oy);
                }
                Ok(false)
            }
            Command::CharPos { layer, storage, pos_name } => {
                let next = if storage == "hide" { None } else { Some(resolve("char", &storage)) };
                let l = &mut self.stage.chars[layer as usize];
                l.set(next, 500);
                l.chibi = storage != "hide" && self.chibi.contains(storage.as_str());
                // pos_name 形如 "far_left 0"：取首词查 positions（config 注入）
                let key = pos_name.split_whitespace().next().unwrap_or("");
                let Some(p) = self.positions.get(key) else {
                    return Err(format!("{ctx}：未知立绘预设位置「{key}」，请在 game.yaml 的 sprites.positions 配置"));
                };
                l.offset = (p.x, p.y);
                Ok(false)
            }
            Command::Cg { storage, fade_ms } => {
                let next = if storage == "hide" { None } else { Some(resolve("cg", &storage)) };
                if next.is_some() {
                    self.unlock_cg(&storage);
                }
                self.stage.cg.set(next, fade_ms);
                Ok(false)
            }
            Command::Clear => {
                self.tw.clear();
                self.cur_name = None;
                Ok(false)
            }
            Command::Wait(ms) => {
                self.state = RunState::WaitTimer { left_ms: ms as f32 };
                Ok(true)
            }
            Command::N(text) => {
                self.cur_name = None;
                self.pending_text = Some(text);
                self.state = RunState::WaitClick;
                Ok(true)
            }
            Command::Name { name, text } => {
                self.cur_name = Some(name);
                self.pending_text = Some(text);
                self.state = RunState::WaitClick;
                Ok(true)
            }
            Command::Flag { var, op, val } => {
                let cur = self.vars.get_or(&var).as_i64();
                let next = if op == '+' { cur + val } else { cur - val };
                self.vars.set(&var, Value::Int(next));
                Ok(false)
            }
            Command::Set { var, expr } => {
                let v = crate::expr::eval(&expr, &self.vars)
                    .map_err(|e| format!("{ctx}：set 求值失败：{e}"))?;
                self.vars.set(&var, v);
                Ok(false)
            }
            Command::If { var, op, val, target } => {
                let hit = crate::expr::eval_cond(&var, &op, &val, &self.vars)
                    .map_err(|e| format!("{ctx}：if 条件错误：{e}"))?;
                if hit {
                    self.jump_label(&target)?;
                }
                Ok(false)
            }
            Command::Jump { file, label } => {
                let target_file = file.unwrap_or_else(|| self.file.clone());
                self.load(&target_file)?;
                let key = (target_file.clone(), label.clone());
                let pc = self
                    .labels
                    .get(&key)
                    .ok_or_else(|| format!("{ctx}：跳转目标不存在：{} *{}", target_file, label))?;
                self.file = target_file;
                self.pc = *pc;
                Ok(false)
            }
            Command::Choice { prompt, items } => {
                self.tw.clear();
                self.cur_name = None;
                self.state = RunState::WaitChoice { prompt, items };
                Ok(true)
            }
            Command::Input { var, prompt, width, default } => {
                self.tw.clear();
                self.cur_name = None;
                self.state =
                    RunState::WaitInput(Box::new(InputSpec { var, prompt, width, default }));
                Ok(true)
            }
            Command::MetaFakeSave { date, time, image } => {
                self.events.push(SysEvent::MetaFake { date, time, image });
                Ok(false)
            }
            Command::MetaCorrupt(n) => {
                self.events.push(SysEvent::MetaCorrupt(n));
                Ok(false)
            }
            Command::MetaDeleteLast => {
                self.events.push(SysEvent::MetaDeleteLast);
                Ok(false)
            }
            Command::TitleEvolve(m) => {
                self.events.push(SysEvent::TitleEvolve(m));
                Ok(false)
            }
            Command::Reach { storage, dur_ms, scale } => {
                if storage == "hide" {
                    self.events.push(SysEvent::ReachHide);
                } else {
                    self.events.push(SysEvent::Reach {
                        path: resolve("cg", &storage),
                        dur_ms,
                        scale,
                    });
                }
                Ok(false)
            }
            Command::WindowFxShake(ms) => {
                self.events.push(SysEvent::Shake(ms));
                Ok(false)
            }
            Command::WindowFxTitle(t) => {
                // 系统级框也要支持 {hero}/{you} 替换（H8 三重确认直呼玩家真名）
                self.events.push(SysEvent::SetTitle(self.replace_names(&t)));
                Ok(false)
            }
            Command::WindowFxTitleRestore => {
                self.events.push(SysEvent::RestoreTitle);
                Ok(false)
            }
            Command::Shutdown(sec) => {
                self.events.push(SysEvent::Shutdown(sec));
                Ok(false)
            }
            Command::DesktopWrite { file, content } => {
                self.events.push(SysEvent::DesktopWrite {
                    file: self.replace_names(&file), // 文件名同样支持 {hero}/{you}
                    // 内容支持 \n 表示换行（日记/信件多行文本）
                    content: self.replace_names(&content).replace("\\n", "\n"),
                });
                Ok(false)
            }
            Command::DesktopOpen(file) => {
                self.events.push(SysEvent::DesktopOpen(self.replace_names(&file)));
                Ok(false)
            }
            Command::End => {
                self.state = RunState::Ended;
                Ok(true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interp() -> Interp {
        Interp::new(Vars::default(), 30.0, "阿岚", "你")
    }

    #[test]
    fn q版名单标记层() {
        let mut it = interp();
        it.chibi.insert("chibi_normal".into());
        it.exec(
            Command::Char { layer: 1, storage: "chibi_normal".into(), x: None, y: None },
            "t.ks:1",
        )
        .unwrap();
        assert!(it.stage.chars[1].chibi);
        it.exec(Command::Char { layer: 1, storage: "hide".into(), x: None, y: None }, "t.ks:2")
            .unwrap();
        assert!(!it.stage.chars[1].chibi);
    }

    #[test]
    fn 预设位置从注入的配置查表() {
        let mut it = interp();
        it.positions.insert("far_left".into(), SpritePos { x: -140, y: 20 });
        it.exec(
            Command::CharPos { layer: 1, storage: "hide".into(), pos_name: "far_left 0".into() },
            "t.ks:1",
        )
        .unwrap();
        assert_eq!(it.stage.chars[1].offset, (-140, 20));
    }

    #[test]
    fn 默认含_left_center_right() {
        let it = interp();
        assert!(it.positions.contains_key("left"));
        assert!(it.positions.contains_key("center"));
        assert!(it.positions.contains_key("right"));
    }

    #[test]
    fn 未知预设位置中文报错带行号() {
        let mut it = interp();
        let e = it
            .exec(
                Command::CharPos { layer: 1, storage: "hide".into(), pos_name: "nope 0".into() },
                "00.ks:7",
            )
            .unwrap_err();
        assert!(e.contains("00.ks:7") && e.contains("nope") && e.contains("预设位置"), "{e}");
    }
}
