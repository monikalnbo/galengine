//! 覆盖层状态机：标题/菜单/存档/鉴赏/音量/仪式/关机 等接管输入与画面的模式。

use gal_config::Config;

#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    None,
    Menu {
        sel: usize,
    },
    Save {
        mode_save: bool,
        sel: usize,
    },
    Gallery {
        sel: usize,
        page: usize,
        full: Option<usize>,
    },
    /// 设置菜单（全功能：速度/音量/全屏/跳过已读/打赏）
    Settings {
        sel: usize,
    },
    Ritual {
        step: usize,
        fade: f32,
        done: bool,
    },
    Backlog {
        scroll: usize,
    },
    Title {
        sel: usize,
    },
}

impl Overlay {
    pub fn active(&self) -> bool {
        !matches!(self, Overlay::None)
    }
}

/// Esc 系统菜单项：`game.menu_items`（位置即动作，作者只改文字，
/// 删减会让后续项前移——activate::menu_activate 按位置分派）
pub fn esc_items(conf: &Config) -> Vec<String> {
    conf.game.menu_items.clone()
}

/// 标题菜单项：`game.title_items`（同上，位置即动作）
pub fn title_items(conf: &Config) -> Vec<String> {
    conf.game.title_items.clone()
}
