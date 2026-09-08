//! 设置菜单（数据层）：行定义 + 上下文。绘制与命中在 draw.rs，交互接线在 input/settings_io.rs。

use gal_config::Config;

pub mod draw;
pub use draw::{draw, hit_test, slider_hit};

/// 设置行控件类型
#[derive(Clone)]
pub enum SettingKind {
    Slider {
        min: i64,
        max: i64,
        get: fn(&SettingsCtx) -> i64,
        set: fn(&mut SettingsCtx, i64),
        fmt: fn(i64) -> String,
    },
    Toggle {
        get: fn(&SettingsCtx) -> bool,
        set: fn(&mut SettingsCtx, bool),
    },
    Action {
        label: String,
        run: fn(&mut SettingsCtx, &Config),
    },
}

pub struct SettingRow {
    pub label: String,
    pub kind: SettingKind,
}

/// 设置上下文（所有值经此读写，apply 统一写回引擎）
pub struct SettingsCtx {
    pub text_speed: i64,
    pub auto_delay: i64,
    pub bgm_vol: i64,
    pub se_vol: i64,
    pub fullscreen: bool,
    pub skip_read: bool,
}

impl SettingsCtx {
    /// sf.* 为准，音量/全屏取引擎/窗口实况
    pub fn from_vars(vars: &gal_script::vars::Vars, bgm: i32, se: i32, fullscreen: bool) -> Self {
        let get = |k: &str, d: i64| vars.sf.get(k).map(|v| v.as_i64()).unwrap_or(d);
        Self {
            text_speed: get("textSpeed", 30),
            auto_delay: get("autoDelay", 1000),
            bgm_vol: bgm as i64,
            se_vol: se as i64,
            fullscreen,
            skip_read: get("skipRead", 0) == 1,
        }
    }
}

/// 设置行列表（顺序即界面顺序；打赏行仅在配置了 URL 时追加）
pub fn rows(conf: &Config) -> Vec<SettingRow> {
    let mut r = vec![
        SettingRow {
            label: "文字速度".into(),
            kind: SettingKind::Slider {
                min: 5,
                max: 200,
                get: |c| c.text_speed,
                set: |c, v| c.text_speed = v,
                fmt: |v| if v <= 10 { "瞬间的".into() } else { format!("{}ms", v) },
            },
        },
        SettingRow {
            label: "自动速度".into(),
            kind: SettingKind::Slider {
                min: 500,
                max: 3000,
                get: |c| c.auto_delay,
                set: |c, v| c.auto_delay = v,
                fmt: |v| format!("{}ms", v),
            },
        },
        SettingRow {
            label: "BGM 音量".into(),
            kind: SettingKind::Slider {
                min: 0,
                max: 100,
                get: |c| c.bgm_vol,
                set: |c, v| c.bgm_vol = v,
                fmt: |v| format!("{}", v),
            },
        },
        SettingRow {
            label: "SE 音量".into(),
            kind: SettingKind::Slider {
                min: 0,
                max: 100,
                get: |c| c.se_vol,
                set: |c, v| c.se_vol = v,
                fmt: |v| format!("{}", v),
            },
        },
        SettingRow {
            label: "全屏模式".into(),
            kind: SettingKind::Toggle { get: |c| c.fullscreen, set: |c, v| c.fullscreen = v },
        },
        SettingRow {
            label: "跳过已读".into(),
            kind: SettingKind::Toggle { get: |c| c.skip_read, set: |c, v| c.skip_read = v },
        },
    ];
    if !conf.meta.donation_url.is_empty() {
        r.push(SettingRow {
            label: conf.meta.donation_text.clone(),
            kind: SettingKind::Action {
                label: "❤ 打赏支持".into(),
                run: |_c, conf| {
                    let _ = gal_platform::desktop_open_url(&conf.meta.donation_url);
                },
            },
        });
    }
    r
}
