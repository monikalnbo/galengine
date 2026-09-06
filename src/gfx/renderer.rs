//! 渲染器：管理 1280x720 离屏游戏画面与 letterbox 呈现。
//!
//! 架构：每帧把游戏内容整体画进离屏纹理 `screen`（闭包内 1:1 逻辑坐标），
//! 再按窗口尺寸以 16:9 letterbox 贴出（居中，四周黑边）。
//! 越界层（reach，docs/20 §3.9）今后在贴出之后、present 之前
//! 直接以窗口坐标绘制，可突破黑边——预留 draw_reach()。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};

use crate::config::{LOGICAL_H, LOGICAL_W};

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

    /// 渲染并呈现一帧：
    /// 1. `draw_game` 闭包以 1:1 逻辑坐标画进离屏画面
    /// 2. 离屏画面 letterbox 贴到窗口（窗口区刷黑作黑边）并 present
    pub fn present_frame<F>(
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

        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        canvas.copy(&self.screen, None, Some(Self::letterbox(canvas)))?;
        // 越界层 reach：此处预留（贴出之后、present 之前画窗口坐标）
        canvas.present();

        if let Some(e) = game_err {
            return Err(e);
        }
        Ok(())
    }
}
