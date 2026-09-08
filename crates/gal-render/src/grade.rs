//! 背景色调处理：解码后、上传纹理前对像素做饱和度缩放（Rec.601 亮度轴）。
//! 只对 bgimage 素材应用（立绘保持原饱和度，主体更突出）。

/// 原地缩放饱和度：sat=1 原样；sat<1 向亮度靠拢（降饱和）；sat>0 允许增强。
pub fn apply_saturation(rgba: &mut [u8], sat: f32) {
    if (sat - 1.0).abs() < 1e-3 {
        return;
    }
    for px in rgba.as_chunks_mut::<4>().0 {
        let (r, g, b) = (px[0] as f32, px[1] as f32, px[2] as f32);
        let lum = 0.299 * r + 0.587 * g + 0.114 * b;
        px[0] = (lum + (r - lum) * sat).clamp(0.0, 255.0) as u8;
        px[1] = (lum + (g - lum) * sat).clamp(0.0, 255.0) as u8;
        px[2] = (lum + (b - lum) * sat).clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 灰色不受影响() {
        let mut px = vec![128, 128, 128, 255];
        apply_saturation(&mut px, 0.5);
        assert_eq!(px, vec![128, 128, 128, 255]);
    }

    #[test]
    fn 纯红降饱和趋向橙灰() {
        let mut px = vec![255, 0, 0, 255];
        apply_saturation(&mut px, 0.5);
        // Rec.601 亮度 76.2；R=76.2+(255-76.2)*0.5≈166，G/B=76.2*0.5≈38
        assert_eq!(px[0], 165);
        assert_eq!(px[1], 38);
        assert_eq!(px[2], 38);
    }

    #[test]
    fn alpha通道不动() {
        let mut px = vec![10, 20, 30, 77];
        apply_saturation(&mut px, 0.2);
        assert_eq!(px[3], 77);
    }
}
