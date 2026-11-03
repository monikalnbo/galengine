//! 解释器指令分发：执行单条 Command，修改 Stage/Vars 或向 SysEvents 队列追加系统事件。

use super::types::{AudioWait, InputSpec, RunState, SysEvent};
use super::Interp;
use super::{ButtonSpec, LinkAction, LinkRegion};
use crate::command::Command;
use crate::vars::{Value, Vars};
use gal_config::paths::resolve;

/// v0.3.4 值 → 台词/日志文本（emb 输出口径：Int/Float 数字、Str 原样、
/// 容器紧凑序列化 [a, b] / %[ k: v ]）
fn value_to_text(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => {
            if f.fract() == 0.0 {
                format!("{f:.1}")
            } else {
                format!("{f}")
            }
        }
        Value::Arr(a) => {
            let items: Vec<String> = a.iter().map(value_to_text).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Dict(d) => {
            let items: Vec<String> =
                d.iter().map(|(k, v)| format!("{}: {}", k, value_to_text(v))).collect();
            format!("%[{}]", items.join(", "))
        }
    }
}

/// v0.3.4 trace 的 `%表达式` 字段：从 tail 起求值到第一个分隔符（空白/逗号/
/// 中文标点/行尾）；求值失败返回 None（调用方按原样保留 `%`）。
/// 返回 (渲染文本, 消耗的字符数)
fn eval_trace_field(tail: &str, vars: &Vars) -> Option<(String, usize)> {
    // 找表达式终点：括号配平后遇到分隔符
    let (mut depth, mut end) = (0i32, tail.len());
    for (i, c) in tail.char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => {
                if depth == 0 {
                    end = i;
                    break;
                }
                depth -= 1;
            }
            ' ' | ',' | '，' | '。' | '、' | '\t' if depth == 0 => {
                end = i;
                break;
            }
            _ => {}
        }
    }
    if end == 0 {
        return None;
    }
    let expr = &tail[..end];
    let v = crate::expr::eval(expr, vars).ok()?;
    Some((value_to_text(&v), end))
}

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
            // ===== v0.3.6 fore/back 双页 + move 动画 =====
            Command::BackLay { n, hide } => {
                let n = n.map(|x| x as usize).unwrap_or(crate::stage::BG_LAYER);
                self.stage.backlay(n, hide)?;
                Ok(false)
            }
            Command::CopyLay { from, to, dur_ms } => {
                self.stage.copylay(from as usize, to as usize, dur_ms)?;
                Ok(false)
            }
            Command::Move { n, frames } => {
                self.stage.start_move(n as usize, frames)?;
                Ok(false)
            }
            Command::MoveEase { n, frames, ease } => {
                let e =
                    crate::stage::Ease::parse(&ease).ok_or(format!("move 未知缓动方法：{ease}"))?;
                self.stage.start_move_ease(n as usize, frames, e)?;
                Ok(false)
            }
            Command::LayOpt { n, visible, opacity } => {
                self.stage.lay_opt(n as usize, visible, opacity)?;
                Ok(false)
            }
            Command::LayCount { n } => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    &format!("laycount：立绘层数上限记录 {n}（本引擎固定 3 槽，记录不生效）"),
                );
                Ok(false)
            }
            Command::StopTrans => {
                self.stage.stop_trans();
                Ok(false)
            }
            Command::AnimStart { n, name } => {
                // .asd 档从数据目录 asd/ 查（复用素材路径解析兜底 data 直下）
                let path = gal_config::paths::resolve("asd", &format!("{name}.asd"));
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| format!("animstart 动画档读取失败 {path}：{e}"))?;
                let (pos, opa) =
                    crate::stage::parse_asd(&text).map_err(|e| format!("animstart {name}：{e}"))?;
                self.stage.start_anim(n as usize, pos, opa)?;
                Ok(false)
            }
            Command::WaitMove => match self.stage.move_left_ms() {
                Some(left) => {
                    self.state = RunState::WaitTimer { left_ms: left as f32 };
                    Ok(true)
                }
                None => Ok(false),
            },
            Command::AnimStop { n } => {
                match n {
                    Some(n) => self.stage.stop_move(n as usize),
                    None => {
                        let ns: Vec<usize> = self.stage.moves.iter().map(|m| m.layer).collect();
                        for n in ns {
                            self.stage.stop_move(n);
                        }
                    }
                }
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    &format!("[animstop] 已取消位移动画（{} 处）", n.is_some() as u8 + 1),
                );
                Ok(false)
            }
            // ===== v0.3.7 forelay/无前缀 trans：back 页一次翻上（双页原子转场） =====
            Command::ForeLay { n, dur_ms, method } => {
                let dur_ms = if self.trans_enabled { dur_ms } else { 0 };
                let method = if self.trans_enabled { method } else { "fade".to_string() };
                let n = n.map(|x| x as usize).unwrap_or(crate::stage::BG_LAYER);
                self.stage.fore_trans(n, dur_ms, crate::stage::TransMethod::parse(&method))?;
                Ok(false)
            }
            Command::WaitTrans => match self.stage.trans_left_ms() {
                Some(left) => {
                    self.state = RunState::WaitTimer { left_ms: left };
                    Ok(true)
                }
                None => Ok(false),
            },
            // ===== v0.3.3 KAG3 wm/wv/wp：进入 WaitAudio，引擎轮询放行 =====
            Command::WaitMusic => {
                self.state = RunState::WaitAudio(AudioWait::MusicDone);
                Ok(true)
            }
            // ===== v0.3.8 KAG3 视频标签组：进 SysEvent 队列，引擎侧按 feature 消费 =====
            Command::Video { params } => {
                self.events.push(SysEvent::VideoSet { params });
                Ok(false)
            }
            Command::PlayVideo { name, ms, skip } => {
                self.events.push(SysEvent::PlayVideo { name, ms, skip });
                Ok(false)
            }
            Command::StopVideo => {
                self.events.push(SysEvent::StopVideo);
                Ok(false)
            }
            Command::OpenVideo { name } => {
                self.events.push(SysEvent::OpenVideo { name });
                Ok(false)
            }
            Command::PauseVideo => {
                self.events.push(SysEvent::PauseVideo);
                Ok(false)
            }
            Command::ResumeVideo => {
                self.events.push(SysEvent::ResumeVideo);
                Ok(false)
            }
            Command::PrepareVideo { name } => {
                self.events.push(SysEvent::PrepareVideo { name });
                Ok(false)
            }
            Command::RewindVideo => {
                self.events.push(SysEvent::RewindVideo);
                Ok(false)
            }
            Command::WaitVideo => {
                self.state = RunState::WaitAudio(AudioWait::VideoDone);
                Ok(true)
            }
            Command::WaitAllSound => {
                self.state = RunState::WaitAudio(AudioWait::AllDone);
                Ok(true)
            }
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
                let if_at = self.pc - 1;
                if hit {
                    // v0.3.4 压栈标记「本链已命中」：后续 elsif/else 走到即跳过
                    self.branch_taken.push((self.cur_scope(), if_at));
                } else {
                    // 假分支：沿 elsif 链找下一分支，否则 else/endif 之后
                    self.pc = self
                        .next_branch_of(&self.scope_file(), if_at, if_at)
                        .ok_or_else(|| format!("{ctx}：if 块配对表缺失（疑似剧本热更新）"))?;
                }
                Ok(false)
            }
            Command::ElsIf { cond } => {
                let elsif_at = self.pc - 1;
                let if_at = self
                    .if_owner_of_branch(&self.scope_file(), elsif_at)
                    .ok_or_else(|| format!("{ctx}：elsif 配对表缺失"))?;
                // 前面已有命中分支落到这里（fall-through）：整段跳过，不弹栈
                if self.branch_taken.last() == Some(&(self.cur_scope(), if_at)) {
                    let (_, end) = *self
                        .blocks_of(&self.scope_file())
                        .and_then(|b| b.get(&if_at))
                        .ok_or_else(|| format!("{ctx}：if 块配对表缺失"))?;
                    self.pc = end + 1;
                    return Ok(false);
                }
                let hit = crate::expr::eval(&cond, &self.vars)
                    .map(|v| crate::expr::truthy(&v))
                    .map_err(|e| format!("{ctx}：elsif 条件错误：{e}"))?;
                if hit {
                    self.branch_taken.push((self.cur_scope(), if_at));
                } else {
                    self.pc = self
                        .next_branch_of(&self.scope_file(), if_at, elsif_at)
                        .ok_or_else(|| format!("{ctx}：if 块配对表缺失"))?;
                }
                Ok(false)
            }
            Command::Emb { expr } => {
                // KAG3 emb：求值结果拼进当前台词（无台词 = 单独一行显示）
                let v = crate::expr::eval(&expr.clone(), &self.vars)
                    .map_err(|e| format!("{ctx}：emb 求值失败：{e}"))?;
                let piece = value_to_text(&v);
                match self.pending_text.take() {
                    Some(t) => self.pending_text = Some(t + &piece),
                    None => self.pending_text = Some(piece),
                }
                Ok(false)
            }
            Command::Eval { expr } => {
                let rv = crate::expr::eval_assign(&expr.clone(), &mut self.vars);
                rv.map_err(|e| format!("{ctx}：eval 失败：{e}"))?;
                Ok(false)
            }
            Command::Trace { text } => {
                // 静默失败留证通道：支持 %表达式 内联求值（求不出按原样保留）
                let mut out = String::new();
                let mut rest = text.as_str();
                while let Some(p) = rest.find('%') {
                    out.push_str(&rest[..p]);
                    let tail = &rest[p + 1..];
                    match eval_trace_field(tail, &self.vars) {
                        Some((rendered, used)) => {
                            out.push_str(&rendered);
                            rest = &tail[used..];
                        }
                        None => {
                            out.push('%');
                            rest = tail;
                        }
                    }
                }
                out.push_str(rest);
                gal_audio::log::log(
                    gal_audio::log::Level::Info,
                    "script",
                    &format!("[trace] {out}"),
                );
                Ok(false)
            }
            Command::Graph { storage } => {
                // v0.3.13 KAG3 graph：\u{b}素材名\u{b} 占位符拼进当前台词（无台词 = 单独一行）
                let piece = format!("\u{b}{storage}\u{b}");
                match self.pending_text.take() {
                    Some(t) => self.pending_text = Some(t + &piece),
                    None => self.pending_text = Some(piece),
                }
                Ok(false)
            }
            Command::RubyTag { text, alt } => {
                // v0.3.13 KAG3 ruby：转成既有 `[基底]^(注音)` 文法（排版/朗读全链路复用）
                let piece = format!("[{text}]^({alt})");
                match self.pending_text.take() {
                    Some(t) => self.pending_text = Some(t + &piece),
                    None => self.pending_text = Some(piece),
                }
                Ok(false)
            }
            Command::Hch { text } => {
                // 行内样式片段：无行内色通道，按普通文本拼进台词 + DEBUG 留证
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    &format!("[hch] 行内样式简化为普通文本：{text}"),
                );
                match self.pending_text.take() {
                    Some(t) => self.pending_text = Some(t + &text),
                    None => self.pending_text = Some(text),
                }
                Ok(false)
            }
            Command::Font { size, color } => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    &format!(
                        "[font] 行内文字属性记录（演出零变化）：size={size:?} color={color:?}"
                    ),
                );
                Ok(false)
            }
            Command::Deffont { size, color } => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    &format!("[deffont] 页面级文字属性记录：size={size:?} color={color:?}"),
                );
                Ok(false)
            }
            Command::Resetfont => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    "[resetfont] 文字属性复位（DEBUG 留证）",
                );
                Ok(false)
            }
            Command::Style { align } | Command::Defstyle { align } => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    &format!("[style] 段落样式记录（演出零变化）：align={align:?}"),
                );
                Ok(false)
            }
            Command::Resetstyle => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    "[resetstyle] 段落样式复位（DEBUG 留证）",
                );
                Ok(false)
            }
            Command::LineBreak => {
                // v0.3.14 KAG3 [r]：行内强制换行（\u{1e} 标记，排版层立即收行）
                self.append_pending("\u{1e}");
                Ok(false)
            }
            Command::LineBreakWait => {
                // KAG3 [l]：换行后等一次点击
                self.append_pending("\u{1e}");
                self.state = RunState::WaitClick;
                Ok(true)
            }
            Command::PageBreakWait => {
                // KAG3 [p]：等点击；点击后续行前文字消去（pending 分段：等待后再清）
                self.append_pending("\u{1e}");
                self.state = RunState::WaitClick;
                Ok(true)
            }
            Command::EraseText => {
                // KAG3 [er]：清当前消息层文字（无等待）
                self.tw.clear();
                self.cur_text.clear();
                Ok(false)
            }
            Command::ClearMessage | Command::ClearMessageAll => {
                // KAG3 [cm]/[ct]：清空消息层（回卷历史保留；ct 的样式复位本引擎无状态可复位）
                self.tw.clear();
                self.cur_text.clear();
                self.cur_name = None;
                Ok(false)
            }
            Command::Indent => {
                self.indent_on = true;
                Ok(false)
            }
            Command::EndIndent => {
                self.indent_on = false;
                Ok(false)
            }
            Command::Locate { x, y } => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    &format!("[locate] 落点记录（按行号排版，坐标忽略）：x={x:?} y={y:?}"),
                );
                Ok(false)
            }
            Command::Position { left, top, width, height, visible, opacity } => {
                // v0.3.16 KAG3 position：写 sf.msg*（引擎 compose_style 每帧对账生效）
                let mut put = |k: &str, v: i64| {
                    self.vars.set(&format!("sf.msg{k}"), Value::Int(v));
                };
                if let Some(v) = left {
                    put("Left", v as i64);
                }
                if let Some(v) = top {
                    put("Top", v as i64);
                }
                if let Some(v) = width {
                    put("Width", v as i64);
                }
                if let Some(v) = height {
                    put("Height", v as i64);
                }
                if let Some(v) = visible {
                    put("Visible", i64::from(v));
                }
                if let Some(v) = opacity {
                    put("Opacity", v as i64);
                }
                Ok(false)
            }
            Command::Current { layer } => {
                // v0.3.16 单消息层引擎：current 记录 + DEBUG 留证（多文字层留待后续批次）
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    &format!("[current] 消息层指定（单消息层引擎，记录留证）：layer={layer:?}"),
                );
                Ok(false)
            }
            Command::Glyph { storage } => {
                // v0.3.17：写 sf.msgGlyph，compose_style 对账 → DialogStyle.glyph → draw 页满贴图
                self.vars.set("sf.msgGlyph", Value::Str(storage.clone()));
                Ok(false)
            }
            Command::NoWait => {
                self.nowait_on = true;
                Ok(false)
            }
            Command::EndNoWait => {
                self.nowait_on = false;
                Ok(false)
            }
            Command::Link { storage, exp } => {
                // v0.3.15 KAG3 link：行内超链接开始。记录起始字符位与动作，
                // endlink 时按当前 pending 长度闭区间成区
                if self.link_open_at.is_some() {
                    return Err("link 嵌套：上一组 link 未 endlink".into());
                }
                self.link_open_at =
                    Some(self.pending_text.as_ref().map(|t| t.chars().count()).unwrap_or(0));
                self.link_action = Some(match (storage, exp) {
                    (Some(s), _) => LinkAction::Jump(s),
                    (_, Some(e)) => LinkAction::Exp(e),
                    _ => unreachable!("parse 已保证 storage/exp 至少一项"),
                });
                Ok(false)
            }
            Command::EndLink => {
                // v0.3.15 endlink：闭区间成区（区间随 apply_pending 落到 self.links）
                match (self.link_open_at.take(), self.link_action.take()) {
                    (Some(start), Some(action)) => {
                        let end =
                            self.pending_text.as_ref().map(|t| t.chars().count()).unwrap_or(0);
                        if end <= start {
                            return Err("endlink：link 与 endlink 之间没有文本".into());
                        }
                        self.pending_links.push(LinkRegion { start, end, action });
                    }
                    _ => return Err("endlink：没有配对的 link".into()),
                }
                Ok(false)
            }
            Command::LockLink => {
                self.links_locked = true;
                Ok(false)
            }
            Command::UnLockLink => {
                self.links_locked = false;
                Ok(false)
            }
            Command::Button { graphic, storage, exp } => {
                // v0.3.15 KAG3 button：图形按钮等待态（image/ 素材整幅可点）
                self.tw.clear();
                self.cur_name = None;
                let action = match (storage, exp) {
                    (Some(s), _) => LinkAction::Jump(s),
                    (_, Some(e)) => LinkAction::Exp(e),
                    _ => unreachable!("parse 已保证 storage/exp 至少一项"),
                };
                self.button = Some(ButtonSpec { graphic, action });
                self.state = RunState::WaitButton;
                Ok(true)
            }
            Command::Else => {
                let else_at = self.pc - 1;
                // 有命中分支标记时是「真分支走完跳过」；无标记 = elsif 全假落 else，照常进
                if let Some(if_at) = self.if_owner_of_branch(&self.scope_file(), else_at) {
                    if self.branch_taken.last() == Some(&(self.cur_scope(), if_at)) {
                        let end = self
                            .else_target_of(&self.scope_file(), else_at)
                            .ok_or_else(|| format!("{ctx}：else 配对表缺失"))?;
                        self.pc = end + 1;
                        return Ok(false);
                    }
                }
                Ok(false)
            }
            Command::EndIf => {
                // 弹掉本块命中分支的标记（嵌套内层先弹）
                let end_at = self.pc - 1;
                if let Some(if_at) = self.if_owner_of_end(&self.scope_file(), end_at) {
                    if self.branch_taken.last() == Some(&(self.cur_scope(), if_at)) {
                        self.branch_taken.pop();
                    }
                }
                Ok(false)
            }
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
            // ===== v0.3.5 宏系统与变量清理 =====
            // 宏定义占位行（宏体在 load 时已注册进宏表，这里直接跳过）
            Command::MacroDefSkip => Ok(false),
            Command::EraseMacro(name) => {
                let removed = self.macros.remove(&name).is_some();
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    &format!(
                        "[macro] erasemacro「{name}」：{}",
                        if removed { "已删除" } else { "宏不存在（忽略）" }
                    ),
                );
                Ok(false)
            }
            Command::ClearVar => {
                let n = self.vars.f.len();
                self.vars.f.clear();
                gal_audio::log::log(
                    gal_audio::log::Level::Info,
                    "script",
                    &format!("[clearvar] 已清空 {n} 个 f.* 游戏变量"),
                );
                Ok(false)
            }
            Command::ClearSysVar => {
                let n = self.vars.sf.len();
                self.vars.sf.clear();
                gal_audio::log::log(
                    gal_audio::log::Level::Info,
                    "script",
                    &format!("[clearsysvar] 已清空 {n} 个 sf.* 系统变量（global.json 结构不变）"),
                );
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
            Command::Quake { ms, max_amp } => {
                // KAG3 quake：本引擎幅度固定包络（帧侧衰减），max_amp 记录进事件留证
                self.events.push(SysEvent::Quake { ms, max_amp });
                Ok(false)
            }
            Command::StopQuake => {
                self.events.push(SysEvent::StopQuake);
                Ok(false)
            }
            Command::WaitQuake => {
                self.state = RunState::WaitAudio(crate::interp::AudioWait::QuakeDone);
                Ok(true)
            }
            Command::WaitClickCount => {
                self.state = RunState::WaitClick;
                Ok(true)
            }
            Command::ResetWait => {
                // KAG3 resetwait：取消当前强制等待（wait/wq 等）立即放行
                if matches!(self.state, RunState::WaitTimer { .. }) {
                    self.run_until_wait()?;
                }
                Ok(false)
            }
            Command::LockSnapshot { name } => {
                self.snapshot_lock = Some(name);
                Ok(false)
            }
            Command::UnlockSnapshot => {
                self.snapshot_lock = None;
                Ok(false)
            }
            // ===== v0.3.12 存档控制标签批 =====
            Command::StopSkip => {
                // KAG3 `s`：停止快进/自动（引擎态经由 SysEvent 通道翻转）
                self.events.push(SysEvent::StopSkip);
                Ok(false)
            }
            Command::DisableStore { on } => {
                self.store_disabled = on;
                Ok(false)
            }
            Command::TempSave => {
                self.events.push(SysEvent::TempSave);
                Ok(false)
            }
            Command::TempLoad => {
                self.events.push(SysEvent::TempLoad);
                Ok(false)
            }
            Command::CopyBookmark { from, to } => {
                self.events.push(SysEvent::CopyBookmark { from, to });
                Ok(false)
            }
            Command::EraseBookmark { slot } => {
                self.events.push(SysEvent::EraseBookmark { slot });
                Ok(false)
            }
            Command::Record => {
                self.events.push(SysEvent::OpenGallery);
                Ok(false)
            }
            Command::GoBack => {
                self.events.push(SysEvent::GoBack);
                Ok(false)
            }
            Command::WaitTrig { name } => {
                // 触发器口径：f.trig_<名> 为真即放行（已置位立即通过）
                let key = format!("trig_{name}");
                let hit = self.vars.get(&format!("f.{key}")).map(crate::expr::truthy);
                if hit != Some(true) {
                    self.state = RunState::WaitAudio(crate::interp::AudioWait::Trig(key));
                }
                Ok(true)
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
