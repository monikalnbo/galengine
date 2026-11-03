//! 验收自动驱动（ES_DEBUG_AUTOCLICK_MS）：标题=开始，台词=点击，选项=选0，输入=默认值

use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::systems::Game;
use gal_ui::savemenu::ThumbCache;

use super::activate;

/// 验收专用设置脚本（`ES_DEBUG_SETTINGS_STEPS`）：
/// `行号:动作:参数[,行号:动作:参数…]`，动作 = `enter` / `left` / `right` / `down`
/// （down 的参数 = 连续按↓多少次）。按顺序每次自动驱动执行一步，用来实测
/// 「设置面板改 → 立即生效 → 重启仍生效」。
fn settings_script(g: &Game) -> Option<Vec<(usize, String, i64)>> {
    let raw = std::env::var("ES_DEBUG_SETTINGS_STEPS").ok()?;
    let step = g.auto_steps;
    if step >= raw.split(',').count() {
        return None;
    }
    let spec = raw.split(',').nth(step)?;
    let mut it = spec.split(':');
    let idx: usize = it.next()?.trim().parse().ok()?;
    let act = it.next()?.trim().to_string();
    let arg: i64 = it.next().and_then(|v| v.trim().parse().ok()).unwrap_or(0);
    Some(vec![(idx, act, arg)])
}

fn do_setting_step(
    g: &mut Game,
    canvas: &mut Canvas<Window>,
    idx: usize,
    act: &str,
    arg: i64,
) -> Result<(), String> {
    g.overlay = gal_ui::overlay::Overlay::Settings { sel: idx };
    match act {
        "enter" => {
            super::settings_io::enter(g, canvas)?;
        }
        "left" => super::settings_io::arrow(g, canvas, -1),
        "right" => super::settings_io::arrow(g, canvas, 1),
        "down" => {
            for _ in 0..arg.max(1) {
                super::settings_io::move_sel(g, sdl2::keyboard::Keycode::Down);
            }
        }
        other => return Err(format!("ES_DEBUG_SETTINGS_STEPS 未知动作：{other}")),
    }
    Ok(())
}

pub fn auto_step(
    g: &mut Game,
    thumbs: &mut ThumbCache,
    canvas: &mut Canvas<Window>,
) -> Result<(), String> {
    // 设置脚本只在进入剧本后执行（先让标题自动「开始」，否则一直停在标题）
    if let Some(steps) = settings_script(g).filter(|_| g.started) {
        let n = g.auto_steps + 1;
        g.auto_steps = n;
        for (idx, act, arg) in steps {
            eprintln!("[dbg settings] 设置脚本第 {n} 步：行 {idx} {act} {arg}");
            do_setting_step(g, canvas, idx, &act, arg)?;
        }
        // 脚本跑完回到原界面状态（默认回剧本/标题），不留在设置里
        if !g.auto_steps_done
            && g.auto_steps
                >= std::env::var("ES_DEBUG_SETTINGS_STEPS").unwrap_or_default().split(',').count()
        {
            g.auto_steps_done = true;
            g.overlay = gal_ui::overlay::Overlay::None;
        }
        return Ok(());
    }
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
        RunState::WaitButton => g.interp.activate_button().map(|_| ()), // v0.3.15 无头驱动：按钮=点它
        _ => Ok(()),
    }
}
