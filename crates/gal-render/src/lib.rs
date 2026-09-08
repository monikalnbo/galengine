//! gal-render —— 渲染层：离屏合成/letterbox/纹理库/预解码/演出层绘制/背景调色与模糊

pub mod assets;
pub mod blur;
pub mod grade;
pub mod prefetch;
pub mod renderer;
pub mod stage_draw;

pub use assets::TextureBank;
pub use prefetch::Prefetcher;
pub use renderer::Renderer;
pub use stage_draw::draw_stage;
