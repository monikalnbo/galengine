//! 字体库：候选链解析 + TTC 面探测（SC=索引2 勿硬编码，用 family 名探测）
//! + 文字纹理缓存（场景切换清空，字体对象保留）。

use std::collections::HashMap;

use sdl2::pixels::Color;
use sdl2::render::{BlendMode, Texture, TextureCreator};
use sdl2::ttf::{Font, Sdl2TtfContext};
use sdl2::video::WindowContext;

const MAX_FACES: u32 = 8;

pub struct FontBook<'a> {
    ttf: &'a Sdl2TtfContext,
    creator: &'a TextureCreator<WindowContext>,
    /// (路径, 面序)——一次解析，-font() 不再重解析
    source: (String, u32),
    fonts: HashMap<u16, Font<'a, 'static>>,
    textures: HashMap<(u16, u32, String), Texture<'a>>,
}

impl<'a> FontBook<'a> {
    pub fn new(
        ttf: &'a Sdl2TtfContext,
        creator: &'a TextureCreator<WindowContext>,
        candidates: &[String],
    ) -> Result<Self, String> {
        for path in candidates {
            if !std::path::Path::new(path).exists() {
                continue;
            }
            let Some(idx) = pick_face(ttf, path)? else { continue };
            let f = ttf
                .load_font_at_index(path, idx, 30u16)
                .map_err(|e| format!("字体打开失败：{path}#{idx}（{e}）"))?;
            f.size_of_char('夏').map_err(|e| format!("字体不可渲染中文：{path}（{e}）"))?;
            return Ok(Self {
                ttf,
                creator,
                source: (path.clone(), idx),
                fonts: HashMap::new(),
                textures: HashMap::new(),
            });
        }
        Err("找不到可用中文字体（候选：随包/Linux Noto/Windows 雅黑）".into())
    }

    pub fn font(&mut self, size: u16) -> Result<&Font<'a, 'static>, String> {
        if !self.fonts.contains_key(&size) {
            let (path, idx) = &self.source;
            let f = self
                .ttf
                .load_font_at_index(path, *idx, size)
                .map_err(|e| format!("字体打开失败：{path}#{idx}（{e}）"))?;
            self.fonts.insert(size, f);
        }
        Ok(self.fonts.get(&size).unwrap())
    }

    /// 渲染文本为纹理（缓存）
    pub fn render_text(
        &mut self,
        size: u16,
        color: Color,
        text: &str,
    ) -> Result<&Texture<'a>, String> {
        let key = (size, rgb_key(color), text.to_string());
        if !self.textures.contains_key(&key) {
            self.font(size)?;
            let font = self.fonts.get(&size).ok_or(format!("字号 {size} 初始化失败"))?;
            let surface = font
                .render(text)
                .blended(color)
                .map_err(|e| format!("文字渲染失败：{text:?}（{e}）"))?;
            let mut tex = self
                .creator
                .create_texture_from_surface(&surface)
                .map_err(|e| format!("文字转纹理失败：{e}"))?;
            tex.set_blend_mode(BlendMode::Blend);
            self.textures.insert(key.clone(), tex);
        }
        Ok(self.textures.get(&key).unwrap())
    }

    pub fn line_h(&mut self, size: u16) -> i32 {
        self.font(size).map(|f| f.recommended_line_spacing()).unwrap_or((size as f32 * 1.5) as i32)
    }

    pub fn clear_text_cache(&mut self) {
        self.textures.clear();
    }
}

fn rgb_key(c: Color) -> u32 {
    ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32
}

/// TTC 面探测：优先 SC/雅黑面；非 TTC 用 0 号面
fn pick_face(ttf: &Sdl2TtfContext, path: &str) -> Result<Option<u32>, String> {
    let mut fallback = None;
    for idx in 0..MAX_FACES {
        match ttf.load_font_at_index(path, idx, 16u16) {
            Ok(f) => {
                let family = f.face_family_name().unwrap_or_default();
                if family.contains("SC") || family.contains("YaHei") {
                    return Ok(Some(idx));
                }
                if fallback.is_none() {
                    fallback = Some(idx);
                }
            }
            Err(_) => break, // 面数耗尽
        }
    }
    Ok(fallback)
}
