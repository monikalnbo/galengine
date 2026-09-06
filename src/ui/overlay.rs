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
    /// 关机倒计时（状态在 sys.rest）
    Rest,
    /// 标题画面（M4）
    Title { sel: usize },
}

impl Overlay {
    pub fn active(&self) -> bool {
        !matches!(self, Overlay::None)
    }
}

/// 系统菜单项（引擎通用 UI 文案，非游戏内容）
pub fn menu_items() -> Vec<String> {
    vec!["继续".into(), "存档".into(), "读档".into(), "退出游戏".into()]
}
