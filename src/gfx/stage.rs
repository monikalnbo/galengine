//! 演出层：背景 1 + 立绘 3 + CG 1，均 crossfade；素材统一 1280×720 整幅。
//! StageSnap 为存档快照（路径序列化，恢复时 0ms 直切）。

use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::gfx::assets::TextureBank;

#[derive(Default)]
pub struct Layer {
    cur: Option<String>,
    prev: Option<String>,
    t: f32,
    dur: f32,
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

    /// 首帧淡入不依赖旧图：无 prev 时 alpha 从 0→1（血泪 #5）
    fn draw(&self, canvas: &mut Canvas<Window>, bank: &mut TextureBank) -> Result<(), String> {
        let p = if self.dur <= 0.0 { 1.0 } else { (self.t / self.dur).min(1.0) };
        if let Some(prev) = &self.prev {
            bank.draw_full_alpha(canvas, prev, 1.0 - p)?;
        }
        if let Some(cur) = &self.cur {
            bank.draw_full_alpha(canvas, cur, p)?;
        }
        Ok(())
    }
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct StageSnap {
    pub bg: Option<String>,
    pub chars: [Option<String>; 3],
    pub cg: Option<String>,
}

pub struct Stage {
    pub bg: Layer,
    pub chars: [Layer; 3],
    pub cg: Layer,
}

impl Default for Stage {
    fn default() -> Self {
        Self { bg: Layer::default(), chars: Default::default(), cg: Layer::default() }
    }
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
            cg: self.cg.cur().cloned(),
        }
    }

    /// 存档恢复：0ms 直切（无渐变）
    pub fn restore(&mut self, snap: StageSnap) {
        self.bg.set(snap.bg, 0);
        for (i, path) in snap.chars.into_iter().enumerate() {
            self.chars[i].set(path, 0);
        }
        self.cg.set(snap.cg, 0);
    }

    /// 当前引用路径集合（场景资源回收 keep 集）
    pub fn collect_paths(&self, out: &mut std::collections::HashSet<String>) {
        for layer in std::iter::once(&self.bg).chain(self.chars.iter()).chain(std::iter::once(&self.cg)) {
            if let Some(p) = &layer.cur {
                out.insert(p.clone());
            }
            if let Some(p) = &layer.prev {
                out.insert(p.clone());
            }
        }
    }

    /// 底到顶：背景 → 立绘 0/1/2 → CG
    pub fn draw(&self, canvas: &mut Canvas<Window>, bank: &mut TextureBank) -> Result<(), String> {
        self.bg.draw(canvas, bank)?;
        for c in &self.chars {
            c.draw(canvas, bank)?;
        }
        self.cg.draw(canvas, bank)?;
        Ok(())
    }
}
