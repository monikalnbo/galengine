//! config/mod.rs —— YAML 配置系统入口
//! game.yaml → 反序列化 → 全模块共享的 Config 结构体
//! 缺失字段自动回退内置默认值（零配置也能跑）

pub mod audio;
pub mod display;
pub mod ext;
pub mod fonts;
pub mod game;
pub mod paths;
pub mod save;
pub mod sprites;
pub mod style;
pub mod ui;

pub use audio::*;
pub use display::*;
pub use ext::*;
pub use game::*;
pub use paths::*;
pub use save::*;
pub use sprites::*;
pub use style::*;
pub use ui::*;

use serde::Deserialize;
use std::path::Path;

use fonts::default_fonts;

// ============================================================
// 顶层 Config（组合所有子配置）
// ============================================================

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Config {
    pub meta: MetaCfg,
    pub display: DisplayCfg,
    pub ui: UiCfg,
    pub sprites: SpritesCfg,
    pub audio: AudioCfg,
    pub fonts: Vec<String>,
    pub save: SaveCfg,
    pub auto: AutoCfg,
    pub game: GameCfg,
    /// 拓展声明（见 ext/ 实现）
    pub extensions: Vec<ExtCfg>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct MetaCfg {
    pub title: String,
    pub version: String,
    pub script_start: String,
    /// 打赏链接（空=设置菜单不显示打赏按钮）
    pub donation_url: String,
    pub donation_text: String,
}
impl Default for MetaCfg {
    fn default() -> Self {
        Self {
            title: "GALGAME".into(),
            version: "0.0.0".into(),
            script_start: "start.ks".into(),
            donation_url: String::new(),
            donation_text: String::new(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AutoCfg {
    pub delay_ms: f32,
}
impl Default for AutoCfg {
    fn default() -> Self {
        Self { delay_ms: 1000.0 }
    }
}

// ============================================================
// 加载：game.yaml → 兼容 config.json → 内置默认
// ============================================================

/// 逻辑分辨率常量（渲染管线用，display 可覆盖窗口尺寸但离屏纹理固定）
pub const LOGICAL_W: u32 = 1280;
pub const LOGICAL_H: u32 = 720;

/// 数据根目录
pub fn data_dir() -> String {
    std::env::var("ES_DATA_DIR").unwrap_or_else(|_| "game/data".to_string())
}

/// 加载配置
pub fn load() -> Config {
    let dir = data_dir();

    // 优先 YAML
    let yaml = format!("{dir}/game.yaml");
    if Path::new(&yaml).exists() {
        match std::fs::read_to_string(&yaml) {
            Ok(text) => {
                let text = text.trim_start_matches('\u{feff}');
                match serde_yaml::from_str::<Config>(text) {
                    Ok(mut c) => {
                        if c.fonts.is_empty() {
                            c.fonts = default_fonts(); // 字体候选链回退
                        }
                        eprintln!("[config] game.yaml 加载成功");
                        return c;
                    }
                    Err(e) => eprintln!("[config] game.yaml 解析失败：{e}，回退默认"),
                }
            }
            Err(e) => eprintln!("[config] game.yaml 读取失败：{e}"),
        }
    }

    eprintln!("[config] 无配置文件，使用内置默认值");
    Config::default()
}
