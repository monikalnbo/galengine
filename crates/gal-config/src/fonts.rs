//! config/fonts.rs —— 字体候选链

/// 默认字体链：随包 → Linux Noto → Windows 雅黑
pub fn default_fonts() -> Vec<String> {
    vec![
        "game/data/system/font.ttf".into(),
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".into(),
        "C:\\Windows\\Fonts\\msyh.ttc".into(),
    ]
}
