//! UI 主题：全引擎界面配色与字号（一处改全局风格）。

use sdl2::pixels::Color;

// —— 配色 ——
pub const C_BG: Color = Color::RGBA(6, 8, 18, 235); // 覆盖层底
pub const C_PANEL: Color = Color::RGBA(20, 24, 42, 230);
pub const C_PANEL_HOT: Color = Color::RGBA(70, 90, 148, 230);
pub const C_BORDER: Color = Color::RGBA(110, 120, 150, 120);
pub const C_BORDER_HOT: Color = Color::RGBA(150, 200, 255, 220);
pub const C_TEXT: Color = Color::RGB(240, 240, 248);
pub const C_TEXT_DIM: Color = Color::RGB(150, 156, 176);
pub const C_TEXT_FAINT: Color = Color::RGB(96, 104, 128);
pub const C_ACCENT: Color = Color::RGB(150, 200, 255);
pub const C_DANGER: Color = Color::RGB(255, 90, 100);
pub const C_LOCK: Color = Color::RGBA(10, 12, 22, 255);

// —— 字号 ——
pub const FS_TITLE: u16 = 40;
pub const FS_ITEM: u16 = 28;
pub const FS_SMALL: u16 = 22;
pub const FS_TINY: u16 = 20;
pub const FS_BIG: u16 = 52;
pub const FS_HUGE: u16 = 96;
