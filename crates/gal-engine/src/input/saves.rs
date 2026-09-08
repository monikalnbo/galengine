//! 存读档执行：界面打开 / 槽位激活 / 存（快照+缩略图+指纹）/ 读（校验+恢复）。

use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::systems::Game;
use gal_config as config;
use gal_render::renderer::Renderer;
use gal_ui::overlay::Overlay;
use gal_ui::savemenu::ThumbCache;

/// 打开存/读界面（刷新槽位快照 + 缩略图缓存）
pub fn open_save_menu(g: &mut Game, thumbs: &mut ThumbCache, mode_save: bool) {
    g.refresh_saves();
    thumbs.build(&g.save_entries, &g.sys.meta.fake_saves);
    g.overlay = Overlay::Save { mode_save, sel: 0 };
}

/// 存档界面槽位激活
pub fn save_activate(
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
pub fn do_save(
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
    let entry = gal_save::slots::SaveEntry {
        slot,
        title: g.interp.last_text.clone(),
        stamp: gal_save::slots::now_stamp(),
        fingerprint: gal_save::slots::fingerprint(&format!(
            "{}/scenario/{file}",
            config::data_dir()
        )),
        file,
        line,
        fvars: g.interp.vars.f.clone(),
        snap: g.interp.stage.snapshot(),
        bgm: g.interp.cur_bgm.clone(),
        thumb: gal_save::slots::thumb_png(&rgba, config::LOGICAL_W, config::LOGICAL_H),
    };
    gal_save::slots::save(&g.sys.save_dir, &entry)?;
    g.refresh_saves();
    thumbs.build(&g.save_entries, &g.sys.meta.fake_saves);
    g.msg(format!("已保存：槽 {slot}"));
    Ok(())
}

/// 读档执行（指纹校验 + 演出层快照恢复）
pub fn do_load(g: &mut Game, slot: usize) {
    match gal_save::slots::load_checked(&g.sys.save_dir, slot, &config::data_dir()) {
        Err(e) => g.msg(e),
        Ok(e) => {
            let res = g.interp.restore(&e.file, e.line, e.snap, e.fvars);
            match res {
                Ok(()) => {
                    // BGM 随档恢复：场景音乐回到存档时刻（有则播，无则停）
                    match &e.bgm {
                        Some(name) => g.sys.audio.play_bgm(name),
                        None => g.sys.audio.stop_bgm(),
                    }
                    g.interp.cur_bgm = e.bgm.clone();
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
