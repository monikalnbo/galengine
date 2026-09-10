//! 解释器指令分发：执行单条 Command，修改 Stage/Vars 或向 SysEvents 队列追加系统事件。

use super::types::{InputSpec, RunState, SysEvent};
use super::Interp;
use crate::command::Command;
use crate::vars::Value;
use gal_config::paths::resolve;

impl Interp {
    /// 执行一条指令；返回 Ok(true) 表示遇到等待点（台词/选项/定时/输入/结束）
    pub(crate) fn exec(&mut self, cmd: Command, ctx: &str) -> Result<bool, String> {
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
                // 系统级框也要支持 {hero}/{you} 替换
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
                    file: self.replace_names(&file),
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
