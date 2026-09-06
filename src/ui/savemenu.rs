//! 存档/读档界面：3×3 槽位网格（缩略图+时间戳+场景标题），
//! meta 不明存档（不可读）与破損标记同屏展示。

use std::collections::HashMap;

use sdl2::image::LoadTexture;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};

use crate::config::LOGICAL_W;
use crate::save::meta::FakeSave;
use crate::save::slots::SaveEntry;
use crate::text::font::FontBook;
use crate::ui::inputbox::button;

const CELL_W: u32 = 360;
const CELL_H: u32 = 150;
const GAP: i32 = 24;
const COLS: usize = 3;
const TOP_Y: i32 = 130;
const FONT: u16 = 22;
const BIG: u16 = 30;

/// 槽位/不明档 布局矩形（idx ≥ slots 为不明档，追加一行展示）
pub fn cell_rect(idx: usize) -> Rect {
    let row = idx / COLS;
    let col = idx % COLS;
    let total_w = COLS as i32 * CELL_W as i32 + (COLS as i32 - 1) * GAP;
    let x0 = (LOGICAL_W as i32 - total_w) / 2;
    Rect::new(
        x0 + col as i32 * (CELL_W as i32 + GAP),
        TOP_Y + row as i32 * (CELL_H as i32 + GAP),
        CELL_W,
        CELL_H,
    )
}

pub fn hit_test(n: usize, x: f32, y: f32) -> Option<usize> {
    (0..n).find(|&i| cell_rect(i).contains_point((x as i32, y as i32)))
}

/// 缩略图纹理缓存（开档界面时构建一次）
pub struct ThumbCache<'a> {
    creator: &'a TextureCreator<WindowContext>,
    map: HashMap<String, Texture<'a>>,
}

impl<'a> ThumbCache<'a> {
    pub fn new(creator: &'a TextureCreator<WindowContext>) -> Self {
        Self { creator, map: HashMap::new() }
    }

    /// entries: (槽号, 内容)；fake: 不明档列表（image 为 data 相对路径）
    pub fn build(&mut self, entries: &[(usize, Option<SaveEntry>)], fake: &[FakeSave]) {
        self.map.clear();
        for (slot, e) in entries {
            if let Some(e) = e {
                if let Some(tex) = png_texture(self.creator, &e.thumb) {
                    self.map.insert(format!("s{slot}"), tex);
                }
            }
        }
        for (i, f) in fake.iter().enumerate() {
            let path = format!("{}/{}", crate::config::data_dir(), f.image);
            if let Ok(tex) = self.creator.load_texture(&path) {
                self.map.insert(format!("f{i}"), tex);
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&Texture<'a>> {
        self.map.get(key)
    }
}

/// PNG 字节 → RGBA 纹理
fn png_texture<'a>(creator: &'a TextureCreator<WindowContext>, png: &[u8]) -> Option<Texture<'a>> {
    let img = image::load_from_memory(png).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let mut tex = creator.create_texture_static(Some(PixelFormatEnum::RGBA8888), w, h).ok()?;
    tex.update(None, &img, w as usize * 4).ok()?;
    Some(tex)
}

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
        draw_cell(canvas, fonts, thumbs, &format!("s{slot}"), r, i == sel, |fonts, canvas| {
            match e {
                Some(e) => {
                    let corrupt = corrupt.contains(slot);
                    let color = if corrupt { Color::RGB(255, 120, 120) } else { Color::RGB(200, 206, 220) };
                    let name = if corrupt {
                        format!("槽{slot} 破損")
                    } else {
                        format!("槽{slot} {}", e.stamp)
                    };
                    let tex = fonts.render_text(FONT, color, &name)?;
                    let q = tex.query();
                    canvas.copy(tex, None, Some(Rect::new(r.x + 190, r.y + 18, q.width, q.height)))?;
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
                    let tex = fonts.render_text(FONT, Color::RGB(110, 116, 132), &format!("槽{slot} —— 空"))?;
                    let q = tex.query();
                    canvas.copy(tex, None, Some(Rect::new(r.x + 190, r.y + 30, q.width, q.height)))
                }
            }
        })?;
    }
    // 不明档（追加展示，不可读）
    for (i, f) in fake.iter().enumerate() {
        let r = cell_rect(entries.len() + i);
        draw_cell(canvas, fonts, thumbs, &format!("f{i}"), r, entries.len() + i == sel, |fonts, canvas| {
            let tex = fonts.render_text(FONT, Color::RGB(255, 200, 120), "不明存档")?;
            let q = tex.query();
            canvas.copy(tex, None, Some(Rect::new(r.x + 190, r.y + 18, q.width, q.height)))?;
            let tex = fonts.render_text(FONT, Color::RGB(190, 160, 130), &format!("{} {}", f.date, f.time))?;
            let q = tex.query();
            canvas.copy(tex, None, Some(Rect::new(r.x + 190, r.y + 52, q.width, q.height)))
        })?;
    }

    button(
        canvas,
        fonts,
        Rect::new(LOGICAL_W as i32 - 216, 54, 140, 52),
        "关闭 Esc",
        false,
    )?;
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
    canvas.set_draw_color(if hot { Color::RGBA(60, 80, 140, 230) } else { Color::RGBA(18, 22, 40, 230) });
    canvas.fill_rect(r)?;
    canvas.set_draw_color(if hot { Color::RGBA(150, 200, 255, 230) } else { Color::RGBA(110, 120, 150, 120) });
    canvas.draw_rect(r)?;
    let tr = Rect::new(r.x + 12, r.y + (r.height() as i32 - 94) / 2, 168, 94);
    canvas.set_draw_color(Color::RGB(8, 10, 20));
    canvas.fill_rect(tr)?;
    if let Some(tex) = thumbs.get(key) {
        canvas.copy(tex, None, Some(tr))?;
    }
    info(fonts, canvas)
}
