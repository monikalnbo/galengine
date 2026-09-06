//! CG 鉴赏：网格缩略 + 锁定剪影 + 全屏查看（←/→翻页，Esc 返回）。

use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::config::LOGICAL_W;
use crate::gfx::assets::{resolve, TextureBank};
use crate::text::font::FontBook;
use crate::ui::theme;
use crate::ui::widgets;

/// 鉴赏状态（selected 网格项 / view 全屏查看中）
#[derive(Clone, Debug)]
pub struct Gallery {
    pub sel: usize,
    pub view: Option<usize>,
    /// 全部 CG storage 清单（app 启动时扫描数据目录）
    pub all: Vec<String>,
    /// 已解锁（sf.cgs）
    pub unlocked: Vec<String>,
}

impl Gallery {
    pub fn unlocked_at(&self, i: usize) -> bool {
        self.all
            .get(i)
            .is_some_and(|s| self.unlocked.iter().any(|u| u == s))
    }

    /// ←/→ 全屏翻页
    pub fn view_next(&mut self, dir: i32) {
        if let Some(v) = self.view.as_mut() {
            let n = self.all.len() as i32;
            *v = ((*v as i32 + dir + n) % n) as usize;
        }
    }

    pub fn grid_rects() -> Vec<Rect> {
        crate::ui::geom::grid((84, 150), 4, 3, (252, 142), 24)
    }

    pub fn hit_grid(x: f32, y: f32) -> Option<usize> {
        crate::ui::geom::hit(&Self::grid_rects(), x, y)
    }

    pub fn draw(
        &self,
        canvas: &mut Canvas<Window>,
        fonts: &mut FontBook,
        bank: &mut TextureBank,
    ) -> Result<(), String> {
        // 全屏查看模式
        if let Some(v) = self.view {
            let path = resolve("cg", &self.all[v]);
            let _ = bank.load(&path);
            if let Some(tex) = bank.get(&path) {
                let _ = canvas.copy(tex, None, None);
            }
            widgets::text_center(
                canvas,
                fonts,
                theme::FS_TINY,
                theme::C_TEXT_DIM,
                &format!("{} / {} ｜ ←→ 翻页 ｜ Esc 返回", v + 1, self.all.len()),
                680,
            )?;
            return Ok(());
        }
        // 网格模式
        canvas.set_draw_color(theme::C_BG);
        canvas.fill_rect(Rect::new(0, 0, LOGICAL_W, 720))?;
        widgets::text_center(canvas, fonts, theme::FS_TITLE, theme::C_TEXT, "C G 鉴 赏", 66)?;
        for (i, storage) in self.all.iter().enumerate() {
            let r = Self::grid_rects()[i];
            widgets::panel(canvas, r, i == self.sel)?;
            if self.unlocked_at(i) {
                let path = resolve("cg", storage);
                let _ = bank.load(&path);
                if let Some(tex) = bank.get(&path) {
                    let _ = canvas.copy(tex, None, Some(r));
                }
            } else {
                canvas.set_draw_color(theme::C_LOCK);
                canvas.fill_rect(r)?;
                widgets::text_center(canvas, fonts, 40, theme::C_TEXT_FAINT, "?", r.y + 44)?;
            }
        }
        Ok(())
    }
}
