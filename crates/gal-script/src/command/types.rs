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
