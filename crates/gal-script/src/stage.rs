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
    pub t: f32,
    pub dur: f32,
    /// 绘制偏移（char 指令 x y 参数；自由层 move 将来复用）
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
    /// 未知/暂不支持的规则名：退化为交叉淡化，并记录原文供诊断
    Unsupported,
}

impl TransMethod {
    pub fn parse(name: &str) -> Self {
        match name {
            "universal" => TransMethod::Universal,
            "crossfade" => TransMethod::Crossfade,
            _ => TransMethod::Unsupported,
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
}

pub struct Stage {
    /// 有序层表（下标即层号，合成按顺序）
    pub layers: Vec<Layer>,
    /// 各立绘层当前的摇晃状态（`charsway` 触发，按周期轮换）
    pub sway: [Option<Sway>; 3],
    /// 在途转场（P1-1；同一时刻只做一段，KAG3 也是一段一段排队）
    pub trans: Option<TransClip>,
}

impl Default for Stage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage {
    pub fn new() -> Self {
        Self { layers: vec![Layer::default(); DEFAULT_LAYERS], sway: [None; 3], trans: None }
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
        }
        StageSnap {
            bg: self.bg().cur().cloned(),
            chars: [0, 1, 2].map(|i| self.char(i).cur().cloned()),
            char_offsets: [0, 1, 2].map(|i| self.char(i).offset),
            cg: self.cg().cur().cloned(),
            layers,
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
        }
        self.trans = None;
        for (n, path) in snap.layers {
            let _ = self.set_layer(n, Some(path), 0);
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
