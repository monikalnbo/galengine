//! 标题画面：背景图（bgimage/title.*，缺省回退深色渐变）+ 大标题 + 菜单；
//! title_evolve 演化作用于整屏（褪色/变绿/小字，效果关键词为引擎规格定义）。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::config::{Config, LOGICAL_H, LOGICAL_W};
use crate::gfx::assets::TextureBank;
use crate::text::font::FontBook;
use crate::ui::menu;

/// 标题背景图路径（bgimage/title.jpg|png，存在才用）
pub fn bg_path() -> Option<String> {
    let data = crate::config::data_dir();
    ["jpg", "png"]
        .iter()
        .map(|ext| format!("{data}/bgimage/title.{ext}"))
        .find(|p| std::path::Path::new(p).exists())
}

/// evolve 阶段 → (标题字号, 标题透明度, 全屏纱: (色, 透明度))
fn evolve_fx(stage: &str) -> (u16, u8, (Color, u8)) {
    match stage {
        "" => (96, 255, (Color::RGB(0, 0, 0), 110)),       // 常规压暗纱，保文字可读
        "fade" => (96, 140, (Color::RGB(0, 0, 0), 175)),   // 褪色：更暗
        "green" => (96, 200, (Color::RGB(30, 120, 70), 150)), // 变绿：绿色纱
        "small" => (40, 170, (Color::RGB(0, 0, 0), 205)),  // 小字残题：近黑
        _ => (96, 200, (Color::RGB(0, 0, 0), 160)),        // 未知阶段：轻微褪色
    }
}

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    bank: &mut TextureBank,
    conf: &Config,
    evolve: &str,
    sel: usize,
    now_ms: f32,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);

    // 背景：title 图（整幅铺满）→ 无图回退两段深色渐变
    match bg_path() {
        Some(path) => bank.draw_scaled(canvas, &path, Rect::new(0, 0, LOGICAL_W, LOGICAL_H), 1.0)?,
        None => {
            canvas.set_draw_color(Color::RGB(6, 8, 20));
            canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, LOGICAL_H / 2))?;
            canvas.set_draw_color(Color::RGB(12, 14, 30));
            canvas.fill_rect(Rect::new(0, LOGICAL_H as i32 / 2, LOGICAL_W, LOGICAL_H / 2))?;
        }
    }

    // 演化纱（整屏，作用于图与渐变之上）
    let (size, alpha, (veil, veil_a)) = evolve_fx(evolve);
    canvas.set_draw_color(Color::RGBA(veil.r, veil.g, veil.b, veil_a));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, LOGICAL_H))?;

    // 大标题
    let title = conf.title.clone();
    let shown = if evolve == "small" {
        format!("{}……", title.chars().take(6).collect::<String>())
    } else {
        title.chars().take(14).collect()
    };
    let color = if evolve == "green" {
        Color::RGBA(140, 220, 170, alpha)
    } else {
        Color::RGBA(238, 240, 248, alpha)
    };
    let tex = fonts.render_text(size, color, &shown)?;
    let q = tex.query();
    let y = if size > 60 { 120 } else { 180 };
    canvas.copy(tex, None, Some(Rect::new((LOGICAL_W - q.width) as i32 / 2, y, q.width, q.height)))?;

    // 呼吸提示
    let a = (150.0 + 100.0 * (now_ms / 900.0).sin()) as u8;
    let tex = fonts.render_text(22, Color::RGBA(170, 178, 200, a), "— select —")?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new((LOGICAL_W - q.width) as i32 / 2, 330, q.width, q.height)))?;

    menu::draw(canvas, fonts, &crate::ui::overlay::title_items(), sel, 380, false)?;
    Ok(())
}
