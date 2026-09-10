//! L1 输入路由：SDL 事件 → 关机 → 覆盖层 → 台词/选项/输入三态。
//! 唯一的 SDL 事件翻译点；具体交互逻辑在各子模块。

mod activate;
mod auto;
mod keyboard;
mod keys;
mod mouse;
mod saves;
mod settings_io;

#[cfg(test)]
mod saves_test;

pub use auto::auto_step;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::systems::Game;
use gal_render::renderer::Renderer;
use gal_text::font::FontBook;
use gal_ui::savemenu::ThumbCache;

/// 处理一条事件；返回 false=请求退出
pub fn dispatch(
    g: &mut Game,
    ev: Event,
    canvas: &mut Canvas<Window>,
    _fonts: &mut FontBook,
    renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    match ev {
        Event::Quit { .. } => Ok(false),

        Event::KeyDown { keycode: Some(k), .. } => {
            keyboard::key_down(g, k, canvas, renderer, thumbs)
        }

        Event::KeyUp { keycode: Some(Keycode::LCtrl | Keycode::RCtrl), .. } => {
            g.ctrl_hold = false;
            Ok(true)
        }

        Event::TextInput { ref text, .. } if g.input_ui.is_some() => {
            if let Some(ui) = g.input_ui.as_mut() {
                ui.push_char(text);
            }
            Ok(true)
        }

        Event::MouseMotion { x, y, .. } if !g.overlay.active() && g.choice_len() > 0 => {
            let (lx, ly) = Renderer::to_logical(canvas, x, y);
            if let Some(i) = gal_ui::choice::hit_test(g.choice_len(), lx, ly) {
                g.choice_sel = i;
            }
            Ok(true)
        }

        Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => {
            g.cursor.set_click();
            let (lx, ly) = Renderer::to_logical(canvas, x, y);
            mouse::click(g, lx, ly, canvas, renderer, thumbs)
        }
        Event::MouseButtonUp { mouse_btn: MouseButton::Left, .. } => {
            g.cursor.set_default();
            Ok(true)
        }

        Event::MouseButtonDown { mouse_btn: MouseButton::Right, .. } => {
            if g.hide_ui {
                g.hide_ui = false;
            } else if g.overlay.active() {
                if !matches!(g.overlay, gal_ui::overlay::Overlay::Title { .. }) {
                    g.overlay = gal_ui::overlay::Overlay::None;
                }
            } else if g.started && g.interp.state == gal_script::interp::RunState::WaitClick {
                g.hide_ui = true;
            }
            Ok(true)
        }

        Event::MouseWheel { y, .. } => {
            if y > 0 {
                // 滚轮向上：在台词态呼出 Backlog / 在 Backlog 中向上滚动
                if let gal_ui::overlay::Overlay::Backlog { scroll } = &mut g.overlay {
                    *scroll += 1;
                } else if !g.overlay.active()
                    && g.started
                    && g.interp.state == gal_script::interp::RunState::WaitClick
                {
                    g.overlay = gal_ui::overlay::Overlay::Backlog { scroll: 0 };
                }
            } else if y < 0 {
                // 滚轮向下：在 Backlog 中向下滚动 / 在台词态推进对白
                if let gal_ui::overlay::Overlay::Backlog { scroll } = &mut g.overlay {
                    *scroll = scroll.saturating_sub(1);
                } else if !g.overlay.active()
                    && g.started
                    && g.interp.state == gal_script::interp::RunState::WaitClick
                {
                    g.interp.click()?;
                }
            }
            Ok(true)
        }
        _ => Ok(true),
    }
}
