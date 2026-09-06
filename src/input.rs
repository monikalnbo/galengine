//! L1 游戏输入路由：事件按「关机 → 覆盖层 → 台词/选项/输入三态」分发。
//! L2 不碰 SDL 事件泵，本层是唯一的 SDL 事件翻译点。

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::config;
use crate::gfx::renderer::Renderer;
use crate::script::interp::RunState;
use crate::script::vars::Value;
use crate::systems::Game;
use crate::text::font::FontBook;
use crate::ui::inputbox::{self, Hit};
use crate::ui::overlay::{esc_items, title_items, Overlay};
use crate::ui::ritual;
use crate::ui::savemenu::{self, ThumbCache};
use crate::ui::{choice, menu};

/// 处理一条事件；返回 false=请求退出
#[allow(clippy::too_many_arguments)]
pub fn dispatch(
    g: &mut Game,
    ev: Event,
    canvas: &mut Canvas<Window>,
    _fonts: &mut FontBook,
    renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    match ev {
        Event::Quit { .. } => return Ok(false),

        Event::KeyDown { keycode: Some(k), .. } => key_down(g, k, canvas, renderer, thumbs),

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
            if let Some(i) = choice::hit_test(g.choice_len(), lx, ly) {
                g.choice_sel = i;
            }
            Ok(true)
        }

        Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => {
            let (lx, ly) = Renderer::to_logical(canvas, x, y);
            click(g, lx, ly, canvas, renderer, thumbs)
        }
        _ => Ok(true),
    }
}

fn key_down(
    g: &mut Game,
    k: Keycode,
    canvas: &mut Canvas<Window>,
    renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    if volume_hotkey(g, k) {
        return Ok(true);
    }
    let mut quit = false;
    match k {
        Keycode::Escape => {
            if g.sys.rest.is_some() {
                g.cancel_rest();
            } else if g.overlay.active() {
                g.overlay = Overlay::None;
            } else {
                g.overlay = Overlay::Menu { sel: 0 };
            }
        }
        Keycode::F9 => {
            do_save(g, renderer, canvas, thumbs, g.sys.slots)?;
        }
        Keycode::F10 => do_load(g, g.sys.slots),
        Keycode::Return | Keycode::KpEnter | Keycode::Space => {
            if !confirm_key(g, canvas, renderer, thumbs)? {
                quit = true;
            }
        }
        Keycode::LCtrl | Keycode::RCtrl => g.ctrl_hold = true,
        Keycode::A if g.interp.state == RunState::WaitClick && !g.overlay.active() => {
            g.auto = !g.auto;
            g.auto_acc = 0.0;
        }
        Keycode::Up | Keycode::Down | Keycode::Left | Keycode::Right if g.overlay.active() => {
            overlay_move(g, k)
        }
        Keycode::Up if g.choice_len() > 0 => g.choice_sel = g.choice_sel.saturating_sub(1),
        Keycode::Down if g.choice_len() > 0 => g.choice_sel = (g.choice_sel + 1).min(g.choice_len() - 1),
        Keycode::Backspace if g.input_ui.is_some() => {
            if let Some(ui) = g.input_ui.as_mut() {
                ui.backspace();
            }
        }
        _ => {}
    }
    Ok(!quit)
}

fn confirm_key(
    g: &mut Game,
    canvas: &mut Canvas<Window>,
    renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    if g.sys.rest.is_some() {
        g.cancel_rest();
        return Ok(true);
    }
    match g.overlay.clone() {
        Overlay::Menu { sel } => menu_activate(g, sel, canvas, renderer, thumbs),
        Overlay::Save { mode_save, sel } => {
            save_activate(g, canvas, renderer, thumbs, mode_save, sel)?;
            Ok(true)
        }
        Overlay::Ritual { .. } => {
            ritual_confirm(g)?;
            Ok(true)
        }
        Overlay::Title { sel } => title_activate(g, sel, thumbs),
        Overlay::Gallery { .. } => {
            gallery_confirm(g);
            Ok(true)
        }
        Overlay::Volume { .. } => {
            g.overlay = Overlay::None;
            Ok(true)
        }
        Overlay::None => match &g.interp.state {
            RunState::WaitChoice { .. } => g.interp.choose(g.choice_sel).map(|_| true),
            RunState::WaitInput(_) => confirm_input(g).map(|_| true),
            RunState::WaitClick => g.interp.click().map(|_| true),
            _ => Ok(true),
        },
    }
}

fn click(
    g: &mut Game,
    lx: f32,
    ly: f32,
    canvas: &mut Canvas<Window>,
    renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    if g.sys.rest.is_some() {
        if crate::ui::restui::cancel_rect().contains_point((lx as i32, ly as i32)) {
            g.cancel_rest();
        }
        return Ok(true);
    }
    if g.overlay.active() {
        match g.overlay.clone() {
            Overlay::Menu { .. } => {
                if let Some(i) = menu::hit_test(esc_items().len(), 250, lx, ly) {
                    g.overlay = Overlay::Menu { sel: i };
                    if !menu_activate(g, i, canvas, renderer, thumbs)? {
                        return Ok(false);
                    }
                }
            }
            Overlay::Save { mode_save, .. } => {
                let n = g.save_entries.len() + g.sys.meta.fake_saves.len();
                if let Some(i) = savemenu::hit_test(n, lx, ly) {
                    g.overlay = Overlay::Save { mode_save, sel: i };
                    save_activate(g, canvas, renderer, thumbs, mode_save, i)?;
                }
            }
            Overlay::Ritual { .. } => {
                if ritual::del_rect().contains_point((lx as i32, ly as i32)) {
                    ritual_confirm(g)?;
                } else if ritual::back_rect().contains_point((lx as i32, ly as i32)) {
                    g.overlay = Overlay::None; // 「回头」路径
                }
            }
            Overlay::Title { .. } => {
                if let Some(i) = menu::hit_test(title_items().len(), 380, lx, ly) {
                    g.overlay = Overlay::Title { sel: i };
                    if !title_activate(g, i, thumbs)? {
                        return Ok(false);
                    }
                }
            }
            Overlay::Gallery { page, full, .. } => match full {
                Some(_) => g.overlay = Overlay::Gallery { sel: 0, page, full: None },
                None => {
                    if let Some(i) = crate::ui::gallery::hit_test(crate::ui::gallery::per_page(), lx, ly) {
                        g.overlay = Overlay::Gallery { sel: i, page, full: None };
                        gallery_confirm(g);
                    }
                }
            },
            Overlay::Volume { .. } => {
                if let Some(i) = crate::ui::volume::hit_row(lx, ly) {
                    g.overlay = Overlay::Volume { sel: i };
                }
            }
            Overlay::None => {}
        }
        return Ok(true);
    }
    match &g.interp.state {
        RunState::WaitChoice { .. } => {
            if let Some(i) = choice::hit_test(g.choice_len(), lx, ly) {
                g.interp.choose(i)?;
            }
        }
        RunState::WaitInput(_) => match g.input_ui.as_ref().and_then(|ui| inputbox::hit_test(ui, lx, ly)) {
            Some(Hit::Preset(i)) => {
                if let Some(ui) = g.input_ui.as_mut() {
                    if let Some(p) = ui.presets.get(i) {
                        ui.buf = p.clone();
                    }
                }
            }
            Some(Hit::Confirm) => confirm_input(g)?,
            None => {}
        },
        RunState::WaitClick => g.interp.click()?,
        _ => {}
    }
    Ok(true)
}

/// 覆盖层方向键移动（菜单/标题竖排；存档/鉴赏网格；音量竖排）
fn overlay_move(g: &mut Game, k: Keycode) {
    if let Overlay::Volume { sel } = &mut g.overlay {
        *sel = match k {
            Keycode::Up => (*sel).saturating_sub(1),
            Keycode::Down => (*sel + 1).min(2),
            Keycode::Left | Keycode::Right => {
                volume_adjust(g, if k == Keycode::Right { 1 } else { -1 });
                return;
            }
            _ => *sel,
        };
        return;
    }
    if let Overlay::Gallery { sel, page, full } = &mut g.overlay {
        if full.is_some() {
            // 全屏：←→ 在已解锁 CG 间翻页
            let unlocked: Vec<usize> = {
                let list = crate::ui::gallery::catalog();
                let opened = g.interp.vars.get_or("sf.cgs").as_str();
                list.iter()
                    .enumerate()
                    .filter(|(_, n)| opened.split(',').any(|x| x == n.as_str()))
                    .map(|(i, _)| i)
                    .collect()
            };
            if unlocked.is_empty() {
                return;
            }
            let cur = full.map_or(0, |v| v);
            let pos = unlocked.iter().position(|&i| i == cur).unwrap_or(0);
            let next = match k {
                Keycode::Left | Keycode::Up => (pos + unlocked.len() - 1) % unlocked.len(),
                Keycode::Right | Keycode::Down => (pos + 1) % unlocked.len(),
                _ => pos,
            };
            let idx = unlocked[next];
            *full = Some(idx);
            *page = idx / crate::ui::gallery::per_page();
            *sel = idx % crate::ui::gallery::per_page();
            return;
        }
        let n = crate::ui::gallery::catalog().len();
        if n == 0 {
            return;
        }
        let per = crate::ui::gallery::per_page();
        let mut idx = *page * per + *sel;
        idx = match k {
            Keycode::Up => idx.saturating_sub(4),
            Keycode::Down => (idx + 4).min(n - 1),
            Keycode::Left => idx.saturating_sub(1),
            Keycode::Right => (idx + 1).min(n - 1),
            _ => idx,
        };
        *page = idx / per;
        *sel = idx % per;
        return;
    }
    let n = match &g.overlay {
        Overlay::Menu { .. } => esc_items().len(),
        Overlay::Title { .. } => title_items().len(),
        Overlay::Save { .. } => g.save_entries.len() + g.sys.meta.fake_saves.len(),
        _ => return,
    };
    if n == 0 {
        return;
    }
    let cur = match &g.overlay {
        Overlay::Menu { sel } | Overlay::Save { sel, .. } | Overlay::Title { sel } => *sel,
        _ => 0,
    };
    let cols = match g.overlay {
        Overlay::Save { .. } => 3,
        _ => 1,
    };
    let next = match k {
        Keycode::Up => cur.saturating_sub(cols),
        Keycode::Down => (cur + cols).min(n - 1),
        Keycode::Left => cur.saturating_sub(1),
        Keycode::Right => (cur + 1).min(n - 1),
        _ => cur,
    };
    match &mut g.overlay {
        Overlay::Menu { sel } | Overlay::Save { sel, .. } | Overlay::Title { sel } => *sel = next,
        _ => {}
    }
}

/// 系统菜单项激活；返回 false=退出游戏（走正常收尾，sf.* 落盘）
fn menu_activate(
    g: &mut Game,
    idx: usize,
    _canvas: &mut Canvas<Window>,
    _renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
) -> Result<bool, String> {
    match idx {
        0 => g.overlay = Overlay::None,          // 继续
        1 => open_save_menu(g, thumbs, true),    // 存档
        2 => open_save_menu(g, thumbs, false),   // 读档
        3 => g.overlay = Overlay::Gallery { sel: 0, page: 0, full: None }, // 鉴赏
        4 => g.overlay = Overlay::Volume { sel: 0 },                        // 音量/速度
        5 => g.back_to_title(),                  // 回标题
        6 => return Ok(false),                   // 退出游戏
        _ => {}
    }
    Ok(true)
}

/// 标题菜单项激活
fn title_activate(g: &mut Game, idx: usize, thumbs: &mut ThumbCache) -> Result<bool, String> {
    match idx {
        0 => g.start_game()?,                    // 开始游戏
        1 => {
            // 继续游戏：最新档；无档提示
            let (dir, n) = (g.sys.save_dir.clone(), g.sys.slots);
            match crate::save::slots::latest_slot(&dir, n) {
                Some(slot) => do_load(g, slot),
                None => g.msg("还没有任何存档。"),
            }
        }
        2 => open_save_menu(g, thumbs, false),   // 读档
        3 => g.overlay = Overlay::Gallery { sel: 0, page: 0, full: None },
        4 => g.overlay = Overlay::Volume { sel: 0 },
        5 => return Ok(false),                   // 退出游戏
        _ => {}
    }
    Ok(true)
}

/// 鉴赏确认：网格→全屏 / 全屏→网格；锁定不可开
fn gallery_confirm(g: &mut Game) {
    if let Overlay::Gallery { sel, page, full } = &mut g.overlay {
        match full {
            None => {
                let idx = *page * crate::ui::gallery::per_page() + *sel;
                let items = crate::ui::gallery::catalog();
                if let Some(name) = items.get(idx) {
                    let unlocked = g.interp.vars.get_or("sf.cgs").as_str();
                    if unlocked.split(',').any(|x| x == name) {
                        *full = Some(idx);
                    } else {
                        g.msg("这张 CG 还没有解锁。");
                    }
                }
            }
            Some(_) => *full = None,
        }
    }
}

/// 音量/速度滑条调整（dir=±1）
fn volume_adjust(g: &mut Game, dir: i64) {
    let Overlay::Volume { sel } = g.overlay else { return };
    match sel {
        0 => {
            g.sys.audio.bgm_vol = (g.sys.audio.bgm_vol as i64 + dir * 5).clamp(0, 100) as i32;
            g.interp.vars.sf.insert("volBgm".into(), Value::Int(g.sys.audio.bgm_vol as i64));
        }
        1 => {
            g.sys.audio.se_vol = (g.sys.audio.se_vol as i64 + dir * 5).clamp(0, 100) as i32;
            g.interp.vars.sf.insert("volSe".into(), Value::Int(g.sys.audio.se_vol as i64));
        }
        _ => {
            let v = (g.interp.tw.interval_ms as i64 + dir * 5).clamp(5, 200);
            g.interp.tw.interval_ms = v as f32;
            g.interp.vars.sf.insert("textSpeed".into(), Value::Int(v));
        }
    }
    g.sys.audio.apply_volumes();
}

/// 音量界面当前值（绘制/启动恢复用）
pub fn volume_values(g: &Game) -> [i64; 3] {
    [
        g.sys.audio.bgm_vol as i64,
        g.sys.audio.se_vol as i64,
        g.interp.tw.interval_ms as i64,
    ]
}

fn open_save_menu(g: &mut Game, thumbs: &mut ThumbCache, mode_save: bool) {
    g.refresh_saves();
    thumbs.build(&g.save_entries, &g.sys.meta.fake_saves);
    g.overlay = Overlay::Save { mode_save, sel: 0 };
}

/// 存档界面槽位激活
fn save_activate(
    g: &mut Game,
    canvas: &mut Canvas<Window>,
    renderer: &mut Renderer,
    thumbs: &mut ThumbCache,
    mode_save: bool,
    sel: usize,
) -> Result<(), String> {
    if sel >= g.save_entries.len() {
        g.msg("这个存档……无法读取。"); // 不明存档
        return Ok(());
    }
    let slot = g.save_entries[sel].0;
    if mode_save {
        do_save(g, renderer, canvas, thumbs, slot)?;
    } else {
        do_load(g, slot);
    }
    Ok(())
}

/// 存档执行（快照演出层+缩略图+指纹）
fn do_save(
    g: &mut Game,
    renderer: &mut Renderer,
    canvas: &mut Canvas<Window>,
    thumbs: &mut ThumbCache,
    slot: usize,
) -> Result<(), String> {
    if !g.can_save() {
        g.msg("现在无法存档。");
        return Ok(());
    }
    let (file, line) = g.interp.resume_point().expect("can_save 已保证");
    let rgba = renderer.read_screen(canvas)?;
    let entry = crate::save::slots::SaveEntry {
        slot,
        title: g.interp.last_text.clone(),
        stamp: crate::save::slots::now_stamp(),
        fingerprint: crate::save::slots::fingerprint(&format!("{}/scenario/{file}", config::data_dir())),
        file,
        line,
        fvars: g.interp.vars.f.clone(),
        snap: g.interp.stage.snapshot(),
        thumb: crate::save::slots::thumb_png(&rgba, config::LOGICAL_W, config::LOGICAL_H),
    };
    crate::save::slots::save(&g.sys.save_dir, &entry)?;
    g.refresh_saves();
    thumbs.build(&g.save_entries, &g.sys.meta.fake_saves);
    g.msg(format!("已保存：槽 {slot}"));
    Ok(())
}

/// 读档执行（指纹校验 + 演出层快照恢复）
fn do_load(g: &mut Game, slot: usize) {
    match crate::save::slots::load_checked(&g.sys.save_dir, slot, &config::data_dir()) {
        Err(e) => g.msg(e),
        Ok(e) => {
            let res = g.interp.restore(&e.file, e.line, e.snap, e.fvars);
            match res {
                Ok(()) => {
                    g.overlay = Overlay::None;
                    g.auto = false;
                    g.started = true; // 从标题快读也直接开局
                    g.msg(format!("已读取：槽 {slot}"));
                }
                Err(e) => g.msg(e),
            }
        }
    }
}

/// 仪式删档：确认推进；第三击后渐白，白满真删
fn ritual_confirm(g: &mut Game) -> Result<(), String> {
    if let Overlay::Ritual { step, .. } = &mut g.overlay {
        *step += 1;
    }
    Ok(())
}

fn confirm_input(g: &mut Game) -> Result<(), String> {
    if let Some(ui) = g.input_ui.take() {
        g.interp.input_result(ui.value())?;
    }
    Ok(())
}

/// 验收自动驱动（ES_DEBUG_AUTOCLICK_MS）：标题=开始，台词=点击，选项=选0，输入=默认值
pub fn auto_step(g: &mut Game, thumbs: &mut ThumbCache) -> Result<(), String> {
    if g.sys.rest.is_some() {
        g.cancel_rest(); // 验收环境自动取消关机，跑完剧本
        return Ok(());
    }
    if g.overlay.active() {
        // 验收驱动：覆盖层默认首项/确认推进（标题开始/菜单继续/仪式确认）
        match g.overlay.clone() {
            Overlay::Title { sel, .. } => {
                if !title_activate(g, sel, thumbs)? {
                    return Ok(());
                }
            }
            Overlay::Menu { .. } => g.overlay = Overlay::None, // 直接继续游戏
            Overlay::Save { .. } | Overlay::Gallery { .. } | Overlay::Volume { .. } => {
                g.overlay = Overlay::None;
            }
            Overlay::Ritual { step, .. } => {
                g.overlay = Overlay::Ritual { step: step + 1, fade: 0.0, done: false };
            }
            _ => {}
        }
        return Ok(());
    }
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

/// F5/F6 BGM 音量 −/+，F7/F8 SE 音量 −/+（存 sf.*，规格 §2.4）
fn volume_hotkey(g: &mut Game, k: Keycode) -> bool {
    let (delta, is_bgm) = match k {
        Keycode::F5 => (-10, true),
        Keycode::F6 => (10, true),
        Keycode::F7 => (-10, false),
        Keycode::F8 => (10, false),
        _ => return false,
    };
    let (name, v) = if is_bgm {
        g.sys.audio.bgm_vol = (g.sys.audio.bgm_vol + delta).clamp(0, 100);
        g.interp.vars.sf.insert("volBgm".into(), Value::Int(g.sys.audio.bgm_vol as i64));
        ("BGM", g.sys.audio.bgm_vol)
    } else {
        g.sys.audio.se_vol = (g.sys.audio.se_vol + delta).clamp(0, 100);
        g.interp.vars.sf.insert("volSe".into(), Value::Int(g.sys.audio.se_vol as i64));
        ("SE", g.sys.audio.se_vol)
    };
    g.sys.audio.apply_volumes();
    g.msg(format!("{name} 音量 {v}"));
    true
}

impl Game {
    pub fn choice_len(&self) -> usize {
        match &self.interp.state {
            RunState::WaitChoice { items, .. } => items.len(),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod sdl_tests {
    use super::*;
    use crate::config;
    use crate::script::interp::Interp;

    /// 存→读 全链路（SDL dummy 驱动，无窗口）：缩略图/指纹/演出层快照/变量恢复
    #[test]
    fn 存读档端到端() {
        std::env::set_var("SDL_VIDEODRIVER", "dummy");
        std::env::set_var("SDL_AUDIODRIVER", "dummy");
        std::env::set_var("ES_DATA_DIR", "testdata/game/data");

        let sdl = sdl2::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video.window("t", 1280, 720).build().unwrap();
        let mut canvas = window.into_canvas().software().build().unwrap();
        let creator = canvas.texture_creator();
        let mut renderer = Renderer::new(&creator).unwrap();
        let mut thumbs = ThumbCache::new(&creator);

        let conf = config::load();
        let mut interp = Interp::new(
            crate::script::vars::Vars::default(),
            30.0,
            &conf.game.hero_default,
            &conf.game.you_default,
        );
        interp.start("a3_smoke.ks", None).unwrap();
        // 走两句台词并设个 f.* 变量，让快照非平凡
        interp.click().unwrap(); // 补全
        interp.click().unwrap(); // 行完 → 下一句
        interp.vars.set("f.probe", Value::Int(7));
        let mut g = Game::new(conf, interp);

        let dir = std::env::temp_dir().join("galengine_e2e_save");
        let _ = std::fs::remove_dir_all(&dir);
        g.sys.save_dir = dir.to_str().unwrap().to_string();

        renderer.compose(&mut canvas, 0, 0, |_| Ok(())).unwrap();
        do_save(&mut g, &mut renderer, &mut canvas, &mut thumbs, 1).unwrap();
        assert!(crate::save::slots::load(&g.sys.save_dir, 1).is_some());

        // 推进几句台词并污染 f.*，验证读回时位置与变量都恢复
        for _ in 0..8 {
            let _ = g.interp.tw.click(); // 补全/翻页，让行完可推进
            g.interp.click().unwrap();
        }
        g.interp.vars.set("f.probe", Value::Int(99));
        do_load(&mut g, 1);
        assert_eq!(g.interp.vars.get_or("f.probe"), Value::Int(7));
        assert_eq!(g.interp.state, RunState::WaitClick);

        // 缩略图缓存构建 + meta 不明档
        g.sys.meta.fake_saves.push(crate::save::meta::FakeSave {
            date: "7月7日".into(),
            time: "23:59".into(),
            image: "bgimage/bg_black.jpg".into(),
        });
        g.refresh_saves();
        thumbs.build(&g.save_entries, &g.sys.meta.fake_saves);
        assert!(thumbs.get("s1").is_some());
        assert!(thumbs.get("f0").is_some());
    }

    impl Interp {
        /// 测试辅助：连续推进直到下一个等待点稳定
        fn tick_check(&mut self) -> RunState {
            self.tick(16.0);
            self.state.clone()
        }
    }
}
