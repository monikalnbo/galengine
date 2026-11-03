//! 演出层（纯数据+计时）：统一有序层表，所有绘制走同一按层序合成路径。
//!
//! 层号约定（与 KAG3 习惯对齐，P0-6）：
//! - `0` 背景（bg）
//! - `1..=3` 立绘（char，带偏移/chibi/摇晃）
//! - `4` CG
//! - `>= 5` 自由层（`layer n` 指令；预留 trans/move/anim 的舞台）
//!
//! 每层一个 `Layer`（cur/prev 交叉淡化），合成顺序 = 层号升序。
//! `StageSnap` 为存档快照（路径序列化，恢复时 0ms 直切），旧档字段全部 `#[serde(default)]`。

/// 背景层号
pub const BG_LAYER: usize = 0;
/// CG 层号
pub const CG_LAYER: usize = 4;
/// 立绘层号范围
pub const CHAR_LAYERS: std::ops::RangeInclusive<usize> = 1..=3;
/// 默认层总数（0..=4 常用层 + 8 个自由层）
pub const DEFAULT_LAYERS: usize = 13;
/// 层号上限（自由层可扩到 63）
pub const MAX_LAYERS: usize = 64;
/// 自由层起点
pub const FREE_LAYER_FIRST: usize = CG_LAYER + 1;

/// 层类别：决定绘制策略（背景可虚化、立绘可摇晃、其余直绘）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerKind {
    Bg,
    Char,
    Cg,
    Free,
}

impl LayerKind {
    /// 层号 → 类别（0=bg、1..=3=立绘、4=cg、>=5 自由层）
    pub fn of(n: usize) -> Self {
        match n {
            BG_LAYER => LayerKind::Bg,
            CG_LAYER => LayerKind::Cg,
            n if CHAR_LAYERS.contains(&n) => LayerKind::Char,
            _ => LayerKind::Free,
        }
    }
}

#[derive(Default, Clone)]
pub struct Layer {
    pub cur: Option<String>,
    pub prev: Option<String>,
    /// v0.3.6 back 页（KAG3 fore/back 双页模型）：`backlay` / 无参 `trans` 的
    /// 目标页。平时不绘制，`trans`（无 layer 前缀形态）时与 fore 页交叉。
    pub back: Option<String>,
    pub t: f32,
    pub dur: f32,
    /// 绘制偏移（char 指令 x y 参数；move 动画改写）
    pub offset: (i32, i32),
    /// 是否Q版立绘（名单见 game.yaml sprites.chibi；在场时背景虚化）
    pub chibi: bool,
}

impl Layer {
    /// 切换内容（None=隐藏）；同内容且无渐变在途时不重复切换
    pub fn set(&mut self, next: Option<String>, dur_ms: u32) {
        if self.cur == next && self.prev.is_none() {
            return;
        }
        self.prev = self.cur.take();
        self.cur = next;
        self.t = 0.0;
        self.dur = dur_ms as f32;
    }

    pub fn cur(&self) -> Option<&String> {
        self.cur.as_ref()
    }

    pub fn visible(&self) -> bool {
        self.cur.is_some() || self.prev.is_some()
    }

    pub fn tick(&mut self, dt_ms: f32) {
        if self.t < self.dur {
            self.t += dt_ms;
            if self.t >= self.dur {
                self.prev = None; // 渐变完成释放旧图
            }
        }
    }

    /// 当前透明度（无渐变=1；首帧淡入从 0→1，血泪 #5）
    pub fn alpha(&self) -> f32 {
        if self.dur <= 0.0 {
            1.0
        } else {
            (self.t / self.dur).min(1.0)
        }
    }
}

/// 存档快照。
///
/// 兼容策略：v0.2.2 及更早的档只有 `bg / chars / cg` 三个字段；新增字段全部
/// `#[serde(default)]`，旧档读出来是空壳（无自由层、无层序覆盖），恢复后
/// 表现与旧版一致。`layer_order` 用于记录 `layer z` 改过的层序。
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StageSnap {
    pub bg: Option<String>,
    pub chars: [Option<String>; 3],
    #[serde(default)]
    pub char_offsets: [(i32, i32); 3],
    pub cg: Option<String>,
    /// 自由层（层号 → 素材路径）；旧档无此字段 = 空
    #[serde(default)]
    pub layers: Vec<(usize, String)>,
    /// v0.3.6 back 页（层号 → 素材路径；旧档无此字段 = 全空）
    #[serde(default)]
    pub backs: Vec<(usize, String)>,
    /// 自由层的绘制偏移（与 layers 对应；旧档无此字段 = 空）
    #[serde(default)]
    pub layer_offsets: Vec<(usize, i32, i32)>,
    /// 层序覆盖（`layer z` 指令；旧档无此字段 = 按默认顺序）
    #[serde(default)]
    pub order: Vec<usize>,
}

/// 立绘摇晃（`charsway 层 次数 周期`）：剩余时间 + 每周期时长。
/// 只影响绘制偏移，不进存档快照（纯瞬时演出）。
#[derive(Clone, Copy, Debug)]
pub struct Sway {
    pub left: f32,
    pub period_ms: f32,
}

/// `trans` 支持的表现形态（第一批：通用交叉淡化）。
///
/// KAG3 的完整形态是 `trans method rule1 rule2 ...`（universal / crossfade /
/// trans / 各种擦除规则）。当前实现把「旧内容 → 新内容」统一抽象成
/// [`TransClip`]：先在目标层上叠旧内容的定格，再按透明度把新内容淡入；
/// `universal` 与 `crossfade` 在单层实现下等价。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransMethod {
    /// `trans` 未给 method（KAG3 默认）
    Universal,
    /// `trans ... crossfade`
    Crossfade,
    /// v0.3.3 滑动类：新内容从方向一侧滑入、旧内容反向滑出（slide l/r/u/d）
    Slide(Direction),
    /// v0.3.3 卷动类：旧内容原位、新内容从方向一侧推入覆盖（scroll l/r/u/d）
    Scroll(Direction),
    /// 未知/暂不支持的规则名：退化为交叉淡化，并记录原文供诊断
    Unsupported,
}

/// v0.3.3 slide/scroll 方向
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "l" | "left" => Some(Direction::Left),
            "r" | "right" => Some(Direction::Right),
            "u" | "up" => Some(Direction::Up),
            "d" | "down" => Some(Direction::Down),
            _ => None,
        }
    }
}

impl TransMethod {
    pub fn parse(name: &str) -> Self {
        match name {
            "universal" => TransMethod::Universal,
            "crossfade" => TransMethod::Crossfade,
            _ => {
                if let Some(rest) = name.strip_prefix("slide") {
                    Direction::parse(rest.trim())
                        .map(TransMethod::Slide)
                        .unwrap_or(TransMethod::Unsupported)
                } else if let Some(rest) = name.strip_prefix("scroll") {
                    Direction::parse(rest.trim())
                        .map(TransMethod::Scroll)
                        .unwrap_or(TransMethod::Unsupported)
                } else {
                    TransMethod::Unsupported
                }
            }
        }
    }
}

/// 一次性转场片段（P1-1）：正在淡入的新内容 + 被遮住的旧层快照。
#[derive(Clone, Debug)]
pub struct TransClip {
    pub method: TransMethod,
    /// 目标层（转场期间该层内容被替换绘制）
    pub layer: usize,
    /// 新内容素材路径
    pub to: String,
    /// 旧内容素材路径（无旧内容 = 从空淡入）
    pub from: Option<String>,
    /// 层偏移（旧层与目标层的绘制偏移在转场期间保持一致）
    pub offset: (i32, i32),
    pub dur_ms: f32,
    pub left_ms: f32,
}

/// v0.3.6 位移动画片段（KAG3 `move` 标签 + `wa`/`animstop`）：
/// 关键帧序列按时间线性插值，驱动 `Layer.offset`。
#[derive(Clone, Debug)]
pub struct MoveClip {
    /// 目标层
    pub layer: usize,
    /// 关键帧 (时间ms, x, y)，时间升序；t=0 缺省帧由 start_move 补当前 offset
    pub frames: Vec<(u32, i32, i32)>,
    /// 总时长 = 末帧时间
    pub dur_ms: u32,
    pub left_ms: u32,
}

impl MoveClip {
    /// 当前进度（0.0-1.0）
    pub fn progress(&self) -> f32 {
        if self.dur_ms == 0 {
            return 1.0;
        }
        (1.0 - self.left_ms as f32 / self.dur_ms as f32).clamp(0.0, 1.0)
    }

    /// 按进度线性插值的当前偏移
    pub fn offset_at(&self, progress: f32) -> (i32, i32) {
        let t = progress * self.dur_ms as f32;
        let mut i = 0;
        while i + 1 < self.frames.len() && self.frames[i + 1].0 as f32 <= t {
            i += 1;
        }
        if i + 1 >= self.frames.len() {
            let f = self.frames.last().copied().unwrap_or((0, 0, 0));
            return (f.1, f.2);
        }
        let (t0, x0, y0) = self.frames[i];
        let (t1, x1, y1) = self.frames[i + 1];
        let span = (t1 - t0).max(1) as f32;
        let k = ((t - t0 as f32) / span).clamp(0.0, 1.0);
        (
            (x0 as f32 + (x1 - x0) as f32 * k).round() as i32,
            (y0 as f32 + (y1 - y0) as f32 * k).round() as i32,
        )
    }

    pub fn done(&self) -> bool {
        self.left_ms == 0
    }
}

impl TransClip {
    /// 新内容透明度（0 → 1 线性）
    pub fn alpha(&self) -> f32 {
        if self.dur_ms <= 0.0 {
            1.0
        } else {
            (1.0 - self.left_ms / self.dur_ms).clamp(0.0, 1.0)
        }
    }

    /// 旧内容透明度（1 → 0，与新内容交叉）
    pub fn old_alpha(&self) -> f32 {
        1.0 - self.alpha()
    }

    pub fn done(&self) -> bool {
        self.left_ms <= 0.0
    }

    /// v0.3.3 slide/scroll 的瞬时偏移：返回 (旧内容偏移, 新内容偏移)（像素）。
    /// - slide l：胶片式——旧内容向左滑出、新内容从右侧整屏跟进
    /// - scroll l：擦除式——旧内容原位、新内容从左侧推入覆盖
    /// - 其余 method 一律 (0,0)（交叉淡化语义）。
    pub fn slide_offsets(&self, progress: f32) -> ((i32, i32), (i32, i32)) {
        let dir = match self.method {
            TransMethod::Slide(d) | TransMethod::Scroll(d) => d,
            _ => return ((0, 0), (0, 0)),
        };
        let (w, h) = (gal_config::LOGICAL_W as f32, gal_config::LOGICAL_H as f32);
        // 行进单位向量：l=-x / r=+x / u=-y / d=+y；对应边长
        let (ux, uy, dim): (f32, f32, f32) = match dir {
            Direction::Left => (-1.0, 0.0, w),
            Direction::Right => (1.0, 0.0, w),
            Direction::Up => (0.0, -1.0, h),
            Direction::Down => (0.0, 1.0, h),
        };
        match self.method {
            // slide：胶片式——旧内容沿方向滑出、新内容从行进反侧整屏跟进
            TransMethod::Slide(_) => (
                ((ux * dim * progress) as i32, (uy * dim * progress) as i32),
                ((ux * dim * (progress - 1.0)) as i32, (uy * dim * (progress - 1.0)) as i32),
            ),
            // scroll：擦除式——旧内容原位、新内容从方向侧推入覆盖
            _ => (
                (0, 0),
                ((ux * dim * (1.0 - progress)) as i32, (uy * dim * (1.0 - progress)) as i32),
            ),
        }
    }
}

pub struct Stage {
    /// 有序层表（下标即层号，合成按顺序）
    pub layers: Vec<Layer>,
    /// 各立绘层当前的摇晃状态（`charsway` 触发，按周期轮换）
    pub sway: [Option<Sway>; 3],
    /// 在途转场（P1-1；同一时刻只做一段，KAG3 也是一段一段排队）
    pub trans: Option<TransClip>,
    /// v0.3.6 在途位移动画（各层独立；`move` 触发，`wa` 等待，`animstop` 取消）
    pub moves: Vec<MoveClip>,
}

impl Default for Stage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage {
    pub fn new() -> Self {
        Self {
            layers: vec![Layer::default(); DEFAULT_LAYERS],
            sway: [None; 3],
            trans: None,
            moves: Vec::new(),
        }
    }

    /// 层号对应的层（越界返回 None，调用方决定扩层或报错）
    pub fn layer(&self, n: usize) -> Option<&Layer> {
        self.layers.get(n)
    }

    /// 层号对应的层（可变）
    pub fn layer_mut(&mut self, n: usize) -> Option<&mut Layer> {
        self.layers.get_mut(n)
    }

    /// 背景层
    pub fn bg(&self) -> &Layer {
        &self.layers[BG_LAYER]
    }

    /// 立绘层（0..3 → 层 1..=3）
    pub fn char(&self, idx: usize) -> &Layer {
        &self.layers[1 + idx]
    }

    /// 立绘层可变借用
    pub fn char_mut(&mut self, idx: usize) -> &mut Layer {
        &mut self.layers[1 + idx]
    }

    /// 全部立绘层
    pub fn chars(&self) -> impl Iterator<Item = &Layer> {
        self.layers[1..=3].iter()
    }

    /// 全部立绘层（可变）
    pub fn chars_mut(&mut self) -> impl Iterator<Item = &mut Layer> {
        self.layers[1..=3].iter_mut()
    }

    /// CG 层
    pub fn cg(&self) -> &Layer {
        &self.layers[CG_LAYER]
    }

    /// 设置某层内容（0 起即层号；自由层按需扩表）
    pub fn set_layer(&mut self, n: usize, next: Option<String>, dur_ms: u32) -> Result<(), String> {
        self.ensure_layer(n)?;
        self.layers[n].set(next, dur_ms);
        Ok(())
    }

    /// 确保层号存在（自由层按需扩展；上限 64，防剧本写错撑爆内存）
    pub fn ensure_layer(&mut self, n: usize) -> Result<(), String> {
        if n >= MAX_LAYERS {
            return Err(format!("图层号 {n} 超出上限（0-{}）", MAX_LAYERS - 1));
        }
        if self.layers.len() <= n {
            self.layers.resize(n + 1, Layer::default());
        }
        Ok(())
    }

    /// 统一合成顺序：层号升序，返回 (层号, 类别, 层)。
    ///
    /// 绘制层只认这一个入口——bg/立绘/cg/自由层同一条路径，层序永远由
    /// `layers` 下标决定（将来 `layer z` 只改这里产出的顺序即可）。
    pub fn ordered_layers(&self) -> impl Iterator<Item = (usize, LayerKind, &Layer)> {
        self.layers.iter().enumerate().map(|(n, l)| (n, LayerKind::of(n), l))
    }

    /// 触发立绘摇晃：count 次、每周期 period_ms
    pub fn start_sway(&mut self, layer: usize, count: u32, period_ms: u32) {
        if let Some(s) = self.sway.get_mut(layer) {
            *s = Some(Sway {
                left: count as f32 * period_ms.max(1) as f32,
                period_ms: period_ms.max(1) as f32,
            });
        }
    }

    /// 当前立绘摇晃偏移（未摇晃=0）
    pub fn sway_offset(&self, layer: usize) -> i32 {
        match self.sway.get(layer) {
            Some(Some(s)) if s.left > 0.0 => {
                // 每半个周期换向；幅度固定（次数由 start_sway 换算成总时长）
                let half = (s.period_ms / 2.0).max(1.0);
                let dir = if ((s.left / half) as u32).is_multiple_of(2) { -1.0 } else { 1.0 };
                (dir * 12.0) as i32
            }
            _ => 0,
        }
    }

    /// 某层的合成偏移（立绘层叠加摇晃；其余层用自身 offset）
    ///
    /// `idx` 是**层号**（0=bg、1..=3=立绘、4=cg、>=5 自由层）。
    pub fn compose_offset(&self, idx: usize) -> (i32, i32) {
        let offset = self.layers[idx].offset;
        let sway = if CHAR_LAYERS.contains(&idx) { self.sway_offset(idx - 1) } else { 0 };
        (offset.0 + sway, offset.1)
    }

    /// 是否有Q版立绘在场（背景改走虚化绘制）
    pub fn chibi_on(&self) -> bool {
        self.chars().any(|c| c.chibi && c.cur.is_some())
    }

    fn tick_sway(&mut self, dt_ms: f32) {
        for slot in self.sway.iter_mut() {
            if let Some(s) = slot {
                s.left = (s.left - dt_ms).max(0.0);
                if s.left <= 0.0 {
                    *slot = None;
                }
            }
        }
    }

    /// v0.3.6 backlay（KAG3）：把 fore 页当前内容复制到 back 页。
    /// `hide` 清空 back 页。back 页只在随后「无 layer 前缀的 trans」中作目标页。
    pub fn backlay(&mut self, n: usize, hide: bool) -> Result<(), String> {
        self.ensure_layer(n)?;
        self.layers[n].back = if hide { None } else { self.layers[n].cur.clone() };
        Ok(())
    }

    /// v0.3.7 forelay/无前缀 trans（KAG3 双页原子转场）：back 页内容一次翻上到 fore 页。
    ///
    /// back 页为空 = 淡出到空（KAG3 口径：trans 无后页目标即隐层）；
    /// fore 页旧内容定格进 TransClip::from 由渲染层叠画，转场结束丢弃。
    /// 存档语义不变：fore 页 cur 立即等于 back 页内容（转场后状态），back 页翻上后清空。
    pub fn fore_trans(&mut self, n: usize, dur_ms: u32, method: TransMethod) -> Result<(), String> {
        let to = self.layers[n].back.clone();
        self.layers[n].back = None;
        if to.is_none() {
            // 淡出到空：与 Trans 指令 hide 分支同构
            let old = self.layers[n].cur.clone();
            self.layers[n].set(None, 0);
            self.trans = Some(TransClip {
                method,
                layer: n,
                to: String::new(),
                from: old,
                offset: self.layers[n].offset,
                dur_ms: dur_ms as f32,
                left_ms: dur_ms as f32,
            });
            return Ok(());
        }
        self.start_trans(n, to.unwrap(), dur_ms, method);
        Ok(())
    }

    /// v0.3.6 copylay（KAG3）：把 from 层的 fore 页内容复制到 to 层 fore 页（0ms 直切）。
    pub fn copylay(&mut self, from: usize, to: usize, dur_ms: u32) -> Result<(), String> {
        self.ensure_layer(from)?;
        self.ensure_layer(to)?;
        let src = self.layers[from].cur.clone();
        self.layers[to].set(src, dur_ms);
        Ok(())
    }

    /// v0.3.6 开始一段位移动画（KAG3 `move`）：frames 时间升序。
    /// t=0 缺省帧补当前 offset；重复调用覆盖同层在途动画。
    pub fn start_move(&mut self, n: usize, mut frames: Vec<(u32, i32, i32)>) -> Result<(), String> {
        self.ensure_layer(n)?;
        if frames.is_empty() {
            return Err(format!("层 {n}：move 关键帧为空"));
        }
        if frames[0].0 != 0 {
            let cur = self.layers[n].offset;
            frames.insert(0, (0, cur.0, cur.1));
        }
        let dur_ms = frames.last().map(|f| f.0).unwrap_or(0);
        self.layers[n].offset = (frames[0].1, frames[0].2);
        self.moves.retain(|m| m.layer != n);
        self.moves.push(MoveClip { layer: n, frames, dur_ms, left_ms: dur_ms });
        Ok(())
    }

    /// v0.3.6 在途位移动画的剩余时长（取各层最长；无在途 = None，`wa` 语义）
    pub fn move_left_ms(&self) -> Option<u32> {
        self.moves.iter().map(|m| m.left_ms).max()
    }

    /// v0.3.6 animstop：取消层 n 的在途动画，offset 停在当前位置
    pub fn stop_move(&mut self, n: usize) {
        self.moves.retain(|m| m.layer != n);
    }

    /// v0.3.6 消费在途 move：写回各层 offset，完成即摘除
    fn tick_moves(&mut self, dt_ms: u32) {
        for m in &mut self.moves {
            m.left_ms = m.left_ms.saturating_sub(dt_ms);
            let off = m.offset_at(m.progress());
            if let Some(l) = self.layers.get_mut(m.layer) {
                l.offset = off;
            }
        }
        let done: Vec<usize> =
            self.moves.iter().enumerate().filter(|(_, m)| m.done()).map(|(i, _)| i).collect();
        for i in done.into_iter().rev() {
            self.moves.swap_remove(i);
        }
    }

    /// 开始一段转场（P1-1）：层 `n` 从当前内容切到 `to`，`dur_ms` 内交叉淡化。
    ///
    /// 立刻把 `Layer.cur` 设成新内容（存档语义 = 转场后的状态），旧内容定格在
    /// [`TransClip::from`] 里由渲染层叠画，转场结束即丢弃。若同一层已有在途
    /// 转场，直接以它当前的目标内容作为本段旧内容（不叠加两段）。
    pub fn start_trans(&mut self, n: usize, to: String, dur_ms: u32, method: TransMethod) {
        let _ = self.ensure_layer(n);
        let from = match self.trans.take() {
            Some(t) if t.layer == n => Some(t.to),
            _ => self.layers[n].cur.clone(),
        };
        self.layers[n].set(Some(to.clone()), 0); // 内容立即生效，交叉由渲染处理
        self.trans = Some(TransClip {
            method,
            layer: n,
            to,
            from,
            offset: self.layers[n].offset,
            dur_ms: dur_ms as f32,
            left_ms: dur_ms as f32,
        });
    }

    pub fn tick(&mut self, dt_ms: f32) {
        for l in &mut self.layers {
            l.tick(dt_ms);
        }
        self.tick_sway(dt_ms);
        self.tick_moves(dt_ms.round().max(0.0) as u32);
        if let Some(t) = &mut self.trans {
            t.left_ms = (t.left_ms - dt_ms).max(0.0);
            if t.left_ms <= 0.0 {
                self.trans = None;
            }
        }
    }

    /// 存档快照
    pub fn snapshot(&self) -> StageSnap {
        let mut layers = Vec::new();
        let mut backs = Vec::new();
        let mut layer_offsets = Vec::new();
        for (i, l) in self.layers.iter().enumerate() {
            // 0..=4 走专用字段，自由层进 layers
            if i <= CG_LAYER {
                continue;
            }
            if let Some(p) = &l.cur {
                layers.push((i, p.clone()));
                if l.offset != (0, 0) {
                    layer_offsets.push((i, l.offset.0, l.offset.1));
                }
            }
            // v0.3.6 back 页也进快照（存档 = 转场后状态，含待转 back 页）
            if let Some(p) = &l.back {
                backs.push((i, p.clone()));
            }
        }
        // 0..=4 的 back 页：非空才记（键即层号）
        for i in 0..=CG_LAYER {
            if let Some(p) = &self.layers[i].back {
                backs.push((i, p.clone()));
            }
        }
        StageSnap {
            bg: self.bg().cur().cloned(),
            chars: [0, 1, 2].map(|i| self.char(i).cur().cloned()),
            char_offsets: [0, 1, 2].map(|i| self.char(i).offset),
            cg: self.cg().cur().cloned(),
            layers,
            backs,
            layer_offsets,
            // 默认层序就是层号升序；只有被 `layer z` 改过才需要记录（此处恒为空，
            // 保留字段是为了旧档/新档结构稳定，等 layer z 落地再填）
            order: Vec::new(),
        }
    }

    /// 存档恢复：0ms 直切（无渐变），恢复立绘位置与自由层
    pub fn restore(&mut self, snap: StageSnap) {
        if self.layers.len() < DEFAULT_LAYERS {
            self.layers.resize(DEFAULT_LAYERS, Layer::default());
        }
        self.layers[BG_LAYER].set(snap.bg, 0);
        for (i, path) in snap.chars.into_iter().enumerate() {
            let l = &mut self.layers[1 + i];
            l.set(path, 0);
            l.offset = snap.char_offsets[i];
        }
        self.layers[CG_LAYER].set(snap.cg, 0);
        // 自由层：先清空 5..，再按快照填回（旧档 layers 为空 → 全部清空）
        for l in self.layers.iter_mut().skip(FREE_LAYER_FIRST) {
            l.set(None, 0);
            l.back = None;
        }
        for l in self.layers.iter_mut().take(FREE_LAYER_FIRST) {
            l.back = None;
        }
        self.trans = None;
        self.moves.clear();
        for (n, path) in snap.layers {
            let _ = self.set_layer(n, Some(path), 0);
        }
        // v0.3.6 back 页恢复
        for (n, path) in snap.backs {
            if let Some(l) = self.layers.get_mut(n) {
                l.back = Some(path);
            }
        }
        for (n, x, y) in snap.layer_offsets {
            if let Some(l) = self.layers.get_mut(n) {
                l.offset = (x, y);
            }
        }
    }

    /// 当前引用路径集合（场景资源回收 keep 集）
    pub fn collect_paths(&self, out: &mut std::collections::HashSet<String>) {
        for layer in &self.layers {
            if let Some(p) = &layer.cur {
                out.insert(p.clone());
            }
            if let Some(p) = &layer.prev {
                out.insert(p.clone());
            }
            if let Some(p) = &layer.back {
                out.insert(p.clone());
            }
        }
        if let Some(t) = &self.trans {
            out.insert(t.to.clone());
            if let Some(f) = &t.from {
                out.insert(f.clone());
            }
        }
    }

    /// 在途转场的剩余时长（无转场 = None）
    pub fn trans_left_ms(&self) -> Option<f32> {
        self.trans.as_ref().map(|t| t.left_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 默认层表覆盖bg立绘cg与自由层() {
        let s = Stage::new();
        assert_eq!(s.layers.len(), DEFAULT_LAYERS);
        assert_eq!(LayerKind::of(0), LayerKind::Bg);
        assert_eq!(LayerKind::of(1), LayerKind::Char);
        assert_eq!(LayerKind::of(3), LayerKind::Char);
        assert_eq!(LayerKind::of(4), LayerKind::Cg);
        assert_eq!(LayerKind::of(5), LayerKind::Free);
    }

    #[test]
    fn 合成顺序按层号升序且自由层可扩展() {
        let mut s = Stage::new();
        s.set_layer(0, Some("bg/a".into()), 0).unwrap();
        s.set_layer(1, Some("char/a".into()), 0).unwrap();
        s.set_layer(4, Some("cg/a".into()), 0).unwrap();
        s.set_layer(7, Some("free/a".into()), 0).unwrap();
        let got: Vec<(usize, LayerKind)> =
            s.ordered_layers().filter(|(_, _, l)| l.visible()).map(|(n, k, _)| (n, k)).collect();
        assert_eq!(
            got,
            vec![
                (0, LayerKind::Bg),
                (1, LayerKind::Char),
                (4, LayerKind::Cg),
                (7, LayerKind::Free),
            ]
        );
        assert_eq!(s.layer(7).unwrap().cur.as_deref(), Some("free/a"));
    }

    #[test]
    fn 层号超上限报错() {
        let mut s = Stage::new();
        assert!(s.set_layer(63, None, 0).is_ok());
        assert!(s.set_layer(64, None, 0).is_err());
    }

    #[test]
    fn 旧版存档快照仍可反序列化() {
        // v0.2.2 时代的档：只有 bg / chars / cg（连 char_offsets 都没有）
        let legacy = r#"{"bg":"bg/a","chars":["char/a",null,null],"cg":null}"#;
        let snap: StageSnap = serde_json::from_str(legacy).unwrap();
        assert_eq!(snap.bg.as_deref(), Some("bg/a"));
        assert_eq!(snap.chars[0].as_deref(), Some("char/a"));
        assert_eq!(snap.char_offsets, [(0, 0); 3]);
        assert!(snap.layers.is_empty() && snap.layer_offsets.is_empty() && snap.order.is_empty());

        let mut s = Stage::new();
        s.restore(snap);
        assert_eq!(s.bg().cur.as_deref(), Some("bg/a"));
        assert_eq!(s.char(0).cur.as_deref(), Some("char/a"));
        assert!(s.layer(FREE_LAYER_FIRST).unwrap().cur.is_none());
    }

    // ── v0.3.6 back 页 + move 动画 ─────────────────────────────

    #[test]
    fn backlay复制fore页且hide清空() {
        let mut s = Stage::new();
        s.set_layer(0, Some("bg/a".into()), 0).unwrap();
        s.backlay(0, false).unwrap();
        assert_eq!(s.layers[0].back.as_deref(), Some("bg/a"));
        // 换 fore 页不影响 back 页
        s.set_layer(0, Some("bg/b".into()), 0).unwrap();
        assert_eq!(s.layers[0].back.as_deref(), Some("bg/a"));
        s.backlay(0, true).unwrap();
        assert!(s.layers[0].back.is_none());
    }

    #[test]
    fn copylay整层复制() {
        let mut s = Stage::new();
        s.set_layer(4, Some("cg/x".into()), 0).unwrap();
        s.copylay(4, 5, 0).unwrap();
        assert_eq!(s.layer(5).unwrap().cur.as_deref(), Some("cg/x"));
    }

    #[test]
    fn move动画插值与wa剩余() {
        let mut s = Stage::new();
        s.set_layer(5, Some("free/a".into()), 0).unwrap();
        s.layers[5].offset = (100, 0);
        s.start_move(5, vec![(500, -50, 20)]).unwrap();
        // t=0 补帧：从当前 offset 起步
        assert_eq!(s.moves[0].frames[0], (0, 100, 0));
        s.tick(250.0);
        assert_eq!(s.layers[5].offset, (25, 10));
        s.tick(250.0);
        assert_eq!(s.layers[5].offset, (-50, 20));
        assert!(s.moves.is_empty());
        assert_eq!(s.move_left_ms(), None);
    }

    #[test]
    fn move多关键帧与animstop停在当前位置() {
        let mut s = Stage::new();
        s.start_move(6, vec![(0, 0, 0), (400, 80, 0), (800, 80, 40)]).unwrap();
        assert_eq!(s.move_left_ms(), Some(800));
        s.tick(400.0);
        assert_eq!(s.layers[6].offset, (80, 0));
        s.stop_move(6);
        assert!(s.moves.is_empty());
        assert_eq!(s.layers[6].offset, (80, 0));
        // 同层重复 move 覆盖在途动画
        s.start_move(6, vec![(100, 0, 0)]).unwrap();
        s.start_move(6, vec![(200, 9, 9)]).unwrap();
        assert_eq!(s.moves.len(), 1);
        assert_eq!(s.moves[0].dur_ms, 200);
    }

    #[test]
    fn move存档后offset保留且动画不进快照() {
        let mut s = Stage::new();
        s.set_layer(7, Some("free/b".into()), 0).unwrap();
        s.start_move(7, vec![(0, 0, 0), (100, 10, 10)]).unwrap();
        s.tick(60.0);
        let snap = s.snapshot();
        let mut r = Stage::new();
        r.restore(snap);
        // 存档语义 = 瞬时状态：offset 保留、动画取消
        assert_eq!(r.layers[7].offset.0, 6);
        assert!(r.moves.is_empty());
    }

    #[test]
    fn back页进存档快照往返() {
        let mut s = Stage::new();
        s.set_layer(0, Some("bg/a".into()), 0).unwrap();
        s.backlay(0, false).unwrap();
        s.set_layer(8, Some("free/c".into()), 0).unwrap();
        s.backlay(8, false).unwrap();
        let mut r = Stage::new();
        r.restore(s.snapshot());
        assert_eq!(r.layers[0].back.as_deref(), Some("bg/a"));
        assert_eq!(r.layer(8).unwrap().back.as_deref(), Some("free/c"));
        // 旧档（无 backs 字段）恢复后 back 全空
        let legacy: StageSnap =
            serde_json::from_str(r#"{"bg":"bg/x","chars":[null,null,null],"cg":null}"#).unwrap();
        r.restore(legacy);
        assert!(r.layers.iter().all(|l| l.back.is_none()));
    }

    // ===== v0.3.7 forelay/无前缀 trans：双页原子转场 =====

    #[test]
    fn fore_trans把back页翻上且back清空() {
        let mut s = Stage::new();
        s.set_layer(0, Some("bg/a".into()), 0).unwrap();
        s.backlay(0, false).unwrap();
        s.set_layer(0, Some("bg/b".into()), 0).unwrap(); // fore 已换，back 仍是 bg/a
        s.fore_trans(0, 500, TransMethod::Crossfade).unwrap();
        assert_eq!(s.layers[0].cur.as_deref(), Some("bg/a")); // back 内容翻上
        assert!(s.layers[0].back.is_none()); // back 页清空
        assert!(s.trans.is_some()); // 转场在途（旧 fore 定格）
        assert_eq!(s.trans.as_ref().unwrap().from.as_deref(), Some("bg/b"));
        assert_eq!(s.trans.as_ref().unwrap().to, "bg/a");
    }

    #[test]
    fn fore_trans空back页淡出到空() {
        let mut s = Stage::new();
        s.set_layer(0, Some("bg/a".into()), 0).unwrap();
        // 不做 backlay → back 页为空 → 淡出
        s.fore_trans(0, 400, TransMethod::Crossfade).unwrap();
        assert!(s.layers[0].cur.is_none());
        let t = s.trans.as_ref().unwrap();
        assert_eq!(t.from.as_deref(), Some("bg/a"));
        assert_eq!(t.to, "");
    }

    #[test]
    fn fore_trans存档语义为转场后状态() {
        let mut s = Stage::new();
        s.set_layer(0, Some("bg/a".into()), 0).unwrap();
        s.backlay(0, false).unwrap();
        s.set_layer(0, Some("bg/b".into()), 0).unwrap();
        s.fore_trans(0, 500, TransMethod::Crossfade).unwrap();
        let mut r = Stage::new();
        r.restore(s.snapshot());
        assert_eq!(r.layers[0].cur.as_deref(), Some("bg/a"));
        assert!(r.trans.is_none()); // 在途转场不进存档（存档=转场后状态）
    }

    #[test]
    fn 新档快照往返保留自由层与偏移() {
        let mut s = Stage::new();
        s.set_layer(0, Some("bg/a".into()), 0).unwrap();
        s.set_layer(2, Some("char/c".into()), 0).unwrap();
        s.layers[2].offset = (-140, 20);
        s.set_layer(9, Some("free/z".into()), 0).unwrap();
        s.layers[9].offset = (7, -3);
        let snap = s.snapshot();
        let json = serde_json::to_string(&snap).unwrap();

        let back: StageSnap = serde_json::from_str(&json).unwrap();
        let mut r = Stage::new();
        r.restore(back);
        assert_eq!(r.char(1).offset, (-140, 20));
        assert_eq!(r.layer(9).unwrap().cur.as_deref(), Some("free/z"));
        assert_eq!(r.layer(9).unwrap().offset, (7, -3));
    }
}
