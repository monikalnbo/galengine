//! YAML 声明式图层叠加：按序绘制 image/fill/text 图层（ui.layers.* 配置）。
//! 用途：标题画面/自定义界面用多张图+文字组合，改 YAML 不改 Rust。

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas};
use sdl2::video::Window;

use gal_config::{data_dir, Config, LayerDef, LOGICAL_H, LOGICAL_W};
use gal_render::assets::TextureBank;
use gal_script::vars::Vars;
use gal_text::font::FontBook;

const DEFAULT_FONT: u16 = 32;

fn layer_rect(l: &LayerDef) -> Rect {
    match l.rect {
        Some([x, y, w, h]) => Rect::new(x, y, w as u32, h as u32),
        None => Rect::new(0, 0, LOGICAL_W, LOGICAL_H),
    }
}

/// 数据目录相对路径 → 绝对路径（绝对路径原样）
fn resolve(path: &str) -> String {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        path.into()
    } else {
        format!("{}/{path}", data_dir())
    }
}

/// {meta.title} / {sf.xxx} / {f.xxx} 变量替换；未知键保留原文
fn subst(text: &str, conf: &Config, vars: &Vars) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) => {
                let key = &after[..end];
                let val = match key.split_once('.') {
                    Some(("meta", "title")) => conf.meta.title.clone(),
                    Some(("meta", "version")) => conf.meta.version.clone(),
                    Some(("sf", name)) => vars.get_or(&format!("sf.{name}")).as_str(),
                    Some(("f", name)) => vars.get_or(&format!("f.{name}")).as_str(),
                    _ => key.to_string(),
                };
                out.push_str(&val);
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(rest);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// 入场动画轨道（`anim` 字段解析结果）
#[derive(Clone, Debug, PartialEq)]
pub enum Anim {
    /// 纯淡入
    Fade { ms: f32 },
    /// 淡入 + 位移（dx, dy 为动画起始偏移，画到最终位置时归零）
    Slide { ms: f32, dx: i32, dy: i32 },
    /// 淡入 + 放大（起始倍率 → 1.0）
    Zoom { ms: f32, from: f32 },
}

/// 解析 `fade:<ms>` / `slide:<ms>:<dx,dy>` / `zoom:<ms>:<倍率>`。
/// 写错或 ms 为 0 → None（直接显示，不报错）。
pub fn parse_anim(spec: &str) -> Option<Anim> {
    let mut it = spec.split(':');
    let kind = it.next()?.trim();
    let ms: f32 = it.next()?.trim().parse().ok()?;
    if ms <= 0.0 || ms.is_nan() {
        return None;
    }
    match kind {
        "fade" => Some(Anim::Fade { ms }),
        "slide" => {
            let (dx, dy) = it.next()?.split_once(',')?;
            Some(Anim::Slide { ms, dx: dx.trim().parse().ok()?, dy: dy.trim().parse().ok()? })
        }
        "zoom" => Some(Anim::Zoom { ms, from: it.next()?.trim().parse().ok()? }),
        _ => None,
    }
}

/// 0..=1 进度（ms 已过/总时长）
fn progress(now_ms: f32, start_ms: f32, dur_ms: f32) -> f32 {
    ((now_ms - start_ms) / dur_ms.max(1.0)).clamp(0.0, 1.0)
}

/// 缓出：起步快、收尾慢
fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

/// 按序绘制图层（image 缺图跳过不炸画面；text 在 rect 内居中）。
/// 传入整组图层的起始时刻，按各层 `anim` 计算入场进度；`anim` 都缺省时
/// 与旧版本逐像素一致。
#[allow(clippy::too_many_arguments)]
pub fn draw_layers(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    bank: &mut TextureBank,
    conf: &Config,
    layers: &[LayerDef],
    vars: &Vars,
    now_ms: f32,
    start_ms: f32,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    for l in layers {
        let anim = l.anim.as_deref().and_then(parse_anim);
        let p = match &anim {
            Some(Anim::Fade { ms }) => progress(now_ms, start_ms, *ms),
            Some(Anim::Slide { ms, .. }) => progress(now_ms, start_ms, *ms),
            Some(Anim::Zoom { ms, .. }) => progress(now_ms, start_ms, *ms),
            None => 1.0,
        };
        let e = ease_out(p);
        let alpha = (255.0 * e) as u8;
        let (ox, oy, scale) = match &anim {
            Some(Anim::Slide { dx, dy, .. }) => {
                (((*dx as f32) * (1.0 - e)) as i32, ((*dy as f32) * (1.0 - e)) as i32, 1.0)
            }
            Some(Anim::Zoom { from, .. }) => (0, 0, *from + (1.0 - *from) * e),
            _ => (0, 0, 1.0),
        };
        let rect = layer_rect(l);
        // 缩放围绕 rect 中心（zoom 动画），平移在缩放之后叠加
        let (w, h) =
            (((rect.width() as f32) * scale) as u32, ((rect.height() as f32) * scale) as u32);
        let dst = Rect::new(
            rect.x + (rect.width() as i32 - w as i32) / 2 + ox,
            rect.y + (rect.height() as i32 - h as i32) / 2 + oy,
            w.max(1),
            h.max(1),
        );
        if let Some(path) = &l.image {
            let full = resolve(path);
            if std::path::Path::new(&full).exists() {
                bank.draw_scaled(canvas, &full, dst, alpha as f32 / 255.0)?;
            } else {
                eprintln!("[layers] 缺图跳过：{path}");
            }
        } else if let Some([r, g, b, a]) = l.fill {
            let a = (a as u32 * alpha as u32 / 255) as u8;
            canvas.set_draw_color(Color::RGBA(r, g, b, a));
            canvas.fill_rect(dst)?;
        }
        if let Some(text) = &l.text {
            let shown = subst(text, conf, vars);
            let size = l.font_size.unwrap_or(DEFAULT_FONT);
            let [r, g, b] = l.color.unwrap_or([255, 255, 255]);
            // 动画缩放按字号近似（文字纹理保持整数像素清晰）
            let size = ((size as f32) * scale).max(1.0) as u16;
            let tex = fonts.render_text(size, Color::RGBA(r, g, b, alpha), &shown)?;
            let q = tex.query();
            let dst = Rect::new(
                dst.x + (dst.width() as i32 - q.width as i32) / 2,
                dst.y + (dst.height() as i32 - q.height as i32) / 2,
                q.width,
                q.height,
            );
            canvas.copy(tex, None, Some(dst))?;
        }
    }
    Ok(())
}

/// 收集某图层组的图片绝对路径（场景 GC 保留集用）
pub fn image_paths(conf: &Config, key: &str) -> Vec<String> {
    conf.ui
        .layers
        .get(key)
        .map(|v| v.iter().filter_map(|l| l.image.as_ref().map(|p| resolve(p))).collect())
        .unwrap_or_default()
}
