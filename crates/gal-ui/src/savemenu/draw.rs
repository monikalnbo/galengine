//! 存档/读档界面绘制

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use super::cache::ThumbCache;
use super::{cell_rect, close_rect};
use crate::widgets::button;
use gal_config::LOGICAL_W;
use gal_save::meta::FakeSave;
use gal_save::slots::SaveEntry;
use gal_text::font::FontBook;

const FONT: u16 = 22;
const BIG: u16 = 30;

/// mode_save=true 存档（标题「存档」），false 读档
#[allow(clippy::too_many_arguments)]
pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    thumbs: &ThumbCache,
    entries: &[(usize, Option<SaveEntry>)],
    fake: &[FakeSave],
    corrupt: &[usize],
    mode_save: bool,
    sel: usize,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Color::RGBA(4, 6, 16, 225));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 720))?;

    let title = if mode_save { "存档" } else { "读档" };
    let tex = fonts.render_text(BIG, Color::RGB(230, 234, 244), title)?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new(76, 60, q.width, q.height)))?;

    for (i, (slot, e)) in entries.iter().enumerate() {
        let r = cell_rect(i);
        draw_cell(
            canvas,
            fonts,
            thumbs,
            &format!("s{slot}"),
            r,
            i == sel,
            |fonts, canvas| match e {
                Some(e) => {
                    let corrupt = corrupt.contains(slot);
                    let color =
                        if corrupt { Color::RGB(255, 120, 120) } else { Color::RGB(200, 206, 220) };
                    let name = if corrupt {
                        format!("槽{slot} 破損")
                    } else {
                        format!("槽{slot} {}", e.stamp)
                    };
                    let tex = fonts.render_text(FONT, color, &name)?;
                    let q = tex.query();
                    canvas.copy(
                        tex,
                        None,
                        Some(Rect::new(r.x + 190, r.y + 18, q.width, q.height)),
                    )?;
                    let mut title = e.title.clone();
                    if corrupt {
                        title = "▓▓▓▓▓▓".into();
                    }
                    let shown: String = title.chars().take(10).collect();
                    let tex = fonts.render_text(FONT, Color::RGB(170, 176, 192), &shown)?;
                    let q = tex.query();
                    canvas.copy(tex, None, Some(Rect::new(r.x + 190, r.y + 52, q.width, q.height)))
                }
                None => {
                    let tex = fonts.render_text(
                        FONT,
                        Color::RGB(110, 116, 132),
                        &format!("槽{slot} —— 空"),
                    )?;
                    let q = tex.query();
                    canvas.copy(tex, None, Some(Rect::new(r.x + 190, r.y + 30, q.width, q.height)))
                }
            },
        )?;
    }
    // 不明档（追加展示，不可读）
    for (i, f) in fake.iter().enumerate() {
        let r = cell_rect(entries.len() + i);
        draw_cell(
            canvas,
            fonts,
            thumbs,
            &format!("f{i}"),
            r,
            entries.len() + i == sel,
            |fonts, canvas| {
                let tex = fonts.render_text(FONT, Color::RGB(255, 200, 120), "不明存档")?;
                let q = tex.query();
                canvas.copy(tex, None, Some(Rect::new(r.x + 190, r.y + 18, q.width, q.height)))?;
                let tex = fonts.render_text(
                    FONT,
                    Color::RGB(190, 160, 130),
                    &format!("{} {}", f.date, f.time),
                )?;
                let q = tex.query();
                canvas.copy(tex, None, Some(Rect::new(r.x + 190, r.y + 52, q.width, q.height)))
            },
        )?;
    }

    button(canvas, fonts, close_rect(), "关闭 Esc", false)?;
    Ok(())
}

/// 单元格：缩略图 168×94 + 右侧信息（info 闭包）
fn draw_cell<F>(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    thumbs: &ThumbCache,
    key: &str,
    r: Rect,
    hot: bool,
    info: F,
) -> Result<(), String>
where
    F: FnOnce(&mut FontBook, &mut Canvas<Window>) -> Result<(), String>,
{
    canvas.set_draw_color(if hot {
        Color::RGBA(60, 80, 140, 230)
    } else {
        Color::RGBA(18, 22, 40, 230)
    });
    canvas.fill_rect(r)?;
    canvas.set_draw_color(if hot {
        Color::RGBA(150, 200, 255, 230)
    } else {
        Color::RGBA(110, 120, 150, 120)
    });
    canvas.draw_rect(r)?;
    let tr = Rect::new(r.x + 12, r.y + (r.height() as i32 - 94) / 2, 168, 94);
    canvas.set_draw_color(Color::RGB(8, 10, 20));
    canvas.fill_rect(tr)?;
    if let Some(tex) = thumbs.get(key) {
        canvas.copy(tex, None, Some(tr))?;
    }
    info(fonts, canvas)
}
