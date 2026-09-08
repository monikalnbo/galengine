//! galengine 可执行入口：引擎 + 游戏数据目录（game.yaml/scenario/素材）= 完整游戏。
//! 游戏内容零编译——换数据目录即换游戏。

fn main() {
    if let Err(e) = gal_engine::run() {
        eprintln!("引擎错误：{e}");
        std::process::exit(1);
    }
}
