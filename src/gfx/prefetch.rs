//! 资源缓存层：后台线程预解码 + 主线程纹理上传，消除场景切换卡帧。
//!
//! 数据流：剧本等待点 → 预读扫描（lookahead）→ request(path)
//!   → 后台线程 image 解码为 RGBA8888 → staging
//!   → 主线程 draw 时 take → SDL 纹理上传（毫秒级）
//! 兜底：staging 未命中时同步解码（保证正确性，仅慢一次）。

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

/// 解码完成的像素数据（RGBA8888，行距 = w*4）
pub struct Decoded {
    pub w: u32,
    pub h: u32,
    pub pixels: Vec<u8>,
}

enum Msg {
    Req(String),
    Done(String, Result<Arc<Decoded>, String>),
}

pub struct Prefetcher {
    tx_req: Sender<String>,
    rx_done: Receiver<Msg>,
    /// 已解码待上传（主线程独占）
    pub staging: HashMap<String, Arc<Decoded>>,
    /// 在途请求去重
    inflight: HashSet<String>,
    pub stats_done: u64,
    pub stats_sync_fallback: u64,
}

impl Prefetcher {
    pub fn new() -> Self {
        let (tx_req, rx_req) = mpsc::channel::<String>();
        let (tx_done, rx_done) = mpsc::channel::<Msg>();
        // 后台解码线程（纯 image crate，不触碰 SDL）
        std::thread::spawn(move || {
            while let Ok(path) = rx_req.recv() {
                let result = decode(&path).map(Arc::new);
                let _ = tx_done.send(Msg::Done(path, result));
            }
        });
        Self {
            tx_req,
            rx_done,
            staging: HashMap::new(),
            inflight: HashSet::new(),
            stats_done: 0,
            stats_sync_fallback: 0,
        }
    }

    /// 请求预解码（自动去重：已在途/staging 的跳过）
    pub fn request(&mut self, path: &str) {
        if self.inflight.contains(path) {
            return;
        }
        self.inflight.insert(path.to_string());
        let _ = self.tx_req.send(path.to_string());
    }

    /// 每帧调用：收取解码完成项
    pub fn pump(&mut self) {
        while let Ok(msg) = self.rx_done.try_recv() {
            if let Msg::Done(path, res) = msg {
                self.inflight.remove(&path);
                match res {
                    Ok(dec) => {
                        self.staging.insert(path, dec);
                        self.stats_done += 1;
                    }
                    Err(_) => {
                        // 解码失败：交给同步兜底路径报中文错误
                    }
                }
            }
        }
    }

    /// 取走解码数据（主线程建纹理后调用；未命中返回 None）
    pub fn take(&mut self, path: &str) -> Option<Arc<Decoded>> {
        self.staging.remove(path)
    }

    /// 场景资源回收（与 TextureBank.retain 同步）
    pub fn retain(&mut self, keep: &HashSet<String>) {
        self.staging.retain(|k, _| keep.contains(k));
    }
}

impl Default for Prefetcher {
    fn default() -> Self {
        Self::new()
    }
}

fn decode(path: &str) -> Result<Decoded, String> {
    let img = image::open(path).map_err(|e| format!("{e}"))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(Decoded {
        w,
        h,
        pixels: rgba.into_raw(),
    })
}
