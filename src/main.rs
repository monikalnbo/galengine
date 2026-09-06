//! 入口：组装并运行引擎。保持薄，具体逻辑在各模块。

mod app;
mod config;
mod gfx;
mod script;
mod text;
mod ui;

fn main() {
    if let Err(e) = app::run() {
        // 错误必须中文（docs/20 §3.8）；后续换 SDL 中文消息框
        eprintln!("引擎错误：{e}");
        std::process::exit(1);
    }
}
