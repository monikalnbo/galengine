//! 覆盖层状态机：标题/菜单/存读档/鉴赏/音量/删档仪式/晚安关机。
//! 层间协议：本层只流转状态与绘制，系统动作以 Action 返回，由 app(L1) 执行。

use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::gfx::assets::TextureBank;
use crate::save::meta::MetaState;
use crate::text::font::FontBook;
use crate::ui::widgets;
use crate::ui::{gallery::Gallery, menu, restui, ritual, savemenu, title, volume};

#[derive(Clone, Debug)]
pub enum Overlay {
    None,
    Title { sel: usize },
    Menu { sel: usize },
    Save { sel: usize },
    Load { sel: usize },
    Gallery(Box<Gallery>),
    Volume { sel: usize, bgm: i32, se: i32, tw_ms: f32 },
    /// 删档仪式：确认步 / 光页渐白 / 尾声
    Ritual { step: u8, slot: usize, flash: f32, phase: u8 },
    /// 晚安关机倒计时
    Rest { left_ms: f32, total: u32, cancelled: bool, finished: bool, grace: f32 },
}

/// 请求 app 执行的系统动作
#[derive(Debug)]
pub enum Action {
    None,
    Close,
    Quit,
    StartNew,
    ContinueRun,
    SaveTo(usize),
    LoadFrom(usize),
    /// 真删（先自动备份）
    DeleteSlot(usize),
    /// 光页结束（台词收尾由剧本负责）
    RitualDone,
    ApplyVolumes { bgm: i32, se: i32, tw_ms: f32 },
    CancelShutdown,
}

/// 绘制/命中依赖的外部数据快照（app 每帧组装；值语义避免借用耦合）
pub struct Ctx {
    pub meta: MetaState,
    pub slots: Vec<Option<crate::save::slots::SaveData>>,
    pub save_dir: String,
    pub all_cgs: Vec<String>,
    pub unlocked_cgs: Vec<String>,
    pub you: String,
    pub volumes: (i32, i32),
    pub tw_ms: f32,
    pub title_stage: u32,
    pub title_main: String,
    pub title_sub: String,
    pub ritual_texts: Vec<String>,
}

pub struct OverlaySys {
    pub cur: Overlay,
}

impl Default for OverlaySys {
    fn default() -> Self {
        Self {
            cur: Overlay::Title { sel: 0 },
        }
    }
}

impl OverlaySys {
    pub fn active(&self) -> bool {
        !matches!(self.cur, Overlay::None)
    }

    pub fn open_gallery(&mut self, ctx: &Ctx) {
        self.cur = Overlay::Gallery(Box::new(Gallery {
            sel: 0,
            view: None,
            all: ctx.all_cgs.clone(),
            unlocked: ctx.unlocked_cgs.clone(),
        }));
    }

    pub fn open_volume(&mut self, ctx: &Ctx) {
        self.cur = Overlay::Volume {
            sel: 0,
            bgm: ctx.volumes.0,
            se: ctx.volumes.1,
            tw_ms: ctx.tw_ms,
        };
    }

    pub fn open_menu(&mut self) {
        self.cur = Overlay::Menu { sel: 0 };
    }
    pub fn open_ritual(&mut self, slot: usize) {
        self.cur = Overlay::Ritual { step: 0, slot, flash: 0.0, phase: 0 };
    }
    pub fn open_rest(&mut self, secs: u32) {
        self.cur = Overlay::Rest {
            left_ms: secs as f32 * 1000.0,
            total: secs,
            cancelled: false,
            finished: false,
            grace: 0.0,
        };
    }

    /// 帧推进（倒计时/动画），返回需要执行的动作
    pub fn tick(&mut self, dt_ms: f32, ctx: &Ctx) -> Action {
        match &mut self.cur {
            Overlay::Ritual { step, flash, phase, .. } => {
                if *phase == 1 {
                    *flash += dt_ms / 2400.0;
                    if *flash >= 1.0 {
                        *phase = 2;
                        return Action::RitualDone;
                    }
                }
                let _ = (step, ctx);
                Action::None
            }
            Overlay::Rest { left_ms, finished, cancelled, grace, .. } => {
                if *cancelled || *finished {
                    *grace += dt_ms;
                    if *grace > 2000.0 {
                        self.cur = Overlay::None;
                    }
                    return Action::None;
                }
                *left_ms -= dt_ms;
                if *left_ms <= 0.0 {
                    *finished = true;
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    pub fn key(&mut self, k: Keycode, ctx: &Ctx) -> Action {
        let nav = |sel: &mut usize, n: usize, k: Keycode| -> bool {
            match k {
                Keycode::Up => {
                    *sel = sel.saturating_sub(1);
                    true
                }
                Keycode::Down => {
                    *sel = (*sel + 1).min(n - 1);
                    true
                }
                _ => false,
            }
        };
        match &mut self.cur {
            Overlay::Title { sel } => {
                nav(sel, title::ITEMS.len(), k);
                let fired = matches!(k, Keycode::Return | Keycode::KpEnter | Keycode::Space);
                let s = *sel;
                if fired {
                    self.title_activate(s, ctx)
                } else {
                    Action::None
                }
            }
            Overlay::Menu { sel } => {
                nav(sel, menu::ITEMS.len(), k);
                let s = *sel;
                match k {
                    Keycode::Return | Keycode::KpEnter | Keycode::Space => self.menu_activate(s, ctx),
                    Keycode::Escape => Action::Close,
                    _ => Action::None,
                }
            }
            Overlay::Save { sel } | Overlay::Load { sel } => {
                nav(sel, 9, k);
                match k {
                    Keycode::Return | Keycode::KpEnter | Keycode::Space => {
                        let slot = *sel + 1;
                        if matches!(self.cur, Overlay::Save { .. }) {
                            Action::SaveTo(slot)
                        } else {
                            Action::LoadFrom(slot)
                        }
                    }
                    Keycode::Escape => Action::Close,
                    _ => Action::None,
                }
            }
            Overlay::Gallery(g) => match k {
                Keycode::Escape => {
                    if g.view.is_some() {
                        g.view = None;
                    } else {
                        return Action::Close;
                    }
                    Action::None
                }
                Keycode::Left => {
                    if g.view.is_some() {
                        g.view_next(-1);
                    } else {
                        g.sel = g.sel.saturating_sub(1);
                    }
                    Action::None
                }
                Keycode::Right => {
                    let n = g.all.len();
                    if g.view.is_some() {
                        g.view_next(1);
                    } else if n > 0 {
                        g.sel = (g.sel + 1).min(n - 1);
                    }
                    Action::None
                }
                Keycode::Return | Keycode::KpEnter => {
                    if g.view.is_none() && g.unlocked_at(g.sel) {
                        g.view = Some(g.sel);
                    }
                    Action::None
                }
                _ => Action::None,
            },
            Overlay::Volume { sel, bgm, se, tw_ms } => {
                nav(sel, volume::N, k);
                let delta = match k {
                    Keycode::Left => -5,
                    Keycode::Right => 5,
                    Keycode::Escape => {
                        return Action::ApplyVolumes {
                            bgm: *bgm,
                            se: *se,
                            tw_ms: *tw_ms,
                        }
                    }
                    _ => 0,
                };
                if delta != 0 {
                    match *sel {
                        0 => *bgm = volume::adjust(0, *bgm, delta),
                        1 => *se = volume::adjust(1, *se, delta),
                        _ => {
                            let sp = volume::speed_from_tw(*tw_ms);
                            *tw_ms = volume::tw_from_speed(volume::adjust(2, sp, delta));
                        }
                    }
                }
                Action::None
            }
            Overlay::Ritual { step, phase, slot, .. } => {
                let fired = matches!(k, Keycode::Return | Keycode::KpEnter);
                let s = *slot;
                if *phase != 0 {
                    return Action::None;
                }
                let _ = step;
                match k {
                    Keycode::Left | Keycode::Right => Action::None, // 按钮鼠标点选
                    Keycode::Return | Keycode::KpEnter if fired => self.ritual_confirm(s, true),
                    Keycode::Escape => Action::Close,
                    _ => Action::None,
                }
            }
            Overlay::Rest { cancelled, .. } => match k {
                Keycode::Escape | Keycode::Return | Keycode::KpEnter if !*cancelled => {
                    Action::CancelShutdown
                }
                _ => Action::None,
            },
            Overlay::None => Action::None,
        }
    }

    pub fn click(&mut self, x: f32, y: f32, _ctx: &Ctx) -> Action {
        match &mut self.cur {
            Overlay::Title { sel } => {
                if let Some(i) = title::hit_menu(x, y) {
                    *sel = i;
                    self.title_activate(i, _ctx)
                } else {
                    Action::None
                }
            }
            Overlay::Menu { sel } => {
                if let Some(i) = menu::hit_menu(x, y) {
                    *sel = i;
                    self.menu_activate(i, _ctx)
                } else {
                    Action::None
                }
            }
            Overlay::Save { sel } | Overlay::Load { sel } => {
                if let Some(i) = savemenu::hit_test(x, y) {
                    *sel = i;
                    let slot = i + 1;
                    if matches!(self.cur, Overlay::Save { .. }) {
                        Action::SaveTo(slot)
                    } else {
                        Action::LoadFrom(slot)
                    }
                } else {
                    Action::None
                }
            }
            Overlay::Gallery(g) => {
                if g.view.is_some() {
                    g.view = None;
                    return Action::None;
                }
                if let Some(i) = Gallery::hit_grid(x, y) {
                    g.sel = i;
                    if g.unlocked_at(i) {
                        g.view = Some(i);
                    }
                }
                Action::None
            }
            Overlay::Volume { .. } => Action::None,
            Overlay::Ritual { slot, phase, .. } => {
                let s = *slot;
                if *phase != 0 {
                    return Action::None;
                }
                match ritual::hit_btns(x, y) {
                    Some(true) => self.ritual_confirm(s, true),
                    Some(false) => Action::Close,
                    None => Action::None,
                }
            }
            Overlay::Rest { cancelled, .. } => {
                if !*cancelled && restui::hit_cancel(x, y) {
                    Action::CancelShutdown
                } else {
                    Action::None
                }
            }
            Overlay::None => Action::None,
        }
    }

    fn title_activate(&mut self, i: usize, ctx: &Ctx) -> Action {
        match i {
            0 => Action::StartNew,
            1 => Action::ContinueRun,
            2 => {
                self.open_gallery(ctx);
                Action::None
            }
            3 => {
                self.open_volume(ctx);
                Action::None
            }
            _ => Action::Quit,
        }
    }

    fn menu_activate(&mut self, i: usize, ctx: &Ctx) -> Action {
        match i {
            0 => Action::Close,
            1 => {
                self.cur = Overlay::Save { sel: 0 };
                Action::None
            }
            2 => {
                self.cur = Overlay::Load { sel: 0 };
                Action::None
            }
            3 => {
                self.cur = Overlay::Gallery(Box::new(Gallery {
                    sel: 0,
                    view: None,
                    all: ctx.all_cgs.to_vec(),
                    unlocked: ctx.unlocked_cgs.to_vec(),
                }));
                Action::None
            }
            4 => {
                self.cur = Overlay::Volume {
                    sel: 0,
                    bgm: ctx.volumes.0,
                    se: ctx.volumes.1,
                    tw_ms: ctx.tw_ms,
                };
                Action::None
            }
            5 => Action::Quit,
            _ => Action::Quit,
        }
    }

    fn ritual_confirm(&mut self, slot: usize, _yes: bool) -> Action {
        if let Overlay::Ritual { step, phase, .. } = &mut self.cur {
            if *step < 2 {
                *step += 1;
                Action::None
            } else {
                *phase = 1; // 光页开始
                Action::DeleteSlot(slot)
            }
        } else {
            Action::None
        }
    }

    pub fn draw(
        &self,
        canvas: &mut Canvas<Window>,
        fonts: &mut FontBook,
        bank: &mut TextureBank,
        ctx: &Ctx,
    ) -> Result<(), String> {
        match &self.cur {
            Overlay::None => {}
            Overlay::Title { sel } => {
                title::draw(canvas, fonts, &ctx.title_main, &ctx.title_sub, *sel, ctx.title_stage)?
            }
            Overlay::Menu { sel } => menu::draw(canvas, fonts, *sel)?,
            Overlay::Save { sel } => {
                savemenu::draw(canvas, fonts, bank, &ctx.slots, &ctx.save_dir, &ctx.meta, true, *sel)?
            }
            Overlay::Load { sel } => {
                savemenu::draw(canvas, fonts, bank, &ctx.slots, &ctx.save_dir, &ctx.meta, false, *sel)?
            }
            Overlay::Gallery(g) => g.draw(canvas, fonts, bank)?,
            Overlay::Volume { sel, bgm, se, tw_ms } => {
                volume::draw(canvas, fonts, *sel, *bgm, *se, *tw_ms)?
            }
            Overlay::Ritual { step, phase, flash, .. } => {
                if *phase == 0 {
                    let text = ctx
                        .ritual_texts
                        .get(*step as usize)
                        .cloned()
                        .unwrap_or_else(|| "要删掉最后一格存档吗？".into());
                    ritual::draw_confirm(canvas, fonts, &text, *step, &ctx.you, true)?;
                } else {
                    ritual::draw_flash(canvas, *flash)?;
                }
            }
            Overlay::Rest { left_ms, total, cancelled, finished, .. } => {
                restui::draw(canvas, fonts, *left_ms, *total, *cancelled, *finished)?;
            }
        }
        let _ = widgets::panel; // 保留引用
        Ok(())
    }
}
