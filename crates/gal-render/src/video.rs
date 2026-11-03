//! v0.3.8 视频子系统（`video` feature，默认关）：
//! ffmpeg-sidecar 子进程把视频解码成 rawvideo rgb24 帧，逐帧上传 SDL texture。
//!
//! - 找不到 ffmpeg 可执行文件 / 子进程失败 → 静默降级为「未在播」，演出不中断（三路日志方针）。
//! - 无音频轨道处理：视频自身声音暂不混入（KAG3 的 playvideo 通常配 playbgm 使用）。

use ffmpeg_sidecar::child::FfmpegChild;
use std::io::Read;

use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::WindowContext;

/// 在播状态（供 wv 轮询）
pub struct VideoPlayer {
    child: Option<FfmpegChild>,
    pipe: Option<std::process::ChildStdout>,
    w: usize,
    h: usize,
    buf: Vec<u8>,
    /// 帧间隔（由 ffmpeg 输出帧率决定，粗略 33ms）
    frame_left_ms: f32,
    /// 自动隐层延时（playvideo ms 参数；>0 时引擎在此毫秒后隐藏消息层）
    pub hide_msg_ms: u32,
    /// 可点击跳过（playvideo skip）
    pub skip: bool,
}

impl Default for VideoPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoPlayer {
    pub fn new() -> Self {
        Self {
            child: None,
            pipe: None,
            w: 0,
            h: 0,
            buf: Vec::new(),
            frame_left_ms: 0.0,
            hide_msg_ms: 0,
            skip: false,
        }
    }

    /// 是否在播（wv 轮询；子进程退出即否）
    pub fn playing(&self) -> bool {
        self.pipe.is_some()
    }

    /// 暂停：直接关闭管道（简单实现：暂停=定格当前帧，恢复=重新开播）
    pub fn pause(&mut self) {
        self.close_pipe();
    }

    /// 开播：spawn ffmpeg 子进程解码为 rgb24 rawvideo 管道。
    /// 失败（无 ffmpeg 等）返回 false，调用方 WARN 留证。
    pub fn play(&mut self, path: &str, w: u32, h: u32) -> bool {
        self.stop();
        let child = ffmpeg_sidecar::command::FfmpegCommand::new()
            .input(path)
            .args([
                "-vf",
                &format!("scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(iw-ow)/2:(ih-oh)/2"),
                "-r",
                "30",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-",
            ])
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                gal_audit_warn(&format!("playvideo：ffmpeg 子进程启动失败（{e}），视频跳过"));
                return false;
            }
        };
        let pipe = match child.take_stdout() {
            Some(p) => p,
            None => return false,
        };
        self.child = Some(child);
        self.pipe = Some(pipe);
        self.w = w as usize;
        self.h = h as usize;
        self.buf = vec![0u8; self.w * self.h * 3];
        true
    }

    fn close_pipe(&mut self) {
        self.pipe = None;
        if let Some(mut c) = self.child.take() {
            let _ = c.as_inner_mut().kill();
            let _ = c.as_inner_mut().wait();
        }
    }

    /// 停止并回收子进程
    pub fn stop(&mut self) {
        self.close_pipe();
        self.buf.clear();
    }

    /// 每帧驱动：到帧间隔时读下一帧；子进程结束即自然停。
    /// 返回是否刷出了新帧（true 时调用方取 `frame_rgba` 上传纹理）。
    pub fn tick(&mut self, dt_ms: f32) -> bool {
        if !self.playing() {
            return false;
        }
        self.frame_left_ms -= dt_ms;
        if self.frame_left_ms > 0.0 {
            return false;
        }
        self.frame_left_ms = 33.0; // ~30fps
        let Some(pipe) = &mut self.pipe else { return false };
        match pipe.read_exact(&mut self.buf) {
            Ok(()) => true,
            Err(_) => {
                // EOF / 管道断 = 播放自然结束
                self.close_pipe();
                false
            }
        }
    }

    /// 当前帧 RGB 转 RGBA（供 create_texture_static 上传；复用内部缓冲，需立即消费）
    pub fn frame_rgba(&mut self) -> Option<(&[u8], usize, usize)> {
        if self.buf.is_empty() {
            return None;
        }
        // 就地扩成 RGBA：倒序展开避免覆盖
        let px = self.w * self.h;
        self.buf.resize(px * 4, 255);
        for i in (0..px).rev() {
            let s = i * 3;
            let d = i * 4;
            self.buf.copy_within(s..s + 3, d);
            // RGB → ABGR8888 布局：SDL 小端下 ABGR8888 字节序为 R,G,B,A
            self.buf[d + 3] = 255;
        }
        Some((&self.buf, self.w, self.h))
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn gal_audit_warn(msg: &str) {
    // 复用 gal-audit 的日志通道（gal-render 不依赖 gal-audio，落 stderr + tracing 由调用方补）
    eprintln!("[video] WARN | {msg}");
}

/// 上传一帧 RGBA 到纹理并等比铺满绘制（引擎层每帧调用）。
/// 纹理每次现建（~30fps 下创建开销可忽略），随作用域释放 —— 全程无 unsafe。
pub fn blit_frame<T: sdl2::render::RenderTarget>(
    data: &[u8],
    w: usize,
    h: usize,
    creator: &TextureCreator<WindowContext>,
    target: &mut Canvas<T>,
) -> Result<(), String> {
    let mut tex = creator
        .create_texture_static(PixelFormatEnum::ABGR8888, w as u32, h as u32)
        .map_err(|e| format!("视频纹理创建失败：{e}"))?;
    tex.update(None, data, w * 4).map_err(|e| format!("视频帧上传失败：{e}"))?;
    let (vw, vh) = target.viewport().size();
    // 等比铺满目标面
    let scale = (vw as f32 / w as f32).max(vh as f32 / h as f32);
    let dw = (w as f32 * scale) as u32;
    let dh = (h as f32 * scale) as u32;
    target
        .copy(
            &tex,
            None,
            sdl2::rect::Rect::new(
                ((vw as i32 - dw as i32) / 2).max(0),
                ((vh as i32 - dh as i32) / 2).max(0),
                dw.min(vw),
                dh.min(vh),
            ),
        )
        .map_err(|e| format!("视频绘制失败：{e}"))
}
