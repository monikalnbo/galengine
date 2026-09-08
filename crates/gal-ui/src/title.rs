//! 标题画面：ui.layers.title_screen（YAML 图层叠加）优先；
//! 无定义回退内置绘制（conf.ui.title 驱动）。演化纱/菜单始终叠加。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::{layers, menu};
use gal_config::{Config, LOGICAL_H, LOGICAL_W};
use gal_render::assets::TextureBank;
use gal_script::vars::Vars;
use gal_text::font::FontBook;

/// 标题背景图路径（bgimage/title.jpg|png，存在才用；场景 GC 保留集）
pub fn bg_path() -> Option<String> {
    let data = gal_config::data_dir();
    ["jpg", "png"]
        .iter()
        .map(|ext| format!("{data}/bgimage/title.{ext}"))
        .find(|p| std::path::Path::new(p).exists())
}

/// evolve 阶段 → (标题字号缩放, 标题透明度, 全屏纱: (色, 透明度))
fn evolve_fx(stage: &str) -> (f32, u8, (Color, u8)) {
    match stage {
        "" => (1.0, 255, (Color::RGB(0, 0, 0), 110)), // 常规压暗纱，保文字可读
        "fade" => (1.0, 140, (Color::RGB(0, 0, 0), 175)), // 褪色：更暗
        "green" => (1.0, 200, (Color::RGB(30, 120, 70), 150)), // 变绿：绿色纱
        "small" => (0.42, 170, (Color::RGB(0, 0, 0), 205)), // 小字残题：近黑
        _ => (1.0, 200, (Color::RGB(0, 0, 0), 160)),  // 未知阶段：轻微褪色
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    bank: &mut TextureBank,
    conf: &Config,
    vars: &Vars,
    evolve: &str,
    sel: usize,
    now_ms: f32,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    let t = &conf.ui.title;

    // 背景+标题文字：YAML 图层定义优先
    let custom = conf.ui.layers.get("title_screen");
    match custom.filter(|v| !v.is_empty()) {
        Some(defs) => layers::draw_layers(canvas, fonts, bank, conf, defs, vars)?,
        None => fallback(canvas, fonts, bank, conf)?,
    }

    // 演化纱（整屏，作用于背景/图层之上）
    let (scale, alpha, (veil, veil_a)) = evolve_fx(evolve);
    canvas.set_draw_color(Color::RGBA(veil.r, veil.g, veil.b, veil_a));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, LOGICAL_H))?;

    // 大标题（YAML 图层模式已随图层绘制，此处仅回退模式需要）
    if custom.filter(|v| !v.is_empty()).is_none() {
        let size = (t.title_font_size as f32 * scale) as u16;
        let title = conf.meta.title.clone();
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
        let y = if size > 60 { t.title_text_y } else { t.title_text_y + 60 };
        canvas.copy(
            tex,
            None,
            Some(Rect::new((LOGICAL_W - q.width) as i32 / 2, y, q.width, q.height)),
        )?;

        // 呼吸提示
        let a = (150.0 + 100.0 * (now_ms / 900.0).sin()) as u8;
        let tex = fonts.render_text(22, Color::RGBA(170, 178, 200, a), &t.hint_text)?;
        let q = tex.query();
        canvas.copy(
            tex,
            None,
            Some(Rect::new((LOGICAL_W - q.width) as i32 / 2, t.hint_y, q.width, q.height)),
        )?;
    }

    menu::draw(canvas, fonts, &crate::overlay::title_items(), sel, t.menu_top_y, false)?;
    Ok(())
}

/// 内置回退：背景图（配置 → 自动探测 → 深色渐变）
fn fallback(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    bank: &mut TextureBank,
    conf: &Config,
) -> Result<(), String> {
    let path = conf
        .ui
        .title
        .background
        .clone()
        .map(|p| format!("{}/{p}", gal_config::data_dir()))
        .or_else(bg_path);
    match path {
        Some(p) if std::path::Path::new(&p).exists() => {
            bank.draw_scaled(canvas, &p, Rect::new(0, 0, LOGICAL_W, LOGICAL_H), 1.0)
        }
        _ => {
            let _ = fonts; // 渐变无需字体
            canvas.set_draw_color(Color::RGB(6, 8, 20));
            canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, LOGICAL_H / 2))?;
            canvas.set_draw_color(Color::RGB(12, 14, 30));
            let _ = canvas.fill_rect(Rect::new(0, LOGICAL_H as i32 / 2, LOGICAL_W, LOGICAL_H / 2));
            Ok(())
        }
    }
}
