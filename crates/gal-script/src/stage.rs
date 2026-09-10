//! 演出层（纯数据+计时）：背景 1 + 立绘 3 + CG 1，均 crossfade；素材统一 1280×720 整幅。
//! StageSnap 为存档快照（路径序列化，恢复时 0ms 直切）；绘制在 gal-render::stage_draw。

#[derive(Default)]
pub struct Layer {
    pub cur: Option<String>,
    pub prev: Option<String>,
    pub t: f32,
    pub dur: f32,
    /// 立绘绘制偏移（char 指令 x y 参数）
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

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StageSnap {
    pub bg: Option<String>,
    pub chars: [Option<String>; 3],
    #[serde(default)]
    pub char_offsets: [(i32, i32); 3],
    pub cg: Option<String>,
}

#[derive(Default)]
pub struct Stage {
    pub bg: Layer,
    pub chars: [Layer; 3],
    pub cg: Layer,
}

impl Stage {
    pub fn tick(&mut self, dt_ms: f32) {
        self.bg.tick(dt_ms);
        for c in &mut self.chars {
            c.tick(dt_ms);
        }
        self.cg.tick(dt_ms);
    }

    /// 存档快照
    pub fn snapshot(&self) -> StageSnap {
        StageSnap {
            bg: self.bg.cur().cloned(),
            chars: [0, 1, 2].map(|i| self.chars[i].cur().cloned()),
            char_offsets: [0, 1, 2].map(|i| self.chars[i].offset),
            cg: self.cg.cur().cloned(),
        }
    }

    /// 存档恢复：0ms 直切（无渐变），恢复立绘位置
    pub fn restore(&mut self, snap: StageSnap) {
        self.bg.set(snap.bg, 0);
        for (i, path) in snap.chars.into_iter().enumerate() {
            self.chars[i].set(path, 0);
            self.chars[i].offset = snap.char_offsets[i];
        }
        self.cg.set(snap.cg, 0);
    }

    /// 当前引用路径集合（场景资源回收 keep 集）
    pub fn collect_paths(&self, out: &mut std::collections::HashSet<String>) {
        for layer in
            std::iter::once(&self.bg).chain(self.chars.iter()).chain(std::iter::once(&self.cg))
        {
            if let Some(p) = &layer.cur {
                out.insert(p.clone());
            }
            if let Some(p) = &layer.prev {
                out.insert(p.clone());
            }
        }
    }
}
