//! 仪式删档 UI（meta_delete_last）：三重确认（文案走 config.ritual_texts）
//! → 全屏渐白光页。确认后的真删（含 backup/ 备份）在 systems 触发，M3 slots 接入。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::config::{Config, LOGICAL_H, LOGICAL_W};
use crate::text::font::FontBook;
use crate::ui::inputbox::button;

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    conf: &Config,
    step: usize,
    fade: f32,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);

    // 压暗底
    canvas.set_draw_color(Color::RGBA(0, 0, 0, 170));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, LOGICAL_H))?;

    // 步骤文案（数据侧 ritual_texts，缺失回退通用默认）
    let texts: Vec<&str> = if conf.game.ritual_texts.len() >= 3 {
        conf.game.ritual_texts.iter().map(|s| s.as_str()).collect()
    } else {
        vec!["真的要删除吗？", "再想一下，删除就回不来了。", "最后确认。亲手，删掉它。"]
    };
    let text = texts[step.min(texts.len() - 1)];
    let tex = fonts.render_text(34, Color::RGB(255, 224, 160), text)?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new((LOGICAL_W - q.width) as i32 / 2, 250, q.width, q.height)))?;

    // 三段进度点
    for i in 0..3 {
        let lit = i <= step;
        canvas.set_draw_color(if lit { Color::RGB(255, 200, 120) } else { Color::RGB(90, 90, 110) });
        canvas.fill_rect(Rect::new(LOGICAL_W as i32 / 2 - 44 + i as i32 * 44, 320, 24, 8))?;
    }

    button(canvas, fonts, Rect::new(LOGICAL_W as i32 / 2 - 220, 420, 200, 62), "「删除」", true)?;
    button(canvas, fonts, Rect::new(LOGICAL_W as i32 / 2 + 20, 420, 200, 62), "「回头」", false)?;

    // 渐白光页（第三击后 fade 0→1）
    if fade > 0.0 {
        let a = (fade * 255.0).min(255.0) as u8;
        canvas.set_draw_color(Color::RGBA(255, 255, 255, a));
        canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, LOGICAL_H))?;
    }
    Ok(())
}
