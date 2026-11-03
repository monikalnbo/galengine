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
    // ===== v0.3.6 fore/back 双页 + 位移动画 =====
    /// backlay [层]：fore 页内容复制到 back 页（`backlay hide` 清空）
    BackLay {
        n: Option<u8>,
        hide: bool,
    },
    /// copylay from to [ms]：from 层 fore 页复制到 to 层 fore 页
    CopyLay {
        from: u8,
        to: u8,
        dur_ms: u32,
    },
    /// move 层 x,y[@ms] …：关键帧位移动画（时间升序，帧间线性插值）
    Move {
        n: u8,
        frames: Vec<(u32, i32, i32)>,
    },
    /// wa：等全部在途位移动画结束
    WaitMove,
    /// animstop [层]：取消在途动画（无参=全部层）
    AnimStop {
        n: Option<u8>,
    },
    // ===== v0.3.9 条件标签收尾 =====
    /// layopt 层 [visible=t/f] [opacity=0-255]：层可见性/不透明度（KAG3 属性写法）
    LayOpt {
        n: u8,
        visible: Option<bool>,
        opacity: Option<u8>,
    },
    /// laycount n：立绘层数上限设定（记录口径，本引擎固定 3 槽）
    LayCount {
        n: u8,
    },
    /// stoptrans：中止在途转场（层停在目标态）
    StopTrans,
    /// animstart 层 档名：加载 data/asd/档名.asd 为多属性关键帧时间轴（v0.3.10）
    AnimStart {
        n: u8,
        name: String,
    },
    /// move 加 ease 参数：缓动方法（linear/easein/easeout/easeinout），v0.3.9
    /// 复用 Move 的 frames，另带 method
    MoveEase {
        n: u8,
        frames: Vec<(u32, i32, i32)>,
        ease: String,
    },
    // ===== v0.3.7 双页原子转场 =====
    /// forelay [层] [ms] [method]：back 页内容一次翻上到 fore 页（层号缺省=0 背景）
    ForeLay {
        n: Option<u8>,
        dur_ms: u32,
        method: String,
    },
    // ===== v0.3.3 KAG3 等待标签（wm/wv/wp）=====
    /// wm：等 BGM 播完
    WaitMusic,
    /// wv：等视频播完（无视频子系统/未开播 → 立即放行）
    WaitVideo,
    /// wp：等全部声音（BGM/SE/语音）播完
    WaitAllSound,
    // ===== v0.3.11 演出等待收尾（quake 族 + wc/resetwait + 存档指纹锁）=====
    /// quake [ms] [maxamp]：画面震动（KAG3 口径：maxamp 幅度本引擎固定包络）
    Quake {
        ms: u32,
        max_amp: u32,
    },
    /// stopquake：中止在途震动
    StopQuake,
    /// wq：等在途震动结束（无震动立即放行）
    WaitQuake,
    /// wc：等点击一次（KAG3 waitclick 的计数形态；本引擎按一次点击口径）
    WaitClickCount,
    /// resetwait：取消当前强制等待（wait/wq/wc）立即放行后续行
    ResetWait,
    /// locksnapshot [名]：锁定当前存档指纹（存档界面显示锁定标记）
    LockSnapshot {
        name: String,
    },
    /// unlocksnapshot：解除存档指纹锁
    UnlockSnapshot,

    // ===== v0.3.12 存档控制标签批 =====
    /// s：停止快进/自动（KAG3 `s`；在跳过态调用即停）
    StopSkip,
    /// disablestore [t/f]：禁用/恢复存档（KAG3 口径；运行期状态）
    DisableStore {
        on: bool,
    },
    /// tempsave [名]：写临时存档（当前时刻场景，不占正式槽位；本引擎落 savedata/temp.json）
    TempSave,
    /// tempload：读临时存档（无则静默 DEBUG 留证）
    TempLoad,
    /// copybookmark 源 目标：复制存档槽
    CopyBookmark {
        from: usize,
        to: usize,
    },
    /// erasebookmark 槽：删除存档槽
    EraseBookmark {
        slot: usize,
    },
    /// record：打开鉴赏模式（CG 回廊）
    Record,
    /// goback：回到前一个存档点（本引擎口径=读最近槽位）
    GoBack,
    /// waittrig 名：等触发器（本引擎以 f.trig_<名> 置 1 为触发口径；轮询）
    WaitTrig {
        name: String,
    },

    // ===== v0.3.8 KAG3 视频标签组（引擎侧未开 `video` feature 时 WARN 放行）=====
    /// video 底层参数设置（KAG3 #115，本引擎收下参数但仅 DEBUG 留证）
    Video {
        params: String,
    },
    /// playvideo 名称 [ms] [skip]：开播视频（ms=自动隐层延时，skip=可点击跳过）
    PlayVideo {
        name: String,
        ms: u32,
        skip: bool,
    },
    /// stopvideo：停止播放
    StopVideo,
    /// openvideo：打开文件准备播放（本引擎与 playvideo 等价）
    OpenVideo {
        name: String,
    },
    /// pausevideo：暂停
    PauseVideo,
    /// resumevideo：恢复
    ResumeVideo,
    /// preparevideo：预载（本引擎与 openvideo 等价）
    PrepareVideo {
        name: String,
    },
    /// rewindvideo：回到开头
    RewindVideo,
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
    /// v0.3.4 块中段 `elsif 条件`：前置分支全假时求值本条件；
    /// 假则跳到下一个 elsif/else 之后或 endif 之后（blocks 表解析）
    ElsIf {
        cond: String,
    },
    /// 块中段 `else`：真分支执行到这里直接跳到 endif 之后
    Else,
    /// 块结束 `endif`：无操作（占位，保证 if/else 目标可计算）
    EndIf,
    /// v0.3.4 `emb 表达式`：求值结果嵌入当前台词（KAG3 emb）
    Emb {
        expr: String,
    },
    /// v0.3.4 `eval 表达式`：求值（可含赋值 `f.a = 5`）不产生输出（KAG3 eval）
    Eval {
        expr: String,
    },
    /// v0.3.4 `trace 文本`：调试输出落 engine.log（KAG3 trace / krkr 控制台）；
    /// 支持 `%表达式` 占位（求值内联）
    Trace {
        text: String,
    },
    /// v0.3.13 `graph 素材名`：行内嵌图（KAG3 graph）——拼 \u{b}占位符进当前台词，
    /// 排版层成 Unit::Image（整体不拆行），渲染时按行高等比贴图
    Graph {
        storage: String,
    },
    /// v0.3.13 `ruby text=基底 alt=注音`：生成 `[基底]^(注音)` 文法拼进当前台词（KAG3 ruby 子集）
    RubyTag {
        text: String,
        alt: String,
    },
    /// v0.3.13 `hch 文本`：行内样式片段（本引擎无行内色通道，DEBUG 留证后按普通文本拼进台词）
    Hch {
        text: String,
    },
    /// v0.3.15 `link [storage=*|exp=]`：行内超链接开始（到配对 endlink 的文本可点；
    /// 点击后 jump storage 或求值 exp，KAG3 link 子集）
    Link {
        storage: Option<String>,
        exp: Option<String>,
    },
    /// v0.3.15 `endlink`：行内超链接结束
    EndLink,
    /// v0.3.15 `button graphic=… storage=…`：图形按钮（进入按钮等待态；KAG3 button 子集）
    Button {
        graphic: String,
        storage: Option<String>,
        exp: Option<String>,
    },
    /// v0.3.15 `locklink` / `unlocklink`：链接禁用/恢复（锁定期链接不可点，视觉变灰）
    LockLink,
    UnLockLink,
    /// v0.3.13 `font size= n color=…`：行内文字属性（记录 + DEBUG 留证，演出零变化）
    Font {
        size: Option<u16>,
        color: Option<String>,
    },
    /// v0.3.13 `deffont …` / `resetfont`：页面级文字属性缺省（记录 + DEBUG 留证）
    Deffont {
        size: Option<u16>,
        color: Option<String>,
    },
    Resetfont,
    /// v0.3.13 `style …` / `defstyle …` / `resetstyle`：段落样式（记录 + DEBUG 留证）
    Style {
        align: Option<String>,
    },
    Defstyle {
        align: Option<String>,
    },
    Resetstyle,
    /// v0.3.14 `r`：行内强制换行（KAG3 [r]）——换行标记拼进当前台词，排版层立即收行
    LineBreak,
    /// v0.3.14 `l`：换行后等点击（KAG3 [l]）——换行标记拼进 + WaitClick
    LineBreakWait,
    /// v0.3.14 `p`：等点击 + 清屏（KAG3 [p]）——点击后文字消去再续
    PageBreakWait,
    /// v0.3.14 `er`：清空当前消息层文字（无等待）
    EraseText,
    /// v0.3.14 `cm`：清空消息层（历史保留）；`ct`：cm + 重置行内样式记录
    ClearMessage,
    ClearMessageAll,
    /// v0.3.14 `indent` / `endindent`：段落缩进（首行前插全角空两个）
    Indent,
    EndIndent,
    /// v0.3.14 `locate x y`：行内文字落点（x 记录，DEBUG 留证；y 支持但按行号排版忽略）
    Locate {
        x: Option<u32>,
        y: Option<u32>,
    },
    /// v0.3.16 `position left= top= width= height= visible= opacity=`：消息层位置/尺寸/可见
    /// （KAG3 position 子集：写 sf.msg* 由引擎每帧对账 compose_style 生效）
    Position {
        left: Option<i32>,
        top: Option<i32>,
        width: Option<i32>,
        height: Option<i32>,
        visible: Option<bool>,
        opacity: Option<i32>,
    },
    /// v0.3.16 `current`：当前输出消息层指定（本引擎单消息层：参数记录 + DEBUG 留证）
    Current {
        layer: Option<String>,
    },
    /// v0.3.17 `glyph 素材名`：页满标记图形（KAG3 glyph 子集；写 sf.msgGlyph，
    /// dialog::draw 页满时用 image/ 素材替代内置「▼」）
    Glyph {
        storage: String,
    },
    /// v0.3.14 `nowait` / `endnowait`：区间打字机瞬时揭示
    NoWait,
    EndNoWait,
    /// 调用子程序：压栈后跳转，`return` 回到下一行
    Call {
        file: Option<String>,
        label: String,
    },
    Return,
    /// v0.3.5 宏定义行（lexer 已把宏体收走；执行到此处只作占位跳过）
    MacroDefSkip,
    /// v0.3.5 `erasemacro 名`：删除宏表中的宏（KAG3 erasemacro）
    EraseMacro(String),
    /// v0.3.5 `clearvar`：清空 f.* 全部游戏变量（存档里的 f 快照独立，不影响存档结构）
    ClearVar,
    /// v0.3.5 `clearsysvar`：清空 sf.* 系统变量（global.json 结构不变，只清键值）
    ClearSysVar,
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
