//! 名字输入框：SDL_TEXTINPUT（IME 透传）+ 预设名按钮兜底 + 确认。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::config::{Config, LOGICAL_W};
use crate::script::interp::InputSpec;
use crate::text::font::FontBook;

const FONT: u16 = 30;
const SMALL: u16 = 24;

pub struct InputUi {
    pub spec: InputSpec,
    pub buf: String,
    pub presets: Vec<String>,
    pub blink: f32,
}

/// 预设名查 config.game.input_presets：按变量基名（heroName→hero）；
/// 查不到给任一组兜底（数据侧没配时不至于空按钮）
pub fn presets_for(conf: &Config, var: &str) -> Vec<String> {
    let base = var.trim_start_matches("sf.").trim_start_matches("f.");
    conf.game
        .input_presets
        .get(base)
        .cloned()
        .or_else(|| conf.game.input_presets.values().next().cloned())
        .unwrap_or_default()
}

impl InputUi {
    pub fn new(spec: InputSpec, presets: Vec<String>) -> Self {
        Self { buf: spec.default.clone(), spec, presets, blink: 0.0 }
    }

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
        if t.is_empty() { self.spec.default.clone() } else { t.to_string() }
    }
}

pub enum Hit {
    Preset(usize),
    Confirm,
}

fn box_rect(w: &InputUi) -> Rect {
    let bw = w.spec.width.max(360);
    Rect::new((LOGICAL_W as i32 - bw as i32) / 2, 270, bw, 70)
}

fn preset_rect(i: usize) -> Rect {
    let gap = 24;
    let total = 3 * 150 + 2 * gap;
    let x0 = (LOGICAL_W as i32 - total) / 2;
    Rect::new(x0 + i as i32 * (150 + gap), 400, 150, 54)
}

fn confirm_rect() -> Rect {
    Rect::new((LOGICAL_W - 260) as i32 / 2, 480, 260, 60)
}

pub fn hit_test(w: &InputUi, x: f32, y: f32) -> Option<Hit> {
    for i in 0..w.presets.len() {
        if preset_rect(i).contains_point((x as i32, y as i32)) {
            return Some(Hit::Preset(i));
        }
    }
    confirm_rect().contains_point((x as i32, y as i32)).then_some(Hit::Confirm)
}

pub fn draw(canvas: &mut Canvas<Window>, fonts: &mut FontBook, w: &InputUi) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);

    // 压暗纱：隔开游戏画面，亮背景下元素不再看不清（v0.1.1）
    canvas.set_draw_color(Color::RGBA(0, 0, 0, 160));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 720))?;

    // 提示语
    let tex = fonts.render_text(FONT, Color::RGB(235, 235, 240), &w.spec.prompt)?;
    let q = tex.query();
    canvas.copy(tex, None, Some(centered(q.width, q.height, 180)))?;

    // 输入框（光标闪烁）
    let br = box_rect(w);
    canvas.set_draw_color(Color::RGBA(10, 12, 26, 225));
    canvas.fill_rect(br)?;
    canvas.set_draw_color(Color::RGBA(150, 200, 255, 170));
    canvas.draw_rect(br)?;
    let (col, text) = if w.buf.is_empty() {
        (Color::RGB(120, 128, 150), w.spec.default.clone())
    } else {
        (Color::RGB(255, 255, 255), w.buf.clone())
    };
    let tex = fonts.render_text(FONT, col, &text)?;
    let q = tex.query();
    canvas.copy(
        tex,
        None,
        Some(centered(q.width, q.height, br.y + (br.height() as i32 - q.height as i32) / 2)),
    )?;
    if (w.blink / 500.0) as i32 % 2 == 0 {
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.fill_rect(Rect::new(br.x + 30 + q.width as i32 + 4, br.y + 18, 3, br.height() - 36))?;
    }

    // 预设按钮
    for (i, name) in w.presets.iter().enumerate() {
        button(canvas, fonts, preset_rect(i), name, false)?;
    }

    // 确认按钮
    button(canvas, fonts, confirm_rect(), "—— 好 ——", true)?;

    // 小字说明
    let tex = fonts.render_text(SMALL, Color::RGB(140, 148, 170), "直接键入（支持输入法）· 或点选预设名")?;
    let q = tex.query();
    canvas.copy(tex, None, Some(centered(q.width, q.height, 575)))
}

/// 半透明按钮（choice/预设/确认/菜单通用原语）
pub fn button(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    r: Rect,
    label: &str,
    accent: bool,
) -> Result<(), String> {
    canvas.set_draw_color(if accent { Color::RGBA(52, 74, 128, 235) } else { Color::RGBA(28, 34, 58, 220) });
    canvas.fill_rect(r)?;
    canvas.set_draw_color(if accent { Color::RGBA(160, 200, 255, 220) } else { Color::RGBA(130, 150, 200, 150) });
    canvas.draw_rect(r)?;
    let tex = fonts.render_text(FONT, if accent { Color::RGB(255, 255, 255) } else { Color::RGB(210, 216, 232) }, label)?;
    let q = tex.query();
    canvas.copy(
        tex,
        None,
        Some(centered(q.width, q.height, r.y + (r.height() as i32 - q.height as i32) / 2)),
    )
}

fn centered(w: u32, h: u32, y: i32) -> Rect {
    Rect::new((LOGICAL_W as i32 - w as i32) / 2, y, w, h)
}
