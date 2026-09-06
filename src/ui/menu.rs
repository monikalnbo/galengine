//! 系统菜单：右键/Esc 唤出的竖条菜单。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use crate::config::LOGICAL_W;
use crate::text::font::FontBook;
use crate::ui::widgets;

pub const ITEMS: &[&str] = &["继续", "存档", "读档", "CG 鉴赏", "音量·速度", "回到标题", "退出"];

pub fn menu_rects() -> Vec<Rect> {
    crate::ui::geom::menu_column(940, 300, 190, ITEMS.len(), 56, 14)
}

pub fn hit_menu(x: f32, y: f32) -> Option<usize> {
    crate::ui::geom::hit(&menu_rects(), x, y)
}

pub fn draw(canvas: &mut Canvas<Window>, fonts: &mut FontBook, selected: usize) -> Result<(), String> {
    // 游戏画面压暗
    canvas.set_blend_mode(BlendMode::Blend);
    canvas.set_draw_color(Color::RGBA(0, 0, 0, 150));
    canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 720))?;
    for (i, item) in ITEMS.iter().enumerate() {
        widgets::button(canvas, fonts, menu_rects()[i], item, i == selected)?;
    }
    Ok(())
}
