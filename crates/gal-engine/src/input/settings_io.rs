//! 设置覆盖层接线：键鼠事件 → SettingsCtx 读写 → apply 统一写回 sf.* / 音频 / 样式 / 窗口。
//!
//! 改动即落地：音量/字号/行距/皮肤当场生效，`savedata/global.json` 立即写盘（重启仍在）。

use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::video::{FullscreenType, Window};

use crate::systems::Game;
use gal_script::vars::Value;
use gal_ui::overlay::Overlay;
use gal_ui::settings::{self, SettingKind, SettingsCtx, UiEnv};

/// 设置界面的候选环境（字体族实况 + 皮肤名单）
pub fn env(g: &Game) -> UiEnv {
    UiEnv {
        families: g.font_families.clone(),
        skins: g.skins.iter().map(|s| s.name.clone()).collect(),
        languages: g.langs.iter().map(|p| p.name.clone()).collect(),
        tts_available: g.tts.backend.is_some(),
    }
}

/// 从引擎实况 + sf.* 构建设置上下文
fn ctx(g: &Game, canvas: &Canvas<Window>) -> SettingsCtx {
    let fs = canvas.window().fullscreen_state() != FullscreenType::Off;
    SettingsCtx::from_vars(&g.interp.vars, &g.conf, &g.keymap(), fs)
}

fn set_sf(g: &mut Game, key: &str, v: i64) {
    g.interp.vars.sf.insert(key.into(), Value::Int(v));
}

/// 立即写盘（设置改完就落 global.json，不用等退出）
pub fn persist(g: &Game) {
    let path = format!("{}/global.json", g.conf.save.dir);
    if let Err(e) = g.interp.vars.save_global(&path) {
        eprintln!("[settings] global.json 写入失败：{e}");
    }
}

/// ctx → 引擎状态（即时生效 + 落盘）
fn apply(g: &mut Game, c: &SettingsCtx, canvas: &mut Canvas<Window>) {
    // 文字：速度立即生效；字号/行距/等待标记走样式重算（下一帧重排）
    if c.text_speed > 0 {
        g.interp.tw.interval_ms = c.text_speed as f32;
    }
    set_sf(g, "textSpeed", c.text_speed);
    set_sf(g, "autoDelay", c.auto_delay);
    set_sf(g, "textSize", c.text_size);
    set_sf(g, "lineSpacing", c.line_spacing);
    set_sf(g, "waitMarker", c.wait_marker as i64);
    set_sf(g, "outline", c.outline);
    set_sf(g, "fontFamily", c.family);
    set_sf(g, "skipRead", c.skip_read as i64);
    set_sf(g, "transOn", c.trans_on as i64);
    set_sf(g, "skinIndex", c.skin);
    set_sf(g, "language", c.language);
    set_sf(g, "ttsOn", c.tts as i64);
    set_sf(g, "ttsRate", c.tts_rate);
    // 朗读开关/语速即时生效（本机无后端 → 开也静默，设置行已标注）
    let can_tts = g.tts.backend.is_some();
    g.tts.enabled = c.tts && can_tts;
    g.tts.rate = c.tts_rate.clamp(-10, 10) as i32;
    if !g.tts.enabled {
        g.tts.stop();
    }
    if c.scan_sys_fonts {
        g.scan_sys_fonts = true;
    }
    g.apply_lang();
    g.interp.trans_enabled = c.trans_on;

    // 音频三通道
    g.sys.audio.bgm_vol = c.bgm_vol.clamp(0, 100) as i32;
    g.sys.audio.se_vol = c.se_vol.clamp(0, 100) as i32;
    g.sys.audio.voice_vol = c.voice_vol.clamp(0, 100) as i32;
    g.sys.audio.apply_volumes();
    set_sf(g, "volBgm", c.bgm_vol);
    set_sf(g, "volSe", c.se_vol);
    set_sf(g, "volVoice", c.voice_vol);

    // 全屏切换（状态变化才动窗口）
    let want = if c.fullscreen { FullscreenType::Desktop } else { FullscreenType::Off };
    if canvas.window().fullscreen_state() != want {
        if let Err(e) = canvas.window_mut().set_fullscreen(want) {
            g.msg(format!("全屏切换失败：{e}"));
        }
    }
    g.style_dirty = true;
    persist(g);
}

/// 当前选中行的全局行号
fn sel(g: &Game) -> Option<usize> {
    match g.overlay {
        Overlay::Settings { sel } => Some(sel),
        _ => None,
    }
}

/// 选中行（按 idx 取）
fn row_at(g: &Game, idx: usize) -> Option<settings::SettingRow> {
    let env = env(g);
    settings::rows(&g.conf, &env).into_iter().nth(idx)
}

/// ←→ 调整当前行（滑条 ±1 步 / 枚举换项）
pub fn arrow(g: &mut Game, canvas: &mut Canvas<Window>, dx: i32) {
    let Some(idx) = sel(g) else { return };
    let Some(row) = row_at(g, idx) else { return };
    let mut c = ctx(g, canvas);
    match row.kind {
        SettingKind::Slider { min, max, get, set, .. } => {
            let step = ((max - min) / 20).max(1);
            let v = (get(&c) + dx as i64 * step).clamp(min, max);
            set(&mut c, v);
        }
        SettingKind::Enum { labels, get, set } => {
            let n = labels.len().max(1) as i64;
            let v = (get(&c) + dx as i64).rem_euclid(n);
            set(&mut c, v);
        }
        _ => return,
    }
    apply(g, &c, canvas);
}

/// ↑↓ 换行（跨页自动翻页）
pub fn move_sel(g: &mut Game, k: Keycode) {
    let env = env(g);
    let n = settings::rows(&g.conf, &env).len();
    if n == 0 {
        return;
    }
    if let Overlay::Settings { sel } = &mut g.overlay {
        *sel = match k {
            Keycode::Up => sel.saturating_sub(1),
            Keycode::Down => (*sel + 1).min(n - 1),
            Keycode::PageUp => sel.saturating_sub(settings::PER_PAGE),
            Keycode::PageDown => (*sel + settings::PER_PAGE).min(n - 1),
            _ => *sel,
        };
    }
}

/// 回车：开关切换 / 枚举换项 / 键位进入重绑 / 动作执行（滑条行不响应）
pub fn enter(g: &mut Game, canvas: &mut Canvas<Window>) -> Result<bool, String> {
    let Some(idx) = sel(g) else { return Ok(true) };
    let Some(row) = row_at(g, idx) else { return Ok(true) };
    let mut c = ctx(g, canvas);
    match row.kind {
        SettingKind::Toggle { get, set } => {
            let v = !get(&c);
            set(&mut c, v);
            apply(g, &c, canvas);
        }
        SettingKind::Enum { labels, get, set } => {
            let n = labels.len().max(1) as i64;
            let v = (get(&c) + 1).rem_euclid(n);
            set(&mut c, v);
            apply(g, &c, canvas);
        }
        SettingKind::Rebind { action, .. } => g.rebind = Some(action),
        SettingKind::Action { run, .. } => {
            let conf = g.conf.clone();
            run(&mut c, &conf);
            apply(g, &c, canvas);
        }
        SettingKind::Slider { .. } => {}
    }
    Ok(true)
}

/// 键位重绑：写入 sf.keymap.<action>（Esc 取消）；返回是否停下
pub fn rebind_key(g: &mut Game, k: Keycode) -> bool {
    let Some(action) = g.rebind.take() else { return false };
    if k == Keycode::Escape {
        return true;
    }
    let name = k.name();
    g.interp.vars.sf.insert(format!("keymap.{action}"), Value::Str(name.clone()));
    persist(g);
    let label = gal_config::keymap::ACTIONS
        .iter()
        .find(|(a, _)| *a == action)
        .map(|(_, l)| *l)
        .unwrap_or(action.as_str());
    g.msg(format!("「{label}」已绑定为 {name}"));
    true
}

/// 鼠标点击：本页滑条（按 x 比例设值）→ 本页行（选中并激活）
pub fn click(g: &mut Game, canvas: &mut Canvas<Window>, lx: f32, ly: f32) {
    let Some(sel_idx) = sel(g) else { return };
    let page = settings::page_of(sel_idx);
    let env = env(g);
    let rows = settings::rows(&g.conf, &env);
    let start = page * settings::PER_PAGE;
    let visible = rows.len().saturating_sub(start).min(settings::PER_PAGE);

    for (i, row) in rows.iter().skip(start).take(visible).enumerate() {
        if let SettingKind::Slider { min, max, set, .. } = &row.kind {
            if let Some(pct) = settings::slider_hit(i, lx, ly) {
                g.overlay = Overlay::Settings { sel: start + i };
                let mut c = ctx(g, canvas);
                let v = (*min + (pct * (max - min).max(1) as f32).round() as i64).clamp(*min, *max);
                set(&mut c, v);
                apply(g, &c, canvas);
                return;
            }
        }
    }
    if let Some(i) = settings::hit_test(visible, lx, ly) {
        let idx = start + i;
        g.overlay = Overlay::Settings { sel: idx };
        let row = rows.into_iter().nth(idx).unwrap();
        let mut c = ctx(g, canvas);
        match row.kind {
            SettingKind::Toggle { get, set } => {
                let v = !get(&c);
                set(&mut c, v);
                apply(g, &c, canvas);
            }
            SettingKind::Enum { labels, get, set } => {
                let n = labels.len().max(1) as i64;
                let v = (get(&c) + 1).rem_euclid(n);
                set(&mut c, v);
                apply(g, &c, canvas);
            }
            SettingKind::Rebind { action, .. } => g.rebind = Some(action),
            SettingKind::Action { run, .. } => {
                let conf = g.conf.clone();
                run(&mut c, &conf);
                apply(g, &c, canvas);
            }
            SettingKind::Slider { .. } => {}
        }
    }
}
