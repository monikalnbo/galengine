//! 图片资源库：路径 → 纹理缓存；命中顺序 GPU缓存 → 预解码staging → 同步兜底。

use std::collections::HashMap;

use sdl2::image::LoadTexture;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};

use crate::gfx::prefetch::{Decoded, Prefetcher};

pub struct TextureBank<'a> {
    creator: &'a TextureCreator<WindowContext>,
    cache: HashMap<String, Texture<'a>>,
    pub prefetch: Prefetcher,
}

impl<'a> TextureBank<'a> {
    pub fn new(creator: &'a TextureCreator<WindowContext>) -> Self {
        Self { creator, cache: HashMap::new(), prefetch: Prefetcher::new() }
    }

    pub fn load(&mut self, path: &str) -> Result<(), String> {
        if self.cache.contains_key(path) {
            return Ok(());
        }
        let tex: Texture = match self.prefetch.take(path) {
            Some(dec) => self.from_decoded(path, &dec)?,
            None => self
                .creator
                .load_texture(path)
                .map_err(|e| format!("图片加载失败：{path}（{e}）"))?,
        };
        let mut tex = tex;
        tex.set_blend_mode(BlendMode::Blend); // 立绘 PNG 透明合成
        self.cache.insert(path.to_string(), tex);
        Ok(())
    }

    fn from_decoded(&self, path: &str, dec: &Decoded) -> Result<Texture<'a>, String> {
        let mut tex = self
            .creator
            .create_texture_static(Some(PixelFormatEnum::RGBA8888), dec.w, dec.h)
            .map_err(|e| format!("纹理创建失败：{path}（{e}）"))?;
        tex.update(None, &dec.pixels, dec.w as usize * 4)
            .map_err(|e| format!("纹理上传失败：{path}（{e}）"))?;
        Ok(tex)
    }

    /// 整幅铺满游戏画面（素材统一 1280×720 整幅画布，无需坐标）
    pub fn draw_full_alpha(
        &mut self,
        canvas: &mut Canvas<Window>,
        path: &str,
        alpha: f32,
    ) -> Result<(), String> {
        self.load(path)?;
        if let Some(tex) = self.cache.get_mut(path) {
            tex.set_alpha_mod((alpha * 255.0).clamp(0.0, 255.0) as u8);
            canvas.copy(tex, None, None)?;
        }
        Ok(())
    }

    /// 窗口坐标任意矩形贴图（reach 越界层用；需已 load 或同步兜底）
    pub fn draw_scaled(
        &mut self,
        canvas: &mut Canvas<Window>,
        path: &str,
        dst: sdl2::rect::Rect,
        alpha: f32,
    ) -> Result<(), String> {
        self.load(path)?;
        if let Some(tex) = self.cache.get_mut(path) {
            tex.set_alpha_mod((alpha * 255.0).clamp(0.0, 255.0) as u8);
            canvas.copy(tex, None, Some(dst))?;
        }
        Ok(())
    }

    /// 场景资源回收（stage 为真相源）
    pub fn retain(&mut self, keep: &std::collections::HashSet<String>) {
        self.cache.retain(|k, _| keep.contains(k));
        self.prefetch.retain(keep);
    }
}

/// 剧本 storage 短名 → 实际路径（bg/cg 查 bgimage jpg|png，立绘查 fgimage png）
pub fn resolve(kind: &str, storage: &str) -> String {
    let data = crate::config::data_dir();
    match kind {
        "bg" | "cg" => first_exists(
            &[format!("{data}/bgimage/{storage}.jpg"), format!("{data}/bgimage/{storage}.png")],
            &format!("{data}/bgimage/{storage}.jpg"),
        ),
        "char" => format!("{data}/fgimage/{storage}.png"),
        _ => format!("{data}/{storage}"),
    }
}

fn first_exists(cands: &[String], fallback: &str) -> String {
    cands.iter().find(|p| std::path::Path::new(p).exists()).cloned().unwrap_or_else(|| fallback.to_string())
}
