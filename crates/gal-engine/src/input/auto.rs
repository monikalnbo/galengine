//! 验收自动驱动（ES_DEBUG_AUTOCLICK_MS）：标题=开始，台词=点击，选项=选0，输入=默认值

use crate::systems::Game;
use gal_ui::savemenu::ThumbCache;

use super::activate;

pub fn auto_step(g: &mut Game, thumbs: &mut ThumbCache) -> Result<(), String> {
    if g.sys.rest.is_some() {
        g.cancel_rest(); // 验收环境自动取消关机，跑完剧本
        return Ok(());
    }
    if g.overlay.active() {
        match g.overlay.clone() {
            gal_ui::overlay::Overlay::Title { sel, .. } => {
                activate::title_activate(g, sel, thumbs)?;
            }
            gal_ui::overlay::Overlay::Menu { .. }
            | gal_ui::overlay::Overlay::Save { .. }
            | gal_ui::overlay::Overlay::Gallery { .. }
            | gal_ui::overlay::Overlay::Settings { .. }
            | gal_ui::overlay::Overlay::Backlog { .. } => {
                g.overlay = gal_ui::overlay::Overlay::None;
            }
            gal_ui::overlay::Overlay::Ritual { step, fade, .. } => {
                g.overlay = gal_ui::overlay::Overlay::Ritual { step: step + 1, fade, done: false };
            }
            _ => {}
        }
        return Ok(());
    }
    use gal_script::interp::RunState;
    match &g.interp.state {
        RunState::WaitClick => g.interp.click(),
        RunState::WaitChoice { .. } => g.interp.choose(0),
        RunState::WaitInput(_) => {
            let v = g.input_ui.as_ref().map(|ui| ui.value()).unwrap_or_default();
            g.interp.input_result(v)
        }
        _ => Ok(()),
    }
}
