//! 覆盖层状态机：标题/菜单/存档/鉴赏/音量/仪式/关机 等接管输入与画面的模式。

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

/// Esc 系统菜单项（引擎通用 UI 文案，非游戏内容）
pub fn esc_items() -> Vec<String> {
    vec![
        "继续".into(),
        "存档".into(),
        "读取存档".into(),
        "CG 鉴赏".into(),
        "设置".into(),
        "回到标题".into(),
        "退出游戏".into(),
    ]
}

/// 标题菜单项
pub fn title_items() -> Vec<String> {
    vec![
        "开始游戏".into(),
        "继续游戏".into(),
        "读取存档".into(),
        "CG 鉴赏".into(),
        "设置".into(),
        "退出游戏".into(),
    ]
}
