//! Q版出场用背景模糊：解码 → 降采样 320×180 → 两趟可分离盒式模糊。
//! 每次 bg 切换一次性 CPU 成本（毫秒级），纹理由 assets 缓存。

use crate::prefetch::Decoded;

const SMALL_W: u32 = 320;
const SMALL_H: u32 = 180;
/// 盒半径 2、两趟 ≈ 高斯观感
const RADIUS: usize = 2;
const PASSES: usize = 2;

/// 读图 → 模糊小图（RGBA）。用于 Q版立绘在场时的背景虚化。
pub fn blurred_small(path: &str) -> Result<Decoded, String> {
    let img = image::open(path).map_err(|e| format!("背景模糊解码失败：{path}（{e}）"))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut small = downscale(rgba.as_raw(), w, h, SMALL_W, SMALL_H);
    for _ in 0..PASSES {
        box_blur(&mut small);
    }
    Ok(small)
}

/// 盒平均降采样（目标像素 = 源块均值；alpha 取均值后二值化保不透明）
fn downscale(src: &[u8], sw: u32, sh: u32, tw: u32, th: u32) -> Decoded {
    let mut out = vec![0u8; (tw * th * 4) as usize];
    for ty in 0..th {
        let y0 = sh * ty / th;
        let y1 = ((sh * (ty + 1) / th).max(y0 + 1)).min(sh);
        for tx in 0..tw {
            let x0 = sw * tx / tw;
            let x1 = ((sw * (tx + 1) / tw).max(x0 + 1)).min(sw);
            let (mut r, mut g, mut b, mut a, mut n) = (0u32, 0u32, 0u32, 0u32, 0u32);
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = ((y * sw + x) * 4) as usize;
                    r += src[i] as u32;
                    g += src[i + 1] as u32;
                    b += src[i + 2] as u32;
                    a += src[i + 3] as u32;
                    n += 1;
                }
            }
            let o = ((ty * tw + tx) * 4) as usize;
            out[o] = (r / n) as u8;
            out[o + 1] = (g / n) as u8;
            out[o + 2] = (b / n) as u8;
            out[o + 3] = if a / n > 127 { 255 } else { 0 };
        }
    }
    Decoded { w: tw, h: th, pixels: out }
}

/// 就地盒式模糊（水平+垂直分离，边界钳位）
fn box_blur(d: &mut Decoded) {
    let (w, h) = (d.w as usize, d.h as usize);
    let mut tmp = d.pixels.clone();
    // 水平
    blur_pass(&d.pixels, &mut tmp, w, h, true);
    // 垂直
    blur_pass(&tmp, &mut d.pixels, w, h, false);
}

fn blur_pass(src: &[u8], dst: &mut [u8], w: usize, h: usize, horizontal: bool) {
    let len = if horizontal { w } else { h };
    for y in 0..h {
        for x in 0..w {
            let base = y * w + x;
            let mut acc = [0u32; 4];
            let mut n = 0;
            for k in -(RADIUS as isize)..=(RADIUS as isize) {
                let coord = if horizontal { x as isize + k } else { y as isize + k };
                let coord = coord.clamp(0, len as isize - 1) as usize;
                let i = if horizontal { (y * w + coord) * 4 } else { (coord * w + x) * 4 };
                for c in 0..4 {
                    acc[c] += src[i + c] as u32;
                }
                n += 1;
            }
            for c in 0..4 {
                dst[base * 4 + c] = (acc[c] / n) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 纯色图模糊不变() {
        let src = vec![200u8; (64 * 48 * 4) as usize];
        let mut d = Decoded { w: 64, h: 48, pixels: src };
        box_blur(&mut d);
        assert!(d.pixels.iter().all(|&v| v == 200));
    }

    #[test]
    fn 亮点被扩散() {
        let mut px = vec![0u8; (64 * 48 * 4) as usize];
        let i = (24 * 64 + 32) * 4;
        px[i] = 255;
        let mut d = Decoded { w: 64, h: 48, pixels: px };
        box_blur(&mut d);
        // 邻近像素应被点亮，远处仍为 0
        assert!(d.pixels[(24 * 64 + 33) * 4] > 0);
        assert!(d.pixels[(24 * 64 + 30) * 4] > 0);
        assert_eq!(d.pixels[(24 * 64 + 40) * 4], 0);
    }

    #[test]
    fn 降采样尺寸正确且均值守恒() {
        let mut src = vec![0u8; (320 * 180 * 4) as usize];
        for px in src.as_chunks_mut::<4>().0 {
            px[0] = 100;
            px[3] = 255;
        }
        let d = downscale(&src, 320, 180, 160, 90);
        assert_eq!((d.w, d.h), (160, 90));
        assert!(d.pixels.iter().step_by(4).all(|&v| v == 100));
    }
}
