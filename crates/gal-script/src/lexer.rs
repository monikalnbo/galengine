//! 剧本词法：文本 -> 行序列。# 注释 / *label / 指令 + 参数 / choice…endchoice 块收集。

#[derive(Debug, Clone)]
pub enum LineKind {
    Label(String),
    Command { name: String, rest: String },
    Choice { prompt: String, items: Vec<(String, String)> },
}

#[derive(Debug, Clone)]
pub struct Line {
    /// 1 起始行号（中文错误定位用）
    pub no: usize,
    pub kind: LineKind,
}

/// 跨行合并暂存：起始行号 + 合并文本 + 参数对
pub type PendingLine = (usize, String, Vec<(String, String)>);

pub fn parse_script(src: &str) -> Result<Vec<Line>, String> {
    let mut out = Vec::new();
    let mut pending: Option<PendingLine> = None;

    for (i, raw) in src.lines().enumerate() {
        let no = i + 1;
        let t = raw.trim();
        if let Some((_no0, _prompt, items)) = pending.as_mut() {
            if t == "endchoice" {
                let (no0, prompt, items) = pending.take().unwrap();
                if items.is_empty() {
                    return Err(format!("第 {no0} 行 choice 没有任何选项"));
                }
                out.push(Line { no: no0, kind: LineKind::Choice { prompt, items } });
            } else {
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
            continue;
        }
        if let Some(label) = t.strip_prefix('*') {
            out.push(Line { no, kind: LineKind::Label(label.trim().to_string()) });
            continue;
        }
        let (name, rest) = match t.split_once(' ') {
            Some((n, r)) => (n.to_string(), r.trim().to_string()),
            None => (t.to_string(), String::new()),
        };
        if name == "choice" {
            pending = Some((no, rest, Vec::new()));
        } else {
            out.push(Line { no, kind: LineKind::Command { name, rest } });
        }
    }
    match pending {
        Some(_) => Err("choice 块缺少 endchoice".into()),
        None => Ok(out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 注释空行与label() {
        let ls = parse_script("# 注\n\n*top\nend").unwrap();
        assert!(matches!(ls[0].kind, LineKind::Label(ref l) if l == "top"));
        assert_eq!(ls[0].no, 3);
    }

    #[test]
    fn choice块收集与错误() {
        let src = "choice 选哪个？\n  A|*a\n  B|*b\nendchoice";
        let ls = parse_script(src).unwrap();
        match &ls[0].kind {
            LineKind::Choice { prompt, items } => {
                assert_eq!(prompt, "选哪个？");
                assert_eq!(items.len(), 2);
                assert_eq!(items[1].1, "*b");
            }
            _ => panic!(),
        }
        assert!(parse_script("choice 提示\n A|*a").is_err());
        assert!(parse_script("choice 提示\nendchoice").is_err());
    }

    #[test]
    fn 指令行拆分() {
        let ls = parse_script("bg bg_room 800\nname 阿岚 台词内容 保留空格").unwrap();
        match &ls[0].kind {
            LineKind::Command { name, rest } => {
                assert_eq!(name, "bg");
                assert_eq!(rest, "bg_room 800");
            }
            _ => panic!(),
        }
    }
}
