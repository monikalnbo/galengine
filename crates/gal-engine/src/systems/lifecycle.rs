//! 游戏与场景生命周期控制：开局、回标题、关机取消、状态判断

use super::Game;
use gal_script::interp::{Interp, RunState};
use gal_ui::overlay::Overlay;

impl Game {
    /// 标题「开始游戏」：从头执行剧本
    pub fn start_game(&mut self) -> Result<(), String> {
        let script = self.start_script.clone();
        let label = self.start_label.clone();
        self.interp.start(&script, label.as_deref())?;
        self.started = true;
        self.overlay = Overlay::None;
        self.auto = false;
        self.hide_ui = false;
        self.choice_sel = 0;
        Ok(())
    }

    /// 回标题：清周目变量（sf.* 保留），不落盘删除任何东西
    pub fn back_to_title(&mut self) {
        let sf = std::mem::take(&mut self.interp.vars.sf);
        let vars = gal_script::vars::Vars { f: Default::default(), sf };
        self.interp = Interp::new(
            vars,
            self.conf.ui.dialog.typewriter_ms,
            &self.conf.game.hero_default,
            &self.conf.game.you_default,
        );
        self.interp.positions = self.conf.sprites.positions.clone();
        self.interp.chibi = self.conf.sprites.chibi.iter().cloned().collect();
        self.started = false;
        self.overlay = Overlay::Title { sel: 0 };
        self.auto = false;
        self.hide_ui = false;
        self.choice_sel = 0;
        self.input_ui = None;
    }

    /// 关机取消（Rest 覆盖层按钮）
    pub fn cancel_rest(&mut self) {
        gal_platform::cancel_shutdown();
        self.sys.rest = None;
    }

    /// 自动模式推进延迟（sf.autoDelay 覆盖配置）
    pub fn auto_delay_ms(&self) -> f32 {
        match self.interp.vars.sf.get("autoDelay") {
            Some(gal_script::vars::Value::Int(v)) if *v > 0 => *v as f32,
            _ => self.conf.auto.delay_ms,
        }
    }

    /// 当前选项数（WaitChoice 时）
    pub fn choice_len(&self) -> usize {
        match &self.interp.state {
            RunState::WaitChoice { items, .. } => items.len(),
            _ => 0,
        }
    }

    /// 台词态是否可存档（等待点=对话行）
    pub fn can_save(&self) -> bool {
        self.interp.state == RunState::WaitClick && self.interp.resume_point().is_some()
    }
}
