//! 覆盖层状态机：标题/菜单/存档/鉴赏/音量/仪式/关机 等接管输入与画面的模式。
//! M4 实装各界面；M2 仅有 None 态（游戏直进）。

#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    None,
    /// 系统菜单（Esc）
    Menu { sel: usize },
    /// 存档/读档（mode=false 读）
    Save { mode_save: bool, sel: usize },
    Gallery { sel: usize, page: usize, full: Option<usize> },
    Volume { sel: usize },
    /// meta_delete_last 三重确认删档
    Ritual { step: usize, fade: f32 },
    /// 关机倒计时
    Rest,
    /// 标题画面（开始游戏前的入口）
    Title { sel: usize },
}

impl Overlay {
    pub fn active(&self) -> bool {
        !matches!(self, Overlay::None)
    }
}
