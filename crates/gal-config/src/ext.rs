//! config/ext.rs —— 拓展声明（game.yaml extensions 段）

use serde::Deserialize;

/// 单个拓展配置：name 定实现，其余字段由该拓展自解释
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ExtCfg {
    pub name: String,
    /// 倒计时毫秒（timer_choice）
    pub default_ms: i64,
    /// 超时自动选含此文字的选项（timer_choice；找不到回退第 0 项）
    pub pick: String,
}
impl Default for ExtCfg {
    fn default() -> Self {
        Self { name: String::new(), default_ms: 7000, pick: "迟疑".into() }
    }
}
