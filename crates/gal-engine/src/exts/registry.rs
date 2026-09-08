//! 内置拓展注册表：game.yaml `extensions:` 按 name 挂载 → 实例。
//! 新增内置拓展在此登记一行（机制通用，参数 yaml 驱动）。

use super::timer_choice::TimerChoice;
use gal_config::ExtCfg;
use gal_ext::Extension;

/// 按 game.yaml 声明批量构建拓展（未知 name 忽略并提示）
pub fn build(cfgs: &[ExtCfg]) -> Vec<Box<dyn Extension>> {
    cfgs.iter()
        .filter_map(|c| match c.name.as_str() {
            "timer_choice" => {
                let e: Box<dyn Extension> = Box::new(TimerChoice::new(c));
                eprintln!("[ext] 挂载 {}", e.name());
                Some(e)
            }
            other => {
                eprintln!("[ext] 未知拓展，已忽略：{other}");
                None
            }
        })
        .collect()
}

/// timer_choice 工厂：name 匹配则建实例（供 Boot::builtin_factories 登记）
pub fn timer_choice(cfg: &ExtCfg) -> Option<Box<dyn Extension>> {
    if cfg.name != "timer_choice" {
        return None;
    }
    Some(Box::new(TimerChoice::new(cfg)))
}
