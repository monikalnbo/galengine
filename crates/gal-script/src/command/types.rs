//! 全部剧本指令的枚举定义（数量以本文件变体为准）

#[derive(Debug, Clone)]
pub enum Command {
    Bg {
        storage: String,
        fade_ms: u32,
    },
    Bgm(String),
    BgmStop,
    BgmFadeOut {
        ms: u32,
    },
    Se(String),
    /// 语音（`voice 名称`；voice 目录下按 `<角色>/<文件>` 或 `<文件>` 查找）
    Voice(String),
    VoiceStop,
    Char {
        layer: u8,
        storage: String,
        x: Option<i32>,
        y: Option<i32>,
    },
    /// 立绘左右摇晃（KAG3 charsway：层 次数 周期ms）
    CharSway {
        layer: u8,
        count: u32,
        period_ms: u32,
    },
    CharPos {
        layer: u8,
        storage: String,
        pos_name: String,
    },
    Cg {
        storage: String,
        fade_ms: u32,
    },
    /// 自由层（KAG3 `layer n storage [ms]`：层号 5-63，合成按层号升序）
    Layer {
        n: u8,
        storage: String,
        fade_ms: u32,
    },
    /// 层转场（P1-1 `trans n [ms] [method]`）：目标层在 ms 内交叉淡化到新素材
    Trans {
        n: u8,
        storage: String,
        dur_ms: u32,
        method: String,
    },
    /// 等待在途转场结束（KAG3 `wt` 的转场形态）
    WaitTrans,
    // ===== v0.3.2 KAG3 音频标签组（双缓冲/淡入淡出/循环点/等待语义）=====
    /// playbgm [ms]：双缓冲播放 BGM（可指定淡入 ms）
    PlayBgm {
        name: String,
        fade_in_ms: u32,
    },
    /// pausebgm：暂停当前 BGM（保留播放头）
    PauseBgm,
    /// resumebgm：恢复暂停
    ResumeBgm,
    /// fadeinbgm ms：定时淡入播放
    FadeInBgm {
        name: String,
        ms: u32,
    },
    /// fadeoutbgm ms：定时淡出并停止
    FadeOutBgm {
        ms: u32,
    },
    /// fadepausebgm ms：淡出后暂停（播放头停在淡出点）
    FadePauseBgm {
        ms: u32,
    },
    /// fadebgm to% ms：在播 BGM 音量淡到指定百分比
    FadeBgm {
        to_pct: i32,
        ms: u32,
    },
    /// xchgbgm name ms：双缓冲交叉淡化切换（无缝切歌）
    XchgBgm {
        name: String,
        ms: u32,
    },
    /// bgmopt start end：循环点（毫秒）
    BgmOpt {
        loop_start_ms: u64,
        loop_end_ms: u64,
    },
    /// playse name：播放 SE（多实例并发）
    PlaySe {
        name: String,
    },
    /// stopse：停掉全部 SE
    StopSe,
    /// fadeinse name ms：淡入播放 SE
    FadeInSe {
        name: String,
        ms: u32,
    },
    /// fadeoutse name ms：淡出并停（新起实例即挂淡出）
    FadeOutSe {
        name: String,
        ms: u32,
    },
    /// fadese to% ms：最近实例音量淡到百分比
    FadeSe {
        to_pct: i32,
        ms: u32,
    },
    /// seopt name：SE 播放选项（预留：目前记录到 trace）
    SeOpt {
        name: String,
    },
    /// wb：等待 BGM 淡入淡出结束
    WaitBgm,
    /// wl：等待 BGM 到达循环点一次
    WaitLoop,
    /// ws：等待 SE 全部播完
    WaitSe,
    /// wf：等待 SE 淡入淡出结束
    WaitSeFade,
    Clear,
    Wait(u32),
    N(String),
    Name {
        name: String,
        text: String,
    },
    Flag {
        var: String,
        op: char,
        val: i64,
    },
    Set {
        var: String,
        expr: String,
    },
    Jump {
        file: Option<String>,
        label: String,
    },
    /// `if 条件 *label`：条件成立时跳到 label（KAG3 单行形态）
    If {
        cond: String,
        target: String,
    },
    /// 块起始 `if 条件`：条件不成立跳到 else 之后或 endif 之后
    /// （跳转目标由 load 阶段的 blocks 表按行下标解析，见 interp::blocks）
    IfBlock {
        cond: String,
    },
    /// 块中段 `else`：真分支执行到这里直接跳到 endif 之后
    Else,
    /// 块结束 `endif`：无操作（占位，保证 if/else 目标可计算）
    EndIf,
    /// 调用子程序：压栈后跳转，`return` 回到下一行
    Call {
        file: Option<String>,
        label: String,
    },
    Return,
    Choice {
        prompt: String,
        items: Vec<(String, String)>,
    },
    Input {
        var: String,
        prompt: String,
        width: u32,
        default: String,
    },
    MetaFakeSave {
        date: String,
        time: String,
        image: String,
    },
    MetaCorrupt(String),
    MetaDeleteLast,
    TitleEvolve(String),
    Reach {
        storage: String,
        dur_ms: u32,
        scale: f32,
    },
    WindowFxShake(u32),
    WindowFxTitle(String),
    WindowFxTitleRestore,
    Shutdown(u32),
    DesktopWrite {
        file: String,
        content: String,
    },
    DesktopOpen(String),
    End,
}
