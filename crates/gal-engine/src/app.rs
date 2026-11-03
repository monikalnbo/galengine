//! L1 主循环：初始化 → 事件泵 → 输入路由 → 解释器推进 → 系统事件 → 渲染。
//! 保持薄：具体逻辑在 input/render/systems。

use std::env;
use std::time::{Duration, Instant};

use crate::systems::Game;
use crate::Boot;
use gal_config as config;
use gal_render::assets::{self, TextureBank};
use gal_render::renderer::Renderer;
use gal_script::interp::{Interp, RunState};
use gal_script::vars::{Value, Vars};
use gal_text::font::FontBook;

// rust-sdl2 未暴露的 SDL API（键盘文本输入，IME 支持）
extern "C" {
    fn SDL_StartTextInput();
    fn SDL_StopTextInput();
}

const FRAME_BUDGET_MS: u64 = 16;

pub fn run_inner(boot: Boot) -> Result<(), String> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let _image = sdl2::image::init(sdl2::image::InitFlag::PNG | sdl2::image::InitFlag::JPG)?;
    let ttf = sdl2::ttf::init().map_err(|e| format!("字体系统初始化失败：{e}"))?;

    let conf = config::load();
    let mut canvas = make_canvas(&video, &conf)?;
    let creator = canvas.texture_creator();
    let mut renderer = Renderer::new(&creator)?;
    let mut bank = TextureBank::new(&creator, conf.display.bg_saturation);
    let families: Vec<gal_text::font::Family> = conf
        .text
        .families
        .iter()
        .map(|f| gal_text::font::Family { name: f.name.clone(), path: f.path.clone() })
        .collect();
    let mut fonts = FontBook::new(&ttf, &creator, &conf.fonts, &families)?;
    // 玩家加载过的系统字体（savedata/fonts.json）：启动即恢复，坏路径静默跳过
    {
        let path = format!("{}/fonts.json", conf.save.dir);
        let mut restored = 0;
        for p in load_sys_fonts(&path) {
            let name = std::path::Path::new(&p)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.clone());
            if fonts.add_family(&p, &name).is_some() {
                restored += 1;
            }
        }
        if restored > 0 && gal_script::interp::dbg() {
            eprintln!("[dbg font] 启动恢复系统字体 {restored} 个（fonts.json）");
        }
    }
    let mut thumbs = gal_ui::savemenu::ThumbCache::new(&creator);

    // 全局变量（sf.*）加载 + 首次启动种子默认名
    let global_path = format!("{}/global.json", conf.save.dir);
    let mut vars = Vars::load_global(&global_path);
    if !vars.sf.contains_key("playerName") {
        vars.sf.insert("playerName".into(), Value::Str(conf.game.you_default.clone()));
    }

    let script = env::var("ES_SCRIPT").unwrap_or_else(|_| conf.meta.script_start.clone());
    let label = env::var("ES_START_LABEL").ok();
    let mut interp = Interp::new(
        vars,
        conf.ui.dialog.typewriter_ms,
        &conf.game.hero_default,
        &conf.game.you_default,
    );
    interp.positions = conf.sprites.positions.clone();
    interp.chibi = conf.sprites.chibi.iter().cloned().collect();
    interp.char_fade_ms = conf.sprites.default_fade_ms;
    // 标题入口启动（不立即执行剧本；开始游戏/快读时才 start）

    let mut g = Game::new(conf, interp);
    // 玩家设置（savedata/global.json）：音量/转场/字体族/字号行距/皮肤，启动即生效
    g.apply_saved_settings(&mut fonts);
    g.cursor = gal_ui::cursor::CursorMgr::new(&g.conf); // 自定义鼠标（未配置=系统）
    g.cursor.set_default();
    let mut exts = crate::exts::registry::build(&g.conf.extensions); // yaml 内置拓展
    exts.extend(boot.extensions); // 程序化注入（Boot API）
    g.exts = exts;
    // 验收：无头环境下模拟设置面板「载入系统字体」
    if env::var("ES_SCAN_FONTS").is_ok() {
        g.scan_sys_fonts = true;
    }
    g.start_script = script;
    g.start_label = label;
    g.overlay = gal_ui::overlay::Overlay::Title { sel: 0, t: 0.0 };
    let autoclick_ms: f32 =
        env::var("ES_DEBUG_AUTOCLICK_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let mut click_acc = 0.0f32;

    // 场景级资源管理：背景变更 = 切场景（回收图片/文字纹理）
    let mut last_bg: Option<String> = None;

    let mut events = sdl.event_pump()?;
    let budget = Duration::from_millis(FRAME_BUDGET_MS);
    let mut last = Instant::now();

    let perf = std::env::var("ES_PERF").ok();
    let (mut perf_frames, mut perf_sum, mut perf_max) = (0u32, 0f64, 0f64);
    'running: loop {
        let t0 = Instant::now();
        let dt = t0.duration_since(last).as_secs_f32() * 1000.0;
        last = t0;

        // 输入态生命周期（进入/离开 WaitInput）
        maintain_input_ui(&mut g);
        for ev in events.poll_iter() {
            if !crate::input::dispatch(
                &mut g,
                ev,
                &mut canvas,
                &mut fonts,
                &mut renderer,
                &mut thumbs,
            )? {
                break 'running;
            }
        }

        // 快进（按住 Ctrl；wait/meta 段不可跳——仅台词态生效；若开启 skip_read 则仅跳已读）
        if g.ctrl_hold && g.interp.state == RunState::WaitClick {
            let skip_read_only = g.conf.game.skip_read_only
                || g.interp.vars.sf.get("skipRead") == Some(&Value::Int(1));
            if !skip_read_only || g.interp.is_cur_line_read() {
                g.interp.click()?;
            }
        }
        // 自动模式：页满延迟推进
        if g.auto && g.interp.state == RunState::WaitClick && g.interp.tw.page_full() {
            g.auto_acc += dt;
            if g.auto_acc >= g.auto_delay_ms() {
                g.auto_acc = 0.0;
                g.interp.click()?;
            }
        } else {
            g.auto_acc = 0.0;
        }
        // 验收自动驱动
        if autoclick_ms > 0.0 {
            click_acc += dt;
            if click_acc >= autoclick_ms {
                click_acc = 0.0;
                crate::input::auto_step(&mut g, &mut thumbs, &mut canvas)?;
            }
        }

        g.tick(dt);
        g.interp.tick(dt);
        g.interp.apply_pending(&mut fonts, &g.style);
        // 无障碍朗读：当前台词出现即朗读（同句去重；无后端静默）
        if g.tts.enabled && g.started && g.interp.state == gal_script::interp::RunState::WaitClick {
            let line = g.interp.cur_text().to_string();
            g.tts.speak(&line);
        }
        g.tts.tick();
        if g.scan_sys_fonts {
            g.scan_sys_fonts = false;
            load_scan_sys_fonts(&mut g, &mut fonts);
        }
        if g.style_dirty {
            g.interp.relayout(&mut fonts, &g.style);
            g.style_dirty = false;
        }
        g.sys.audio.pump();
        // v0.3.2 音频等待（wb/wl/ws/wf）：轮询音频状态，满足即放行解释器
        use gal_script::interp::{AudioWait, RunState};
        if let RunState::WaitAudio(w) = &g.interp.state {
            if *w == AudioWait::VideoDone && !g.sys.audio.ok {
                // 无声卡路径 match 被短路，wv 的作者提醒单独留证（一次等待一次）
                g.sys.audio.note_video_wait();
            }
            let a = &g.sys.audio;
            // 无声卡降级：条件立即满足（不卡演出）
            // v0.3.8：video feature 开启时 wv 以播放器状态为准（不看声卡）
            #[cfg(feature = "video")]
            let ok = match w {
                AudioWait::VideoDone => !g.sys.video.playing(),
                _ => {
                    !a.ok
                        || match w {
                            AudioWait::BgmFade => !a.bgm_fading(),
                            AudioWait::BgmLoopHit => a.peek_loop_hit(),
                            AudioWait::SeDone => !a.se_playing(),
                            AudioWait::SeFade => !a.se_fading(),
                            // v0.3.3 wm/wv/wp
                            AudioWait::MusicDone => !a.bgm_playing(),
                            AudioWait::VideoDone => !a.video_playing(),
                            AudioWait::AllDone => !a.any_playing(),
                            AudioWait::QuakeDone => g.sys.shake.is_none(),
                            AudioWait::Trig(k) => {
                                g.interp.vars.get(&format!("f.{k}")).map(gal_script::expr::truthy)
                                    == Some(true)
                            }
                        }
                }
            };
            #[cfg(not(feature = "video"))]
            let ok = !a.ok
                || match w {
                    AudioWait::BgmFade => !a.bgm_fading(),
                    AudioWait::BgmLoopHit => a.peek_loop_hit(),
                    AudioWait::SeDone => !a.se_playing(),
                    AudioWait::SeFade => !a.se_fading(),
                    AudioWait::MusicDone => !a.bgm_playing(),
                    AudioWait::VideoDone => !a.video_playing(),
                    AudioWait::AllDone => !a.any_playing(),
                    AudioWait::QuakeDone => g.sys.shake.is_none(),
                    AudioWait::Trig(k) => {
                        g.interp.vars.get(&format!("f.{k}")).map(gal_script::expr::truthy)
                            == Some(true)
                    }
                };
            if ok {
                g.interp.audio_wait_release()?;
            }
        }

        // 拓展驱动（倒计时选项等）
        for e in g.exts.iter_mut() {
            let mut ectx = gal_ext::ExtContext::new(&mut g.interp);
            e.tick(dt, &mut ectx);
        }

        // 文字速度可调（sf.textSpeed 覆盖配置）
        if let Some(Value::Int(v)) = g.interp.vars.sf.get("textSpeed") {
            if *v > 0 {
                g.interp.tw.interval_ms = *v as f32;
            }
        }

        // 玩家设置每帧对账：脚本也能写 sf.*，以最终值为准（值变才重算，开销可忽略）
        poll_player_settings(&mut g, &mut fonts);

        // 自动存档（save.auto_save_ms；仅台词等待态累计）
        if g.conf.save.auto_save_ms > 0 && g.started && g.can_save() {
            g.autosave_acc += dt;
            if g.autosave_acc >= g.conf.save.auto_save_ms as f32 {
                g.autosave_acc = 0.0;
                crate::input::saves::autosave(&mut g, &mut renderer, &mut canvas, &mut thumbs);
            }
        } else {
            g.autosave_acc = 0.0;
        }

        // 缓存层收货 + 预读喂料 + 场景切换回收
        bank.prefetch.pump();
        for (kind, storage) in g.interp.lookahead_storages(40) {
            bank.prefetch_asset(&assets::resolve(kind, &storage));
        }
        let cur_bg = g.interp.stage.bg().cur().cloned();
        if cur_bg != last_bg {
            last_bg = cur_bg;
            let mut keep = std::collections::HashSet::new();
            g.interp.stage.collect_paths(&mut keep);
            if let Some(r) = &g.sys.reach {
                keep.insert(r.path.clone());
            }
            if let Some(t) = gal_ui::title::bg_path() {
                keep.insert(t); // 标题背景常驻（回标题不再重加载）
            }
            for p in gal_ui::layers::image_paths(&g.conf, "title_screen") {
                keep.insert(p); // YAML 标题图层素材常驻
            }
            bank.retain(&keep);
            fonts.clear_text_cache();
            if gal_script::interp::dbg() {
                eprintln!(
                    "[dbg] 场景切换：保留 {} 张图，预解码 {} 张",
                    keep.len(),
                    bank.prefetch.stats_done
                );
            }
        }

        g.drain_events(&mut canvas)?;

        if g.started && g.interp.state == RunState::Ended {
            if gal_script::interp::dbg() {
                eprintln!("[dbg t={:.2}] ended", g.now_ms);
            }
            // 剧本错误（未知指令/参数）只提示不退出：停在当前句，玩家可继续或退出
            if let Some(e) = g.interp.error.take() {
                eprintln!("[剧本错误] {e}");
                if autoclick_ms > 0.0 {
                    break 'running; // 无头冒烟：报错即停，便于 CI 断言
                }
                g.msg(format!("剧本错误：{e}"));
                g.interp.state = RunState::WaitClick;
            } else if autoclick_ms > 0.0 {
                break 'running;
            } else {
                g.back_to_title();
                g.msg("剧本终了");
            }
        }

        crate::frame::frame(
            &mut g,
            &mut fonts,
            &mut bank,
            &mut renderer,
            &mut canvas,
            &mut thumbs,
            crate::frame::FrameExt { dt_ms: dt, creator: &creator },
        )?;

        let spent = t0.elapsed();
        if perf.is_some() {
            // ES_PERF：帧耗时统计（报告里的性能证据）
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            perf_frames += 1;
            perf_sum += ms;
            perf_max = perf_max.max(ms);
        }
        if spent < budget {
            std::thread::sleep(budget - spent);
        }
    }

    if perf_frames > 0 {
        eprintln!(
            "[perf] 帧数 {perf_frames} 平均 {:.2}ms 最大 {:.2}ms（约 {:.0} FPS）",
            perf_sum / perf_frames as f64,
            perf_max,
            1000.0 / (perf_sum / perf_frames as f64)
        );
    }
    g.interp.vars.save_global(&global_path)?;
    Ok(())
}

/// 输入态进入/离开：建 UI + 开/关 IME 文本输入
fn maintain_input_ui(g: &mut Game) {
    match (&g.interp.state, g.input_ui.is_some()) {
        (RunState::WaitInput(spec), false) => {
            let presets = gal_ui::inputbox::presets_for(&g.conf, &spec.var);
            g.input_ui = Some(gal_ui::inputbox::InputUi::new(
                (**spec).clone(),
                presets,
                g.conf.ui.input.max_len,
            ));
            unsafe { SDL_StartTextInput() };
        }
        (RunState::WaitInput(_), true) => {}
        _ => {
            if g.input_ui.take().is_some() {
                unsafe { SDL_StopTextInput() };
            }
        }
    }
}

/// 玩家设置对账：字体族索引变化 → 切族并清缓存；字号/行距/皮肤/转场变化 → 重算
fn poll_player_settings(g: &mut Game, fonts: &mut FontBook) {
    // 快照 sf.*（几十项，克隆开销可忽略）避免闭包长期借用 g 与下面的可变调用冲突
    let sf = g.interp.vars.sf.clone();
    let int = |k: &str| sf.get(k).map(|v| v.as_i64());
    let want_font = int("fontFamily").unwrap_or(0).max(0) as usize;
    if want_font != fonts.family_index() && want_font < fonts.family_names().len() {
        fonts.set_family(want_font);
        g.style_dirty = true;
    }
    g.interp.trans_enabled = int("transOn").map(|v| v != 0).unwrap_or(true);
    // 语言/朗读：脚本或设置改了 sf.* 都走同一条对账（值变才动）
    let tts_on = int("ttsOn");
    let tts_rate = int("ttsRate");
    let lang_now = crate::systems::lang_for(&g.langs, &g.interp.vars);
    if lang_now.code != g.lang.code || g.interp.lang != g.lang.strings {
        g.apply_lang();
    }
    let want_tts = tts_on.map(|v| v == 1).unwrap_or(g.tts.enabled) && g.tts.backend.is_some();
    if want_tts != g.tts.enabled {
        g.tts.enabled = want_tts;
        if !want_tts {
            g.tts.stop();
        }
    }
    g.tts.rate = tts_rate.unwrap_or(g.tts.rate as i64).clamp(-10, 10) as i32;
    // 字号/行距/等待标记/皮肤：与当前样式不同才重算（重算会触发台词重排）
    let want = crate::systems::compose_style(&g.conf, &g.skins, None, &g.interp.vars);
    let changed = want.font_size != g.style.font_size
        || want.line_spacing != g.style.line_spacing
        || want.wait_marker != g.style.wait_marker
        || want.fill != g.style.fill
        || want.border != g.style.border
        || want.text_color != g.style.text_color;
    if changed {
        if gal_script::interp::dbg() {
            eprintln!(
                "[dbg settings] 运行期重算：字号 {}→{} 行距 {}→{} 标记 {}→{}",
                g.style.font_size,
                want.font_size,
                g.style.line_spacing,
                want.line_spacing,
                g.style.wait_marker,
                want.wait_marker
            );
        }
        g.style = want;
        let idx = int("skinIndex").unwrap_or(0).max(0) as usize;
        g.skin = g.skins.get(idx).cloned().unwrap_or_else(|| g.skins[0].clone());
        g.style_dirty = true;
    }
}

/// 可自由缩放窗口；首选加速+vsync，无 GPU（xvfb 验收）回退软件渲染
fn make_canvas(
    video: &sdl2::VideoSubsystem,
    conf: &config::Config,
) -> Result<sdl2::render::Canvas<sdl2::video::Window>, String> {
    let (w, h) = (conf.display.width.max(320), conf.display.height.max(240));
    let resizable = conf.display.resizable;
    let title = conf.meta.title.as_str();
    let build = || {
        video.window(title, w, h).position_centered().resizable().build().map_err(|e| e.to_string())
    };
    let mut canvas = match build()?.into_canvas().accelerated().present_vsync().build() {
        Ok(c) => c,
        Err(_) => build()?.into_canvas().software().build().map_err(|e| e.to_string())?,
    };
    canvas.window_mut().set_resizable(resizable);
    Ok(canvas)
}

/// savedata/fonts.json：玩家运行期加载过的系统字体路径
fn load_sys_fonts(path: &str) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("fonts").cloned())
        .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
        .unwrap_or_default()
}

fn save_sys_fonts(path: &str, list: &[String]) {
    let obj = serde_json::json!({ "fonts": list });
    if let Some(dir) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match serde_json::to_string_pretty(&obj) {
        Ok(s) => {
            if let Err(e) = std::fs::write(path, s) {
                eprintln!("[font] fonts.json 写入失败：{e}");
            }
        }
        Err(e) => eprintln!("[font] fonts.json 序列化失败：{e}"),
    }
}

/// 设置面板「载入系统字体」：扫描常见字体目录，新族追加到 FontBook，
/// 路径存 savedata/fonts.json（重启仍在），并切到第一个新族。
fn load_scan_sys_fonts(g: &mut Game, fonts: &mut FontBook) {
    let path = format!("{}/fonts.json", g.conf.save.dir);
    let mut list = load_sys_fonts(&path);
    let before = fonts.family_names().len();
    'outer: for dir in gal_platform::font_dirs() {
        for p in gal_platform::list_font_files(&dir) {
            if fonts.family_names().len() >= before + 16 {
                break 'outer;
            }
            let name = std::path::Path::new(&p)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.clone());
            if fonts.add_family(&p, &name).is_some() {
                list.push(p);
            }
        }
    }
    let after = fonts.family_names().len();
    if after > before {
        list.dedup();
        save_sys_fonts(&path, &list);
        // 切到第一个新载入的族，玩家立刻看到效果
        g.interp.vars.sf.insert("fontFamily".into(), Value::Int(before as i64));
        fonts.set_family(before);
        g.font_families = fonts.family_names().to_vec();
        g.style_dirty = true;
        g.msg(format!("已载入 {} 个系统字体（字体行可切换）", after - before));
        eprintln!("[dbg font] 载入系统字体 {} 个（总 {} 族）", after - before, after);
    } else {
        g.msg("没找到可用的系统字体");
    }
}
