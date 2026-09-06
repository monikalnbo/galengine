//! L1 游戏输入路由：choice/input/dialog 三态事件（从 app.rs 拆出）。
//! 持有游戏内交互状态；Overlay/系统键仍归 app.rs。

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::gfx::renderer::Renderer;
use crate::script::interp::{InputSpec, Interp, RunState};
use crate::ui::{choice, inputbox};

/// 游戏内交互状态（选择项/输入框/自动/快进）
pub struct InputRouter {
    pub choice_sel: usize,
    pub input_ui: Option<inputbox::InputUi>,
    pub auto: bool,
    pub auto_acc: f32,
    pub ctrl: bool,
    /// 预设名（hero/player，来自 game.input_presets）
    presets: (Vec<String>, Vec<String>),
}

impl InputRouter {
    pub fn new(presets: (Vec<String>, Vec<String>)) -> Self {
        Self {
            choice_sel: 0,
            input_ui: None,
            auto: false,
            auto_acc: 0.0,
            ctrl: false,
            presets,
        }
    }

    /// 输入框生命周期（进入 WaitInput 创建 / 离开销毁）
    pub fn maintain_input(&mut self, interp: &Interp) {
        if self.input_ui.is_none() {
            if let RunState::WaitInput(spec) = &interp.state {
                let presets = if spec.var.ends_with("playerName") {
                    self.presets.1.clone()
                } else {
                    self.presets.0.clone()
                };
                self.input_ui = Some(inputbox::InputUi::new(
                    &spec.var,
                    &spec.prompt,
                    spec.width,
                    &spec.default,
                    presets,
                ));
                unsafe { SDL_StartTextInput() };
            }
        } else if !matches!(interp.state, RunState::WaitInput(_)) {
            self.input_ui = None;
        }
        if let Some(ui) = self.input_ui.as_mut() {
            ui.blink += 1.0;
        }
    }

    /// 游戏态事件 → 是否消费（false=app 继续处理）
    /// 返回 Some(err) 表示需要上抛的错误
    pub fn on_event(
        &mut self,
        ev: &Event,
        interp: &mut Interp,
        canvas: &Canvas<Window>,
    ) -> Result<bool, String> {
        match ev {
            // —— 选项 ——
            Event::KeyDown {
                keycode: Some(k @ (Keycode::Up | Keycode::Down)),
                ..
            } if self.in_choice(interp) => {
                let n = self.choice_n(interp);
                if n > 0 {
                    self.choice_sel = if *k == Keycode::Up {
                        self.choice_sel.saturating_sub(1)
                    } else {
                        (self.choice_sel + 1).min(n - 1)
                    };
                }
                Ok(true)
            }
            Event::KeyDown {
                keycode: Some(Keycode::Return | Keycode::KpEnter | Keycode::Space),
                ..
            } if self.in_choice(interp) => {
                interp.choose(self.choice_sel)?;
                Ok(true)
            }
            Event::MouseMotion { x, y, .. } if self.in_choice(interp) => {
                if let Some(i) = self.choice_hit(interp, canvas, *x, *y) {
                    self.choice_sel = i;
                }
                Ok(true)
            }
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } if self.in_choice(interp) => {
                if let Some(i) = self.choice_hit(interp, canvas, *x, *y) {
                    interp.choose(i)?;
                }
                Ok(true)
            }
            // —— 名字输入 ——
            Event::TextInput { text, .. } if self.in_input(interp) => {
                if let Some(ui) = self.input_ui.as_mut() {
                    ui.push_char(text);
                }
                Ok(true)
            }
            Event::KeyDown {
                keycode: Some(Keycode::Backspace),
                ..
            } if self.in_input(interp) => {
                if let Some(ui) = self.input_ui.as_mut() {
                    ui.backspace();
                }
                Ok(true)
            }
            Event::KeyDown {
                keycode: Some(Keycode::Return | Keycode::KpEnter),
                ..
            } if self.in_input(interp) => {
                self.confirm(interp);
                Ok(true)
            }
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } if self.in_input(interp) => {
                let (lx, ly) = Renderer::to_logical(canvas, *x, *y);
                let hit = self
                    .input_ui
                    .as_ref()
                    .and_then(|ui| inputbox::hit_test(ui, lx, ly));
                match hit {
                    Some(inputbox::Hit::Preset(i)) => {
                        if let Some(ui) = self.input_ui.as_mut() {
                            if let Some(p) = ui.presets.get(i) {
                                ui.buf = p.clone();
                            }
                        }
                    }
                    Some(inputbox::Hit::Confirm) => self.confirm(interp),
                    None => {}
                }
                Ok(true)
            }
            // —— 台词 ——
            Event::KeyDown {
                keycode: Some(Keycode::A),
                ..
            } if interp.state == RunState::WaitClick => {
                self.auto = !self.auto;
                self.auto_acc = 0.0;
                Ok(true)
            }
            Event::KeyDown {
                keycode: Some(Keycode::Space | Keycode::Return | Keycode::KpEnter),
                ..
            } if interp.state == RunState::WaitClick => {
                interp.click()?;
                Ok(true)
            }
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                ..
            } if interp.state == RunState::WaitClick => {
                interp.click()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// 帧驱动：快进/自动（app 在无 Overlay 时调用）
    pub fn tick(&mut self, interp: &mut Interp, auto_delay_ms: f32, dt: f32) -> Result<(), String> {
        if self.ctrl && interp.state == RunState::WaitClick {
            interp.click()?;
        }
        if self.auto && interp.state == RunState::WaitClick {
            if interp.tw.line_finished() {
                self.auto_acc += dt;
                if self.auto_acc >= auto_delay_ms {
                    self.auto_acc = 0.0;
                    interp.click()?;
                }
            } else {
                self.auto_acc = 0.0;
            }
        }
        Ok(())
    }

    fn confirm(&mut self, interp: &mut Interp) {
        if let Some(ui) = self.input_ui.take() {
            let _ = interp.input_result(ui.value());
        }
    }

    fn in_choice(&self, interp: &Interp) -> bool {
        matches!(interp.state, RunState::WaitChoice { .. })
    }
    fn in_input(&self, interp: &Interp) -> bool {
        matches!(interp.state, RunState::WaitInput(_))
    }
    fn choice_n(&self, interp: &Interp) -> usize {
        match &interp.state {
            RunState::WaitChoice { items } => items.len(),
            _ => 0,
        }
    }
    fn choice_hit(
        &self,
        interp: &Interp,
        canvas: &Canvas<Window>,
        x: i32,
        y: i32,
    ) -> Option<usize> {
        let n = self.choice_n(interp);
        let (lx, ly) = Renderer::to_logical(canvas, x, y);
        let idxs: Vec<usize> = (0..n).collect();
        choice::hit_test(&idxs, lx, ly)
    }
}

extern "C" {
    fn SDL_StartTextInput();
}
