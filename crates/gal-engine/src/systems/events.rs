//! 解释器系统事件消费与平台分发：音频/窗口/平台/存档 meta/越界/抖动/关机

use sdl2::render::Canvas;
use sdl2::video::Window;

use super::types::{ReachSt, RestSt, ShakeSt};
use super::Game;
use gal_script::interp::SysEvent;
use gal_ui::overlay::Overlay;

impl Game {
    /// 消费解释器系统事件（每帧）
    pub fn drain_events(&mut self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let events = std::mem::take(&mut self.interp.events);
        for ev in events {
            self.apply_event(ev, canvas)?;
        }
        Ok(())
    }

    fn apply_event(&mut self, ev: SysEvent, canvas: &mut Canvas<Window>) -> Result<(), String> {
        match ev {
            SysEvent::Bgm(n) => self.sys.audio.play_bgm(&n),
            // ===== v0.3.2 KAG3 音频标签组 =====
            SysEvent::PlayBgm { name, fade_in_ms } => {
                if fade_in_ms > 0 {
                    self.sys.audio.fade_in_bgm(&name, fade_in_ms);
                } else {
                    self.sys.audio.play_bgm(&name);
                }
            }
            SysEvent::PauseBgm => self.sys.audio.pause_bgm(),
            SysEvent::ResumeBgm => self.sys.audio.resume_bgm(),
            SysEvent::FadeInBgm { name, ms } => self.sys.audio.fade_in_bgm(&name, ms),
            SysEvent::FadeOutBgm { ms } => self.sys.audio.fade_out_bgm(ms),
            SysEvent::FadePauseBgm { ms } => self.sys.audio.fade_pause_bgm(ms),
            SysEvent::FadeBgm { to_pct, ms } => self.sys.audio.fade_bgm(to_pct, ms),
            SysEvent::XchgBgm { name, ms } => self.sys.audio.xchg_bgm(&name, ms),
            SysEvent::BgmOpt { loop_start_ms, loop_end_ms } => {
                self.sys.audio.bgmopt(loop_start_ms, loop_end_ms)
            }
            SysEvent::PlaySe { name } => self.sys.audio.play_se(&name),
            SysEvent::StopSe => self.sys.audio.stop_se(None),
            SysEvent::FadeInSe { name, ms } => self.sys.audio.play_se_impl(&name, ms, 0, None),
            SysEvent::FadeOutSe { name, ms } => self.sys.audio.play_se_impl(&name, 0, ms, None),
            SysEvent::FadeSe { to_pct, ms } => {
                self.sys.audio.fade_se(to_pct, ms);
            }
            SysEvent::SeOpt { name } => self.sys.audio.seopt(&name),
            SysEvent::BgmStop => self.sys.audio.stop_bgm(),
            SysEvent::BgmFadeOut { ms } => self.sys.audio.fade_out_bgm(ms),
            SysEvent::Se(n) => self.sys.audio.play_se(&n),
            SysEvent::Voice(n) => self.sys.audio.play_voice(&n),
            SysEvent::VoiceStop => self.sys.audio.stop_voice(),
            SysEvent::MetaFake { date, time, image } => {
                self.sys.meta.fake_saves.push(gal_save::meta::FakeSave { date, time, image });
                self.sys.meta.save(&self.sys.meta_path);
            }
            SysEvent::MetaCorrupt(n) => {
                let all: Vec<usize> = (1..=self.sys.slots).collect();
                let target: Vec<usize> = if n == "all" {
                    all
                } else {
                    vec![n.parse().map_err(|_| format!("meta_corrupt 参数无效：{n}"))?]
                };
                for s in target {
                    if !self.sys.meta.corrupt.contains(&s) {
                        self.sys.meta.corrupt.push(s);
                    }
                }
                self.sys.meta.save(&self.sys.meta_path);
            }
            SysEvent::MetaDeleteLast => {
                if gal_save::slots::latest_slot(&self.sys.save_dir, self.sys.slots).is_some() {
                    self.overlay = Overlay::Ritual { step: 0, fade: 0.0, done: false };
                } else {
                    self.msg("这里没有可以删除的存档。");
                }
            }
            SysEvent::TitleEvolve(m) => {
                self.sys.meta.title_evolve = m;
                self.sys.meta.save(&self.sys.meta_path);
            }
            SysEvent::Reach { path, dur_ms, scale } => {
                self.sys.reach =
                    Some(ReachSt { path, t: 0.0, dur_ms: dur_ms as f32, scale, hiding: false });
            }
            SysEvent::ReachHide => {
                if let Some(r) = &mut self.sys.reach {
                    r.hiding = true;
                    r.t = 0.0;
                }
            }
            SysEvent::CharSway { layer, count, period_ms } => {
                self.interp.stage.start_sway(layer as usize, count, period_ms);
            }
            SysEvent::Shake(ms) => {
                self.sys.shake = Some(ShakeSt { left_ms: ms as f32, dur_ms: ms as f32 });
            }
            // ===== v0.3.12 存档控制标签事件 =====
            SysEvent::StopSkip => {
                self.auto = false;
                self.ctrl_hold = false;
            }
            SysEvent::TempSave => match crate::input::saves::script_temp_save(self) {
                Ok(()) => {}
                Err(e) => gal_audio::log::log(
                    gal_audio::log::Level::Warn,
                    "script",
                    &format!("[tempsave] 失败：{e}"),
                ),
            },
            SysEvent::TempLoad => {
                if let Err(e) = crate::input::saves::script_temp_load(self) {
                    gal_audio::log::log(
                        gal_audio::log::Level::Debug,
                        "script",
                        &format!("[tempload] 无临时存档或失败：{e}"),
                    );
                }
            }
            SysEvent::CopyBookmark { from, to } => {
                if let Err(e) = gal_save::slots::copy_slot(&self.sys.save_dir, from, to) {
                    gal_audio::log::log(
                        gal_audio::log::Level::Warn,
                        "script",
                        &format!("[copybookmark] {from}→{to} 失败：{e}"),
                    );
                }
                self.refresh_saves();
            }
            SysEvent::EraseBookmark { slot } => {
                let path = gal_save::slots::slot_path(&self.sys.save_dir, slot);
                match std::fs::remove_file(&path) {
                    Ok(()) => gal_audio::log::log(
                        gal_audio::log::Level::Info,
                        "script",
                        &format!("[erasebookmark] 已删除槽 {slot}"),
                    ),
                    Err(e) => gal_audio::log::log(
                        gal_audio::log::Level::Debug,
                        "script",
                        &format!("[erasebookmark] 槽 {slot} 删除跳过：{e}"),
                    ),
                }
                self.refresh_saves();
            }
            SysEvent::OpenGallery => {
                self.overlay = Overlay::Gallery { sel: 0, page: 0, full: None };
            }
            SysEvent::GoBack => {
                // goback：读最近槽位（无档 WARN 留证不崩）
                if let Some(slot) =
                    gal_save::slots::latest_slot(&self.sys.save_dir, self.sys.slots.max(1))
                {
                    crate::input::saves::do_load(self, slot);
                } else {
                    gal_audio::log::log(
                        gal_audio::log::Level::Warn,
                        "script",
                        "[goback] 无任何存档可回退（演出继续）",
                    );
                }
            }

            // v0.3.11 quake 族：与 window_fx shake 同一包络通道（maxamp 留证于 DEBUG）
            SysEvent::Quake { ms, max_amp } => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    &format!("[quake] ms={ms} maxamp={max_amp}（幅度固定包络）"),
                );
                self.sys.shake = Some(ShakeSt { left_ms: ms as f32, dur_ms: ms as f32 });
            }
            SysEvent::StopQuake => {
                self.sys.shake = None;
            }
            SysEvent::SetTitle(t) => {
                let _ = canvas.window_mut().set_title(&t);
            }
            SysEvent::RestoreTitle => {
                let base = self.sys.base_title.clone();
                let _ = canvas.window_mut().set_title(&base);
            }
            SysEvent::Shutdown(sec) => {
                gal_platform::arm_shutdown(sec);
                self.sys.rest = Some(RestSt { left_ms: sec as f32 * 1000.0 });
            }
            SysEvent::DesktopWrite { file, content } => {
                if let Err(e) = gal_platform::desktop_write(&file, &content) {
                    self.msg(e);
                }
            }
            SysEvent::DesktopOpen(file) => {
                if let Err(e) = gal_platform::desktop_open(&file) {
                    self.msg(e);
                }
            }
            // ===== v0.3.8 KAG3 视频标签组（feature `video` 关闭时 WARN 放行）=====
            #[cfg(feature = "video")]
            SysEvent::VideoSet { params } => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    &format!("[video] 参数设置（记录不生效）：{params}"),
                );
            }
            #[cfg(not(feature = "video"))]
            SysEvent::VideoSet { params } => {
                gal_audio::log::log(
                    gal_audio::log::Level::Warn,
                    "script",
                    &format!("[video] 未开启 video feature：参数设置忽略（{params}）"),
                );
            }
            #[cfg(feature = "video")]
            SysEvent::PlayVideo { name, ms, skip } => {
                self.sys.video.hide_msg_ms = ms;
                self.sys.video.skip = skip;
                let path = gal_config::paths::resolve("video", &name);
                if !self.sys.video.play(&path, 1280, 720) {
                    gal_audio::log::log(
                        gal_audio::log::Level::Warn,
                        "script",
                        &format!("[video] playvideo 启动失败：{path}"),
                    );
                }
            }
            #[cfg(not(feature = "video"))]
            SysEvent::PlayVideo { name, .. } => {
                gal_audio::log::log(
                    gal_audio::log::Level::Warn,
                    "script",
                    &format!("[video] 未开启 video feature：playvideo 跳过（{name}）"),
                );
            }
            #[cfg(feature = "video")]
            SysEvent::StopVideo => self.sys.video.stop(),
            #[cfg(not(feature = "video"))]
            SysEvent::StopVideo => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    "[video] 未开启 video feature：stopvideo 忽略",
                );
            }
            #[cfg(feature = "video")]
            SysEvent::OpenVideo { name } | SysEvent::PrepareVideo { name } => {
                // 本引擎 open/prepare 与 play 等价（无独立预载管线）
                let path = gal_config::paths::resolve("video", &name);
                if !self.sys.video.play(&path, 1280, 720) {
                    gal_audio::log::log(
                        gal_audio::log::Level::Warn,
                        "script",
                        &format!("[video] openvideo 启动失败：{path}"),
                    );
                }
            }
            #[cfg(not(feature = "video"))]
            SysEvent::OpenVideo { name } | SysEvent::PrepareVideo { name } => {
                gal_audio::log::log(
                    gal_audio::log::Level::Warn,
                    "script",
                    &format!("[video] 未开启 video feature：openvideo/preparevideo 跳过（{name}）"),
                );
            }
            #[cfg(feature = "video")]
            SysEvent::PauseVideo => self.sys.video.pause(),
            #[cfg(not(feature = "video"))]
            SysEvent::PauseVideo => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    "[video] 未开启 video feature：pausevideo 忽略",
                );
            }
            #[cfg(feature = "video")]
            SysEvent::ResumeVideo => {
                // 简单实现：恢复=重播（无播放头记忆），DEBUG 留证
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    "[video] resumevideo：本引擎以重播实现（无播放头记忆）",
                );
            }
            #[cfg(not(feature = "video"))]
            SysEvent::ResumeVideo => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    "[video] 未开启 video feature：resumevideo 忽略",
                );
            }
            #[cfg(feature = "video")]
            SysEvent::RewindVideo => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    "[video] rewindvideo：本引擎以重播实现（需先 openvideo 指定片源）",
                );
            }
            #[cfg(not(feature = "video"))]
            SysEvent::RewindVideo => {
                gal_audio::log::log(
                    gal_audio::log::Level::Debug,
                    "script",
                    "[video] 未开启 video feature：rewindvideo 忽略",
                );
            }
        }
        Ok(())
    }
}
