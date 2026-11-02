//! `if … [else] … endif` 块配对（P0-4 地基）。
//!
//! 词法层不认缩进，块范围必须在剧本加载后一次性算出来：
//! 每个 `if`（块形态）记录「else 下标」和「配对 endif 下标」，
//! 每个 `else` 记录「配对 endif 下标」。解释器执行时按行下标跳转。
//!
//! 嵌套靠栈：内层先配对，外层等到自己的 endif 才落表。

use std::collections::HashMap;

use super::Interp;
use crate::lexer::{Line, LineKind};

/// 行下标 → (else 下标, endif 下标)。`else` 为 None 表示没有 else 分支。
pub type Blocks = HashMap<usize, (Option<usize>, usize)>;
/// else 行下标 → endif 行下标
pub type ElseTargets = HashMap<usize, usize>;

pub fn pair(lines: &[Line]) -> Result<(Blocks, ElseTargets), String> {
    let mut blocks = Blocks::new();
    let mut else_targets = ElseTargets::new();
    // 栈元素：(if 行下标, 已见到的 else 下标)
    let mut stack: Vec<(usize, Option<usize>)> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let LineKind::Command { name, .. } = &line.kind else { continue };
        match name.as_str() {
            "if" if is_block_if(line) => stack.push((i, None)),
            "else" => {
                let Some(top) = stack.last_mut() else {
                    return Err(format!("第 {} 行 else 没有配对的 if", line.no));
                };
                if top.1.is_some() {
                    return Err(format!("第 {} 行 else 重复（同一 if 只能有一个 else）", line.no));
                }
                top.1 = Some(i);
            }
            "endif" => {
                let (start, else_at) =
                    stack.pop().ok_or_else(|| format!("第 {} 行 endif 没有配对的 if", line.no))?;
                blocks.insert(start, (else_at, i));
                if let Some(e) = else_at {
                    else_targets.insert(e, i);
                }
            }
            _ => {}
        }
    }
    if let Some((start, _)) = stack.pop() {
        return Err(format!("第 {} 行 if 块缺少 endif", lines[start].no));
    }
    Ok((blocks, else_targets))
}

/// 块形态的 if：参数里没有 `*label`（单行形态会带）
fn is_block_if(line: &Line) -> bool {
    let LineKind::Command { rest, .. } = &line.kind else { return false };
    !rest.split_whitespace().any(|t| t.starts_with('*'))
}

impl Interp {
    /// 块跳转表（加载时算好）
    pub(crate) fn blocks_of(&self, file: &str) -> Option<&Blocks> {
        self.blocks.get(file)
    }

    /// else 行 → 配对 endif 行
    pub(crate) fn else_target_of(&self, file: &str, else_at: usize) -> Option<usize> {
        self.else_targets.get(file).and_then(|m| m.get(&else_at)).copied()
    }
}
