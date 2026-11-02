//! 字体库：候选链解析 + TTC 面探测（SC=索引2 勿硬编码，用 family 名探测）
//! + 可切换字体族（作者 text.families，玩家设置里切）+ 文字纹理缓存。
//!
//! 字体族之间共享同一份字号缓存表：`set_family(idx)` 只换当前族，
//! 渲染/度量都走「当前族」；纹理缓存键含族号，切换后不会串脸。

use std::collections::HashMap;
use std::sync::Arc;

use sdl2::pixels::Color;
use sdl2::render::{BlendMode, Texture, TextureCreator};
use sdl2::ttf::{Font, Sdl2TtfContext};
use sdl2::video::WindowContext;

const MAX_FACES: u32 = 8;
/// 描边/阴影已是纹理的一部分：这些属性变了必须换缓存键
const DEFAULT_PROBE_SIZE: u16 = 30;

/// 一个可选字体族（名字在设置面板显示，路径相对数据目录或绝对）
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Family {
    pub name: String,
    pub path: String,
}

/// 一族字体的解析结果（路径 + TTC 面号）
#[derive(Clone, Debug)]
struct Source {
    path: String,
    index: u32,
}

pub struct FontBook<'a> {
    ttf: &'a Sdl2TtfContext,
    creator: &'a TextureCreator<WindowContext>,
    /// 全部可用族（第 0 个是内置候选链选中的那个）
    families: Vec<Source>,
    /// 面板上显示的族名
    names: Vec<String>,
    cur: usize,
    fonts: HashMap<(usize, u16), Font<'a, 'static>>,
    /// 键 = (族, 字号, 颜色, 描边宽度, 文本)；描边宽度 0 = 普通文字
    textures: HashMap<(usize, u16, u32, u8, Arc<str>), Texture<'a>>,
}

impl<'a> FontBook<'a> {
    /// 候选链 + 作者额外字体族；找不到任何可用中文字体即报错
    pub fn new(
        ttf: &'a Sdl2TtfContext,
        creator: &'a TextureCreator<WindowContext>,
        candidates: &[String],
        extra: &[Family],
    ) -> Result<Self, String> {
        let mut families: Vec<Source> = Vec::new();
        let mut names: Vec<String> = Vec::new();

        for path in candidates {
            if families.len() >= 8 {
                break;
            }
            let Some(src) = resolve(ttf, path)? else { continue };
            if families.iter().any(|f| f.path == src.path && f.index == src.index) {
                continue;
            }
            let name = face_name(ttf, &src)?;
            families.push(src);
            names.push(name);
        }
        if families.is_empty() {
            return Err("找不到可用中文字体（候选：随包/Linux Noto/Windows 雅黑）".into());
        }

        // 作者声明的字体族：坏文件/非中文字体跳过并提示（优雅降级）
        for f in extra {
            if f.path.trim().is_empty() {
                continue;
            }
            let path = crate::resolve_path(&f.path);
            match resolve(ttf, &path) {
                Ok(Some(src)) => {
                    if families.iter().any(|x| x.path == src.path && x.index == src.index) {
                        continue;
                    }
                    let label = if f.name.trim().is_empty() {
                        crate::file_stem(&path)
                    } else {
                        f.name.clone()
                    };
                    families.push(src);
                    names.push(label);
                }
                _ => eprintln!("[font] 忽略不可用的字体族：{}（{}）", f.name, f.path),
            }
        }

        let mut book = Self {
            ttf,
            creator,
            families,
            names,
            cur: 0,
            fonts: HashMap::new(),
            textures: HashMap::new(),
        };
        book.font(DEFAULT_PROBE_SIZE)?;
        Ok(book)
    }

    /// 可切换的字体族名（顺序即设置面板顺序；设置面板值 = 索引+1，0=默认不写）
    pub fn family_names(&self) -> &[String] {
        &self.names
    }

    pub fn family_index(&self) -> usize {
        self.cur
    }

    /// 切换字体族；越界静默忽略（旧配置里的族可能已被作者删掉）
    pub fn set_family(&mut self, idx: usize) -> bool {
        if idx == self.cur || idx >= self.families.len() {
            return false;
        }
        self.cur = idx;
        self.textures.clear();
        true
    }

    /// 运行期追加一个字体族（玩家加载系统字体）；返回新族名。
    /// 重复/坏文件/不渲染中文 → None（静默跳过，不打断游戏）。
    pub fn add_family(&mut self, path: &str, label: &str) -> Option<String> {
        if self.families.len() >= 32 {
            return None;
        }
        let src = resolve(self.ttf, path).ok().flatten()?;
        if self.families.iter().any(|f| f.path == src.path && f.index == src.index) {
            return None;
        }
        let name = if label.trim().is_empty() {
            face_name(self.ttf, &src).ok()?
        } else {
            label.to_string()
        };
        self.families.push(src);
        self.names.push(name.clone());
        Some(name)
    }

    pub fn font(&mut self, size: u16) -> Result<&Font<'a, 'static>, String> {
        let key = (self.cur, size);
        if !self.fonts.contains_key(&key) {
            let src = self.families[key.0].clone();
            let f = self
                .ttf
                .load_font_at_index(&src.path, src.index, size)
                .map_err(|e| format!("字体打开失败：{}#{}（{e}）", src.path, src.index))?;
            self.fonts.insert(key, f);
        }
        Ok(self.fonts.get(&key).unwrap())
    }

    /// 渲染文本为纹理（缓存）
    pub fn render_text(
        &mut self,
        size: u16,
        color: Color,
        text: &str,
    ) -> Result<&Texture<'a>, String> {
        self.render_outlined(size, color, text, 0)
    }

    /// 带描边渲染：`width>0` 时字形外扩 width 像素（SDL_ttf Outline），
    /// 绘制端需把纹理左上角向左上偏移 width 才能与正文对齐。
    pub fn render_outlined(
        &mut self,
        size: u16,
        color: Color,
        text: &str,
        width: u8,
    ) -> Result<&Texture<'a>, String> {
        let key = (self.cur, size, rgba_key(color), width, Arc::<str>::from(text));
        if !self.textures.contains_key(&key) {
            let idx = (key.0, key.1);
            if !self.fonts.contains_key(&idx) {
                self.font(idx.1)?;
            }
            let font = self.fonts.get_mut(&idx).ok_or(format!("字号 {size} 初始化失败"))?;
            // 描边宽度是 Font 上的可变状态，换宽度时 SDL_ttf 会重排 glyph
            font.set_outline_width(width as u16);
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

    /// 文本像素宽度（按当前字体族与字号；用于 ruby 基底/注音居中）
    pub fn text_width(&mut self, size: u16, text: &str) -> u32 {
        if text.is_empty() {
            return 0;
        }
        let Ok(font) = self.font(size) else { return 0 };
        font.size_of(text).map(|(w, _)| w).unwrap_or_else(|_| {
            text.chars().map(|c| font.size_of_char(c).map(|(w, _)| w).unwrap_or(0)).sum()
        })
    }

    /// 计算单行前 N 个字符的像素宽度（打字机裁剪绘制用）
    pub fn line_prefix_w(&mut self, size: u16, line: &str, char_count: usize) -> u32 {
        let Ok(font) = self.font(size) else { return 0 };
        let prefix: String = line.chars().take(char_count).collect();
        if prefix.is_empty() {
            return 0;
        }
        font.size_of(&prefix).map(|(w, _)| w).unwrap_or_else(|_| {
            prefix.chars().map(|c| font.size_of_char(c).map(|(w, _)| w).unwrap_or(0)).sum()
        })
    }

    /// 行高（字体推荐行高 + 玩家/作者设定的额外行距）
    pub fn line_h(&mut self, size: u16) -> i32 {
        self.line_h_spaced(size, 0)
    }

    pub fn line_h_spaced(&mut self, size: u16, extra: i32) -> i32 {
        let base = self
            .font(size)
            .map(|f| f.recommended_line_spacing())
            .unwrap_or((size as f32 * 1.5) as i32);
        (base + extra).max(size as i32 / 2)
    }

    pub fn clear_text_cache(&mut self) {
        self.textures.clear();
    }
}

/// 解析一个路径：存在 + 有可用中文面 → Some(源)
fn resolve(ttf: &Sdl2TtfContext, path: &str) -> Result<Option<Source>, String> {
    if path.trim().is_empty() || !std::path::Path::new(path).exists() {
        return Ok(None);
    }
    let Some(idx) = pick_face(ttf, path)? else { return Ok(None) };
    let f = ttf
        .load_font_at_index(path, idx, DEFAULT_PROBE_SIZE)
        .map_err(|e| format!("字体打开失败：{path}#{idx}（{e}）"))?;
    // 无法渲染中文的字体不能当正文族（英文专用字体）
    f.size_of_char('夏').map_err(|e| format!("字体不可渲染中文：{path}（{e}）"))?;
    Ok(Some(Source { path: path.to_string(), index: idx }))
}

fn face_name(ttf: &Sdl2TtfContext, src: &Source) -> Result<String, String> {
    Ok(ttf
        .load_font_at_index(&src.path, src.index, 16u16)
        .ok()
        .and_then(|f| f.face_family_name())
        .unwrap_or_else(|| crate::file_stem(&src.path)))
}

fn rgba_key(c: Color) -> u32 {
    ((c.r as u32) << 24) | ((c.g as u32) << 16) | ((c.b as u32) << 8) | c.a as u32
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
