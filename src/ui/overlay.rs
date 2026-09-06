//! 覆盖层状态机：标题/菜单/存档/鉴赏/音量/仪式/关机 等接管输入与画面的模式。

#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    None,
    /// 系统菜单（Esc）
    Menu { sel: usize },
    /// 存档/读档（mode_save=true 存）
    Save { mode_save: bool, sel: usize },
    Gallery { sel: usize, page: usize, full: Option<usize> },
    Volume { sel: usize },
    /// meta_delete_last 三重确认删档（step≥3 渐白，done=已真删）
    Ritual { step: usize, fade: f32, done: bool },
    /// 标题画面（M4）
    Title { sel: usize },
}

impl Overlay {
    pub fn active(&self) -> bool {
        !matches!(self, Overlay::None)
    }
}

/// 系统菜单项（引擎通用 UI 文案，非游戏内容）
pub fn esc_items() -> Vec<String> {
    vec!["继续".into(), "存档".into(), "读档".into(), "CG 鉴赏".into(), "音量/速度".into(), "回到标题".into(), "退出游戏".into()]
}

/// 标题菜单项
pub fn title_items() -> Vec<String> {
    vec!["开始游戏".into(), "继续游戏".into(), "读档".into(), "CG 鉴赏".into(), "音量/速度".into(), "退出游戏".into()]
}
