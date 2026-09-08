//! gal-text —— 文字层：ttf 字体库 / 断行 / 打字机（SDL_ttf）

pub mod font;
pub mod layout;
pub mod writer;

pub use font::FontBook;
pub use writer::Typewriter;
