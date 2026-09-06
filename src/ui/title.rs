//! 标题画面：标题字（可演化）+ 主菜单。演化规则由 title_stage 驱动。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::text::font::FontBook;
use crate::ui::theme;
use crate::ui::widgets;

pub const ITEMS: &[&str] = &["开始", "继续", "CG 鉴赏", "音量·速度", "退出"];

pub fn menu_rects() -> Vec<Rect> {
    crate::ui::geom::menu_column(520, 240, 380, ITEMS.len(), 62, 22)
}

pub fn hit_menu(x: f32, y: f32) -> Option<usize> {
    crate::ui::geom::hit(&menu_rects(), x, y)
}

/// 标题字绘制（演化：0=白 / 1=褪色灰 / 2=绿 / 3=追加违和感小字）
pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    title_main: &str,
    title_sub: &str,
    selected: usize,
    title_stage: u32,
) -> Result<(), String> {
    let main_color = match title_stage {
        0 => Color::RGB(255, 255, 255),
        1 => Color::RGB(150, 152, 158),
        _ => Color::RGB(120, 200, 150),
    };
    widgets::text_center(canvas, fonts, theme::FS_HUGE, main_color, title_main, 150)?;
    widgets::text_center(canvas, fonts, theme::FS_SMALL, theme::C_TEXT_DIM, title_sub, 290)?;
    if title_stage >= 3 {
        widgets::text_center(canvas, fonts, theme::FS_TINY, Color::RGB(150, 210, 160), "违和感：7/7", 336)?;
    }
    for (i, item) in ITEMS.iter().enumerate() {
        widgets::button(canvas, fonts, menu_rects()[i], item, i == selected)?;
    }
    Ok(())
}
