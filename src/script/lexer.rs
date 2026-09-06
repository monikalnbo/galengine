//! 剧本词法：文本文件 -> 行序列（注释/label/指令/choice 块收集）。
//! 语法（docs/20 第四节）：
//!   # 注释      *label      指令 参数...      选项体「显示|*目标」
//!   choice 与 endchoice 之间的缩进行收集为选项体。

#[derive(Debug, Clone)]
pub enum LineKind {
    Label(String),
    Command {
        name: String,
        /// 原始剩余参数（各指令自行切分）
        rest: String,
    },
    Choice {
        prompt: String,
        items: Vec<(String, String)>, // (显示文本, *目标label)
    },
}

#[derive(Debug, Clone)]
pub struct Line {
    /// 1 起始行号（错误报告用）
    pub no: usize,
    pub kind: LineKind,
}

/// 解析整个剧本文件
pub fn parse_script(src: &str) -> Result<Vec<Line>, String> {
    let mut out = Vec::new();
    let mut pending_choice: Option<(usize, String, Vec<(String, String)>)> = None;

    for (i, raw) in src.lines().enumerate() {
        let no = i + 1;
        let line = raw.trim_end();
        let t = line.trim();

        if pending_choice.is_some() {
            if t == "endchoice" {
                let (no0, prompt, items) = pending_choice.take().unwrap();
                if items.is_empty() {
                    return Err(format!("第 {no0} 行 choice 没有任何选项"));
                }
                out.push(Line {
                    no: no0,
                    kind: LineKind::Choice { prompt, items },
                });
            } else if let Some(rest) = t.strip_prefix("endchoice") {
                return Err(format!("第 {no} 行 endchoice 后有多余内容"));
            } else if let Some((_, _, items)) = pending_choice.as_mut() {
                match t.split_once('|') {
                    Some((text, target)) if target.starts_with('*') => {
                        items.push((text.trim().to_string(), target.trim().to_string()));
                    }
                    _ => {
                        return Err(format!(
                            "第 {no} 行选项格式应为「显示文本|*目标label」，实际：{t}"
                        ))
                    }
                }
            }
            continue;
        }

        if t.is_empty() || t.starts_with('#') {
            continue; // 空行/注释
        }
        if let Some(label) = t.strip_prefix('*') {
            out.push(Line {
                no,
                kind: LineKind::Label(label.trim().to_string()),
            });
            continue;
        }
        // 指令行：首 token + 剩余
        let (name, rest) = match t.split_once(' ') {
            Some((n, r)) => (n.to_string(), r.trim().to_string()),
            None => (t.to_string(), String::new()),
        };
        if name == "choice" {
            pending_choice = Some((no, rest, Vec::new()));
            continue;
        }
        out.push(Line {
            no,
            kind: LineKind::Command { name, rest },
        });
    }
    if pending_choice.is_some() {
        return Err("choice 块缺少 endchoice".into());
    }
    Ok(out)
}
