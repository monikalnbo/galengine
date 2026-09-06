//! 名字输入框：提示语 + 键入区（SDL_TEXTINPUT/Backspace）+ 预设名快捷按钮 + 确认。
//! IME 不可用环境的兜底=预设名点选（docs/20 §3.5）。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::config::LOGICAL_W;
use crate::text::font::FontBook;

const FONT: u16 = 30;
const SMALL: u16 = 24;

pub struct InputUi {
    #[allow(dead_code)]
    pub var: String,
    pub prompt: String,
    pub width: u32,
    pub default: String,
    pub buf: String,
    /// 预设候选（var 相关）
    pub presets: Vec<String>,
    /// 光标闪烁计时
    pub blink: f32,
}

impl InputUi {
    /// presets 由游戏侧配置提供（game.input_presets）；空则不显示预设按钮
    pub fn new(var: &str, prompt: &str, width: u32, default: &str, presets: Vec<String>) -> Self {
        Self {
            var: var.to_string(),
            prompt: prompt.to_string(),
            width,
            default: default.to_string(),
            buf: default.to_string(),
            presets,
            blink: 0.0,
        }
    }

    /// 键入一个字符（TEXTINPUT）
    pub fn push_char(&mut self, c: &str) {
        if self.buf.chars().count() < 12 {
            self.buf.push_str(c);
        }
    }

    pub fn backspace(&mut self) {
        self.buf.pop();
    }

    /// 有效值：空则默认值
    pub fn value(&self) -> String {
        let t = self.buf.trim();
        if t.is_empty() { self.default.clone() } else { t.to_string() }
    }
}

/// 命中区域
pub enum Hit {
    Preset(usize),
    Confirm,
}

fn box_rect(w: &InputUi) -> Rect {
    let bw = w.width.max(360);
    Rect::new(
        (LOGICAL_W as i32 - bw as i32) / 2,
        320,
        bw,
        70,
    )
}

fn preset_rect(i: usize) -> Rect {
    let n = 3;
    let gap = 24;
    let total = n as i32 * 150 + (n - 1) as i32 * gap;
    let x0 = (LOGICAL_W as i32 - total) / 2;
    Rect::new(x0 + i as i32 * (150 + gap), 430, 150, 54)
}

fn confirm_rect() -> Rect {
    Rect::new((LOGICAL_W - 260) as i32 / 2, 520, 260, 60)
}

pub fn hit_test(w: &InputUi, x: f32, y: f32) -> Option<Hit> {
    for i in 0..w.presets.len() {
        if preset_rect(i).contains_point((x as i32, y as i32)) {
            return Some(Hit::Preset(i));
        }
    }
    if confirm_rect().contains_point((x as i32, y as i32)) {
        return Some(Hit::Confirm);
    }
    None
}

pub fn draw(canvas: &mut Canvas<Window>, fonts: &mut FontBook, w: &InputUi) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);

    // 提示语
    let tex = fonts.render_text(FONT, Color::RGB(235, 235, 240), &w.prompt)?;
    let q = tex.query();
    canvas.copy(
        tex,
        None,
        Some(centered(q.width, q.height, 220)),
    )?;

    // 输入框 + 键入内容（光标闪烁）
    let br = box_rect(w);
    canvas.set_draw_color(Color::RGBA(10, 12, 26, 225));
    canvas.fill_rect(br)?;
    canvas.set_draw_color(Color::RGBA(150, 200, 255, 170));
    canvas.draw_rect(br)?;
    let shown = if w.buf.is_empty() { w.default.as_str() } else { w.buf.as_str() };
    let (col, text) = if w.buf.is_empty() {
        (Color::RGB(120, 128, 150), shown.to_string())
    } else {
        (Color::RGB(255, 255, 255), shown.to_string())
    };
    let tex = fonts.render_text(FONT, col, &text)?;
    let q = tex.query();
    canvas.copy(tex, None, Some(centered(q.width, q.height, br.y + (br.height() as i32 - q.height as i32) / 2)))?;
    if (w.blink / 500.0) as i32 % 2 == 0 {
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.fill_rect(Rect::new(
            br.x + 30 + q.width as i32 + 4,
            br.y + 18,
            3,
            br.height() - 36,
        ))?;
    }

    // 预设按钮
    for (i, name) in w.presets.iter().enumerate() {
        let r = preset_rect(i);
        canvas.set_draw_color(Color::RGBA(28, 34, 58, 220));
        canvas.fill_rect(r)?;
        canvas.set_draw_color(Color::RGBA(130, 150, 200, 150));
        canvas.draw_rect(r)?;
        let tex = fonts.render_text(SMALL, Color::RGB(210, 216, 232), name)?;
        let q = tex.query();
        canvas.copy(
            tex,
            None,
            Some(centered(q.width, q.height, r.y + (r.height() as i32 - q.height as i32) / 2)),
        )?;
    }

    // 确认按钮
    let r = confirm_rect();
    canvas.set_draw_color(Color::RGBA(52, 74, 128, 235));
    canvas.fill_rect(r)?;
    canvas.set_draw_color(Color::RGBA(160, 200, 255, 220));
    canvas.draw_rect(r)?;
    let tex = fonts.render_text(FONT, Color::RGB(255, 255, 255), "—— 好 ——")?;
    let q = tex.query();
    canvas.copy(tex, None, Some(centered(q.width, q.height, r.y + (r.height() as i32 - q.height as i32) / 2)))?;

    // 小字说明
    let hint = "直接键入（支持输入法）· 或点选预设名";
    let tex = fonts.render_text(SMALL, Color::RGB(140, 148, 170), hint)?;
    let q = tex.query();
    canvas.copy(tex, None, Some(centered(q.width, q.height, 610)))?;
    Ok(())
}

fn centered(w: u32, h: u32, y: i32) -> Rect {
    Rect::new((LOGICAL_W as i32 - w as i32) / 2, y, w, h)
}
