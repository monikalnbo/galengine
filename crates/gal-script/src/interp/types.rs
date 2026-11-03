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
    // ===== v0.3.12 存档控制标签事件 =====
    /// s：停止快进/自动
    StopSkip,
    /// tempsave：写临时存档（引擎落 savedata/temp.json）
    TempSave,
    /// tempload：读临时存档
    TempLoad,
    /// copybookmark from to：复制存档槽
    CopyBookmark {
        from: usize,
        to: usize,
    },
    /// erasebookmark slot：删除存档槽
    EraseBookmark {
        slot: usize,
    },
    /// record：打开鉴赏模式
    OpenGallery,
    /// goback：读最近存档槽
    GoBack,

    // ===== v0.3.11 quake 族 =====
    /// quake ms maxamp：画面震动（maxamp 留证；包络固定）
    Quake {
        ms: u32,
        max_amp: u32,
    },
    /// stopquake：中止在途震动
    StopQuake,
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
    // ===== v0.3.8 KAG3 视频标签组（引擎侧未开 `video` feature 时 WARN 放行）=====
    VideoSet {
        params: String,
    },
    PlayVideo {
        name: String,
        ms: u32,
        skip: bool,
    },
    StopVideo,
    OpenVideo {
        name: String,
    },
    PauseVideo,
    ResumeVideo,
    PrepareVideo {
        name: String,
    },
    RewindVideo,
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
    /// v0.3.15 button：图形按钮等待（点击命中后触发动作）
    WaitButton,
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
    /// wm（v0.3.3）：BGM/音乐播完（含淡出落停）
    MusicDone,
    /// wv（v0.3.3）：视频播完（本引擎暂无视频子系统 → 恒立即放行 + 留证）
    VideoDone,
    /// wp（v0.3.3）：全部声音（BGM/SE/语音）播完
    AllDone,
    /// wq（v0.3.11）：画面震动结束（无声卡口径下由引擎按 shake 态轮询放行）
    QuakeDone,
    /// waittrig（v0.3.12）：等触发器（键 = f.trig_<名>）
    Trig(String),
}

/// 输入框配置规格
#[derive(Debug, Clone, PartialEq)]
pub struct InputSpec {
    pub var: String,
    pub prompt: String,
    pub width: u32,
    pub default: String,
}
