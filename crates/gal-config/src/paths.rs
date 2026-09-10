//! config/paths.rs —— 素材路径解析策略（bg/cg/char → 数据目录布局）

use super::data_dir;

/// 指令 storage 名 → 实际文件路径（jpg/png 双探测，兼容 bgimage/fgimage 与 bg/char/cg 目录）
pub fn resolve(kind: &str, storage: &str) -> String {
    let data = data_dir();
    match kind {
        "bg" | "cg" => first_exists(
            &[
                format!("{data}/bgimage/{storage}.jpg"),
                format!("{data}/bgimage/{storage}.png"),
                format!("{data}/bg/{storage}.jpg"),
                format!("{data}/bg/{storage}.png"),
                format!("{data}/cg/{storage}.jpg"),
                format!("{data}/cg/{storage}.png"),
            ],
            &format!("{data}/bgimage/{storage}.jpg"),
        ),
        "char" => first_exists(
            &[
                format!("{data}/fgimage/{storage}.png"),
                format!("{data}/fgimage/{storage}.jpg"),
                format!("{data}/char/{storage}.png"),
                format!("{data}/char/{storage}.jpg"),
            ],
            &format!("{data}/fgimage/{storage}.png"),
        ),
        _ => format!("{data}/{storage}"),
    }
}

fn first_exists(cands: &[String], fallback: &str) -> String {
    cands
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

