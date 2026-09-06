//! 渲染层：1280x720 离屏游戏画面 + letterbox 呈现 + 越界层。
//!
//! 两段式：`render_screen`（游戏内容画进离屏，闭包内 1:1 逻辑坐标）
//! → `present`（letterbox 贴窗口，越界层 reach 直画窗口坐标突破黑边）。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};

use crate::config::{LOGICAL_H, LOGICAL_W};

/// 越界层绘制参数（reach，docs/20 §3.9）：图片放大伸出游戏区、画进黑边
pub struct ReachDraw<'a, 't> {
    pub tex: &'a mut Texture<'t>,
    /// 0..1 动画进度（尺寸=窗口×scale×ease，透明度=ease）
    pub progress: f32,
    pub scale: f32,
}

pub struct Renderer<'a> {
    /// 游戏画面（整帧渲染目标，1280x720）
    screen: Texture<'a>,
}

impl<'a> Renderer<'a> {
    pub fn new(creator: &'a TextureCreator<WindowContext>) -> Result<Self, String> {
        let mut screen = creator
            .create_texture_target(None, LOGICAL_W, LOGICAL_H)
            .map_err(|e| format!("创建离屏画面失败：{e}"))?;
        screen.set_blend_mode(BlendMode::Blend);
        Ok(Self { screen })
    }

    /// 窗口内 16:9 letterbox 目标矩形（居中，四周黑边）
    pub fn letterbox(canvas: &Canvas<Window>) -> Rect {
        let (ww, wh) = canvas.output_size().unwrap_or((LOGICAL_W, LOGICAL_H));
        let scale = (ww as f32 / LOGICAL_W as f32).min(wh as f32 / LOGICAL_H as f32);
        let w = ((LOGICAL_W as f32 * scale).round() as u32).max(1);
        let h = ((LOGICAL_H as f32 * scale).round() as u32).max(1);
        Rect::new(((ww - w) / 2) as i32, ((wh - h) / 2) as i32, w, h)
    }

    /// 窗口坐标 -> 逻辑坐标（1280x720，鼠标命中测试用）
    pub fn to_logical(canvas: &Canvas<Window>, x: i32, y: i32) -> (f32, f32) {
        let r = Self::letterbox(canvas);
        let sx = (x - r.x) as f32 / r.width() as f32 * LOGICAL_W as f32;
        let sy = (y - r.y) as f32 / r.height() as f32 * LOGICAL_H as f32;
        (sx, sy)
    }

    /// 渲染并呈现一帧（无越界层的便捷入口）
    #[allow(dead_code)]
    pub fn present_frame<F>(
        &mut self,
        canvas: &mut Canvas<Window>,
        clear: Color,
        draw_game: F,
    ) -> Result<(), String>
    where
        F: FnOnce(&mut Canvas<Window>) -> Result<(), String>,
    {
        self.render_screen(canvas, clear, draw_game)?;
        self.present(canvas, None)
    }

    /// 第一步：游戏内容画进离屏画面（闭包内 1:1 逻辑坐标）
    pub fn render_screen<F>(
        &mut self,
        canvas: &mut Canvas<Window>,
        clear: Color,
        draw_game: F,
    ) -> Result<(), String>
    where
        F: FnOnce(&mut Canvas<Window>) -> Result<(), String>,
    {
        let mut game_err: Option<String> = None;
        canvas
            .with_texture_canvas(&mut self.screen, |tc| {
                tc.set_draw_color(clear);
                tc.clear();
                if let Err(e) = draw_game(tc) {
                    game_err = Some(e);
                }
            })
            .map_err(|e| format!("离屏渲染失败：{e:?}"))?;
        if let Some(e) = game_err {
            return Err(e);
        }
        Ok(())
    }

    /// 第二步：letterbox 贴到窗口（+越界层）并 present
    pub fn present(
        &self,
        canvas: &mut Canvas<Window>,
        reach: Option<ReachDraw<'_, '_>>,
    ) -> Result<(), String> {
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        canvas.copy(&self.screen, None, Some(Self::letterbox(canvas)))?;
        if let Some(r) = reach {
            let (ww, wh) = canvas.output_size().unwrap_or((1280, 720));
            let p = r.progress.clamp(0.0, 1.0);
            let ease = p * p * (3.0 - 2.0 * p); // smoothstep
            let w = ((ww as f32 * r.scale * ease) as u32).max(1);
            let h = ((wh as f32 * r.scale * ease) as u32).max(1);
            let x = (ww as i32 - w as i32) / 2;
            let y = (wh as i32 - h as i32) / 2;
            let a = if p >= 0.999 { 255 } else { (ease * 255.0) as u8 };
            r.tex.set_alpha_mod(a);
            let _ = canvas.copy(r.tex, None, Some(Rect::new(x, y, w, h)));
        }
        canvas.present();
        Ok(())
    }
}
