//! meta 状态持久化（savedata/meta.json）：不明存档/破損槽位/标题演化阶段。

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FakeSave {
    pub date: String,
    pub time: String,
    pub image: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct MetaState {
    pub fake_saves: Vec<FakeSave>,
    /// 破損槽位号（1 起）
    pub corrupt: Vec<usize>,
    /// 标题演化阶段（""=初始）
    pub title_evolve: String,
}

impl MetaState {
    pub fn load(path: &str) -> MetaState {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => MetaState::default(),
        }
    }

    pub fn save(&self, path: &str) {
        if let Some(dir) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }
}
