//! 存档缩略图纹理缓存

use std::collections::HashMap;

use sdl2::image::LoadTexture;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;

use gal_save::meta::FakeSave;
use gal_save::slots::SaveEntry;

/// 缩略图纹理缓存（开档界面时构建一次）
pub struct ThumbCache<'a> {
    creator: &'a TextureCreator<WindowContext>,
    map: HashMap<String, Texture<'a>>,
}

impl<'a> ThumbCache<'a> {
    pub fn new(creator: &'a TextureCreator<WindowContext>) -> Self {
        Self { creator, map: HashMap::new() }
    }

    /// entries: (槽号, 内容)；fake: 不明档列表（image 为 data 相对路径）
    pub fn build(&mut self, entries: &[(usize, Option<SaveEntry>)], fake: &[FakeSave]) {
        self.map.clear();
        for (slot, e) in entries {
            if let Some(e) = e {
                if let Some(tex) = png_texture(self.creator, &e.thumb) {
                    self.map.insert(format!("s{slot}"), tex);
                }
            }
        }
        for (i, f) in fake.iter().enumerate() {
            let path = format!("{}/{}", gal_config::data_dir(), f.image);
            if let Ok(tex) = self.creator.load_texture(&path) {
                self.map.insert(format!("f{i}"), tex);
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&Texture<'a>> {
        self.map.get(key)
    }
}

/// PNG 字节 → RGBA 纹理（小端架构必须用 ABGR8888，对齐错题本 B5 规约）
fn png_texture<'a>(creator: &'a TextureCreator<WindowContext>, png: &[u8]) -> Option<Texture<'a>> {
    let img = image::load_from_memory(png).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let mut tex = creator.create_texture_static(Some(PixelFormatEnum::ABGR8888), w, h).ok()?;
    tex.update(None, &img, w as usize * 4).ok()?;
    Some(tex)
}
