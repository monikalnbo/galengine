//! config/lang.rs —— 多语言包（`lang/<code>.yaml`）
//!
//! 一个语言包 = `name`/`code` + 扁平键值表。剧本与界面文字里写 `{@键}`，
//! 切换语言只换查表结果；**没写 `{@键}` 的旧单语言剧本零改动、原样显示**。
//!
//! ```yaml
//! name: 简体中文
//! code: zh-Hans
//! strings:
//!   hero.greeting: 早上好
//!   menu.start: 开始游戏
//! ```

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::data_dir;

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct LangCfg {
    /// 语言包目录（相对数据目录；不存在 = 无多语言）
    pub dir: String,
    /// 默认语言代码（空 = 目录里第一个按文件名排序的包）
    pub default: String,
}

impl Default for LangCfg {
    fn default() -> Self {
        Self { dir: "lang".into(), default: String::new() }
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct LangPack {
    /// 显示名（设置面板里给玩家看）
    pub name: String,
    /// 语言代码（文件名去扩展名，可被本字段覆盖）
    pub code: String,
    /// 键 → 文案
    pub strings: HashMap<String, String>,
}

/// 扫描语言包目录（按文件名排序；坏文件跳过并提示）
pub fn scan(dir: &str) -> Vec<LangPack> {
    let root = if Path::new(dir).is_absolute() {
        dir.to_string()
    } else {
        format!("{}/{}", data_dir(), dir)
    };
    let Ok(rd) = std::fs::read_dir(&root) else { return Vec::new() };
    let mut files: Vec<_> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| matches!(p.extension().and_then(|s| s.to_str()), Some("yaml" | "yml")))
        .collect();
    files.sort();
    files
        .into_iter()
        .filter_map(|p| {
            let text = std::fs::read_to_string(&p).ok()?;
            match serde_yaml::from_str::<LangPack>(&text) {
                Ok(mut pack) => {
                    if pack.code.trim().is_empty() {
                        pack.code = p.file_stem()?.to_string_lossy().into_owned();
                    }
                    if pack.name.trim().is_empty() {
                        pack.name = pack.code.clone();
                    }
                    Some(pack)
                }
                Err(e) => {
                    eprintln!("[lang] 跳过坏语言包 {}：{e}", p.display());
                    None
                }
            }
        })
        .collect()
}

/// 取某个语言包（找不到 → 空表，原文照旧显示）
pub fn pick(packs: &[LangPack], code: &str) -> LangPack {
    packs
        .iter()
        .find(|p| p.code == code)
        .cloned()
        .or_else(|| packs.first().cloned())
        .unwrap_or_default()
}

/// 把 `{@键}` 换成对应文案；`{@键|兜底}` 在键缺失时用兜底。
/// 未知键与没有 `@` 的 `{}` 原样保留（与既有 `{meta.title}` 等宏互不干扰）。
pub fn subst(text: &str, strings: &HashMap<String, String>) -> String {
    if !text.contains("{@") {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{@") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let body = &after[..end];
                let (key, fallback) = match body.split_once('|') {
                    Some((k, f)) => (k, Some(f)),
                    None => (body, None),
                };
                match strings.get(key) {
                    Some(v) => out.push_str(v),
                    None => match fallback {
                        Some(f) => out.push_str(f),
                        None => {
                            out.push_str("{@");
                            out.push_str(body);
                            out.push('}');
                        }
                    },
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(rest);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> HashMap<String, String> {
        HashMap::from([("a".to_string(), "甲".to_string())])
    }

    #[test]
    fn 键存在则替换_缺失用兜底_未知保留原文() {
        let t = table();
        assert_eq!(subst("{@a}", &t), "甲");
        assert_eq!(subst("{@b|乙}", &t), "乙");
        assert_eq!(subst("{@b}", &t), "{@b}");
        assert_eq!(subst("没有宏", &t), "没有宏");
    }

    #[test]
    fn 与既有大括号宏共存() {
        let t = table();
        assert_eq!(subst("{meta.title} / {@a}", &t), "{meta.title} / 甲");
    }
}

/// 朗读配置段（`tts:`）
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct TtsCfg {
    /// 后端偏好（空=自动探测）：sapi / say / espeak-ng / espeak / spd-say
    pub backend: String,
    /// 首次启动默认语速（-10..10；玩家可在设置里改）
    pub rate: i32,
    /// 首次启动默认开启（false = 玩家自行开启）
    pub on: bool,
}
