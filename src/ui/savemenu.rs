//! 存读档界面：9 槽网格（缩略图+时间），含 meta 演出（假档/破損）。
//! 布局在 geom，配色在 theme，本文件只做组装与数据呈现。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::config::LOGICAL_W;
use crate::gfx::assets::TextureBank;
use crate::save::meta::MetaState;
use crate::save::slots::SaveData;
use crate::text::font::FontBook;
use crate::ui::theme;
use crate::ui::widgets;

/// 9 槽网格（geom::grid 的特化）
pub fn cells() -> Vec<Rect> {
    crate::ui::geom::grid((88, 150), 3, 3, (330, 196), 36)
}

pub fn hit_test(x: f32, y: f32) -> Option<usize> {
    crate::ui::geom::hit(&cells(), x, y)
}

/// 单槽绘制（数据→视觉映射，含假档/破損）
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub fn draw_cell(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    bank: &mut TextureBank,
    slots: &[Option<SaveData>],
    save_dir: &str,
    meta: &MetaState,
    i: usize,
    hot: bool,
    mode_save: bool,
) -> Result<(), String> {
    let r = cells()[i];
    let slot = i + 1;
    let data: Option<&SaveData> = slots.get(i).and_then(|d| d.as_ref());
    let is_fake = !mode_save && meta.fake_save.is_some() && slot == 9;
    let corrupted = meta.is_corrupted(slot);

    widgets::panel(canvas, r, hot)?;

    if is_fake {
        let f = meta.fake_save.as_ref().unwrap();
        canvas.set_draw_color(theme::C_LOCK);
        canvas.fill_rect(r)?;
        widgets::text_center(canvas, fonts, theme::FS_ITEM, theme::C_TEXT_DIM, "不明存档", r.y + 52)?;
        widgets::text_center(
            canvas,
            fonts,
            theme::FS_SMALL,
            theme::C_TEXT_FAINT,
            &format!("{} {}", f.date, f.time),
            r.y + 108,
        )?;
    } else if let Some(d) = &data {
        let thumb = format!("{save_dir}/{}", d.thumb);
        let _ = bank.load(&thumb);
        if let Some(tex) = bank.get(&thumb) {
            let _ = canvas.copy(tex, None, Some(r));
        }
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 175));
        canvas.fill_rect(Rect::new(r.x, r.bottom() - 34, r.width(), 34))?;
        widgets::text_left(
            canvas,
            fonts,
            theme::FS_TINY,
            theme::C_TEXT,
            &format!("{} {}", d.chapter, d.saved_at),
            r.x + 10,
            r.bottom() - 29,
        )?;
    } else {
        widgets::text_center(
            canvas,
            fonts,
            theme::FS_SMALL,
            theme::C_TEXT_FAINT,
            "—— 空 ——",
            r.y + (r.height() as i32 - 30) / 2,
        )?;
    }

    if corrupted {
        canvas.set_draw_color(Color::RGBA(140, 20, 30, 110));
        canvas.fill_rect(r)?;
        widgets::text_center(canvas, fonts, 34, theme::C_DANGER, "破 損", r.y + 72)?;
    }
    Ok(())
}

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    bank: &mut TextureBank,
    slots: &[Option<SaveData>],
    save_dir: &str,
    meta: &MetaState,
    mode_save: bool,
    selected: usize,
) -> Result<(), String> {
    canvas.set_draw_color(theme::C_BG);
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 720))?;
    widgets::text_center(
        canvas,
        fonts,
        theme::FS_TITLE,
        theme::C_TEXT,
        if mode_save { "存 档" } else { "读 档" },
        66,
    )?;
    for i in 0..9 {
        draw_cell(canvas, fonts, bank, slots, save_dir, meta, i, i == selected, mode_save)?;
    }
    widgets::text_center(
        canvas,
        fonts,
        theme::FS_SMALL,
        theme::C_TEXT_FAINT,
        if mode_save { "选择格子存档 ｜ Esc 返回" } else { "选择格子读档 ｜ Esc 返回" },
        660,
    )?;
    Ok(())
}
