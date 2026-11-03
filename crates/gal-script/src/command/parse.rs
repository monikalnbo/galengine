//! 剧本指令语法解析（ctx 为「文件:行号」中文错误定位）

use super::types::Command;
use crate::lexer::LineKind;

impl Command {
    pub fn parse(kind: &LineKind, ctx: &str) -> Result<Command, String> {
        let (name, rest) = match kind {
            LineKind::Command { name, rest } => (name.as_str(), rest.as_str()),
            LineKind::Choice { prompt, items } => {
                return Ok(Command::Choice { prompt: prompt.clone(), items: items.clone() })
            }
            LineKind::Label(_) => return Err(format!("{ctx}：label 不应作为指令执行")),
            // v0.3.5 宏定义行在 lexer 已收走宏体，正常不会流到这里（宏注册时过滤）
            LineKind::MacroDef { .. } => return Ok(Command::MacroDefSkip),
        };
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        let err = |msg: &str| Err(format!("{ctx}：{msg}"));

        match name {
            "bg" => match tokens.as_slice() {
                [s] => Ok(Command::Bg { storage: s.to_string(), fade_ms: 700 }),
                [s, ms] => Ok(Command::Bg { storage: s.to_string(), fade_ms: num(ms)? }),
                _ => err("bg 用法：bg 素材 [淡入ms]"),
            },
            "bgm" => match tokens.as_slice() {
                [s] if s == &"stop" => Ok(Command::BgmStop),
                [s, ms] if s == &"fadeout" => Ok(Command::BgmFadeOut { ms: num(ms)? }),
                [s] => Ok(Command::Bgm(s.to_string())),
                _ => err("bgm 用法：bgm 名称 / bgm stop / bgm fadeout ms"),
            },
            "se" => match tokens.as_slice() {
                [s] => Ok(Command::Se(s.to_string())),
                _ => err("se 用法：se 名称"),
            },
            "voice" => match tokens.as_slice() {
                [s] if s == &"stop" => Ok(Command::VoiceStop),
                [s] => Ok(Command::Voice(s.to_string())),
                _ => err("voice 用法：voice 名称 / voice stop"),
            },
            "char" => match tokens.as_slice() {
                [l, s] => Ok(Command::Char {
                    layer: l
                        .parse::<u8>()
                        .ok()
                        .filter(|n| *n < 3)
                        .ok_or(format!("{ctx}：立绘层必须是 0-2"))?,
                    storage: s.to_string(),
                    x: None,
                    y: None,
                }),
                [l, s, pos] => {
                    let lx = l
                        .parse::<u8>()
                        .ok()
                        .filter(|n| *n < 3)
                        .ok_or(format!("{ctx}：立绘层必须是 0-2"))?;
                    Ok(Command::CharPos {
                        layer: lx,
                        storage: s.to_string(),
                        pos_name: pos.to_string(),
                    })
                }
                [l, s, x, y] => {
                    let lx = l
                        .parse::<u8>()
                        .ok()
                        .filter(|n| *n < 3)
                        .ok_or(format!("{ctx}：立绘层必须是 0-2"))?;
                    let (ox, oy) = match (x.parse::<i32>(), y.parse::<i32>()) {
                        (Ok(nx), Ok(ny)) => (Some(nx), Some(ny)),
                        _ => {
                            return Ok(Command::CharPos {
                                layer: lx,
                                storage: s.to_string(),
                                pos_name: format!("{x} {y}"),
                            });
                        }
                    };
                    Ok(Command::Char { layer: lx, storage: s.to_string(), x: ox, y: oy })
                }
                _ => err("char 用法：char 层(0-2) 素材|hide [x y | 位置名]"),
            },
            "charsway" => match tokens.as_slice() {
                [l, n] => Ok(Command::CharSway {
                    layer: layer_of(l, ctx)?,
                    count: num(n)?,
                    period_ms: 300,
                }),
                [l, n, ms] => Ok(Command::CharSway {
                    layer: layer_of(l, ctx)?,
                    count: num(n)?,
                    period_ms: num(ms)?,
                }),
                _ => err("charsway 用法：charsway 层(0-2) 次数 [周期ms]"),
            },
            "cg" => match tokens.as_slice() {
                // `cg hide [ms]`：隐藏并（可选）以指定时长淡出；此前该分支漏判，
                // 导致 `cg hide 800` 被吞成默认 700ms。
                [s] if s == &"hide" => Ok(Command::Cg { storage: s.to_string(), fade_ms: 700 }),
                [s, ms] if s == &"hide" => {
                    Ok(Command::Cg { storage: s.to_string(), fade_ms: num(ms)? })
                }
                [s] => Ok(Command::Cg { storage: s.to_string(), fade_ms: 700 }),
                [s, ms] => Ok(Command::Cg { storage: s.to_string(), fade_ms: num(ms)? }),
                _ => err("cg 用法：cg 素材 [淡入ms] / cg hide [ms]"),
            },
            "layer" => match tokens.as_slice() {
                // 自由层：层号 >=5（0-4 是 bg/立绘/cg 的保留层号）
                [n, s] => Ok(Command::Layer {
                    n: free_layer_of(n, ctx)?,
                    storage: s.to_string(),
                    fade_ms: 700,
                }),
                [n, s, ms] => Ok(Command::Layer {
                    n: free_layer_of(n, ctx)?,
                    storage: s.to_string(),
                    fade_ms: num(ms)?,
                }),
                _ => err("layer 用法：layer 层号(5-63) 素材|hide [淡入ms]"),
            },
            "trans" => match tokens.as_slice() {
                // v0.3.7 无前缀形态（KAG3 trans）：trans [ms|method] —— 把已备好的 back 页
                // 一次翻上（原子转场）。仅 0/1 参数归 ForeLay（≥2 参数维持旧版
                // 「trans 层号 素材 …」语法，避免与层号歧义）；双参数形态用 forelay。
                [] => Ok(Command::ForeLay { n: None, dur_ms: 700, method: "universal".into() }),
                [ms] if num::<u32>(ms).is_ok() => Ok(Command::ForeLay {
                    n: None,
                    dur_ms: num::<u32>(ms)?,
                    method: "universal".into(),
                }),
                [m] => Ok(Command::ForeLay { n: None, dur_ms: 700, method: m.to_string() }),
                // 双参数消歧：首参不是合法层号（如 500）或第二参是纯数字 → 无前缀
                // `trans ms method`（如 trans 500 crossfade）；否则旧版「trans 层号 素材」。
                [a, b] if trans_layer_of(a, ctx).is_err() || num::<u32>(b).is_ok() => {
                    Ok(Command::ForeLay {
                        n: None,
                        dur_ms: num::<u32>(a).unwrap_or(700),
                        method: b.to_string(),
                    })
                }
                // 形式：trans 层号 素材 [ms] [method]；ms 可省略（默认 700，与 bg/cg 一致）
                [n, s] => Ok(Command::Trans {
                    n: trans_layer_of(n, ctx)?,
                    storage: s.to_string(),
                    dur_ms: 700,
                    method: "universal".into(),
                }),
                [n, s, ms] => match num::<u32>(ms) {
                    Ok(dur) => Ok(Command::Trans {
                        n: trans_layer_of(n, ctx)?,
                        storage: s.to_string(),
                        dur_ms: dur,
                        method: "universal".into(),
                    }),
                    // 第三参数不是毫秒 → 当作 method（`trans 0 bg/b 500 crossfade`
                    // 是四参数形态，三参数时不可能是 method）
                    Err(_) => Ok(Command::Trans {
                        n: trans_layer_of(n, ctx)?,
                        storage: s.to_string(),
                        dur_ms: 700,
                        method: ms.to_string(),
                    }),
                },
                [n, s, ms, m] => Ok(Command::Trans {
                    n: trans_layer_of(n, ctx)?,
                    storage: s.to_string(),
                    dur_ms: num(ms)?,
                    method: m.to_string(),
                }),
                _ => err("trans 用法：trans 层(0-63) 素材 [ms] [universal|crossfade]"),
            },
            // ===== v0.3.6 fore/back 双页 + move 动画 =====
            "backlay" => match tokens.as_slice() {
                [] => Ok(Command::BackLay { n: None, hide: false }),
                ["hide"] => Ok(Command::BackLay { n: None, hide: true }),
                [n] => Ok(Command::BackLay { n: Some(layer_num(n, ctx)?), hide: false }),
                [n, "hide"] => Ok(Command::BackLay { n: Some(layer_num(n, ctx)?), hide: true }),
                _ => err("backlay 用法：backlay [层号] / backlay [层号] hide"),
            },
            // ===== v0.3.7 forelay：back 页翻上到 fore 页（KAG3 双页原子转场） =====
            "forelay" => {
                let mut n = None;
                let mut dur_ms = 700;
                let mut method = "universal".to_string();
                for t in &tokens {
                    if let Ok(ms) = num::<u32>(t) {
                        dur_ms = ms;
                    } else if n.is_none() && t.parse::<u8>().is_ok() {
                        // method 名（crossfade/slide-l…）与毫秒都不会撞纯数字层号
                        n = t.parse::<u8>().ok();
                    } else {
                        method = t.to_string();
                    }
                }
                Ok(Command::ForeLay { n, dur_ms, method })
            }
            "copylay" => match tokens.as_slice() {
                [f, t] => Ok(Command::CopyLay {
                    from: layer_num(f, ctx)?,
                    to: layer_num(t, ctx)?,
                    dur_ms: 0,
                }),
                [f, t, ms] => Ok(Command::CopyLay {
                    from: layer_num(f, ctx)?,
                    to: layer_num(t, ctx)?,
                    dur_ms: num(ms)?,
                }),
                _ => err("copylay 用法：copylay 源层 目标层 [ms]"),
            },
            "move" if tokens.len() >= 2 => {
                let n = layer_num(tokens[0], ctx)?;
                let mut frames = Vec::new();
                let mut ease: Option<String> = None;
                for t in &tokens[1..] {
                    // v0.3.9 缓动方法名尾参（linear/easein/easeout/easeinout，非帧写法即识别）
                    if matches!(&**t, "linear" | "easein" | "easeout" | "easeinout") {
                        ease = Some(t.to_string());
                        continue;
                    }
                    // 帧形态 x,y 或 x,y@ms（缺 @ms 延续上一帧时间，首个缺省 0）
                    let (xy, ms) = match t.split_once('@') {
                        Some((xy, ms)) => {
                            let m = ms
                                .parse::<u32>()
                                .map_err(|_| format!("{ctx}：move 帧时间无效：{t}"))?;
                            (xy, Some(m))
                        }
                        None => (*t, None),
                    };
                    let (x, y) = xy
                        .split_once(',')
                        .ok_or(format!("{ctx}：move 帧写法应为 x,y 或 x,y@ms：{t}"))?;
                    let t_prev = frames.last().map(|f: &(u32, i32, i32)| f.0).unwrap_or(0);
                    frames.push((ms.unwrap_or(t_prev), num(x)?, num(y)?));
                }
                match ease {
                    Some(ease) => Ok(Command::MoveEase { n, frames, ease }),
                    None => Ok(Command::Move { n, frames }),
                }
            }
            // ===== v0.3.9 条件标签收尾 =====
            "layopt" if !tokens.is_empty() => {
                let n = layer_num(tokens[0], ctx)?;
                let mut visible = None;
                let mut opacity = None;
                for t in &tokens[1..] {
                    if let Some(v) = t.strip_prefix("visible=") {
                        visible = Some(matches!(v, "true" | "t" | "1"));
                    } else if let Some(o) = t.strip_prefix("opacity=") {
                        opacity = Some(num::<u8>(o)?);
                    } else {
                        return err("layopt 用法：layopt 层 [visible=t/f] [opacity=0-255]");
                    }
                }
                Ok(Command::LayOpt { n, visible, opacity })
            }
            "laycount" => match tokens.as_slice() {
                [n] => Ok(Command::LayCount { n: num(n)? }),
                _ => err("laycount 用法：laycount 层数"),
            },
            "stoptrans" if tokens.is_empty() => Ok(Command::StopTrans),
            "animstart" => match tokens.as_slice() {
                [n, name] => {
                    Ok(Command::AnimStart { n: layer_num(n, ctx)?, name: name.to_string() })
                }
                _ => err("animstart 用法：animstart 层 档名"),
            },
            // move 首参是层号：`move 层 x,y@ms …`
            "wa" if tokens.is_empty() => Ok(Command::WaitMove),
            "animstop" => match tokens.as_slice() {
                [] => Ok(Command::AnimStop { n: None }),
                [n] => Ok(Command::AnimStop { n: Some(layer_num(n, ctx)?) }),
                _ => err("animstop 用法：animstop [层号]"),
            },
            // ===== v0.3.2 KAG3 音频标签组 =====
            "playbgm" => match tokens.as_slice() {
                [s] => Ok(Command::PlayBgm { name: s.to_string(), fade_in_ms: 0 }),
                [s, ms] => Ok(Command::PlayBgm { name: s.to_string(), fade_in_ms: num(ms)? }),
                _ => err("playbgm 用法：playbgm 名称 [淡入ms]"),
            },
            "pausebgm" if tokens.is_empty() => Ok(Command::PauseBgm),
            "resumebgm" if tokens.is_empty() => Ok(Command::ResumeBgm),
            "fadeinbgm" => match tokens.as_slice() {
                [s, ms] => Ok(Command::FadeInBgm { name: s.to_string(), ms: num(ms)? }),
                _ => err("fadeinbgm 用法：fadeinbgm 名称 ms"),
            },
            "fadeoutbgm" => match tokens.as_slice() {
                [ms] => Ok(Command::FadeOutBgm { ms: num(ms)? }),
                _ => err("fadeoutbgm 用法：fadeoutbgm ms"),
            },
            "fadepausebgm" => match tokens.as_slice() {
                [ms] => Ok(Command::FadePauseBgm { ms: num(ms)? }),
                _ => err("fadepausebgm 用法：fadepausebgm ms"),
            },
            "fadebgm" => match tokens.as_slice() {
                [pct, ms] => Ok(Command::FadeBgm { to_pct: num(pct)?, ms: num(ms)? }),
                _ => err("fadebgm 用法：fadebgm 百分比 ms"),
            },
            "xchgbgm" => match tokens.as_slice() {
                [s, ms] => Ok(Command::XchgBgm { name: s.to_string(), ms: num(ms)? }),
                _ => err("xchgbgm 用法：xchgbgm 名称 ms"),
            },
            "bgmopt" => match tokens.as_slice() {
                [a, b] => Ok(Command::BgmOpt { loop_start_ms: num(a)?, loop_end_ms: num(b)? }),
                _ => err("bgmopt 用法：bgmopt 循环起点ms 循环终点ms"),
            },
            "playse" => match tokens.as_slice() {
                [s] => Ok(Command::PlaySe { name: s.to_string() }),
                _ => err("playse 用法：playse 名称"),
            },
            "stopse" if tokens.is_empty() => Ok(Command::StopSe),
            "fadeinse" => match tokens.as_slice() {
                [s, ms] => Ok(Command::FadeInSe { name: s.to_string(), ms: num(ms)? }),
                _ => err("fadeinse 用法：fadeinse 名称 ms"),
            },
            "fadeoutse" => match tokens.as_slice() {
                [s, ms] => Ok(Command::FadeOutSe { name: s.to_string(), ms: num(ms)? }),
                _ => err("fadeoutse 用法：fadeoutse 名称 ms"),
            },
            "fadese" => match tokens.as_slice() {
                [pct, ms] => Ok(Command::FadeSe { to_pct: num(pct)?, ms: num(ms)? }),
                _ => err("fadese 用法：fadese 百分比 ms"),
            },
            "seopt" => match tokens.as_slice() {
                [s] => Ok(Command::SeOpt { name: s.to_string() }),
                _ => err("seopt 用法：seopt 名称"),
            },
            "wb" if tokens.is_empty() => Ok(Command::WaitBgm),
            "wl" if tokens.is_empty() => Ok(Command::WaitLoop),
            "ws" if tokens.is_empty() => Ok(Command::WaitSe),
            "wf" if tokens.is_empty() => Ok(Command::WaitSeFade),
            // v0.3.3 KAG3 wm/wv/wp：等 BGM 播完 / 等视频播完 / 等全部声音播完
            "wm" if tokens.is_empty() => Ok(Command::WaitMusic),
            "wv" if tokens.is_empty() => Ok(Command::WaitVideo),
            // ===== v0.3.8 KAG3 视频标签组（引擎侧未开 video feature 时 WARN 放行）=====
            "video" => Ok(Command::Video { params: tokens.join(" ") }),
            "playvideo" => match tokens.as_slice() {
                [name] => Ok(Command::PlayVideo { name: name.to_string(), ms: 0, skip: false }),
                [name, ms] => match num::<u32>(ms) {
                    Ok(v) => Ok(Command::PlayVideo { name: name.to_string(), ms: v, skip: false }),
                    Err(_) => Ok(Command::PlayVideo {
                        name: name.to_string(),
                        ms: 0,
                        skip: ms == &"skip",
                    }),
                },
                [name, ms, rest_tokens @ ..] => {
                    let skip = rest_tokens.iter().any(|t| t == &"skip");
                    Ok(Command::PlayVideo {
                        name: name.to_string(),
                        ms: num::<u32>(ms).unwrap_or(0),
                        skip,
                    })
                }
                _ => err("playvideo 用法：playvideo 名称 [ms] [skip]"),
            },
            "stopvideo" if tokens.is_empty() => Ok(Command::StopVideo),
            "openvideo" => match tokens.as_slice() {
                [name] => Ok(Command::OpenVideo { name: name.to_string() }),
                _ => err("openvideo 用法：openvideo 名称"),
            },
            "pausevideo" if tokens.is_empty() => Ok(Command::PauseVideo),
            "resumevideo" if tokens.is_empty() => Ok(Command::ResumeVideo),
            "preparevideo" => match tokens.as_slice() {
                [name] => Ok(Command::PrepareVideo { name: name.to_string() }),
                _ => err("preparevideo 用法：preparevideo 名称"),
            },
            "rewindvideo" if tokens.is_empty() => Ok(Command::RewindVideo),
            "wp" if tokens.is_empty() => Ok(Command::WaitAllSound),
            // KAG3 `wt`：多种等待类型；当前只实装转场等待（其余参数忽略）
            "wt" => Ok(Command::WaitTrans),
            "clear" => Ok(Command::Clear),
            "wait" => match tokens.as_slice() {
                [ms] => Ok(Command::Wait(num(ms)?)),
                _ => err("wait 用法：wait 毫秒"),
            },
            "n" => Ok(Command::N(rest.into())),
            "name" => match rest.split_once(' ') {
                Some((n, text)) => {
                    Ok(Command::Name { name: n.to_string(), text: text.trim().into() })
                }
                None => err("name 用法：name 名字 台词"),
            },
            "flag" => {
                let (var, op, val) = match tokens.as_slice() {
                    [v, op, val] if *op == "+" || *op == "-" => {
                        (v.to_string(), op.to_string(), val.to_string())
                    }
                    [v, oval] if oval.starts_with('+') || oval.starts_with('-') => {
                        (v.to_string(), oval[..1].to_string(), oval[1..].to_string())
                    }
                    _ => return err("flag 用法：flag 变量 +2 / -1"),
                };
                Ok(Command::Flag { var, op: op.chars().next().unwrap(), val: num::<i64>(&val)? })
            }
            "set" => {
                let (v, after) =
                    rest.split_once('=').ok_or(format!("{ctx}：set 用法：set 变量 = 表达式"))?;
                let expr = after.trim().trim_start_matches('=').trim().to_string();
                if expr.is_empty() {
                    return err("set 表达式为空");
                }
                Ok(Command::Set { var: v.trim().into(), expr })
            }
            "jump" => match tokens.as_slice() {
                [t] => Ok(jump(None, t)),
                [f, t] => Ok(jump(Some(f.to_string()), t)),
                _ => err("jump 用法：jump *label / jump 文件 *label"),
            },
            "if" => match tokens.as_slice() {
                // 单行形态：`if f.a == 1 *label` / `if f.a + 1 > f.b *next`
                [.., t] if t.starts_with('*') => {
                    let cond = tokens[..tokens.len() - 1].join(" ");
                    if cond.is_empty() {
                        return err("if 用法：if 条件 *label 或 if 条件 […] ");
                    }
                    Ok(Command::If { cond, target: t.trim_start_matches('*').to_string() })
                }
                // 块形态：`if 条件` … `[else]` … `endif`
                _ if !tokens.is_empty() => Ok(Command::IfBlock { cond: rest.trim().to_string() }),
                _ => err("if 用法：if 条件 *label / if 条件 … endif"),
            },
            // v0.3.4 块中段：`elsif 条件`（KAG3 elsif）
            "elsif" if !tokens.is_empty() => Ok(Command::ElsIf { cond: rest.trim().to_string() }),
            "else" if tokens.is_empty() => Ok(Command::Else),
            "endif" if tokens.is_empty() => Ok(Command::EndIf),
            // v0.3.4 表达式族：emb（嵌入台词）/ eval（纯求值可含赋值）/ trace（日志）
            "emb" if !rest.trim().is_empty() => Ok(Command::Emb { expr: rest.trim().to_string() }),
            "eval" if !rest.trim().is_empty() => {
                Ok(Command::Eval { expr: rest.trim().to_string() })
            }
            "trace" if !rest.trim().is_empty() => {
                Ok(Command::Trace { text: rest.trim().to_string() })
            }
            "call" => match tokens.as_slice() {
                [t] => Ok(call(None, t)),
                [f, t] => Ok(call(Some(f.to_string()), t)),
                _ => err("call 用法：call *label / call 文件 *label"),
            },
            "return" if tokens.is_empty() => Ok(Command::Return),
            // v0.3.5 宏系统：定义行是占位（宏体已由 lexer 收走）；erasemacro 删宏
            "macro" | "endmacro" => Ok(Command::MacroDefSkip),
            "erasemacro" => match tokens.as_slice() {
                [n] => Ok(Command::EraseMacro(n.to_string())),
                _ => err("erasemacro 用法：erasemacro 宏名"),
            },
            // v0.3.5 变量清理：f.*/sf.* 清表（global.json 结构不变）
            "clearvar" if tokens.is_empty() => Ok(Command::ClearVar),
            "clearsysvar" if tokens.is_empty() => Ok(Command::ClearSysVar),
            "input" => {
                let (v, tail) = rest
                    .split_once(' ')
                    .ok_or(format!("{ctx}：input 用法：input 变量 提示|宽度|默认值"))?;
                let parts: Vec<&str> = tail.split('|').collect();
                match parts.as_slice() {
                    [prompt, w, def] => Ok(Command::Input {
                        var: v.to_string(),
                        prompt: prompt.trim().into(),
                        width: num(w)?,
                        default: def.trim().into(),
                    }),
                    [prompt, w] => Ok(Command::Input {
                        var: v.to_string(),
                        prompt: prompt.trim().into(),
                        width: num(w)?,
                        default: String::new(),
                    }),
                    _ => err("input 用法：input 变量 提示|宽度|默认值"),
                }
            }
            "meta_fake_save" => match rest.split('|').collect::<Vec<_>>()[..] {
                [d, t, img] => Ok(Command::MetaFakeSave {
                    date: d.trim().to_string(),
                    time: t.trim().to_string(),
                    image: img.trim().to_string(),
                }),
                _ => err("meta_fake_save 用法：meta_fake_save 日期|时间|图片"),
            },
            "meta_corrupt" => match tokens.as_slice() {
                [n] => Ok(Command::MetaCorrupt(n.to_string())),
                _ => err("meta_corrupt 用法：meta_corrupt N|all"),
            },
            "meta_delete_last" if tokens.is_empty() => Ok(Command::MetaDeleteLast),
            "title_evolve" => match tokens.as_slice() {
                [m] => Ok(Command::TitleEvolve(m.to_string())),
                _ => err("title_evolve 用法：title_evolve 阶段"),
            },
            "reach" => match tokens.as_slice() {
                [s] if s == &"hide" => {
                    Ok(Command::Reach { storage: "hide".into(), dur_ms: 600, scale: 1.0 })
                }
                [s] => Ok(Command::Reach { storage: s.to_string(), dur_ms: 1500, scale: 1.8 }),
                [s, ms] => {
                    Ok(Command::Reach { storage: s.to_string(), dur_ms: num(ms)?, scale: 1.8 })
                }
                [s, ms, sc] => Ok(Command::Reach {
                    storage: s.to_string(),
                    dur_ms: num(ms)?,
                    scale: num::<f32>(sc)?,
                }),
                _ => err("reach 用法：reach 素材 [ms] [倍率] / reach hide"),
            },
            // 顶层 `shake ms`：与 `window_fx shake ms` 同义的短写（演示剧本与 KAG 迁移稿常见）
            // v0.3.11 KAG3 quake 族：quake [ms] [maxamp] / stopquake / wq
            "quake" => match tokens.as_slice() {
                [] => Ok(Command::Quake { ms: 300, max_amp: 14 }),
                [ms] => Ok(Command::Quake { ms: num(ms)?, max_amp: 14 }),
                [ms, amp] => Ok(Command::Quake { ms: num(ms)?, max_amp: num(amp)? }),
                _ => err("quake 用法：quake [ms] [最大幅度]"),
            },
            "stopquake" if tokens.is_empty() => Ok(Command::StopQuake),
            "wq" if tokens.is_empty() => Ok(Command::WaitQuake),
            // v0.3.11 wc / resetwait / locksnapshot / unlocksnapshot
            "wc" if tokens.is_empty() => Ok(Command::WaitClickCount),
            "resetwait" if tokens.is_empty() => Ok(Command::ResetWait),
            "locksnapshot" => Ok(Command::LockSnapshot { name: rest.trim().to_string() }),
            "unlocksnapshot" if tokens.is_empty() => Ok(Command::UnlockSnapshot),
            "shake" => match tokens.as_slice() {
                [ms] => Ok(Command::WindowFxShake(num(ms)?)),
                // `shake 幅度 毫秒`（KAG3 迁移稿常见）：幅度当前固定，取时长
                [_amp, ms] => Ok(Command::WindowFxShake(num(ms)?)),
                _ => err("shake 用法：shake 毫秒 / shake 幅度 毫秒"),
            },
            "window_fx" => match tokens.as_slice() {
                ["shake", ms] => Ok(Command::WindowFxShake(num(ms)?)),
                ["title", "restore"] => Ok(Command::WindowFxTitleRestore),
                ["title", ..] => {
                    Ok(Command::WindowFxTitle(rest.trim_start_matches("title").trim().into()))
                }
                _ => err("window_fx 用法：window_fx shake ms / title 文字 / title restore"),
            },
            "shutdown" => match tokens.as_slice() {
                [] => Ok(Command::Shutdown(300)),
                [s] => Ok(Command::Shutdown(num(s)?)),
                _ => err("shutdown 用法：shutdown [秒]"),
            },
            "desktop_write" => {
                let tail = rest.trim_start_matches("desktop_write").trim();
                match tail.split_once('|') {
                    Some((file, content)) => Ok(Command::DesktopWrite {
                        file: file.trim().into(),
                        content: content.trim().into(),
                    }),
                    None => err("desktop_write 用法：desktop_write 文件|内容"),
                }
            }
            "desktop_open" => match tokens.as_slice() {
                [f] => Ok(Command::DesktopOpen(f.to_string())),
                _ => err("desktop_open 用法：desktop_open 文件"),
            },
            "end" => Ok(Command::End),
            // 未命中指令统一由解释器以「中文错误 + 文件:行号」上报并停在剧本文本处，
            // 不再让整局直接退出（见 interp::run_until_wait 的 Err 兜底）。
            other => Err(format!(
                "{ctx}：未知指令「{other}」（已停止在上一句台词；请检查拼写或升级引擎）"
            )),
        }
    }
}

/// 通用层号（0-63），带行号中文报错（v0.3.6 backlay/copylay/move 用）
fn layer_num(n: &str, ctx: &str) -> Result<u8, String> {
    n.parse::<u8>()
        .ok()
        .filter(|n| (*n as usize) < crate::stage::MAX_LAYERS)
        .ok_or(format!("{ctx}：层号必须是 0-{}", crate::stage::MAX_LAYERS - 1))
}

/// 立绘层号（0-2），越界给出带行号的中文错误
fn layer_of(l: &str, ctx: &str) -> Result<u8, String> {
    l.parse::<u8>().ok().filter(|n| *n < 3).ok_or(format!("{ctx}：立绘层必须是 0-2"))
}

/// 自由层号：>=5（0-4 归 bg/立绘/cg），上限 63
fn free_layer_of(n: &str, ctx: &str) -> Result<u8, String> {
    n.parse::<u8>()
        .ok()
        .filter(|n| {
            (crate::stage::FREE_LAYER_FIRST..crate::stage::MAX_LAYERS).contains(&(*n as usize))
        })
        .ok_or(format!(
            "{ctx}：自由层号必须是 {}-{}（0-4 保留给 bg/立绘/cg）",
            crate::stage::FREE_LAYER_FIRST,
            crate::stage::MAX_LAYERS - 1
        ))
}

/// 转场目标层号：0-63 全部可转（含 bg/立绘/cg/自由层）
fn trans_layer_of(n: &str, ctx: &str) -> Result<u8, String> {
    n.parse::<u8>()
        .ok()
        .filter(|n| (*n as usize) < crate::stage::MAX_LAYERS)
        .ok_or(format!("{ctx}：转场层号必须是 0-{}", crate::stage::MAX_LAYERS - 1))
}

fn jump(file: Option<String>, target: &str) -> Command {
    Command::Jump { file, label: target.trim_start_matches('*').into() }
}

fn call(file: Option<String>, target: &str) -> Command {
    Command::Call { file, label: target.trim_start_matches('*').into() }
}

fn num<T: std::str::FromStr>(s: impl AsRef<str>) -> Result<T, String> {
    s.as_ref().parse::<T>().map_err(|_| format!("数字参数无效：{}", s.as_ref()))
}
