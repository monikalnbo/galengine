//! 全 26 条指令的枚举与解析（一处看全所有语法）。ctx 为「文件:行号」中文错误定位。

use crate::lexer::LineKind;

#[derive(Debug, Clone)]
pub enum Command {
    Bg { storage: String, fade_ms: u32 },
    Bgm(String),
    BgmStop,
    BgmFadeOut { ms: u32 },
    Se(String),
    Char { layer: u8, storage: String, x: Option<i32>, y: Option<i32> },
    CharPos { layer: u8, storage: String, pos_name: String },
    Cg { storage: String, fade_ms: u32 },
    Clear,
    Wait(u32),
    N(String),
    Name { name: String, text: String },
    Flag { var: String, op: char, val: i64 },
    Set { var: String, expr: String },
    Jump { file: Option<String>, label: String },
    If { var: String, op: String, val: String, target: String },
    Choice { prompt: String, items: Vec<(String, String)> },
    Input { var: String, prompt: String, width: u32, default: String },
    MetaFakeSave { date: String, time: String, image: String },
    MetaCorrupt(String),
    MetaDeleteLast,
    TitleEvolve(String),
    Reach { storage: String, dur_ms: u32, scale: f32 },
    WindowFxShake(u32),
    WindowFxTitle(String),
    WindowFxTitleRestore,
    Shutdown(u32),
    DesktopWrite { file: String, content: String },
    DesktopOpen(String),
    End,
}

impl Command {
    pub fn parse(kind: &LineKind, ctx: &str) -> Result<Command, String> {
        let (name, rest) = match kind {
            LineKind::Command { name, rest } => (name.as_str(), rest.as_str()),
            LineKind::Choice { prompt, items } => {
                return Ok(Command::Choice { prompt: prompt.clone(), items: items.clone() })
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
                [s] if s == &"stop" => Ok(Command::BgmStop),
                [s, ms] if s == &"fadeout" => Ok(Command::BgmFadeOut { ms: num(ms)? }),
                [s] => Ok(Command::Bgm(s.to_string())),
                _ => err("bgm 用法：bgm 名称 / bgm stop / bgm fadeout ms"),
            },
            "se" => match tokens.as_slice() {
                [s] => Ok(Command::Se(s.to_string())),
                _ => err("se 用法：se 名称"),
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
                [l, s, x, y] => {
                    let lx = l
                        .parse::<u8>()
                        .ok()
                        .filter(|n| *n < 3)
                        .ok_or(format!("{ctx}：立绘层必须是 0-2"))?;
                    // 尝试解析数字，否则当作预设位置名
                    let (ox, oy) = match (x.parse::<i32>(), y.parse::<i32>()) {
                        (Ok(nx), Ok(ny)) => (Some(nx), Some(ny)),
                        _ => {
                            // 预设名（需要访问 config，暂存字符串，interp 时解析）
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
            "cg" => match tokens.as_slice() {
                [s, ms] if s != &"hide" => {
                    Ok(Command::Cg { storage: s.to_string(), fade_ms: num(ms)? })
                }
                [s] | [s, _] => Ok(Command::Cg { storage: s.to_string(), fade_ms: 700 }),
                _ => err("cg 用法：cg 素材 [淡入ms] / cg hide [ms]"),
            },
            "clear" if tokens.is_empty() => Ok(Command::Clear),
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
                [v, op, val, t] if ["==", "!=", ">=", "<=", ">", "<"].contains(op) => {
                    Ok(Command::If {
                        var: v.to_string(),
                        op: op.to_string(),
                        val: val.to_string(),
                        target: t.trim_start_matches('*').to_string(),
                    })
                }
                _ => err("if 用法：if 变量 == 值 *label（支持 == != >= <= > <）"),
            },
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
            "end" if tokens.is_empty() => Ok(Command::End),
            other => Err(format!("{ctx}：未知指令「{other}」")),
        }
    }
}

fn jump(file: Option<String>, target: &str) -> Command {
    Command::Jump { file, label: target.trim_start_matches('*').into() }
}

fn num<T: std::str::FromStr>(s: impl AsRef<str>) -> Result<T, String> {
    s.as_ref().parse::<T>().map_err(|_| format!("数字参数无效：{}", s.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(name: &str, rest: &str) -> Result<Command, String> {
        Command::parse(&LineKind::Command { name: name.into(), rest: rest.into() }, "t.ks:1")
    }

    #[test]
    fn 基本指令解析() {
        assert!(
            matches!(cmd("bg", "a 800").unwrap(), Command::Bg { storage, fade_ms: 800 } if storage == "a")
        );
        assert!(
            matches!(cmd("char", "0 hide").unwrap(), Command::Char { layer: 0, ref storage, x: None, y: None } if storage == "hide")
        );
        assert!(cmd("char", "3 x").is_err());
        assert!(
            matches!(cmd("flag", "a +2").unwrap(), Command::Flag { var, op: '+', val: 2 } if var == "a")
        );
        assert!(matches!(cmd("flag", "a -1").unwrap(), Command::Flag { op: '-', val: 1, .. }));
        assert!(
            matches!(cmd("reach", "hide").unwrap(), Command::Reach { ref storage, .. } if storage == "hide")
        );
        assert!(matches!(
            cmd("window_fx", "title restore").unwrap(),
            Command::WindowFxTitleRestore
        ));
        assert!(matches!(cmd("shutdown", "").unwrap(), Command::Shutdown(300)));
    }

    #[test]
    fn set表达式与desktop内容含空格() {
        match cmd("set", "a = 1 + 2").unwrap() {
            Command::Set { var, expr } => {
                assert_eq!(var, "a");
                assert_eq!(expr, "1 + 2");
            }
            _ => panic!(),
        }
        match cmd("desktop_write", "letter.txt|还记得吗。{hero}").unwrap() {
            Command::DesktopWrite { file, content } => {
                assert_eq!(file, "letter.txt");
                assert_eq!(content, "还记得吗。{hero}");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn 未知指令中文报错() {
        assert_eq!(cmd("foo", "").unwrap_err(), "t.ks:1：未知指令「foo」");
    }
}
