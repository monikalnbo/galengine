//! CG 鉴赏：目录=data/bgimage/cg_*（命名约定=数据侧契约）；解锁记 sf.cgs；
//! 网格（锁定=剪影?）+ 全屏翻页。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::config::{self, LOGICAL_H, LOGICAL_W};
use crate::gfx::assets::TextureBank;
use crate::text::font::FontBook;

const COLS: usize = 4;
const ROWS: usize = 2;
const CELL_W: u32 = 280;
const CELL_H: u32 = 200;
const GAP: i32 = 20;
const FONT: u16 = 20;

/// 目录扫描：bgimage 下 cg_ 前缀素材（排序稳定）
pub fn catalog() -> Vec<String> {
    let dir = format!("{}/bgimage", config::data_dir());
    let mut out: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("cg_") && (n.ends_with(".jpg") || n.ends_with(".png")))
        .map(|n| n.trim_end_matches(".jpg").trim_end_matches(".png").to_string())
        .collect();
    out.sort();
    out.dedup();
    out
}

pub fn per_page() -> usize {
    COLS * ROWS
}

fn grid_rect(idx: usize) -> Rect {
    let row = idx / COLS;
    let col = idx % COLS;
    let total_w = COLS as i32 * CELL_W as i32 + (COLS as i32 - 1) * GAP;
    let x0 = (LOGICAL_W as i32 - total_w) / 2;
    Rect::new(
        x0 + col as i32 * (CELL_W as i32 + GAP),
        150 + row as i32 * (CELL_H as i32 + GAP),
        CELL_W,
        CELL_H,
    )
}

pub fn hit_test(page_len: usize, x: f32, y: f32) -> Option<usize> {
    (0..page_len).find(|&i| grid_rect(i).contains_point((x as i32, y as i32)))
}

/// 网格绘制（unlocked: cg 名集合；sel: 页内下标；page: 页码）
pub fn draw_grid(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    bank: &mut TextureBank,
    items: &[String],
    unlocked: &[String],
    page: usize,
    sel: usize,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Color::RGBA(4, 6, 16, 235));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, LOGICAL_H))?;

    let title = format!("CG 鉴赏  {}/{}", unlocked.len(), items.len());
    let tex = fonts.render_text(30, Color::RGB(230, 234, 244), &title)?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new(76, 70, q.width, q.height)))?;

    let start = page * per_page();
    for i in 0..per_page() {
        let Some(name) = items.get(start + i) else { break };
        let r = grid_rect(i);
        let open = unlocked.contains(name);
        let hot = i == sel;
        canvas.set_draw_color(if hot { Color::RGBA(60, 80, 140, 230) } else { Color::RGBA(18, 22, 40, 230) });
        canvas.fill_rect(r)?;
        canvas.set_draw_color(if hot { Color::RGBA(150, 200, 255, 230) } else { Color::RGBA(110, 120, 150, 120) });
        canvas.draw_rect(r)?;
        let img = Rect::new(r.x + 8, r.y + 8, r.width() - 16, (r.height() - 16) as u32 * 3 / 4);
        if open {
            bank.draw_scaled(canvas, &format!("{}/bgimage/{name}.jpg", config::data_dir()), img, 1.0)?;
        } else {
            // 锁定剪影：深色块 + ?
            canvas.set_draw_color(Color::RGB(10, 12, 22));
            canvas.fill_rect(img)?;
            let tex = fonts.render_text(40, Color::RGB(60, 66, 90), "?")?;
            let q = tex.query();
            canvas.copy(
                tex,
                None,
                Some(Rect::new(
                    img.x + (img.width() as i32 - q.width as i32) / 2,
                    img.y + (img.height() as i32 - q.height as i32) / 2,
                    q.width,
                    q.height,
                )),
            )?;
        }
        let label = if open { name.clone() } else { "— 未解锁 —".into() };
        let tex = fonts.render_text(FONT, if open { Color::RGB(200, 206, 220) } else { Color::RGB(110, 116, 132) }, &label)?;
        let q = tex.query();
        canvas.copy(
            tex,
            None,
            Some(Rect::new(r.x + 8, r.bottom() as i32 - 34, q.width, q.height)),
        )?;
    }
    Ok(())
}

/// 全屏鉴赏（大图 + 页码角标）
pub fn draw_full(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    bank: &mut TextureBank,
    name: &str,
    idx: usize,
    total: usize,
) -> Result<(), String> {
    bank.draw_scaled(canvas, &format!("{}/bgimage/{name}.jpg", config::data_dir()), Rect::new(0, 0, LOGICAL_W, LOGICAL_H), 1.0)?;
    let tex = fonts.render_text(FONT, Color::RGBA(255, 255, 255, 200), &format!("{}/{}", idx + 1, total))?;
    let q = tex.query();
    canvas.copy(tex, None, Some(Rect::new(LOGICAL_W as i32 - 100, 20, q.width, q.height)))
}
