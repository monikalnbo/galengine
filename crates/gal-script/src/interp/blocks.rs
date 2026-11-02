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
/// v0.3.4 if 行下标 → 有序 elsif 行下标链（含链尾 else/endif 的定位靠 Blocks/ElseTargets）
pub type ElsIfChains = HashMap<usize, Vec<usize>>;

pub fn pair(lines: &[Line]) -> Result<(Blocks, ElseTargets, ElsIfChains), String> {
    let mut blocks = Blocks::new();
    let mut else_targets = ElseTargets::new();
    let mut chains = ElsIfChains::new();
    // 栈元素：(if 行下标, 已见到的 elsif 下标链, 已见到的 else 下标)
    let mut stack: Vec<(usize, Vec<usize>, Option<usize>)> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let LineKind::Command { name, .. } = &line.kind else { continue };
        match name.as_str() {
            "if" if is_block_if(line) => stack.push((i, Vec::new(), None)),
            // v0.3.4 elsif：挂到栈顶 if 的链上（else 之后再出现 = 剧本写错）
            "elsif" => {
                let Some(top) = stack.last_mut() else {
                    return Err(format!("第 {} 行 elsif 没有配对的 if", line.no));
                };
                if top.2.is_some() {
                    return Err(format!("第 {} 行 elsif 出现在 else 之后", line.no));
                }
                top.1.push(i);
            }
            "else" => {
                let Some(top) = stack.last_mut() else {
                    return Err(format!("第 {} 行 else 没有配对的 if", line.no));
                };
                if top.2.is_some() {
                    return Err(format!("第 {} 行 else 重复（同一 if 只能有一个 else）", line.no));
                }
                top.2 = Some(i);
            }
            "endif" => {
                let (start, elsis, else_at) =
                    stack.pop().ok_or_else(|| format!("第 {} 行 endif 没有配对的 if", line.no))?;
                blocks.insert(start, (else_at, i));
                if !elsis.is_empty() {
                    chains.insert(start, elsis);
                }
                if let Some(e) = else_at {
                    else_targets.insert(e, i);
                }
            }
            _ => {}
        }
    }
    if let Some((start, _, _)) = stack.pop() {
        return Err(format!("第 {} 行 if 块缺少 endif", lines[start].no));
    }
    Ok((blocks, else_targets, chains))
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

    /// v0.3.4 endif 行 → 所属 if 行（active 分支栈弹栈用）
    pub(crate) fn if_owner_of_end(&self, file: &str, end_at: usize) -> Option<usize> {
        self.blocks.get(file)?.iter().find(|(_, (_, end))| *end == end_at).map(|(start, _)| *start)
    }

    /// v0.3.4 elsif/else 行 → 所属 if 行
    pub(crate) fn if_owner_of_branch(&self, file: &str, at: usize) -> Option<usize> {
        let blocks = self.blocks.get(file)?;
        if let Some((start, _)) = blocks.iter().find(|(_, (e, _))| *e == Some(at)) {
            return Some(*start);
        }
        self.elsif_chains
            .get(file)?
            .iter()
            .find(|(_, chain)| chain.contains(&at))
            .map(|(start, _)| *start)
    }

    /// v0.3.4 if 行 → 有序 elsif 链
    pub(crate) fn elsif_chain_of(&self, file: &str, if_at: usize) -> Option<&Vec<usize>> {
        self.elsif_chains.get(file).and_then(|m| m.get(&if_at))
    }

    /// v0.3.4 elsif/else 假分支的下一跳：链里下一个 elsif 之后；没有则
    /// else 之后（else_at 传 None 表示查不到 = endif 之后）
    pub(crate) fn next_branch_of(&self, file: &str, if_at: usize, after: usize) -> Option<usize> {
        let (else_at, end) = *self.blocks_of(file)?.get(&if_at)?;
        // 没有 elsif 的旧剧本：链为空 = 直接 else/endif 之后
        let chain = self.elsif_chain_of(file, if_at).map(|c| c.as_slice()).unwrap_or(&[]);
        match chain.iter().find(|&&e| e > after) {
            // 跳到 elsif 行本身：由它求值条件决定进分支还是继续找下一分支
            Some(&next) => Some(next),
            None => Some(match else_at {
                Some(e) => e + 1,
                None => end + 1,
            }),
        }
    }
}
