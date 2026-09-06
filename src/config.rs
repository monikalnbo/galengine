//! 顶层总配置：全引擎唯一配置源（data/config.json）。
//! 分层：Config（顶层）→ app（组装）→ 各模块按需取参；
//! 场景级资源按需加载/回收见 gfx::assets（stage 为场景真相源）。
//! 缺失字段自动回退内置默认值（Default 与 data/config.json 保持一致）。

use serde::Deserialize;

// 架构常量（渲染管线假设，不进运行时配置）
pub const LOGICAL_W: u32 = 1280;
pub const LOGICAL_H: u32 = 720;
pub const FRAME_BUDGET_MS: u64 = 16;
pub const DATA_DIR: &str = "game/data";

/// 数据根目录（运行时解析）：ES_DATA_DIR 环境变量覆盖，默认 game/data（cwd 相对）。
/// 引擎独立仓自带 testdata，游戏仓指向 game/data。
pub fn data_dir() -> String {
    std::env::var("ES_DATA_DIR").unwrap_or_else(|_| DATA_DIR.to_string())
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub title: String,
    /// 默认启动剧本
    pub script_start: String,
    pub save_dir: String,
    pub save_slots: usize,
    /// 字体候选（随包 → Linux Noto → Windows 雅黑 → 子集兜底）
    pub fonts: Vec<String>,
    pub dialog: DialogCfg,
    /// 打字机每字间隔 ms
    pub typewriter_ms: f32,
    /// 自动模式行完延迟 ms
    pub auto_delay_ms: f32,
    pub audio: AudioCfg,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct DialogCfg {
    /// [x, y, w, h]
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

impl Default for Config {
    fn default() -> Self {
        Self {
            title: "第八个夏天 EIGHTH SUMMER".into(),
            script_start: "a3_smoke.ks".into(),
            save_dir: "savedata".into(),
            save_slots: 9,
            fonts: vec![
                "game/data/system/font.ttf".into(),
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".into(),
                "C:\\Windows\\Fonts\\msyh.ttc".into(),
                "game/data/system_polyfill/font.ttf".into(),
            ],
            dialog: DialogCfg::default(),
            typewriter_ms: 30.0,
            auto_delay_ms: 1000.0,
            audio: AudioCfg::default(),
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

impl Default for AudioCfg {
    fn default() -> Self {
        Self { bgm_volume: 80, se_volume: 90 }
    }
}

impl Config {
    /// 读取 data/config.json；不存在/字段缺失回退默认
    pub fn load() -> Self {
        let path = format!("{}/config.json", data_dir());
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }
}
