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
            SysEvent::BgmStop => self.sys.audio.stop_bgm(),
            SysEvent::BgmFadeOut { ms } => self.sys.audio.fade_out_bgm(ms),
            SysEvent::Se(n) => self.sys.audio.play_se(&n),
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
            SysEvent::Shake(ms) => {
                self.sys.shake = Some(ShakeSt { left_ms: ms as f32, dur_ms: ms as f32 });
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
        }
        Ok(())
    }
}
