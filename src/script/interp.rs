//! 解释器：指令流调度状态机。
//! 执行模型：run_until_wait 连续执行指令直到遇到等待点
//! （台词 WaitClick / 强制等待 WaitTimer / End），由 app 每帧 tick/click 驱动。

use std::collections::HashMap;

use crate::config;
use crate::gfx::assets::resolve;
use crate::gfx::stage::Stage;
use crate::script::command::Command;
use crate::script::lexer::{self, Line, LineKind};
use crate::script::vars::Vars;
use crate::text::font::FontBook;
use crate::text::writer::{ClickResult, Typewriter};
use crate::ui::dialog::DialogStyle;

/// 验收调试日志开关（ES_DEBUG=1）
pub fn dbg() -> bool {
    std::env::var("ES_DEBUG").is_ok()
}

#[derive(Debug, PartialEq)]
pub enum RunState {
    /// 台词打字中/等点击
    WaitClick,
    /// 强制等待（不可点击跳过/不可快进）
    WaitTimer { left_ms: f32 },
    /// 选项等待
    WaitChoice { items: Vec<(String, String)> },
    /// 名字输入等待
    WaitInput(Box<InputSpec>),
    /// 剧本 end
    Ended,
}

/// input 指令规格（UI 构建用）
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
    /// 文件名（不含路径）-> 行序列
    scripts: HashMap<String, Vec<Line>>,
    /// (文件名, label) -> 行号
    labels: HashMap<(String, String), usize>,
    file: String,
    pc: usize,
    pub state: RunState,
    pub stage: Stage,
    pub vars: Vars,
    pub tw: Typewriter,
    /// 当前说话人（None=旁白）
    pub cur_name: Option<String>,
    /// 待断行台词（需 FontBook，由 app 每帧 apply_pending 处理）
    pending_text: Option<String>,
}

impl Interp {
    pub fn new(vars: Vars, typewriter_ms: f32) -> Self {
        Self {
            scripts: HashMap::new(),
            labels: HashMap::new(),
            file: String::new(),
            pc: 0,
            state: RunState::Ended,
            stage: Stage::default(),
            vars,
            tw: Typewriter::new(typewriter_ms),
            cur_name: None,
            pending_text: None,
        }
    }

    /// 预读扫描：往后 n 条指令将用到的图片资源（缓存层喂料）
    pub fn lookahead_storages(&self, n: usize) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        if let Some(lines) = self.scripts.get(&self.file) {
            for line in lines.iter().skip(self.pc).take(n) {
                if let Ok(cmd) = Command::parse(&line.kind, "") {
                    match cmd {
                        Command::Bg { storage, .. } => out.push(("bg", storage)),
                        Command::Char { storage, .. } if storage != "hide" => {
                            out.push(("char", storage))
                        }
                        Command::Cg { storage, .. } if storage != "hide" => out.push(("cg", storage)),
                        _ => {}
                    }
                }
            }
        }
        out
    }

    fn script_path(file: &str) -> String {
        format!("{}/scenario/{}", crate::config::data_dir(), file)
    }

    /// 加载剧本文件（解析+label 索引，带缓存）
    fn load(&mut self, file: &str) -> Result<(), String> {
        if self.scripts.contains_key(file) {
            return Ok(());
        }
        let path = Self::script_path(file);
        let src = std::fs::read_to_string(&path)
            .map_err(|e| format!("剧本文件读取失败：{path}（{e}）"))?;
        let lines = lexer::parse_script(&src).map_err(|e| format!("{file}：{e}"))?;
        for (i, line) in lines.iter().enumerate() {
            if let LineKind::Label(label) = &line.kind {
                self.labels
                    .insert((file.to_string(), label.clone()), i);
            }
        }
        self.scripts.insert(file.to_string(), lines);
        Ok(())
    }

    /// 从头开始执行指定剧本
    pub fn start(&mut self, file: &str) -> Result<(), String> {
        self.load(file)?;
        self.file = file.to_string();
        self.pc = 0;
        self.run_until_wait()
    }

    /// app 每帧：把待显台词交给 FontBook 断行（消除解释器对字体依赖）
    /// 同时做双名字替换：{hero}->f.heroName，{you}->sf.playerName（空则「你」）
    pub fn apply_pending(&mut self, fonts: &mut FontBook, style: &DialogStyle) {
        if let Some(text) = self.pending_text.take() {
            let hero = {
                let s = self.vars.get_or("f.heroName").as_str();
                if s.is_empty() { "拓海".to_string() } else { s }
            };
            let you = {
                let s = self.vars.get_or("sf.playerName").as_str();
                if s.is_empty() { "你".to_string() } else { s }
            };
            let text = text.replace("{hero}", &hero).replace("{you}", &you);
            if dbg() {
                eprintln!("[dbg] line @{}:{} -> {:?}", self.file, self.pc, &text.chars().take(12).collect::<String>());
            }
            if let Ok(font) = fonts.font(style.font_size) {
                let max_w = (config::LOGICAL_W - 208) as u32;
                self.tw.set_text(&text, font, max_w, style.lines_per_page);
            }
        }
    }

    /// 帧驱动：打字机推进 / 强制等待倒计时 / 层渐变
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

    /// 玩家点击推进（仅 WaitClick 状态有效）
    pub fn click(&mut self) -> Result<(), String> {
        if self.state == RunState::WaitClick && self.tw.click() == ClickResult::LineFinished {
            self.run_until_wait()?;
        }
        Ok(())
    }

    /// 选项选择（WaitChoice）：跳转对应 label 并继续
    pub fn choose(&mut self, idx: usize) -> Result<(), String> {
        if let RunState::WaitChoice { items } = &self.state {
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

    /// 输入完成（WaitInput）：写回变量并继续
    pub fn input_result(&mut self, value: String) -> Result<(), String> {
        if let RunState::WaitInput(spec) = &self.state {
            let var = spec.var.clone();
            self.vars.set(&var, crate::script::vars::Value::Str(value));
            self.run_until_wait()
        } else {
            Ok(())
        }
    }

    /// 跳本文件 label（choose/if 共用）；容忍 * 前缀
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
            // label 定义行只作为跳转目标存在，顺序执行时直接跳过
            if matches!(line.kind, LineKind::Label(_)) {
                continue;
            }
            let ctx = format!("{}:{}", self.file, line.no);
            let cmd = Command::parse(&line.kind, &ctx)?;
            let stop = self.exec(cmd, &ctx)?;
            if stop {
                return Ok(());
            }
        }
        Err(format!("连续执行超过 {MAX_STEPS} 条（{}）：疑似 label 死循环", self.file))
    }

    /// 执行一条指令；返回 true=遇到等待点停止
    fn exec(&mut self, cmd: Command, ctx: &str) -> Result<bool, String> {
        let unimpl = |name: &str| -> Result<bool, String> {
            Err(format!("{ctx}：指令「{name}」尚未实现（A5/A6 里程碑）"))
        };
        match cmd {
            Command::Bg { storage, fade_ms } => {
                self.stage.bg.set(Some(resolve("bg", &storage)), fade_ms);
                Ok(false)
            }
            Command::Bgm(_) | Command::Se(_) => Ok(false), // 接口预留：静默跳过（docs/20 §3.3）
            Command::Char { layer, storage } => {
                let next = if storage == "hide" {
                    None
                } else {
                    Some(resolve("char", &storage))
                };
                self.stage.chars[layer as usize].set(next, 500);
                Ok(false)
            }
            Command::Cg { storage, fade_ms } => {
                let next = if storage == "hide" {
                    None
                } else {
                    Some(resolve("cg", &storage))
                };
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
            Command::End => {
                self.state = RunState::Ended;
                Ok(true)
            }
            Command::Flag { var, op, val } => {
                let cur = self.vars.get_or(&var).as_i64();
                let next = if op == '+' { cur + val } else { cur - val };
                self.vars.set(&var, crate::script::vars::Value::Int(next));
                Ok(false)
            }
            Command::Set { var, expr } => {
                let v = crate::script::expr::eval(&expr, &self.vars)
                    .map_err(|e| format!("{ctx}：set 求值失败：{e}"))?;
                self.vars.set(&var, v);
                Ok(false)
            }
            Command::If { var, op, val, target } => {
                let hit = crate::script::expr::eval_cond(&var, &op, &val, &self.vars)
                    .map_err(|e| format!("{ctx}：if 条件错误：{e}"))?;
                if hit {
                    let t = target.trim_start_matches('*').to_string();
                    self.jump_label(&t)?;
                }
                Ok(false)
            }
            Command::Choice { items, .. } => {
                self.tw.clear();
                self.cur_name = None;
                self.state = RunState::WaitChoice { items };
                Ok(true)
            }
            Command::Input { var, prompt, width, default } => {
                self.tw.clear();
                self.cur_name = None;
                self.state = RunState::WaitInput(Box::new(InputSpec {
                    var,
                    prompt,
                    width,
                    default,
                }));
                Ok(true)
            }
            Command::MetaFakeSave { .. }
            | Command::MetaCorrupt(_)
            | Command::MetaDeleteLast
            | Command::TitleEvolve(_) => unimpl("meta_*"),
            Command::Reach { .. }
            | Command::WindowFxShake(_)
            | Command::WindowFxTitle(_) => unimpl("reach/window_fx"),
            Command::Shutdown(_) => unimpl("shutdown"),
        }
    }
}
