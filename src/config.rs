//! 顶层唯一配置（data/config.json）。缺失字段回退内置默认；读取剥 BOM。

use serde::Deserialize;
use std::collections::HashMap;

pub const LOGICAL_W: u32 = 1280;
pub const LOGICAL_H: u32 = 720;
pub const DATA_DIR: &str = "game/data";

/// 数据根目录：ES_DATA_DIR 覆盖（引擎仓指向 testdata，游戏仓指向 game/data）
pub fn data_dir() -> String {
    std::env::var("ES_DATA_DIR").unwrap_or_else(|_| DATA_DIR.to_string())
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub title: String,
    pub script_start: String,
    pub save_dir: String,
    pub save_slots: usize,
    /// 字体候选链：随包 → Linux Noto → Windows 雅黑 → 子集
    pub fonts: Vec<String>,
    pub dialog: DialogCfg,
    pub typewriter_ms: f32,
    pub auto_delay_ms: f32,
    pub audio: AudioCfg,
    pub game: GameCfg,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct DialogCfg {
    pub box_rect: [i32; 4],
    pub text_x: i32,
    pub text_y: i32,
    pub font_size: u16,
    pub name_font_size: u16,
    pub lines_per_page: usize,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AudioCfg {
    pub bgm_volume: i32,
    pub se_volume: i32,
}

impl Default for AudioCfg {
    fn default() -> Self {
        Self { bgm_volume: 80, se_volume: 90 }
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct GameCfg {
    pub hero_default: String,
    pub you_default: String,
    pub input_presets: HashMap<String, Vec<String>>,
    pub name_colors: HashMap<String, [u8; 3]>,
    pub shutdown_message: String,
    /// meta_delete_last 三重确认文案（3 条，引擎给通用默认，游戏可覆盖）
    pub ritual_texts: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title: "GALGAME".into(),
            script_start: "start.ks".into(),
            save_dir: "savedata".into(),
            save_slots: 9,
            fonts: vec![
                "game/data/system/font.ttf".into(),
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".into(),
                "C:\\Windows\\Fonts\\msyh.ttc".into(),
            ],
            dialog: DialogCfg::default(),
            typewriter_ms: 30.0,
            auto_delay_ms: 1000.0,
            audio: AudioCfg { bgm_volume: 80, se_volume: 90 },
            game: GameCfg {
                hero_default: "他".into(),
                you_default: "你".into(),
                input_presets: HashMap::new(),
                name_colors: HashMap::new(),
                shutdown_message: "晚安。".into(),
                ritual_texts: vec![
                    "真的要删除吗？".into(),
                    "再想一下，删除就回不来了。".into(),
                    "最后确认。亲手，删掉它。".into(),
                ],
            },
        }
    }
}

impl Default for DialogCfg {
    fn default() -> Self {
        Self {
            box_rect: [56, LOGICAL_H as i32 - 206, LOGICAL_W as i32 - 112, 172],
            text_x: 104,
            text_y: LOGICAL_H as i32 - 178,
            font_size: 30,
            name_font_size: 30,
            lines_per_page: 3,
        }
    }
}

/// 读取配置；文件不存在/解析失败回退默认（不 panic，验收环境零配置也能跑）
pub fn load() -> Config {
    let path = format!("{}/config.json", data_dir());
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let text = text.trim_start_matches('\u{feff}'); // 剥 BOM
            serde_json::from_str(text).unwrap_or_default()
        }
        Err(_) => Config::default(),
    }
}
