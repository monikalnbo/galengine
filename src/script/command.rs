//! 指令集：全量 26 条指令的枚举与解析（一处看全所有语法）。
//! 解释器按里程碑逐步实现（A3 最小集 / A5 分歧输入 / A6 meta）。

use crate::script::lexer::LineKind;

#[derive(Debug, Clone)]
pub enum Command {
    /// bg 素材 [淡入ms]
    Bg { storage: String, fade_ms: u32 },
    /// bgm 名称（循环）/ se 名称（一次）——接口预留，缺文件静默跳过
    Bgm(String),
    Se(String),
    /// char 层 素材|hide
    Char { layer: u8, storage: String },
    /// cg 素材 [淡入ms] / cg hide [ms]
    Cg { storage: String, fade_ms: u32 },
    /// 清空对话框
    Clear,
    /// 强制等待 ms（不可点击跳过）
    Wait(u32),
    /// 旁白
    N(String),
    /// 对话：名字 + 台词
    Name { name: String, text: String },
    /// flag 变量 +2（+=/-=）
    Flag { var: String, op: char, val: i64 },
    /// set 变量 = 表达式
    Set { var: String, expr: String },
    /// jump *label / jump 文件 *label
    Jump { file: Option<String>, label: String },
    /// if 条件 *target（== != >= <= > <）
    If { var: String, op: String, val: String, target: String },
    /// choice 提示 + 选项（lexer 已收集；prompt 保留给选项界面的标题显示，暂未消费）
    Choice {
        #[allow(dead_code)]
        prompt: String,
        items: Vec<(String, String)>,
    },
    /// input 变量 提示|宽度|默认值
    Input { var: String, prompt: String, width: u32, default: String },
    // ---- Meta API（A6）----
    MetaFakeSave { date: String, time: String, image: String },
    MetaCorrupt(String),
    MetaDeleteLast,
    TitleEvolve(String),
    // ---- 打破第四面墙（§3.9）----
    Reach { storage: String, dur_ms: u32, scale: f32 },
    WindowFxShake(u32),
    WindowFxTitle(String),
    /// shutdown [秒]
    Shutdown(u32),
    /// 桌面创建/追加文本文件（meta 演出）：desktop_write 文件|内容
    DesktopWrite { file: String, content: String },
    /// 打开桌面文件：desktop_open 文件
    DesktopOpen(String),
    /// 收回越界层
    ReachHide,
    /// 恢复窗口标题
    WindowFxTitleRestore,
    End,
}

impl Command {
    /// 解析一行（ctx 用于中文错误定位：文件:行号）
    pub fn parse(kind: &LineKind, ctx: &str) -> Result<Command, String> {
        let (name, rest) = match kind {
            LineKind::Command { name, rest } => (name.as_str(), rest.as_str()),
            LineKind::Choice { prompt, items } => {
                return Ok(Command::Choice {
                    prompt: prompt.clone(),
                    items: items.clone(),
                })
            }
            LineKind::Label(_) => return Err(format!("{ctx}：label 不应作为指令执行")),
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
                [s] => Ok(Command::Bgm(s.to_string())),
                _ => err("bgm 用法：bgm 名称"),
            },
            "se" => match tokens.as_slice() {
                [s] => Ok(Command::Se(s.to_string())),
                _ => err("se 用法：se 名称"),
            },
            "char" => match tokens.as_slice() {
                [l, s] => Ok(Command::Char { layer: layer(l, ctx)?, storage: s.to_string() }),
                _ => err("char 用法：char 层(0-2) 素材|hide"),
            },
            "cg" => match tokens.as_slice() {
                [s] if s == &"hide" => Ok(Command::Cg { storage: "hide".into(), fade_ms: 700 }),
                [s] => Ok(Command::Cg { storage: s.to_string(), fade_ms: 700 }),
                [s, ms] => Ok(Command::Cg { storage: s.to_string(), fade_ms: num(ms)? }),
                _ => err("cg 用法：cg 素材 [淡入ms] / cg hide [ms]"),
            },
            "clear" if tokens.is_empty() => Ok(Command::Clear),
            "wait" => match tokens.as_slice() {
                [ms] => Ok(Command::Wait(num(ms)?)),
                _ => err("wait 用法：wait 毫秒"),
            },
            "n" => Ok(Command::N(rest.to_string())),
            "name" => {
                let (n, text) = rest
                    .split_once(' ')
                    .ok_or_else(|| format!("{ctx}：name 用法：name 名字 台词"))?;
                Ok(Command::Name { name: n.to_string(), text: text.trim().to_string() })
            }
            "flag" => {
                let (var, op, val) = match tokens.as_slice() {
                    [v, op, val] if *op == "+" || *op == "-" => {
                        (v.to_string(), op.to_string(), val.to_string())
                    }
                    [v, oval] if oval.starts_with('+') || oval.starts_with('-') => (
                        v.to_string(),
                        oval[..1].to_string(),
                        oval[1..].to_string(),
                    ),
                    _ => return err("flag 用法：flag 变量 +2 / -1"),
                };
                Ok(Command::Flag {
                    var,
                    op: op.chars().next().unwrap(),
                    val: num::<i64>(&val)?,
                })
            }
            "set" => {
                // set var = 表达式（表达式可含空格）
                let (v, after) = rest
                    .split_once('=')
                    .ok_or_else(|| format!("{ctx}：set 用法：set 变量 = 表达式"))?;
                let expr = after.trim().trim_start_matches('=').trim().to_string();
                if expr.is_empty() {
                    return err("set 表达式为空");
                }
                Ok(Command::Set { var: v.trim().to_string(), expr })
            }
            "jump" => match tokens.as_slice() {
                [t] => Ok(jump(None, t)),
                [f, t] => Ok(jump(Some(f.to_string()), t)),
                _ => err("jump 用法：jump *label / jump 文件 *label"),
            },
            "if" => match tokens.as_slice() {
                [v, op, val, t] if ["==", "!=", ">=", "<=", ">", "<"].contains(op) => {
                    Ok(Command::If {
                        var: v.to_string(),
                        op: op.to_string(),
                        val: val.to_string(),
                        target: t.to_string(),
                    })
                }
                _ => err("if 用法：if f.变量 == 值 *label（支持 == != >= <= > <）"),
            },
            "input" => {
                // input 变量 提示|宽度|默认
                let (v, tail) = rest
                    .split_once(' ')
                    .ok_or_else(|| format!("{ctx}：input 用法：input 变量 提示|宽度|默认值"))?;
                let parts: Vec<&str> = tail.split('|').collect();
                match parts.as_slice() {
                    [prompt, w, def] => Ok(Command::Input {
                        var: v.to_string(),
                        prompt: prompt.trim().to_string(),
                        width: num(w)?,
                        default: def.trim().to_string(),
                    }),
                    [prompt, w] => Ok(Command::Input {
                        var: v.to_string(),
                        prompt: prompt.trim().to_string(),
                        width: num(w)?,
                        default: String::new(),
                    }),
                    _ => err("input 用法：input 变量 提示|宽度|默认值"),
                }
            }
            "meta_fake_save" => {
                let parts: Vec<&str> = rest.split('|').collect();
                match parts.as_slice() {
                    [d, t, img] => Ok(Command::MetaFakeSave {
                        date: d.trim().to_string(),
                        time: t.trim().to_string(),
                        image: img.trim().to_string(),
                    }),
                    _ => err("meta_fake_save 用法：meta_fake_save 日期|时间|图片"),
                }
            }
            "meta_corrupt" => match tokens.as_slice() {
                [n] => Ok(Command::MetaCorrupt(n.to_string())),
                _ => err("meta_corrupt 用法：meta_corrupt N|all"),
            },
            "meta_delete_last" if tokens.is_empty() => Ok(Command::MetaDeleteLast),
            "title_evolve" => match tokens.as_slice() {
                [mode] => Ok(Command::TitleEvolve(mode.to_string())),
                _ => err("title_evolve 用法：title_evolve check"),
            },
            "reach" => match tokens.as_slice() {
                ["hide"] => Ok(Command::ReachHide),
                [s] => Ok(Command::Reach { storage: s.to_string(), dur_ms: 1500, scale: 1.8 }),
                [s, ms] => Ok(Command::Reach { storage: s.to_string(), dur_ms: num(ms)?, scale: 1.8 }),
                [s, ms, sc] => Ok(Command::Reach {
                    storage: s.to_string(),
                    dur_ms: num(ms)?,
                    scale: num::<f32>(sc)?,
                }),
                _ => err("reach 用法：reach 素材 [ms] [倍率]"),
            },
            "window_fx" => match tokens.as_slice() {
                ["title", "restore"] | ["title", "default"] => Ok(Command::WindowFxTitleRestore),
                ["shake", ms] => Ok(Command::WindowFxShake(num(ms)?)),
                ["title", ..] => Ok(Command::WindowFxTitle(rest.trim_start_matches("title").trim().to_string())),
                _ => err("window_fx 用法：window_fx shake ms / window_fx title 文字"),
            },
            "shutdown" => match tokens.as_slice() {
                [] => Ok(Command::Shutdown(300)),
                [s] => Ok(Command::Shutdown(num(s)?)),
                _ => err("shutdown 用法：shutdown [秒]"),
            },
            "desktop_write" => {
                let (f, c) = rest
                    .split_once('|')
                    .ok_or_else(|| format!("{ctx}：desktop_write 用法：desktop_write 文件名|内容"))?;
                Ok(Command::DesktopWrite {
                    file: f.trim().to_string(),
                    content: c.to_string(),
                })
            }
            "desktop_open" => match tokens.as_slice() {
                [f] => Ok(Command::DesktopOpen(f.to_string())),
                _ => err("desktop_open 用法：desktop_open 文件名"),
            },
            "end" if tokens.is_empty() => Ok(Command::End),
            other => Err(format!("{ctx}：未知指令「{other}」")),
        }
    }
}

fn jump(file: Option<String>, target: &str) -> Command {
    let label = target.trim_start_matches('*').to_string();
    Command::Jump { file, label }
}

fn layer(s: &str, ctx: &str) -> Result<u8, String> {
    s.parse::<u8>().ok().filter(|n| *n < 3).ok_or_else(|| format!("{ctx}：立绘层必须是 0-2"))
}

fn num<T: std::str::FromStr>(s: &str) -> Result<T, String> {
    s.parse::<T>().map_err(|_| format!("数字参数无效：{s}"))
}
