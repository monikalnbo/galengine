//! Meta 演出状态：假存档/破損/标题演化（savedata/meta.json 持久化）。
//! 数据层；演出 UI 在 ui/overlay。

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FakeSave {
    pub date: String,
    pub time: String,
    pub thumb: String,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct MetaState {
    /// 存档列表中插入的「不明存档」（点击不可读取）
    pub fake_save: Option<FakeSave>,
    /// 标记为「破損」的槽位
    pub corrupted: Vec<usize>,
    /// 标题演化阶段（0=原样，1=褪色，2=变绿，3=加小字「违和感：7/7」……规则问 docs/10）
    pub title_stage: u32,
}

impl MetaState {
    pub fn load(dir: &str) -> Self {
        let p = format!("{dir}/meta.json");
        match std::fs::read_to_string(&p) {
            Ok(t) => serde_json::from_str(&t).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, dir: &str) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("meta 目录创建失败（{e}）"))?;
        let t = serde_json::to_string_pretty(self).map_err(|e| format!("meta 序列化失败（{e}）"))?;
        std::fs::write(format!("{dir}/meta.json"), t).map_err(|e| format!("meta 写盘失败（{e}）"))
    }

    pub fn is_corrupted(&self, slot: usize) -> bool {
        self.corrupted.contains(&slot)
    }
}
