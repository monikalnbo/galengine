//! 设置菜单（数据层）：行定义 + 上下文。绘制与命中在 draw.rs，交互接线在 input/settings_io.rs。
//!
//! 所有值都走 [`SettingsCtx`]：默认值取自 game.yaml（`authored`），
//! 玩家改动写进 `sf.*`（savedata/global.json）。因此「恢复默认」= 用 `authored` 重置。
//! 行按组排序、分页显示（每页 [`PER_PAGE`] 行）。

use gal_config::keymap::ACTIONS;
use gal_config::{Config, FONT_SIZE_MAX, FONT_SIZE_MIN, FX_MAX, LINE_SPACING_MAX};

pub mod draw;
pub use draw::{draw, hit_test, page_of, slider_hit, PER_PAGE};

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
    /// 枚举（字体族/皮肤）：labels 由 [`UiEnv`] 提供，值 = 下标
    Enum {
        labels: Vec<String>,
        get: fn(&SettingsCtx) -> i64,
        set: fn(&mut SettingsCtx, i64),
    },
    /// 键位：进入「按键重绑」状态（引擎侧接管下一次按键）
    Rebind {
        action: String,
        keys: String,
    },
    Action {
        label: String,
        run: fn(&mut SettingsCtx, &Config),
    },
}

pub struct SettingRow {
    /// 组名（每页首行画分组标题）
    pub group: &'static str,
    pub label: String,
    pub kind: SettingKind,
}

/// 运行期可选项（字体族实际可用名单 / 皮肤名单 / 语言包 / 朗读后端）
#[derive(Clone, Default)]
pub struct UiEnv {
    pub families: Vec<String>,
    pub skins: Vec<String>,
    /// 语言包显示名（空 = 无多语言，不显示该行）
    pub languages: Vec<String>,
    /// 朗读后端是否可用（false = 行灰显 + 提示「本机没有可用的朗读后端」）
    pub tts_available: bool,
}

/// 设置上下文（所有值经此读写；apply 统一写回引擎与 sf.*）
#[derive(Clone)]
pub struct SettingsCtx {
    pub text_speed: i64,
    pub auto_delay: i64,
    /// 正文字号（0 = 跟随作者配置）
    pub text_size: i64,
    /// 行距（0 = 跟随作者配置）
    pub line_spacing: i64,
    pub wait_marker: bool,
    /// 正文字描边宽度（0 = 无描边）
    pub outline: i64,
    pub bgm_vol: i64,
    pub se_vol: i64,
    pub voice_vol: i64,
    pub fullscreen: bool,
    pub skip_read: bool,
    pub trans_on: bool,
    /// 皮肤下标（ui.skin 列表）
    pub skin: i64,
    /// 字体族下标（FontBook 名单）
    pub family: i64,
    /// 多语言下标（0 = 作者原文；下标 = lang 包顺序 + 1）
    pub language: i64,
    /// 无障碍朗读开关
    pub tts: bool,
    /// 朗读语速（相对 -10..10）
    pub tts_rate: i64,
    /// 「载入系统字体」动作已请求（引擎下一帧执行后清掉）
    pub scan_sys_fonts: bool,
    /// 键位：动作 → 设置面板展示串
    pub keys: Vec<(String, String)>,
}

impl SettingsCtx {
    /// 作者默认值（「恢复默认」与首次启动的基准）
    pub fn authored(conf: &Config) -> Self {
        let size = conf.text.base_size(conf.ui.dialog.font_size) as i64;
        Self {
            text_speed: conf.ui.dialog.typewriter_ms.max(0.0) as i64,
            auto_delay: conf.auto.delay_ms as i64,
            text_size: size,
            line_spacing: conf.text.line_spacing as i64,
            wait_marker: conf.text.wait_marker,
            outline: conf.text.outline as i64,
            bgm_vol: conf.audio.bgm_volume as i64,
            se_vol: conf.audio.se_volume as i64,
            voice_vol: conf.voice.volume as i64,
            fullscreen: false,
            skip_read: conf.game.skip_read_only,
            trans_on: true,
            skin: 0,
            family: 0,
            language: 0,
            tts: false,
            tts_rate: 0,
            scan_sys_fonts: false,
            keys: Vec::new(),
        }
    }

    /// 作者默认 + 玩家 sf.* 覆盖（键位串由 effective 提供）
    pub fn from_vars(
        vars: &gal_script::vars::Vars,
        conf: &Config,
        keymap: &std::collections::HashMap<String, Vec<String>>,
        fullscreen: bool,
    ) -> Self {
        let mut c = Self::authored(conf);
        let get = |k: &str| vars.sf.get(k).map(|v| v.as_i64());
        if let Some(v) = get("textSpeed") {
            c.text_speed = v;
        }
        if let Some(v) = get("autoDelay") {
            c.auto_delay = v;
        }
        if let Some(v) = get("textSize") {
            c.text_size = v;
        }
        if let Some(v) = get("lineSpacing") {
            c.line_spacing = v;
        }
        if let Some(v) = get("volBgm") {
            c.bgm_vol = v;
        }
        if let Some(v) = get("volSe") {
            c.se_vol = v;
        }
        if let Some(v) = get("volVoice") {
            c.voice_vol = v;
        }
        if let Some(v) = get("skinIndex") {
            c.skin = v;
        }
        if let Some(v) = get("fontFamily") {
            c.family = v;
        }
        if let Some(v) = get("language") {
            c.language = v;
        }
        if let Some(v) = get("ttsOn") {
            c.tts = v == 1;
        }
        if let Some(v) = get("ttsRate") {
            c.tts_rate = v;
        }
        if let Some(v) = get("skipRead") {
            c.skip_read = v == 1;
        }
        if let Some(v) = get("transOn") {
            c.trans_on = v == 1;
        }
        if let Some(v) = get("waitMarker") {
            c.wait_marker = v == 1;
        }
        if let Some(v) = get("outline") {
            c.outline = v;
        }
        c.fullscreen = fullscreen;
        c.keys = ACTIONS
            .iter()
            .map(|(action, _)| {
                let key = vars
                    .sf
                    .get(&format!("keymap.{action}"))
                    .map(|v| v.as_str())
                    .unwrap_or_default();
                let shown = if key.is_empty() {
                    keymap.get(*action).map(|ks| ks.join(" / ")).unwrap_or_default()
                } else {
                    key
                };
                (action.to_string(), shown)
            })
            .collect();
        c
    }

    /// 当前键位串（动作 → 展示串）
    pub fn key_of(&self, action: &str) -> String {
        self.keys.iter().find(|(a, _)| a == action).map(|(_, k)| k.clone()).unwrap_or_default()
    }
}

fn secs(v: i64) -> String {
    format!("{:.1}s", v as f32 / 1000.0)
}

fn ms_fmt(v: i64) -> String {
    if v <= 10 {
        "瞬间".into()
    } else {
        format!("{v}ms")
    }
}

/// 设置行列表（顺序即界面顺序）
pub fn rows(conf: &Config, env: &UiEnv) -> Vec<SettingRow> {
    let mut r = vec![
        row(
            "文字",
            "文字速度",
            SettingKind::Slider {
                min: 5,
                max: 200,
                get: |c| c.text_speed,
                set: |c, v| c.text_speed = v,
                fmt: ms_fmt,
            },
        ),
        row(
            "文字",
            "自动播放间隔",
            SettingKind::Slider {
                min: 500,
                max: 3000,
                get: |c| c.auto_delay,
                set: |c, v| c.auto_delay = v,
                fmt: secs,
            },
        ),
        row(
            "文字",
            "字号",
            SettingKind::Slider {
                min: FONT_SIZE_MIN as i64,
                max: FONT_SIZE_MAX as i64,
                get: |c| c.text_size,
                set: |c, v| c.text_size = v,
                fmt: |v| format!("{v}px"),
            },
        ),
        row(
            "文字",
            "行距",
            SettingKind::Slider {
                min: 0,
                max: LINE_SPACING_MAX as i64,
                get: |c| c.line_spacing,
                set: |c, v| c.line_spacing = v,
                fmt: |v| format!("+{v}px"),
            },
        ),
        row(
            "文字",
            "等待标记 ▼",
            SettingKind::Toggle { get: |c| c.wait_marker, set: |c, v| c.wait_marker = v },
        ),
        row(
            "文字",
            "文字描边",
            SettingKind::Slider {
                min: 0,
                max: FX_MAX as i64,
                get: |c| c.outline,
                set: |c, v| c.outline = v,
                fmt: |v| if v == 0 { "无".into() } else { format!("{v}px") },
            },
        ),
        row(
            "文字",
            "字体",
            SettingKind::Enum {
                labels: if env.families.is_empty() {
                    vec!["默认".into()]
                } else {
                    env.families.clone()
                },
                get: |c| c.family,
                set: |c, v| c.family = v,
            },
        ),
        row(
            "文字",
            "语言",
            SettingKind::Enum {
                labels: if env.languages.is_empty() {
                    vec!["作者原文".into()]
                } else {
                    std::iter::once("作者原文".to_string()).chain(env.languages.clone()).collect()
                },
                get: |c| c.language,
                set: |c, v| c.language = v,
            },
        ),
        row(
            "文字",
            "系统字体",
            SettingKind::Action {
                label: "载入…".into(),
                // 真加载在主循环（需要 FontBook）：置标记，下一帧扫描系统字体目录
                run: |c, _conf| c.scan_sys_fonts = true,
            },
        ),
        row(
            "文字",
            "跳过已读",
            SettingKind::Toggle { get: |c| c.skip_read, set: |c, v| c.skip_read = v },
        ),
        row(
            "显示",
            "全屏",
            SettingKind::Toggle { get: |c| c.fullscreen, set: |c, v| c.fullscreen = v },
        ),
        row(
            "显示",
            "转场动画",
            SettingKind::Toggle { get: |c| c.trans_on, set: |c, v| c.trans_on = v },
        ),
        row(
            "显示",
            "界面皮肤",
            SettingKind::Enum {
                labels: if env.skins.is_empty() {
                    vec!["默认".into()]
                } else {
                    env.skins.clone()
                },
                get: |c| c.skin,
                set: |c, v| c.skin = v,
            },
        ),
        row(
            "音频",
            "BGM 音量",
            SettingKind::Slider {
                min: 0,
                max: 100,
                get: |c| c.bgm_vol,
                set: |c, v| c.bgm_vol = v,
                fmt: |v| format!("{v}"),
            },
        ),
        row(
            "音频",
            "音效音量",
            SettingKind::Slider {
                min: 0,
                max: 100,
                get: |c| c.se_vol,
                set: |c, v| c.se_vol = v,
                fmt: |v| format!("{v}"),
            },
        ),
        row(
            "音频",
            "语音音量",
            SettingKind::Slider {
                min: 0,
                max: 100,
                get: |c| c.voice_vol,
                set: |c, v| c.voice_vol = v,
                fmt: |v| format!("{v}"),
            },
        ),
        row(
            "无障碍",
            if env.tts_available {
                "朗读台词"
            } else {
                "朗读台词（本机无可用后端）"
            },
            SettingKind::Toggle { get: |c| c.tts, set: |c, v| c.tts = v },
        ),
        row(
            "无障碍",
            "朗读语速",
            SettingKind::Slider {
                min: -10,
                max: 10,
                get: |c| c.tts_rate,
                set: |c, v| c.tts_rate = v,
                fmt: |v| {
                    if v == 0 {
                        "标准".into()
                    } else {
                        format!("{v:+}")
                    }
                },
            },
        ),
    ];
    for (action, label) in ACTIONS {
        r.push(SettingRow {
            group: "键位",
            label: (*label).into(),
            kind: SettingKind::Rebind { action: (*action).into(), keys: String::new() },
        });
    }
    r.push(SettingRow {
        group: "其他",
        label: "恢复默认设置".into(),
        kind: SettingKind::Action {
            label: "恢复".into(),
            run: |c, conf| *c = SettingsCtx::authored(conf),
        },
    });
    if !conf.meta.donation_url.is_empty() {
        r.push(SettingRow {
            group: "其他",
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

fn row(group: &'static str, label: &str, kind: SettingKind) -> SettingRow {
    SettingRow { group, label: label.to_string(), kind }
}

/// 描边/阴影上限（文档与设置面板共用的常量再导出）
pub const FX_LIMIT: i64 = FX_MAX as i64;

#[cfg(test)]
mod tests {
    use super::*;

    fn conf() -> Config {
        Config { fonts: vec!["/nonexistent.ttf".into()], ..Config::default() }
    }

    #[test]
    fn 默认值取作者配置() {
        let c = SettingsCtx::authored(&conf());
        assert_eq!(c.text_speed, 30);
        assert_eq!(c.text_size, 30);
        assert_eq!(c.bgm_vol, 80);
        assert_eq!(c.se_vol, 90);
        assert!(c.wait_marker);
    }

    #[test]
    fn 玩家值覆盖作者默认且缺省项保留默认() {
        let mut vars = gal_script::vars::Vars::default();
        vars.sf.insert("textSize".into(), gal_script::vars::Value::Int(36));
        vars.sf.insert("volBgm".into(), gal_script::vars::Value::Int(20));
        let km = conf().keymap.effective();
        let c = SettingsCtx::from_vars(&vars, &conf(), &km, true);
        assert_eq!(c.text_size, 36);
        assert_eq!(c.bgm_vol, 20);
        assert_eq!(c.se_vol, 90); // 未设置 → 作者默认
        assert!(c.fullscreen); // 引擎实况
        assert_eq!(c.key_of("menu"), "Escape");
    }

    #[test]
    fn 行列表含全部组且键位与动作表一致() {
        let env =
            UiEnv { families: vec![], skins: vec![], languages: vec![], tts_available: false };
        let rows = rows(&conf(), &env);
        let keys = rows.iter().filter(|r| r.group == "键位").count();
        assert_eq!(keys, ACTIONS.len());
        let names: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
        for need in ["字号", "行距", "文字描边", "字体", "界面皮肤", "语音音量", "恢复默认设置"]
        {
            assert!(names.contains(&need), "缺设置项：{need}");
        }
        assert!(rows.len() >= 22, "只剩 {} 项", rows.len());
    }
}
