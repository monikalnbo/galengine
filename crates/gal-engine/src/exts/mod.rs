//! 内置拓展（引擎侧，game.yaml 按 name 挂载）：机制通用，参数 yaml 驱动。
//! 能力契约 trait 在 gal-ext；具体实现（timer_choice 等）在此。

pub mod registry;
pub mod timer_choice;
