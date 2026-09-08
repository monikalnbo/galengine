//! 图片资源库：路径 → 纹理缓存；命中顺序 GPU缓存 → 预解码staging → 同步兜底。

use std::collections::HashMap;

use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};

use crate::blur;
use crate::grade::apply_saturation;
use crate::prefetch::{self, Decoded, Prefetcher};
use gal_config::{LOGICAL_H, LOGICAL_W};

pub struct TextureBank<'a> {
    creator: &'a TextureCreator<WindowContext>,
    cache: HashMap<String, Texture<'a>>,
    /// Q版虚化用模糊背景缓存（切场景随主缓存一同回收）
    blur_cache: HashMap<String, Texture<'a>>,
    /// 背景饱和度缩放（display.bg_saturation，仅作用于 bgimage 路径）
    bg_saturation: f32,
    pub prefetch: Prefetcher,
}

impl<'a> TextureBank<'a> {
    pub fn new(creator: &'a TextureCreator<WindowContext>, bg_saturation: f32) -> Self {
        Self {
            creator,
            cache: HashMap::new(),
            blur_cache: HashMap::new(),
            bg_saturation,
            prefetch: Prefetcher::new(),
        }
    }

    pub fn load(&mut self, path: &str) -> Result<(), String> {
        if self.cache.contains_key(path) {
            return Ok(());
        }
        // 预解码staging → 同步解码兜底（统一走像素管线，调色一致）
        let dec = match self.prefetch.take(path) {
            Some(dec) => dec,
            None => {
                prefetch::decode(path).map_err(|e| format!("图片加载失败：{path}（{e}）"))?.into()
            }
        };
        let tex = self.build_decoded(path, &dec)?;
        self.cache.insert(path.to_string(), tex);
        Ok(())
    }

    fn build_decoded(&self, path: &str, dec: &Decoded) -> Result<Texture<'a>, String> {
        // 护眼降饱和：仅背景/CG（立绘保持原饱和度突出主体）
        let pixels = if path.contains("/bgimage/") && self.bg_saturation < 1.0 {
            let mut px = dec.pixels.clone();
            apply_saturation(&mut px, self.bg_saturation);
            px
        } else {
            dec.pixels.clone()
        };
        // 小端字节序陷阱：[R,G,B,A] 字节流必须配 ABGR8888（RGBA8888 会 R↔A 互换：
        // 立绘变半透明红、预解码背景偏红——错题本 #bg红）
        let mut tex = self
            .creator
            .create_texture_static(Some(PixelFormatEnum::ABGR8888), dec.w, dec.h)
            .map_err(|e| format!("纹理创建失败：{path}（{e}）"))?;
        tex.update(None, &pixels, dec.w as usize * 4)
            .map_err(|e| format!("纹理上传失败：{path}（{e}）"))?;
        tex.set_blend_mode(BlendMode::Blend);
        Ok(tex)
    }

    /// 整幅绘制 + 偏移（char 指令 x y 参数）
    pub fn draw_full_alpha_offset(
        &mut self,
        canvas: &mut Canvas<Window>,
        path: &str,
        alpha: f32,
        dx: i32,
        dy: i32,
    ) -> Result<(), String> {
        self.load(path)?;
        if let Some(tex) = self.cache.get_mut(path) {
            tex.set_alpha_mod((alpha * 255.0).clamp(0.0, 255.0) as u8);
            let q = tex.query();
            let dst = sdl2::rect::Rect::new(dx, dy, q.width, q.height);
            canvas.copy(tex, None, Some(dst))?;
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

    /// Q版虚化背景：低分辨率模糊纹理拉伸整幅（首次用时一次性 CPU 成本）
    pub fn draw_blur(&mut self, canvas: &mut Canvas<Window>, path: &str) -> Result<(), String> {
        if !self.blur_cache.contains_key(path) {
            let dec = blur::blurred_small(path)?;
            let tex = self.build_decoded(path, &dec)?;
            self.blur_cache.insert(path.to_string(), tex);
        }
        if let Some(tex) = self.blur_cache.get_mut(path) {
            let dst = sdl2::rect::Rect::new(0, 0, LOGICAL_W, LOGICAL_H);
            canvas.copy(tex, None, Some(dst))?;
        }
        Ok(())
    }

    /// 场景资源回收（stage 为真相源）
    pub fn retain(&mut self, keep: &std::collections::HashSet<String>) {
        self.cache.retain(|k, _| keep.contains(k));
        self.blur_cache.retain(|k, _| keep.contains(k));
        self.prefetch.retain(keep);
    }
}

/// 剧本 storage 短名 → 实际路径（bg/cg 查 bgimage jpg|png，立绘查 fgimage png）
pub fn resolve(kind: &str, storage: &str) -> String {
    let data = gal_config::data_dir();
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
    cands
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}
