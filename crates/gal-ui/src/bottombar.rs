//! 对话框下方功能条：自动 / 快进 / 存档 / 读档 / 设置。
//! 鼠标党直达（键盘 A/Ctrl/F9/F10/Esc 依旧全有效）；自动/快进高亮指示当前状态。
//! 按钮集合/几何/字号来自 `ui.bottombar`；配色来自当前皮肤（`ui.skin`）。
//! 动作按标签顺序固定（第 1..5 个 = 自动/快进/存档/读档/设置），
//! 作者可删自己不要的按钮，位置随剩余按钮右对齐。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use gal_config::{BottombarCfg, SkinCfg};
use gal_text::font::FontBook;

/// 标签 → 动作序号（activate::bottombar_activate 的分派顺序）
pub fn action_of(label: &str) -> Option<usize> {
    ["自动", "快进", "存档", "读档", "设置"].iter().position(|l| *l == label)
}

pub fn item_rect(i: usize, cfg: &BottombarCfg) -> Rect {
    let n = cfg.buttons.len() as i32;
    let total = n * cfg.button_width as i32 + (n - 1).max(0) * cfg.gap as i32;
    Rect::new(
        cfg.right_align - total + i as i32 * (cfg.button_width + cfg.gap) as i32,
        cfg.y,
        cfg.button_width,
        cfg.button_height,
    )
}

/// 鼠标命中 → 原始按钮序号（调用方再经 `action_of` 转动作）
pub fn hit(cfg: &BottombarCfg, x: i32, y: i32) -> Option<usize> {
    (0..cfg.buttons.len()).find(|&i| item_rect(i, cfg).contains_point((x, y)))
}

pub fn draw(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    cfg: &BottombarCfg,
    skin: &SkinCfg,
    auto_on: bool,
    ff_on: bool,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    for (i, label) in cfg.buttons.iter().enumerate() {
        let r = item_rect(i, cfg);
        let on = (action_of(label) == Some(0) && auto_on) || (action_of(label) == Some(1) && ff_on);
        let (fill, border) = if on {
            (skin.bar_accent, skin.dialog_border)
        } else {
            (skin.bar_fill, skin.bar_border)
        };
        canvas.set_draw_color(Color::RGBA(fill[0], fill[1], fill[2], fill[3]));
        canvas.fill_rect(r)?;
        canvas.set_draw_color(Color::RGBA(border[0], border[1], border[2], border[3]));
        canvas.draw_rect(r)?;
        let tc = if on { [255, 255, 255] } else { skin.bar_text };
        let tex = fonts.render_text(cfg.font_size, Color::RGB(tc[0], tc[1], tc[2]), label)?;
        let q = tex.query();
        canvas.copy(
            tex,
            None,
            Some(Rect::new(
                r.x + (r.width() as i32 - q.width as i32) / 2,
                r.y + (r.height() as i32 - q.height as i32) / 2,
                q.width,
                q.height,
            )),
        )?;
    }
    Ok(())
}
