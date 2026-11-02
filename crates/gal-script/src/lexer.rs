//! 剧本词法：文本 -> 行序列。# 注释 / *label / 指令 + 参数 / choice…endchoice 块收集。
//! v0.3.5：macro…endmacro 宏定义块收集（宏体原样保留，展开在解释器运行期）；
//! ignore…endignore 解析期注释块整段跳过。

#[derive(Debug, Clone)]
pub enum LineKind {
    Label(String),
    Command {
        name: String,
        rest: String,
    },
    Choice {
        prompt: String,
        items: Vec<(String, String)>,
    },
    /// v0.3.5 宏定义（macro…endmacro 收集结果）：宏名 + 宏体行。
    /// 解释器 load 时注册进宏表；执行到本行时跳过。
    MacroDef {
        name: String,
        body: Vec<Line>,
    },
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
    // v0.3.5 宏定义暂存：宏名 + 起始行号 + 宏体行
    let mut macro_pending: Option<(String, usize, Vec<Line>)> = None;
    // v0.3.5 ignore…endignore：解析期注释块（真=正在跳过）
    let mut ignoring = false;

    for (i, raw) in src.lines().enumerate() {
        let no = i + 1;
        let t = raw.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if ignoring {
            // 注释块只有 endignore 能出去
            if first_word(t) == "endignore" {
                ignoring = false;
            }
            continue;
        }
        // 宏体收集：除 endmacro 外原样保留（嵌套 choice 不允许——宏体行不做 choice 收集）
        if let Some((_name, _no0, body)) = macro_pending.as_mut() {
            if first_word(t) == "endmacro" {
                let (name, no0, body) = macro_pending.take().unwrap();
                if body.is_empty() {
                    return Err(format!("第 {no0} 行宏「{name}」没有宏体"));
                }
                out.push(Line { no: no0, kind: LineKind::MacroDef { name, body } });
            } else if first_word(t) == "macro" {
                return Err(format!(
                    "第 {no} 行 macro 不能嵌套在宏定义里（先 endmacro 再定义下一个）"
                ));
            } else {
                let (name2, rest2) = split_cmd(t);
                body.push(Line { no, kind: LineKind::Command { name: name2, rest: rest2 } });
            }
            continue;
        }
        if let Some((_no0, _prompt, items)) = pending.as_mut() {
            if t == "endchoice" {
                let (no0, prompt, items) = pending.take().unwrap();
                if items.is_empty() {
                    return Err(format!("第 {no0} 行 choice 没有任何选项"));
                }
                out.push(Line { no: no0, kind: LineKind::Choice { prompt, items } });
            } else {
                // 从右取 `|*label`：选项文本里可以含 `{@键|兜底}` 的竖线
                match t.rsplit_once('|') {
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
        if let Some(label) = t.strip_prefix('*') {
            out.push(Line { no, kind: LineKind::Label(label.trim().to_string()) });
            continue;
        }
        let (name, rest) = split_cmd(t);
        if name == "choice" {
            pending = Some((no, rest, Vec::new()));
        } else if name == "macro" {
            // v0.3.5 宏定义开始：`macro 名` 或 `macro 名="值"`（KAG3 语法，默认值随属性表走）
            let macro_name = rest.split_whitespace().next().unwrap_or("");
            let macro_name = macro_name.split('=').next().unwrap_or("");
            if macro_name.is_empty() {
                return Err(format!("第 {no} 行 macro 缺少宏名"));
            }
            macro_pending = Some((macro_name.to_string(), no, Vec::new()));
        } else if name == "ignore" {
            // v0.3.5 解析期注释块：到 endignore 前整段跳过
            ignoring = true;
        } else {
            out.push(Line { no, kind: LineKind::Command { name, rest } });
        }
    }
    match pending {
        Some(_) => Err("choice 块缺少 endchoice".into()),
        None => match macro_pending {
            Some((name, no0, _)) => Err(format!("第 {no0} 行宏「{name}」缺少 endmacro")),
            None => match ignoring {
                true => Err("ignore 块缺少 endignore".into()),
                false => Ok(out),
            },
        },
    }
}

/// 行首词（指令名）
fn first_word(t: &str) -> &str {
    t.split_whitespace().next().unwrap_or(t)
}

/// 指令行拆分：首词 + 其余
fn split_cmd(t: &str) -> (String, String) {
    match t.split_once(' ') {
        Some((n, r)) => (n.to_string(), r.trim().to_string()),
        None => (t.to_string(), String::new()),
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
    fn choice块内含空行与注释() {
        let src = "choice 选哪个？\n\n  # 注释A\n  A|*a\n\n  # 注释B\n  B|*b\n\nendchoice";
        let ls = parse_script(src).unwrap();
        match &ls[0].kind {
            LineKind::Choice { prompt, items } => {
                assert_eq!(prompt, "选哪个？");
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].0, "A");
                assert_eq!(items[1].0, "B");
            }
            _ => panic!(),
        }
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

    #[test]
    fn v035_宏定义块收集() {
        let src = "macro greet\neval f.n = 1\nn 你好 %name\ngreet 3\nendmacro\nmacro k=\"x\"\nn 值\nendmacro";
        let ls = parse_script(src).unwrap();
        assert_eq!(ls.len(), 2);
        match &ls[0].kind {
            LineKind::MacroDef { name, body } => {
                assert_eq!(name, "greet");
                assert_eq!(body.len(), 3);
                assert_eq!(body[0].no, 2);
                match &body[2].kind {
                    LineKind::Command { name, rest } => {
                        assert_eq!(name, "greet");
                        assert_eq!(rest, "3");
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
        match &ls[1].kind {
            LineKind::MacroDef { name, .. } => assert_eq!(name, "k"),
            _ => panic!(),
        }
        assert_eq!(ls[1].no, 6);
    }

    #[test]
    fn v035_ignore块跳过与错误() {
        let src = "n 前\nignore\nn 被跳过\nendignore\nn 后";
        let ls = parse_script(src).unwrap();
        assert_eq!(ls.len(), 2);
        match &ls[0].kind {
            LineKind::Command { name, rest } => {
                assert_eq!(name, "n");
                assert_eq!(rest, "前");
            }
            _ => panic!(),
        }
        assert!(parse_script("ignore\nn x").is_err());
        assert!(parse_script("macro a\nmacro b\nendmacro").is_err());
    }
}
