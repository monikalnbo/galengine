//! gal-engine —— 组装门面：主循环/输入路由/系统事件/内置拓展。
//! 游戏只依赖本 crate（或直接用 galengine 可执行文件 + 游戏数据目录）。
//! 全家族经此再导出：gal_engine::gal_script::RunState …

pub mod app;
pub mod exts;
pub mod frame;
pub mod input;
pub mod systems;

pub use gal_audio;
pub use gal_config;
pub use gal_ext;
pub use gal_platform;
pub use gal_render;
pub use gal_save;
pub use gal_script;
pub use gal_text;
pub use gal_ui;

use gal_ext::Extension;

/// 引擎启动参数：游戏侧注入额外拓展（内置拓展按 game.yaml 自动挂载）
#[derive(Default)]
pub struct Boot {
    pub extensions: Vec<Box<dyn Extension>>,
}

impl Boot {
    /// 注册程序化拓展（yaml 声明的内置拓展之外）
    pub fn with(mut self, ext: Box<dyn Extension>) -> Self {
        self.extensions.push(ext);
        self.extensions.shrink_to_fit();
        self
    }
}

/// 启动引擎（默认 Boot）
pub fn run() -> Result<(), String> {
    app::run_inner(Boot::default())
}

/// 启动引擎（自定义 Boot）
pub fn run_with(boot: Boot) -> Result<(), String> {
    app::run_inner(boot)
}
