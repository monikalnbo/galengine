//! gal-ext —— 拓展契约：Extension trait + 上下文（拓展作者只需依赖本 crate）。
//! 实现见 gal_ext::Extension；内置拓展在 gal-engine::exts。

use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::video::Window;

use gal_script::interp::Interp;
use gal_text::font::FontBook;
/// 拓展可操作的引擎上下文（配置经 ExtCfg 在构造时注入）
pub struct ExtContext<'a> {
    pub interp: &'a mut Interp,
}

#[derive(Debug, PartialEq)]
pub enum ExtResult {
    /// 继续正常处理（不拦截事件）
    Continue,
    /// 拓展已消化此事件（引擎跳过默认处理）
    Handled,
}

pub trait Extension {
    fn name(&self) -> &str;
    /// 每帧驱动（dt 毫秒）
    fn tick(&mut self, dt_ms: f32, ctx: &mut ExtContext) -> ExtResult;
    fn on_click(&mut self, x: f32, y: f32, ctx: &mut ExtContext) -> ExtResult;
    fn on_key(&mut self, key: Keycode, ctx: &mut ExtContext) -> ExtResult;
    /// 叠加绘制（选项/对话之上）
    fn draw(&self, canvas: &mut Canvas<Window>, fonts: &mut FontBook) -> Result<(), String>;
}

/// 便捷构造上下文（Game 字段拆借）
impl<'a> ExtContext<'a> {
    pub fn new(interp: &'a mut Interp) -> Self {
        Self { interp }
    }
}
