//! 解释器核心类型定义与系统事件

/// 验收调试日志开关（ES_DEBUG）
pub fn dbg() -> bool {
    std::env::var("ES_DEBUG").is_ok()
}

/// L2 → L1 系统事件（systems 层消费）
#[derive(Debug, Clone)]
pub enum SysEvent {
    Bgm(String),
    BgmStop,
    BgmFadeOut { ms: u32 },
    Se(String),
    MetaFake { date: String, time: String, image: String },
    MetaCorrupt(String),
    MetaDeleteLast,
    TitleEvolve(String),
    Reach { path: String, dur_ms: u32, scale: f32 },
    ReachHide,
    Shake(u32),
    SetTitle(String),
    RestoreTitle,
    Shutdown(u32),
    DesktopWrite { file: String, content: String },
    DesktopOpen(String),
}

/// 历史对白回卷条目
#[derive(Clone, Debug)]
pub struct BacklogItem {
    pub name: Option<String>,
    pub text: String,
}

/// 解释器运行状态
#[derive(Debug, PartialEq, Clone)]
pub enum RunState {
    WaitClick,
    WaitTimer { left_ms: f32 },
    WaitChoice { prompt: String, items: Vec<(String, String)> },
    WaitInput(Box<InputSpec>),
    Ended,
}

/// 输入框配置规格
#[derive(Debug, Clone, PartialEq)]
pub struct InputSpec {
    pub var: String,
    pub prompt: String,
    pub width: u32,
    pub default: String,
}
