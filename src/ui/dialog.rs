//! 底部对话框：半透明盒 + 名字牌（角色色，游戏侧配置）+ 正文（打字机部分显示）。

use std::collections::HashMap;

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::render::BlendMode;
use sdl2::video::Window;

use crate::text::font::FontBook;
use crate::text::writer::Typewriter;

/// 名字颜色（查不到时白；色表来自配置 game.name_colors）
pub fn name_color(name: &str, colors: &HashMap<String, [u8; 3]>) -> Color {
    colors
        .get(name)
        .map(|[r, g, b]| Color::RGB(*r, *g, *b))
        .unwrap_or(Color::RGB(235, 235, 240))
}

#[derive(Clone)]
pub struct DialogStyle {
    pub box_rect: Rect,
    pub text_x: i32,
    pub text_y: i32,
    pub font_size: u16,
    pub name_font_size: u16,
    /// 单页最大行数
    pub lines_per_page: usize,
    /// 角色名颜色表
    pub name_colors: HashMap<String, [u8; 3]>,
}

impl Default for DialogStyle {
    fn default() -> Self {
        Self::from_cfg(&crate::config::DialogCfg::default(), HashMap::new())
    }
}

impl DialogStyle {
    /// 从顶层配置构建（分层化：唯一配置源 data/config.json）
    pub fn from_cfg(c: &crate::config::DialogCfg, colors: HashMap<String, [u8; 3]>) -> Self {
        Self {
            box_rect: Rect::new(c.box_rect[0], c.box_rect[1], c.box_rect[2] as u32, c.box_rect[3] as u32),
            text_x: c.text_x,
            text_y: c.text_y,
            font_size: c.font_size,
            name_font_size: c.name_font_size,
            lines_per_page: c.lines_per_page,
            name_colors: colors,
        }
    }
}

/// 绘制对话框（在游戏画面坐标下）
pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    style: &DialogStyle,
    name: Option<&str>,
    tw: &Typewriter,
    now_ms: f32,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);

    // 盒子：半透明深底 + 细边
    canvas.set_draw_color(Color::RGBA(10, 12, 26, 205));
    canvas.fill_rect(style.box_rect)?;
    canvas.set_draw_color(Color::RGBA(255, 255, 255, 60));
    canvas.draw_rect(style.box_rect)?;

    // 名字牌：悬于盒子上沿左侧
    if let Some(name) = name {
        if !name.is_empty() {
            let color = name_color(name, &style.name_colors);
            let tex = fonts.render_text(style.name_font_size, color, name)?;
            let q = tex.query();
            let plate = Rect::new(
                style.box_rect.x + 24,
                style.box_rect.y - 24,
                q.width + 44,
                46,
            );
            canvas.set_draw_color(Color::RGBA(color.r, color.g, color.b, 56));
            canvas.fill_rect(plate)?;
            canvas.set_draw_color(Color::RGBA(color.r, color.g, color.b, 200));
            canvas.draw_rect(plate)?;
            let name_dst = Rect::new(plate.x + 22, plate.y + (plate.height() as i32 - q.height as i32) / 2, q.width, q.height);
            canvas.copy(tex, None, Some(name_dst))?;
        }
    }

    // 正文：已完行全显，当前行按 reveal 字数裁剪
    let text_color = Color::RGB(242, 242, 246);
    let (cur_line, cur_chars) = tw.reveal_pos();
    let line_h = fonts.line_h(style.font_size);
    let page = tw.current_page();

    for (i, line) in page.iter().enumerate() {
        if i > cur_line {
            break;
        }
        let shown = if i < cur_line { line.chars().count() } else { cur_chars };
        if shown == 0 {
            continue;
        }
        let visible: String = line.chars().take(shown).collect();
        let tex = fonts.render_text(style.font_size, text_color, &visible)?;
        let q = tex.query();
        let dst = Rect::new(style.text_x, style.text_y + (i as i32) * line_h, q.width, q.height);
        canvas.copy(tex, None, Some(dst))?;
    }

    // 行完指示：右下角「▼」呼吸
    if tw.line_finished() {
        let a = (140.0 + 100.0 * (now_ms / 400.0).sin()) as u8;
        canvas.set_draw_color(Color::RGBA(255, 255, 255, a));
        let bx = style.box_rect.right() - 34;
        let by = style.box_rect.bottom() - 26;
        for dy in 0..8i32 {
            let half = (8 - dy) as u32;
            canvas.fill_rect(Rect::new(bx - (half / 2) as i32 + 4, by + dy - 8, half.max(1), 1))?;
        }
    }
    Ok(())
}
