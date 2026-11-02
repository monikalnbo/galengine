//! 解释器指令分发：执行单条 Command，修改 Stage/Vars 或向 SysEvents 队列追加系统事件。

use super::types::{AudioWait, InputSpec, RunState, SysEvent};
use super::Interp;
use crate::command::Command;
use crate::vars::Value;
use gal_config::paths::resolve;

/// 自由层素材路径：先按 bg/cg 图库解析，找不到再用数据目录直取。
fn resolve_layer(_n: u8, storage: &str) -> String {
    let p = resolve("cg", storage);
    if std::path::Path::new(&p).exists() {
        return p;
    }
    let direct = format!("{}/{}", gal_config::data_dir(), storage);
    if std::path::Path::new(&direct).exists() {
        return direct;
    }
    p
}

impl Interp {
    /// 执行一条指令；返回 Ok(true) 表示遇到等待点（台词/选项/定时/输入/结束）
    pub(crate) fn exec(&mut self, cmd: Command, ctx: &str) -> Result<bool, String> {
        match cmd {
            Command::Bg { storage, fade_ms } => {
                let next = if storage == "hide" { None } else { Some(resolve("bg", &storage)) };
                self.stage.set_layer(crate::stage::BG_LAYER, next, fade_ms)?;
                Ok(false)
            }
            Command::Bgm(n) => {
                self.cur_bgm = Some(n.clone());
                self.events.push(SysEvent::Bgm(n));
                Ok(false)
            }
            // ===== v0.3.2 KAG3 音频标签组 =====
            Command::PlayBgm { name, fade_in_ms } => {
                self.events.push(SysEvent::PlayBgm { name, fade_in_ms });
                Ok(false)
            }
            Command::PauseBgm => {
                self.events.push(SysEvent::PauseBgm);
                Ok(false)
            }
            Command::ResumeBgm => {
                self.events.push(SysEvent::ResumeBgm);
                Ok(false)
            }
            Command::FadeInBgm { name, ms } => {
                self.events.push(SysEvent::FadeInBgm { name, ms });
                Ok(false)
            }
            Command::FadeOutBgm { ms } => {
                self.events.push(SysEvent::FadeOutBgm { ms });
                Ok(false)
            }
            Command::FadePauseBgm { ms } => {
                self.events.push(SysEvent::FadePauseBgm { ms });
                Ok(false)
            }
            Command::FadeBgm { to_pct, ms } => {
                self.events.push(SysEvent::FadeBgm { to_pct, ms });
                Ok(false)
            }
            Command::XchgBgm { name, ms } => {
                self.events.push(SysEvent::XchgBgm { name, ms });
                Ok(false)
            }
            Command::BgmOpt { loop_start_ms, loop_end_ms } => {
                self.events.push(SysEvent::BgmOpt { loop_start_ms, loop_end_ms });
                Ok(false)
            }
            Command::PlaySe { name } => {
                self.events.push(SysEvent::PlaySe { name });
                Ok(false)
            }
            Command::StopSe => {
                self.events.push(SysEvent::StopSe);
                Ok(false)
            }
            Command::FadeInSe { name, ms } => {
                self.events.push(SysEvent::FadeInSe { name, ms });
                Ok(false)
            }
            Command::FadeOutSe { name, ms } => {
                self.events.push(SysEvent::FadeOutSe { name, ms });
                Ok(false)
            }
            Command::FadeSe { to_pct, ms } => {
                self.events.push(SysEvent::FadeSe { to_pct, ms });
                Ok(false)
            }
            Command::SeOpt { name } => {
                self.events.push(SysEvent::SeOpt { name });
                Ok(false)
            }
            // 等待标签：引擎轮询音频状态，由 gal-engine 侧把 WaitAudio 换成放行
            Command::WaitBgm => {
                self.state = RunState::WaitAudio(AudioWait::BgmFade);
                Ok(true)
            }
            Command::WaitLoop => {
                self.state = RunState::WaitAudio(AudioWait::BgmLoopHit);
                Ok(true)
            }
            Command::WaitSe => {
                self.state = RunState::WaitAudio(AudioWait::SeDone);
                Ok(true)
            }
            Command::WaitSeFade => {
                self.state = RunState::WaitAudio(AudioWait::SeFade);
                Ok(true)
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
            Command::Voice(n) => {
                self.events.push(SysEvent::Voice(n));
                Ok(false)
            }
            Command::VoiceStop => {
                self.events.push(SysEvent::VoiceStop);
                Ok(false)
            }
            Command::Char { layer, storage, x, y } => {
                let next = if storage == "hide" { None } else { Some(resolve("char", &storage)) };
                let l = self.stage.char_mut(layer as usize);
                l.set(next, self.char_fade_ms);
                l.chibi = storage != "hide" && self.chibi.contains(storage.as_str());
                if let (Some(ox), Some(oy)) = (x, y) {
                    l.offset = (ox, oy);
                }
                Ok(false)
            }
            Command::CharPos { layer, storage, pos_name } => {
                let next = if storage == "hide" { None } else { Some(resolve("char", &storage)) };
                let l = self.stage.char_mut(layer as usize);
                l.set(next, self.char_fade_ms);
                l.chibi = storage != "hide" && self.chibi.contains(storage.as_str());
                // pos_name 形如 "far_left 0"：取首词查 positions（config 注入）
                let key = pos_name.split_whitespace().next().unwrap_or("");
                let Some(p) = self.positions.get(key) else {
                    return Err(format!("{ctx}：未知立绘预设位置「{key}」，请在 game.yaml 的 sprites.positions 配置"));
                };
                l.offset = (p.x, p.y);
                Ok(false)
            }
            Command::CharSway { layer, count, period_ms } => {
                // 纯表现，不阻塞剧本；与窗口抖动同属系统层演出
                self.events.push(SysEvent::CharSway { layer, count, period_ms });
                Ok(false)
            }
            Command::Cg { storage, fade_ms } => {
                let next = if storage == "hide" { None } else { Some(resolve("cg", &storage)) };
                if next.is_some() {
                    self.unlock_cg(&storage);
                }
                self.stage.set_layer(crate::stage::CG_LAYER, next, fade_ms)?;
                Ok(false)
            }
            Command::Layer { n, storage, fade_ms } => {
                if storage == "hide" {
                    self.stage.ensure_layer(n as usize)?;
                    self.stage.layers[n as usize].set(None, fade_ms);
                    return Ok(false);
                }
                // 自由层素材：先探 bg/cg 图库，再回落到数据目录直取（兼容任意贴图）
                let path = resolve_layer(n, &storage);
                self.stage.set_layer(n as usize, Some(path), fade_ms)?;
                Ok(false)
            }
            Command::Trans { n, storage, dur_ms, method } => {
                // 玩家关掉转场（sf.transOn=0）：直接换图，不建转场片段
                let dur_ms = if self.trans_enabled { dur_ms } else { 0 };
                let method = if self.trans_enabled { method } else { "fade".to_string() };
                let to = match storage.as_str() {
                    // hide 等价于淡出到空：旧内容作 from、目标为空，不换图
                    "hide" => String::new(),
                    _ => {
                        if n as usize == crate::stage::CG_LAYER {
                            self.unlock_cg(&storage);
                        }
                        resolve_layer(n, &storage)
                    }
                };
                if to.is_empty() {
                    // 淡出：把旧内容定格成转场片段、目标层立即清空
                    let old = self.stage.layer(n as usize).and_then(|l| l.cur.clone());
                    self.stage.ensure_layer(n as usize)?;
                    self.stage.layers[n as usize].set(None, 0);
                    self.stage.trans = Some(crate::stage::TransClip {
                        method: crate::stage::TransMethod::parse(&method),
                        layer: n as usize,
                        to: String::new(),
                        from: old,
                        offset: self.stage.layers[n as usize].offset,
                        dur_ms: dur_ms as f32,
                        left_ms: dur_ms as f32,
                    });
                } else {
                    let m = crate::stage::TransMethod::parse(&method);
                    self.stage.start_trans(n as usize, to, dur_ms, m);
                }
                Ok(false)
            }
            Command::WaitTrans => match self.stage.trans_left_ms() {
                Some(left) => {
                    self.state = RunState::WaitTimer { left_ms: left };
                    Ok(true)
                }
                None => Ok(false),
            },
            Command::Clear => {
                self.tw.clear();
                self.cur_text.clear();
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
            Command::If { cond, target } => {
                let hit = crate::expr::eval(&cond, &self.vars)
                    .map(|v| crate::expr::truthy(&v))
                    .map_err(|e| format!("{ctx}：if 条件错误：{e}"))?;
                if hit {
                    self.jump_label(&target)?;
                }
                Ok(false)
            }
            Command::IfBlock { cond } => {
                let hit = crate::expr::eval(&cond, &self.vars)
                    .map(|v| crate::expr::truthy(&v))
                    .map_err(|e| format!("{ctx}：if 条件错误：{e}"))?;
                if !hit {
                    // 假分支：进入 else 分支首行；没有 else 则跳到 endif 之后
                    let (else_at, end) = self
                        .blocks_of(&self.file)
                        .and_then(|b| b.get(&(self.pc - 1)).copied())
                        .ok_or_else(|| format!("{ctx}：if 块配对表缺失（疑似剧本热更新）"))?;
                    self.pc = match else_at {
                        Some(e) => e + 1,
                        None => end + 1,
                    };
                }
                Ok(false)
            }
            Command::Else => {
                // 能走到 else 说明真分支已执行完：跳过剩余分支
                let end = self
                    .else_target_of(&self.file, self.pc - 1)
                    .ok_or_else(|| format!("{ctx}：else 配对表缺失"))?;
                self.pc = end + 1;
                Ok(false)
            }
            Command::EndIf => Ok(false),
            Command::Call { file, label } => {
                if self.call_stack.len() >= super::MAX_CALL_DEPTH {
                    return Err(format!(
                        "{ctx}：call 嵌套超过 {} 层（疑似递归子程序）",
                        super::MAX_CALL_DEPTH
                    ));
                }
                let target_file = file.unwrap_or_else(|| self.file.clone());
                self.load(&target_file)?;
                let key = (target_file.clone(), label.clone());
                let pc = *self
                    .labels
                    .get(&key)
                    .ok_or_else(|| format!("{ctx}：调用目标不存在：{} *{}", target_file, label))?;
                // 压栈：返回时回到 call 的下一行
                let ret_file = self.file.clone();
                let ret_pc = self.pc;
                self.call_stack.push((ret_file, ret_pc));
                self.file = target_file;
                self.pc = pc;
                Ok(false)
            }
            Command::Return => {
                let (file, pc) = self
                    .call_stack
                    .pop()
                    .ok_or_else(|| format!("{ctx}：return 没有对应的 call"))?;
                self.file = file;
                self.pc = pc;
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
                let prompt = self.replace_names(&prompt);
                let items = items.into_iter().map(|(t, l)| (self.replace_names(&t), l)).collect();
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
