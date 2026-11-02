//! v0.3.5 宏展开：调用处（未知指令名命中宏表）→ 解析属性 → 宏体行 %代入 → 压帧。
//!
//! KAG3 语义（Macro.html 对齐子集）：
//! - 定义 `macro 名` … `endmacro`；调用即「宏名 [属性…]」
//! - 属性 `key=value` / `key="值"` / `key='值'`；布尔属性（无 =）按空串
//! - 宏体内 `%key` 代入实参；未传按空串 + DEBUG 留证（不崩演出）
//! - 展开是文本层代入（词法期），不是运行期变量——与 KAG3 一致
//! - 递归防护：MAX_MACRO_DEPTH（宏 A 展开里又调宏 A…）

use super::{Interp, MacroFrame, MAX_MACRO_DEPTH};
use crate::lexer::Line;

/// 调用处属性解析：`rest` 拆成 (key, value) 列表。
/// 语法：`key=value` / `key="值 里 有 空格"` / `key='单引'` / 裸词（布尔属性，值空串）。
fn parse_attrs(rest: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let kstart = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' {
            i += 1;
        }
        let key = rest[kstart..i].to_string();
        if key.is_empty() {
            i += 1;
            continue;
        }
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            let (v, next) = if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let q = bytes[i];
                let vstart = i + 1;
                let vend =
                    rest[vstart..].find(q as char).map(|p| vstart + p).unwrap_or(bytes.len());
                (&rest[vstart..vend], (vend + 1).min(bytes.len()))
            } else {
                let vstart = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                (&rest[vstart..i], i)
            };
            out.push((key, v.to_string()));
            i = next;
        } else {
            out.push((key, String::new()));
        }
    }
    out
}

/// 宏体行文本 %代入：`%key` → 属性值（最长键优先，避免 %hp 与 %hpmax 混淆）；
/// 未命中的 `%` 按原样保留（与 trace 同口径——代入层绝不崩演出）。
/// `%%` 转义为字面 `%`。
fn substitute(body: &str, attrs: &[(String, String)]) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(p) = rest.find('%') {
        out.push_str(&rest[..p]);
        let tail = &rest[p + 1..];
        if let Some(stripped) = tail.strip_prefix('%') {
            out.push('%');
            rest = stripped;
            continue;
        }
        // 候选键：以 tail 开头的属性键里取最长
        let hit = attrs
            .iter()
            .filter(|(k, _)| !k.is_empty() && tail.starts_with(k.as_str()))
            .max_by_key(|(k, _)| k.len());
        match hit {
            Some((k, v)) => {
                out.push_str(v);
                rest = &tail[k.len()..];
            }
            None => {
                out.push('%');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

impl Interp {
    /// 尝试把一行未知指令按宏调用展开：命中 → 压帧返回 Ok(true)；
    /// 名字不在宏表 → Ok(false)（调用方回退原错误）。
    pub(crate) fn try_expand_macro(&mut self, line: &Line, ctx: &str) -> Result<bool, String> {
        let crate::lexer::LineKind::Command { name, rest } = &line.kind else {
            return Ok(false);
        };
        let Some(body) = self.macros.get(name).cloned() else {
            return Ok(false);
        };
        if self.macro_stack.len() >= MAX_MACRO_DEPTH {
            return Err(format!(
                "{ctx}：宏展开嵌套超过 {MAX_MACRO_DEPTH} 层（宏「{name}」，疑似递归宏）"
            ));
        }
        let attrs = parse_attrs(rest);
        // DEBUG 留证：宏调用 + 未传入参键
        gal_audio::log::log(
            gal_audio::log::Level::Debug,
            "script",
            &format!("[macro] 展开「{name}」（{} 个属性）", attrs.len()),
        );
        // 宏体内引用了未传属性：DEBUG 留证（按空串代入，KAG3 缺省口径）
        let body_src: Vec<String> = body
            .iter()
            .map(|l| match &l.kind {
                crate::lexer::LineKind::Command { name: n, rest: r } => {
                    format!("{n} {r}")
                }
                crate::lexer::LineKind::Label(lb) => format!("*{lb}"),
                _ => String::new(),
            })
            .collect();
        for src in &body_src {
            for k in attrs_missing(src, &attrs) {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    &format!("[macro] 「{name}」属性 %{k} 未传，按空串代入"),
                );
            }
        }
        // KAG3 缺省口径：body 引用了但未传的属性按空串代入（留证已在上面打点）
        let mut attrs = attrs;
        for src in &body_src {
            for k in attrs_missing(src, &attrs) {
                attrs.push((k, String::new()));
            }
        }
        // 逐行代入：文本层展开（指令名同样参与，宏可展开成任意指令名/嵌套宏调用）
        let body = body
            .into_iter()
            .zip(body_src)
            .map(|(mut l, src)| {
                let subbed = substitute(&src, &attrs);
                if let crate::lexer::LineKind::Command { name: n, rest: r } = &mut l.kind {
                    // 台词行（n/name 指令）：首词是指令名本身，代入结果整体进台词
                    if n == "n" || n == "name" {
                        *r = subbed.split_once(' ').map(|(_, t)| t.to_string()).unwrap_or_default();
                    } else {
                        let subbed = subbed.trim().to_string();
                        match subbed.split_once(' ') {
                            Some((n2, r2)) => {
                                *n = n2.to_string();
                                *r = r2.trim().to_string();
                            }
                            None => {
                                *n = subbed;
                                *r = String::new();
                            }
                        }
                    }
                }
                l
            })
            .collect();
        self.macro_scope_seq += 1;
        let scope = self.macro_scope_seq;
        self.macro_stack.push(MacroFrame {
            name: name.clone(),
            body,
            pc: 0,
            scope,
            call_pc: self.pc - 1,
        });
        Ok(true)
    }
}

/// 找出 body 行里引用了、但属性表里没有的 %键（只扫标识符首段，DEBUG 提示用）
fn attrs_missing(line_src: &str, attrs: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line_src;
    while let Some(p) = rest.find('%') {
        let tail = &rest[p + 1..];
        if tail.starts_with('%') {
            rest = tail;
            continue;
        }
        // 标识符段：字母数字下划线（中文属性名也认）
        let key: String = tail.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        let klen = key.len();
        if !key.is_empty() && !attrs.iter().any(|(k, _)| *k == key) {
            out.push(key);
        }
        rest = &tail[klen..];
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 属性解析全形态() {
        let a = parse_attrs("name=夏 hp=100 quote=\"空格 里\" flag");
        assert_eq!(a.len(), 4);
        assert_eq!(a[0], ("name".into(), "夏".into()));
        assert_eq!(a[1], ("hp".into(), "100".into()));
        assert_eq!(a[2], ("quote".into(), "空格 里".into()));
        assert_eq!(a[3], ("flag".into(), "".into()));
    }

    #[test]
    fn 代入与最长键优先() {
        let attrs =
            vec![("hp".to_string(), "7".to_string()), ("hpmax".to_string(), "99".to_string())];
        assert_eq!(substitute("血量 %hp / %hpmax", &attrs), "血量 7 / 99");
        assert_eq!(substitute("字面 %%hp", &attrs), "字面 %hp");
        // 未命中按原样保留
        assert_eq!(substitute("%nope", &attrs), "%nope");
    }
}
