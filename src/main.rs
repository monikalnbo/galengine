//! galengine：通用 galgame 引擎（Rust+SDL2）。
//! 分层：L1 app/input/render/systems → L2 script/ui → L3 gfx/text/audio/save → L4 platform/config。
//! 游戏内容只存在于数据侧（data/ + config.json），引擎零游戏特定字符串。

mod app;
mod audio;
mod config;
mod gfx;
mod input;
mod platform;
mod render;
mod save;
mod script;
mod systems;
mod text;
mod ui;

fn main() {
    if let Err(e) = app::run() {
        eprintln!("引擎错误：{e}");
        std::process::exit(1);
    }
}
