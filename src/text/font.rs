//! 字体库：中文字体解析、文字纹理渲染与缓存。
//!
//! 字体查找顺序（docs/20 §二：优先完整字体，无 4MB 限制）：
//! 1. 随包完整字体 game/data/system/font.ttf（A8 打包时放入）
//! 2. Linux 完整 Noto Sans CJK（开发/xvfb 验收环境）
//! 3. Windows 微软雅黑（玩家机器，零分发成本）
//! 4. 旧子集字体兜底
//! TTC 多面文件自动探测 SC 面。

use std::collections::HashMap;

use sdl2::pixels::Color;
use sdl2::render::{Texture, TextureCreator};
use sdl2::ttf::{Font, Sdl2TtfContext};
use sdl2::video::WindowContext;

const FONT_CANDIDATES: &[&str] = &[
    "game/data/system/font.ttf",                                  // 随包（未来）
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",     // Linux
    "C:\\Windows\\Fonts\\msyh.ttc",                               // Windows
    "game/data/system_polyfill/font.ttf",                         // 子集兜底
];

/// TTC 面探测上限（Noto CJK 共 5 面，留余量）
const MAX_FACES: u32 = 8;

pub struct FontBook<'a> {
    ttf: &'a Sdl2TtfContext,
    creator: &'a TextureCreator<WindowContext>,
    fonts: HashMap<u16, Font<'a, 'static>>,
    /// (字号, 颜色, 文本) -> 纹理
    textures: HashMap<(u16, u32, String), Texture<'a>>,
    /// 实际命中的字体源（诊断显示用）
    pub source: String,
}

impl<'a> FontBook<'a> {
    pub fn new(
        ttf: &'a Sdl2TtfContext,
        creator: &'a TextureCreator<WindowContext>,
        candidates: &[String],
    ) -> Result<Self, String> {
        // 命中一个可用字体文件，探测合适的面
        for path in candidates {
            if !std::path::Path::new(path).exists() {
                continue;
            }
            let Some(idx) = pick_face(ttf, path)? else {
                continue; // 打不开，试下一候选
            };
            // 预热常用字号验证可渲染
            let f = ttf
                .load_font_at_index(path, idx, 30u16)
                .map_err(|e| format!("字体打开失败：{path}#{idx}（{e}）"))?;
            let _ = f.size_of_char('夏').map_err(|e| format!("字体不可渲染中文：{path}（{e}）"))?;
            return Ok(Self {
                ttf,
                creator,
                fonts: HashMap::new(),
                textures: HashMap::new(),
                source: format!("{path}#{idx}"),
            });
        }
        Err("找不到可用中文字体（候选：随包/Linux Noto/Windows 雅黑/子集）".into())
    }

    /// 内置候选（配置未提供时）
    pub fn builtin_candidates() -> Vec<String> {
        FONT_CANDIDATES.iter().map(|s| s.to_string()).collect()
    }

    /// 取（或创建）指定字号字体
    pub fn font(&mut self, size: u16) -> Result<&Font<'a, 'static>, String> {
        if !self.fonts.contains_key(&size) {
            let (path, idx) = self.parse_source()?;
            let f = self
                .ttf
                .load_font_at_index(path, idx, size)
                .map_err(|e| format!("字体打开失败：{}#{idx}（{e}）", path))?;
            self.fonts.insert(size, f);
        }
        Ok(self.fonts.get(&size).unwrap())
    }

    fn parse_source(&self) -> Result<(&str, u32), String> {
        // source 形如 "path#idx"
        match self.source.rfind('#') {
            Some(p) => Ok((&self.source[..p], self.source[p + 1..].parse().unwrap_or(0))),
            None => Ok((&self.source, 0)),
        }
    }

    /// 渲染文本为纹理（带缓存）；返回纹理供绘制
    pub fn render_text(
        &mut self,
        size: u16,
        color: Color,
        text: &str,
    ) -> Result<&Texture<'a>, String> {
        let key = (size, rgb_key(color), text.to_string());
        if !self.textures.contains_key(&key) {
            self.font(size)?; // 确保字号已加载
            let Self {
                fonts, creator, ..
            } = self;
            let font = fonts
                .get(&size)
                .ok_or_else(|| format!("字号 {size} 初始化失败"))?;
            let surface = font
                .render(text)
                .blended(color)
                .map_err(|e| format!("文字渲染失败：{text:?}（{e}）"))?;
            let mut tex = creator
                .create_texture_from_surface(&surface)
                .map_err(|e| format!("文字转纹理失败：{e}"))?;
            tex.set_blend_mode(sdl2::render::BlendMode::Blend);
            self.textures.insert(key.clone(), tex);
        }
        Ok(self.textures.get(&key).unwrap())
    }

    /// 单字符宽度（打字机部分显示/断行用）
    pub fn char_w(&mut self, size: u16, ch: char) -> u32 {
        self.font(size)
            .ok()
            .and_then(|f| f.size_of_char(ch).ok())
            .map(|(w, _)| w)
            .unwrap_or(0)
    }

    /// 行高（推荐行距）
    pub fn line_h(&mut self, size: u16) -> i32 {
        self.font(size).map(|f| f.recommended_line_spacing()).unwrap_or((size as f32 * 1.5) as i32)
    }

    /// 场景切换：清空全部文字纹理缓存（对话文本不跨场景复用；
    /// 字体对象保留，名字牌重建成本 <1ms）
    pub fn clear_text_cache(&mut self) {
        self.textures.clear();
    }
}

fn rgb_key(c: Color) -> u32 {
    ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32
}

/// 探测 TTC 面：优先 SC（简中）面；非 TTC 或探测失败返回 None（换下一候选）
fn pick_face(ttf: &Sdl2TtfContext, path: &str) -> Result<Option<u32>, String> {
    let mut fallback: Option<u32> = None;
    for idx in 0..MAX_FACES {
        if let Ok(f) = ttf.load_font_at_index(path, idx, 16u16) {
            let family = f.face_family_name().unwrap_or_default();
            if family.contains("SC") || family.contains("YaHei") {
                return Ok(Some(idx));
            }
            if fallback.is_none() {
                fallback = Some(idx); // 单一面的字体用 0 号面
            }
        } else {
            break; // 面数耗尽
        }
    }
    Ok(fallback)
}
