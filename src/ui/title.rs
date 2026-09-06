//! 标题画面：大标题 + 菜单；title_evolve 演化（褪色/变绿/小字，
//! 效果关键词为引擎规格 §2.4 定义，非游戏内容）。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::config::{Config, LOGICAL_H, LOGICAL_W};
use crate::text::font::FontBook;
use crate::ui::menu;

/// evolve 阶段 → (字号, 透明度 0-1, 绿色叠加)
fn evolve_fx(stage: &str) -> (u16, u8, bool) {
    match stage {
        "" => (96, 255, false),
        "fade" => (96, 140, false),
        "green" => (96, 200, true),
        "small" => (40, 170, false),
        _ => (96, 200, false), // 未知阶段：轻微褪色
    }
}

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    conf: &Config,
    evolve: &str,
    sel: usize,
    now_ms: f32,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    // 夜空渐变底（两块矩形近似）
    canvas.set_draw_color(Color::RGB(6, 8, 20));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, LOGICAL_H / 2))?;
    canvas.set_draw_color(Color::RGB(12, 14, 30));
    canvas.fill_rect(Rect::new(0, LOGICAL_H as i32 / 2, LOGICAL_W, LOGICAL_H / 2))?;

    let (size, alpha, green) = evolve_fx(evolve);
    let title = conf.title.clone();
    let shown = if evolve == "small" {
        format!("{}……", title.chars().take(6).collect::<String>())
    } else {
        title.chars().take(14).collect()
    };
    let color = if green {
        Color::RGBA(140, 220, 170, alpha)
    } else {
        Color::RGBA(238, 240, 248, alpha)
    };
    let tex = fonts.render_text(size, color, &shown)?;
    let q = tex.query();
    let y = if size > 60 { 120 } else { 180 };
    canvas.set_blend_mode(BlendMode::Blend);
    canvas.copy(
        tex,
        None,
        Some(Rect::new((LOGICAL_W - q.width) as i32 / 2, y, q.width, q.height)),
    )?;

    // 呼吸提示
    let a = (150.0 + 100.0 * (now_ms / 900.0).sin()) as u8;
    let tex = fonts.render_text(22, Color::RGBA(170, 178, 200, a), "— select —")?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new((LOGICAL_W - q.width) as i32 / 2, 330, q.width, q.height)))?;

    menu::draw(canvas, fonts, &crate::ui::overlay::title_items(), sel, 380, false)?;
    Ok(())
}
