//! config/keymap.rs —— 键位映射（作者默认 ← YAML，玩家覆盖 ← global.json）
//!
//! 键名用 SDL 的 Keycode 名称（`Space` / `Return` / `A` / `LCtrl` / `F9` / `PageUp` …），
//! 多个键用列表表示；动作名固定，见 [`ACTIONS`]。
//! 玩家侧改动写进 `sf.keymap.<action>`（逗号分隔），读取时覆盖作者默认。

use std::collections::HashMap;

use serde::Deserialize;

/// 可重映射动作表：`(动作键, 中文标签)`；顺序即设置界面顺序
pub const ACTIONS: &[(&str, &str)] = &[
    ("next", "推进对话"),
    ("auto", "自动播放"),
    ("skip", "快进"),
    ("backlog", "对白历史"),
    ("menu", "系统菜单"),
    ("save", "快速存档"),
    ("load", "快速读档"),
    ("fullscreen", "全屏切换"),
];

/// 动作键是否合法（未知动作忽略，不报错——旧/新配置互不影响）
pub fn is_action(name: &str) -> bool {
    ACTIONS.iter().any(|(a, _)| *a == name)
}

#[derive(Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct KeyMapCfg {
    /// 动作 → 键名列表（缺省动作走内置默认表）
    pub bindings: HashMap<String, Vec<String>>,
}

/// 内置默认键位（与 v0.2.x 硬编码行为逐键一致，另补 F11 全屏）
pub fn default_bindings() -> HashMap<String, Vec<String>> {
    [
        ("next", vec!["Return", "KpEnter", "Space"]),
        ("auto", vec!["A"]),
        ("skip", vec!["LCtrl", "RCtrl"]),
        ("backlog", vec!["PageUp"]),
        ("menu", vec!["Escape"]),
        ("save", vec!["F9"]),
        ("load", vec!["F10"]),
        ("fullscreen", vec!["F11"]),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.into_iter().map(String::from).collect()))
    .collect()
}

impl KeyMapCfg {
    /// 作者默认 + 内置默认合成（作者未列出的动作补内置）
    pub fn effective(&self) -> HashMap<String, Vec<String>> {
        let mut out = default_bindings();
        for (action, keys) in &self.bindings {
            if is_action(action) && !keys.is_empty() {
                out.insert(action.clone(), keys.clone());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 作者未列出的动作补内置默认() {
        let mut c = KeyMapCfg::default();
        c.bindings.insert("next".into(), vec!["Space".into()]);
        let m = c.effective();
        assert_eq!(m["next"], vec!["Space".to_string()]);
        assert_eq!(m["menu"], vec!["Escape".to_string()]); // 未覆盖 → 内置
    }

    #[test]
    fn 未知动作与空列表被忽略() {
        let mut c = KeyMapCfg::default();
        c.bindings.insert("nope".into(), vec!["X".into()]);
        c.bindings.insert("auto".into(), Vec::new());
        let m = c.effective();
        assert!(!m.contains_key("nope"));
        assert_eq!(m["auto"], vec!["A".to_string()]); // 空列表不覆盖
    }
}
