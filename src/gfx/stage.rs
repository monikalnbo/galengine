//! 演出层状态：背景 1 + 立绘 3 + CG 1，均支持 crossfade。
//! 所有素材为 1280x720 整幅画布（立绘透明 PNG），直接铺满。

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
    /// 切换内容（None=隐藏层）；同内容不重复切换
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
        if self.t >= self.dur {
            return; // 已稳定（含 dur=0 直切）
        }
        self.t += dt_ms;
        if self.t >= self.dur {
            self.prev = None; // 渐变完成，释放旧图
        }
    }

    fn draw(&self, canvas: &mut Canvas<Window>, bank: &mut TextureBank) -> Result<(), String> {
        let p = if self.dur <= 0.0 {
            1.0
        } else {
            (self.t / self.dur).min(1.0)
        };
        if let Some(prev) = &self.prev {
            bank.draw_full_alpha(canvas, prev, 1.0 - p)?;
        }
        if let Some(cur) = &self.cur {
            bank.draw_full_alpha(canvas, cur, p)?;
        }
        Ok(())
    }
}

pub struct Stage {
    pub bg: Layer,
    pub chars: [Layer; 3],
    pub cg: Layer,
}

impl Default for Stage {
    fn default() -> Self {
        Self {
            bg: Layer::default(),
            chars: [Layer::default(), Layer::default(), Layer::default()],
            cg: Layer::default(),
        }
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

    /// 收集所有层当前引用的路径（场景资源回收的 keep 集合）
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

    /// 从底到顶：背景 → 立绘 0/1/2 → CG
    pub fn draw(&self, canvas: &mut Canvas<Window>, bank: &mut TextureBank) -> Result<(), String> {
        self.bg.draw(canvas, bank)?;
        for c in &self.chars {
            c.draw(canvas, bank)?;
        }
        self.cg.draw(canvas, bank)?;
        Ok(())
    }
}
