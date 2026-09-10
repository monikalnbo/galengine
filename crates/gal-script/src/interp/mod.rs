//! 解释器：run_until_wait 连续执行指令直到等待点（台词/强制等待/选项/输入/end），
//! 由 app 每帧 tick/click 驱动。演出类指令（音频/meta/越界/窗口/关机/桌面）
//! 不在此执行，进 SysEvent 队列由 L1 systems 层消费——L2 不碰平台。

mod exec;
mod history;
mod types;

#[cfg(test)]
mod tests;

pub use types::{dbg, BacklogItem, InputSpec, RunState, SysEvent};

use std::collections::HashMap;

use crate::command::Command;
use crate::lexer::{self, Line, LineKind};
use crate::stage::{Stage, StageSnap};
use crate::vars::{Value, Vars};
use gal_config as config;
use gal_config::sprites::{SpritePos, SpritesCfg};
use gal_text::writer::{ClickResult, Typewriter};

/// 连续执行保护上限（防 label 死循环）
const MAX_STEPS: usize = 10_000;

pub struct Interp {
    pub(crate) scripts: HashMap<String, Vec<Line>>,
    pub(crate) labels: HashMap<(String, String), usize>,
    pub(crate) file: String,
    pub(crate) pc: usize,
    /// 当前等待点所在行（存档恢复用：重执行该行即恢复画面+台词）
    pub(crate) stop_line: Option<usize>,
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
    pub(crate) hero_default: String,
    pub(crate) you_default: String,
    pub(crate) pending_text: Option<String>,
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
                        ("char", [_, s, ..]) if *s != "hide" => out.push(("char", s.to_string())),
                        ("reach", [s, ..]) if *s != "hide" => out.push(("cg", s.to_string())),
                        _ => {}
                    }
                }
            }
        }
        out
    }

    pub(crate) fn load(&mut self, file: &str) -> Result<(), String> {
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

    pub(crate) fn jump_label(&mut self, label: &str) -> Result<(), String> {
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
                self.state = RunState::Ended;
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
                self.stop_line = Some(self.pc - 1);
                return Ok(());
            }
        }
        Err(format!("连续执行超过 {MAX_STEPS} 条（{}）：疑似 label 死循环", self.file))
    }
}
