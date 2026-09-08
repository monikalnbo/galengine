//! timer_choice：倒计时选项。进入选项态启动倒计时，超时自动选择含 pick 文字的
//! 选项（默认「迟疑」），找不到回退第 0 项。顶部绘制剩余时间条。

use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use gal_config::{ExtCfg, LOGICAL_W};
use gal_ext::{ExtContext, ExtResult, Extension};
use gal_script::interp::RunState;
use gal_text::font::FontBook;

pub struct TimerChoice {
    total_ms: f32,
    pick: String,
    /// 剩余毫秒；None=不在选项态
    left_ms: Option<f32>,
}

impl TimerChoice {
    pub fn new(c: &ExtCfg) -> Self {
        Self { total_ms: c.default_ms.max(1) as f32, pick: c.pick.clone(), left_ms: None }
    }
}

impl Extension for TimerChoice {
    fn name(&self) -> &str {
        "timer_choice"
    }

    fn tick(&mut self, dt_ms: f32, ctx: &mut ExtContext) -> ExtResult {
        let items = match ctx.interp.state.clone() {
            RunState::WaitChoice { items, .. } => items,
            _ => {
                self.left_ms = None; // 离开选项态重置
                return ExtResult::Continue;
            }
        };
        // 进入选项态：起表
        let left = *self.left_ms.get_or_insert(self.total_ms) - dt_ms;
        *self.left_ms.as_mut().unwrap() = left;
        if left <= 0.0 {
            let idx = items.iter().position(|(text, _)| text.contains(&self.pick)).unwrap_or(0);
            let _ = ctx.interp.choose(idx); // 已离开 WaitChoice 时的保护
            self.left_ms = None;
        }
        ExtResult::Continue
    }

    fn on_click(&mut self, _x: f32, _y: f32, _ctx: &mut ExtContext) -> ExtResult {
        ExtResult::Continue // 正常选项点击即可，tick 里检测状态切换自动重置
    }

    fn on_key(&mut self, _key: Keycode, _ctx: &mut ExtContext) -> ExtResult {
        ExtResult::Continue
    }

    fn draw(&self, canvas: &mut Canvas<Window>, _fonts: &mut FontBook) -> Result<(), String> {
        let Some(left) = self.left_ms else { return Ok(()) };
        let pct = (left / self.total_ms).clamp(0.0, 1.0);
        canvas.set_blend_mode(BlendMode::Blend);
        // 轨道
        canvas.set_draw_color(Color::RGBA(20, 24, 40, 200));
        canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 6))?;
        // 剩余：<50% 琥珀，<25% 红
        let color = if pct < 0.25 {
            Color::RGB(235, 80, 80)
        } else if pct < 0.5 {
            Color::RGB(235, 180, 90)
        } else {
            Color::RGB(120, 190, 130)
        };
        canvas.set_draw_color(color);
        canvas.fill_rect(Rect::new(0, 0, (LOGICAL_W as f32 * pct) as u32, 6))?;
        Ok(())
    }
}
