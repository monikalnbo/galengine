//! L1 渲染组装：一帧的绘制接线（游戏画面 → 覆盖层 → 消息/AUTO → 越界层）。

use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;

use crate::config;
use crate::gfx::assets::TextureBank;
use crate::gfx::renderer::{ReachDraw, Renderer};
use crate::input::InputRouter;
use crate::script::interp::{Interp, RunState};
use crate::systems::Systems;
use crate::text::font::FontBook;
use crate::ui::dialog::{self, DialogStyle};
use crate::ui::overlay::{Ctx, Overlay, OverlaySys};
use crate::ui::widgets;
use crate::ui::{choice, inputbox};

#[allow(clippy::too_many_arguments)]
pub fn frame(
    renderer: &mut Renderer,
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    bank: &mut TextureBank,
    fonts: &mut FontBook,
    style: &DialogStyle,
    interp: &Interp,
    ir: &InputRouter,
    overlay: &OverlaySys,
    ctx: &Ctx,
    systems: &mut Systems,
    started: bool,
    now_ms: f32,
    msg: Option<&str>,
) -> Result<(), String> {
    let name_opt = interp.cur_name.clone();
    let sel = ir.choice_sel;
    let is_auto = ir.auto && started;
    let overlay_active = overlay.active();

    renderer.render_screen(canvas, Color::BLACK, |tc| {
        if started {
            interp.stage.draw(tc, bank)?;
        } else {
            // 标题背景（游戏侧数据）
            let _ = bank.draw_full(tc, &format!("{}/bgimage/bg_tanabata.jpg", config::data_dir()));
        }
        if started && !overlay_active {
            match &interp.state {
                RunState::WaitChoice { items } => {
                    choice::draw(tc, fonts, items, sel)?;
                }
                RunState::WaitInput(_) => {
                    if let Some(ui) = ir.input_ui.as_ref() {
                        inputbox::draw(tc, fonts, ui)?;
                    }
                }
                _ => {
                    dialog::draw(tc, fonts, style, name_opt.as_deref(), &interp.tw, now_ms)?;
                }
            }
        } else if started {
            // 覆盖层压暗时保留台词底图
            dialog::draw(tc, fonts, style, name_opt.as_deref(), &interp.tw, now_ms)?;
        }
        overlay.draw(tc, fonts, bank, ctx)?;
        if let Some(text) = msg {
            widgets::text_center(
                tc,
                fonts,
                crate::ui::theme::FS_SMALL,
                Color::RGB(255, 200, 120),
                text,
                20,
            )?;
        }
        if is_auto && !overlay_active {
            let tex = fonts.render_text(22, Color::RGB(110, 220, 255), "AUTO")?;
            tc.copy(tex, None, Some(sdl2::rect::Rect::new(1170, 486, 60, 26)))?;
        }
        Ok(())
    })?;

    // 越界层（窗口坐标，突破 letterbox）
    let reach_info = systems
        .reach
        .as_ref()
        .map(|r| (r.path.clone(), (r.t / r.dur).min(1.0), r.scale));
    let reach = reach_info.and_then(|(path, progress, scale)| {
        let _ = bank.load(&path);
        bank.get_mut(&path).map(|tex| ReachDraw {
            tex,
            progress,
            scale,
        })
    });
    renderer.present(canvas, reach)
}

/// Rest 覆盖层期间禁用游戏点击（演出保护）
pub fn game_clickable(interp: &Interp, overlay: &OverlaySys, started: bool) -> bool {
    started && !overlay.active() && interp.state != RunState::Ended
}

/// 右键菜单可用条件
pub fn menu_openable(interp: &Interp, overlay: &OverlaySys, started: bool) -> bool {
    started && !overlay.active() && interp.state != RunState::Ended && !rest_active(overlay)
}

pub fn rest_active(overlay: &OverlaySys) -> bool {
    matches!(overlay.cur, Overlay::Rest { .. })
}

/// 供 input.rs 转换坐标的便捷（未用 MouseButtonDown 以外事件时优化）
pub fn _unused(_: MouseButton) {}
