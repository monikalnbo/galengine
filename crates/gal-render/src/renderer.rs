//! 渲染器：1280×720 离屏游戏画面 + 16:9 letterbox 居中呈现。
//! 越界层/抖动在 render.rs 组装阶段处理（贴出后、present 前以窗口坐标画）。

use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};

use gal_config::{LOGICAL_H, LOGICAL_W};

pub struct Renderer<'a> {
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

    /// 16:9 letterbox 目标矩形（居中黑边），可带抖动偏移（window_fx shake）
    pub fn letterbox(canvas: &Canvas<Window>, dx: i32, dy: i32) -> Rect {
        let (ww, wh) = canvas.output_size().unwrap_or((LOGICAL_W, LOGICAL_H));
        let scale = (ww as f32 / LOGICAL_W as f32).min(wh as f32 / LOGICAL_H as f32);
        let w = ((LOGICAL_W as f32 * scale).round() as u32).max(1);
        let h = ((LOGICAL_H as f32 * scale).round() as u32).max(1);
        Rect::new(((ww - w) / 2) as i32 + dx, ((wh - h) / 2) as i32 + dy, w, h)
    }

    /// 窗口坐标 → 逻辑坐标（鼠标命中测试）
    pub fn to_logical(canvas: &Canvas<Window>, x: i32, y: i32) -> (f32, f32) {
        let r = Self::letterbox(canvas, 0, 0);
        (
            (x - r.x) as f32 / r.width() as f32 * LOGICAL_W as f32,
            (y - r.y) as f32 / r.height() as f32 * LOGICAL_H as f32,
        )
    }

    /// 游戏内容画进离屏 → letterbox 贴窗（黑底）。不 present——越界层等窗口层绘制由 render.rs 接续后自行 present
    pub fn compose<F>(
        &mut self,
        canvas: &mut Canvas<Window>,
        dx: i32,
        dy: i32,
        draw_game: F,
    ) -> Result<(), String>
    where
        F: FnOnce(&mut Canvas<Window>) -> Result<(), String>,
    {
        let mut game_err: Option<String> = None;
        canvas
            .with_texture_canvas(&mut self.screen, |tc| {
                tc.set_draw_color(Color::BLACK);
                tc.clear();
                if let Err(e) = draw_game(tc) {
                    game_err = Some(e);
                }
            })
            .map_err(|e| format!("离屏渲染失败：{e:?}"))?;

        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        canvas.copy(&self.screen, None, Some(Self::letterbox(canvas, dx, dy)))?;
        game_err.map_or(Ok(()), Err)
    }

    /// 存档缩略图：读离屏画面（须在 present 前调用）
    pub fn read_screen(&mut self, canvas: &mut Canvas<Window>) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        canvas
            .with_texture_canvas(&mut self.screen, |tc| {
                out = tc.read_pixels(None, PixelFormatEnum::RGBA8888).unwrap_or_default();
            })
            .map_err(|e| format!("读取画面失败：{e:?}"))?;
        Ok(out)
    }
}
