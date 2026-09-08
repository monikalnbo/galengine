//! 设置覆盖层接线：键鼠事件 → SettingsCtx.set 闭包 → apply 统一写回 sf.*/音频/文字速度/全屏。

use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::video::{FullscreenType, Window};

use crate::systems::Game;
use gal_script::vars::Value;
use gal_ui::overlay::Overlay;
use gal_ui::settings::{self, SettingKind, SettingsCtx};

/// 从引擎实况构建设置上下文（音量/全屏取真实状态）
fn ctx(g: &Game, canvas: &Canvas<Window>) -> SettingsCtx {
    let fs = canvas.window().fullscreen_state() != FullscreenType::Off;
    SettingsCtx::from_vars(&g.interp.vars, g.sys.audio.bgm_vol, g.sys.audio.se_vol, fs)
}

/// ctx → 引擎状态（即时生效 + sf.* 随退出落盘）
fn apply(g: &mut Game, c: &SettingsCtx, canvas: &mut Canvas<Window>) {
    if c.text_speed > 0 {
        g.interp.tw.interval_ms = c.text_speed as f32;
    }
    g.sys.audio.bgm_vol = c.bgm_vol.clamp(0, 100) as i32;
    g.sys.audio.se_vol = c.se_vol.clamp(0, 100) as i32;
    g.sys.audio.apply_volumes();
    let sf = &mut g.interp.vars.sf;
    sf.insert("textSpeed".into(), Value::Int(c.text_speed));
    sf.insert("autoDelay".into(), Value::Int(c.auto_delay));
    sf.insert("volBgm".into(), Value::Int(c.bgm_vol));
    sf.insert("volSe".into(), Value::Int(c.se_vol));
    sf.insert("skipRead".into(), Value::Int(c.skip_read as i64));
    // 全屏切换（状态变化才动窗口）
    let want = if c.fullscreen { FullscreenType::Desktop } else { FullscreenType::Off };
    if canvas.window().fullscreen_state() != want {
        if let Err(e) = canvas.window_mut().set_fullscreen(want) {
            g.msg(format!("全屏切换失败：{e}"));
        }
    }
}

/// ←→ 调整当前行滑条（dx=±1）
pub fn arrow(g: &mut Game, canvas: &mut Canvas<Window>, dx: i32) {
    let sel = match g.overlay {
        Overlay::Settings { sel } => sel,
        _ => return,
    };
    let row = settings::rows(&g.conf).into_iter().nth(sel);
    let Some(row) = row else { return };
    let mut c = ctx(g, canvas);
    if let SettingKind::Slider { min, max, get, set, .. } = row.kind {
        let step = ((max - min) / 20).max(1);
        let v = (get(&c) + dx as i64 * step).clamp(min, max);
        set(&mut c, v);
        apply(g, &c, canvas);
    }
}

/// ↑↓ 换行
pub fn move_sel(g: &mut Game, k: Keycode) {
    let n = settings::rows(&g.conf).len();
    if n == 0 {
        return;
    }
    if let Overlay::Settings { sel } = &mut g.overlay {
        *sel = match k {
            Keycode::Up => sel.saturating_sub(1),
            Keycode::Down => (*sel + 1).min(n - 1),
            _ => *sel,
        };
    }
}

/// 回车：开关切换 / 动作执行（滑条行不响应）
pub fn enter(g: &mut Game, canvas: &mut Canvas<Window>) -> Result<bool, String> {
    let sel = match g.overlay {
        Overlay::Settings { sel } => sel,
        _ => return Ok(true),
    };
    let row = settings::rows(&g.conf).into_iter().nth(sel);
    let Some(row) = row else { return Ok(true) };
    let mut c = ctx(g, canvas);
    match row.kind {
        SettingKind::Toggle { get, set, .. } => {
            let v = !get(&c);
            set(&mut c, v);
            apply(g, &c, canvas);
        }
        SettingKind::Action { run, .. } => run(&mut c, &g.conf),
        SettingKind::Slider { .. } => {}
    }
    Ok(true)
}

/// 鼠标点击：先试滑条（按 x 比例设值）→ 再试行（选中并激活）
pub fn click(g: &mut Game, canvas: &mut Canvas<Window>, lx: f32, ly: f32) {
    let rows = settings::rows(&g.conf);
    for (i, row) in rows.iter().enumerate() {
        if let SettingKind::Slider { min, max, set, .. } = &row.kind {
            if let Some(pct) = settings::slider_hit(i, lx, ly) {
                g.overlay = Overlay::Settings { sel: i };
                let mut c = ctx(g, canvas);
                let v = (*min + (pct * (max - min) as f32).round() as i64).clamp(*min, *max);
                set(&mut c, v);
                apply(g, &c, canvas);
                return;
            }
        }
    }
    if let Some(i) = settings::hit_test(rows.len(), lx, ly) {
        g.overlay = Overlay::Settings { sel: i };
        let row = rows.into_iter().nth(i).unwrap();
        let mut c = ctx(g, canvas);
        match row.kind {
            SettingKind::Toggle { get, set, .. } => {
                let v = !get(&c);
                set(&mut c, v);
                apply(g, &c, canvas);
            }
            SettingKind::Action { run, .. } => run(&mut c, &g.conf),
            SettingKind::Slider { .. } => {}
        }
    }
}
