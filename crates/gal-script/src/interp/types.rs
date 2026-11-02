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
    BgmFadeOut {
        ms: u32,
    },
    // ===== v0.3.2 KAG3 音频标签组事件 =====
    PlayBgm {
        name: String,
        fade_in_ms: u32,
    },
    PauseBgm,
    ResumeBgm,
    FadeInBgm {
        name: String,
        ms: u32,
    },
    FadeOutBgm {
        ms: u32,
    },
    FadePauseBgm {
        ms: u32,
    },
    FadeBgm {
        to_pct: i32,
        ms: u32,
    },
    XchgBgm {
        name: String,
        ms: u32,
    },
    BgmOpt {
        loop_start_ms: u64,
        loop_end_ms: u64,
    },
    PlaySe {
        name: String,
    },
    StopSe,
    FadeInSe {
        name: String,
        ms: u32,
    },
    FadeOutSe {
        name: String,
        ms: u32,
    },
    FadeSe {
        to_pct: i32,
        ms: u32,
    },
    SeOpt {
        name: String,
    },
    Se(String),
    Voice(String),
    VoiceStop,
    MetaFake {
        date: String,
        time: String,
        image: String,
    },
    MetaCorrupt(String),
    MetaDeleteLast,
    TitleEvolve(String),
    Reach {
        path: String,
        dur_ms: u32,
        scale: f32,
    },
    ReachHide,
    Shake(u32),
    /// 立绘左右摇晃：层 / 次数 / 单周期ms
    CharSway {
        layer: u8,
        count: u32,
        period_ms: u32,
    },
    SetTitle(String),
    RestoreTitle,
    Shutdown(u32),
    DesktopWrite {
        file: String,
        content: String,
    },
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
    WaitTimer {
        left_ms: f32,
    },
    WaitChoice {
        prompt: String,
        items: Vec<(String, String)>,
    },
    WaitInput(Box<InputSpec>),
    /// v0.3.2 音频等待（wb/wl/ws/wf）：记录等待开始时间，引擎轮询音频状态决定放行
    WaitAudio(AudioWait),
    Ended,
}

/// 音频等待种类（RunState::WaitAudio）
#[derive(Debug, PartialEq, Clone)]
pub enum AudioWait {
    /// wb：BGM 淡入淡出结束
    BgmFade,
    /// wl：BGM 到达循环点
    BgmLoopHit,
    /// ws：SE 全部播完
    SeDone,
    /// wf：SE 淡入淡出结束
    SeFade,
}

/// 输入框配置规格
#[derive(Debug, Clone, PartialEq)]
pub struct InputSpec {
    pub var: String,
    pub prompt: String,
    pub width: u32,
    pub default: String,
}
