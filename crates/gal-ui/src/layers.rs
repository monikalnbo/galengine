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

/// 按序绘制图层（image 缺图跳过不炸画面；text 在 rect 内居中）
pub fn draw_layers(
    canvas: &mut Canvas<Window>,
    fonts: &mut FontBook,
    bank: &mut TextureBank,
    conf: &Config,
    layers: &[LayerDef],
    vars: &Vars,
) -> Result<(), String> {
    canvas.set_blend_mode(BlendMode::Blend);
    for l in layers {
        if let Some(path) = &l.image {
            let full = resolve(path);
            if std::path::Path::new(&full).exists() {
                bank.draw_scaled(canvas, &full, layer_rect(l), 1.0)?;
            } else {
                eprintln!("[layers] 缺图跳过：{path}");
            }
        } else if let Some([r, g, b, a]) = l.fill {
            canvas.set_draw_color(Color::RGBA(r, g, b, a));
            canvas.fill_rect(layer_rect(l))?;
        }
        if let Some(text) = &l.text {
            let shown = subst(text, conf, vars);
            let size = l.font_size.unwrap_or(DEFAULT_FONT);
            let [r, g, b] = l.color.unwrap_or([255, 255, 255]);
            let tex = fonts.render_text(size, Color::RGB(r, g, b), &shown)?;
            let q = tex.query();
            let rect = layer_rect(l);
            let dst = Rect::new(
                rect.x + (rect.width() as i32 - q.width as i32) / 2,
                rect.y + (rect.height() as i32 - q.height as i32) / 2,
                q.width,
                q.height,
            );
            canvas.copy(tex, None, Some(dst))?;
        }
        // ponytail: anim 字段未实现动画（当前无需求）
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
