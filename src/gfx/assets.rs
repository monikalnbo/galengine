//! 图片资源库：路径 -> 纹理 的加载与缓存。
//! 背景（JPG）/立绘（PNG）/CG 统一为 1280x720 画布，整幅铺满即可。

use std::collections::HashMap;

use sdl2::image::LoadTexture;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};

use crate::gfx::prefetch::Prefetcher;

pub struct TextureBank<'a> {
    creator: &'a TextureCreator<WindowContext>,
    cache: HashMap<String, Texture<'a>>,
    /// 后台预解码缓存层（消除场景切换卡帧）
    pub prefetch: Prefetcher,
}

impl<'a> TextureBank<'a> {
    pub fn new(creator: &'a TextureCreator<WindowContext>) -> Self {
        Self {
            creator,
            cache: HashMap::new(),
            prefetch: Prefetcher::new(),
        }
    }

    /// 加载并缓存。命中顺序：GPU缓存 → staging(已解码) → 同步兜底。
    pub fn load(&mut self, path: &str) -> Result<(), String> {
        if self.cache.contains_key(path) {
            return Ok(());
        }
        let tex: Texture = if let Some(dec) = self.prefetch.take(path) {
            self.texture_from_decoded(path, &dec)?
        } else {
            self.prefetch.stats_sync_fallback += 1;
            self.creator
                .load_texture(path)
                .map_err(|e| format!("图片加载失败：{path}（{e}）"))?
        };
        let mut tex = tex;
        tex.set_blend_mode(BlendMode::Blend); // 立绘 PNG 透明合成
        self.cache.insert(path.to_string(), tex);
        Ok(())
    }

    /// 解码像素 → GPU 纹理（毫秒级上传）
    fn texture_from_decoded(&self, path: &str, dec: &crate::gfx::prefetch::Decoded) -> Result<Texture<'a>, String> {
        let mut tex = self
            .creator
            .create_texture_static(Some(PixelFormatEnum::RGBA8888), dec.w, dec.h)
            .map_err(|e| format!("纹理创建失败：{path}（{e}）"))?;
        tex.update(None, &dec.pixels, dec.w as usize * 4)
            .map_err(|e| format!("纹理上传失败：{path}（{e}）"))?;
        Ok(tex)
    }

    /// 整幅铺满游戏画面（1280x720 同规格，无需坐标计算）
    pub fn draw_full(&mut self, canvas: &mut Canvas<Window>, path: &str) -> Result<(), String> {
        self.load(path)?;
        if let Some(tex) = self.cache.get_mut(path) {
            tex.set_alpha_mod(255);
            canvas.copy(tex, None, None)?;
        }
        Ok(())
    }

    /// 取已加载纹理（未加载返回 None；不触发加载）
    pub fn get(&self, path: &str) -> Option<&Texture<'a>> {
        self.cache.get(path)
    }

    /// 取可变纹理（越界层 alpha 调制等）
    pub fn get_mut(&mut self, path: &str) -> Option<&mut Texture<'a>> {
        self.cache.get_mut(path)
    }

    /// 整幅铺满，带透明度（crossfade 用）
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

    /// 场景资源回收：只保留 keep 集合中的纹理（stage 为场景真相源）
    pub fn retain(&mut self, keep: &std::collections::HashSet<String>) {
        self.cache.retain(|k, _| keep.contains(k));
        self.prefetch.retain(keep);
    }
}

/// 剧本 storage 短名 -> 实际资源路径
/// bg_* 查 bgimage（jpg/png），立绘名查 fgimage（png），cg 查 bgimage
pub fn resolve(kind: &str, storage: &str) -> String {
    let data = crate::config::data_dir();
    match kind {
        "bg" | "cg" => first_exists(
            &[
                format!("{data}/bgimage/{storage}.jpg"),
                format!("{data}/bgimage/{storage}.png"),
            ],
            &format!("{data}/bgimage/{storage}.jpg"),
        ),
        "char" => format!("{data}/fgimage/{storage}.png"),
        "img" => first_exists(
            &[
                format!("{data}/image/{storage}.png"),
                format!("{data}/image/{storage}.jpg"),
            ],
            &format!("{data}/image/{storage}.png"),
        ),
        _ => format!("{data}/{storage}"),
    }
}

fn first_exists(cands: &[String], fallback: &str) -> String {
    cands.iter()
        .find(|p| std::path::Path::new(p).exists())
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}
