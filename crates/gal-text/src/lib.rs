//! gal-text —— 文字层：ttf 字体库 / 断行 / 打字机（SDL_ttf）

pub mod font;
pub mod layout;
pub mod writer;

pub use font::FontBook;
pub use writer::Typewriter;

/// 字体族路径解析：绝对路径原样，相对路径按数据目录展开
pub fn resolve_path(p: &str) -> String {
    let path = std::path::Path::new(p);
    if path.is_absolute() {
        p.to_string()
    } else {
        format!("{}/{}", gal_config::data_dir(), p)
    }
}

/// 取文件名（去目录与扩展名），字体族没写名字时给设置面板兜底显示
pub fn file_stem(p: &str) -> String {
    std::path::Path::new(p)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string())
}
