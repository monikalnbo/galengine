//! 后台预解码：请求 → 解码线程（image crate，不碰 SDL）→ staging → 主线程纹理上传。
//! 兜底：staging 未命中走同步解码（保证正确性，仅慢一次）。

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

pub struct Decoded {
    pub w: u32,
    pub h: u32,
    pub pixels: Vec<u8>,
}

enum Msg {
    Done(String, Result<Arc<Decoded>, String>),
}

pub struct Prefetcher {
    tx_req: Sender<String>,
    rx_done: Receiver<Msg>,
    staging: HashMap<String, Arc<Decoded>>,
    inflight: HashSet<String>,
    pub stats_done: u64,
}

impl Prefetcher {
    pub fn new() -> Self {
        let (tx_req, rx_req) = mpsc::channel::<String>();
        let (tx_done, rx_done) = mpsc::channel::<Msg>();
        std::thread::spawn(move || {
            while let Ok(path) = rx_req.recv() {
                let dec = decode(&path).map(Arc::new);
                let _ = tx_done.send(Msg::Done(path, dec));
            }
        });
        Self { tx_req, rx_done, staging: HashMap::new(), inflight: HashSet::new(), stats_done: 0 }
    }

    pub fn request(&mut self, path: &str) {
        if self.inflight.insert(path.to_string()) {
            let _ = self.tx_req.send(path.to_string());
        }
    }

    /// 每帧收取解码完成项
    pub fn pump(&mut self) {
        while let Ok(Msg::Done(path, res)) = self.rx_done.try_recv() {
            self.inflight.remove(&path);
            if let Ok(dec) = res {
                self.staging.insert(path, dec);
                self.stats_done += 1;
            }
        }
    }

    pub fn take(&mut self, path: &str) -> Option<Arc<Decoded>> {
        self.staging.remove(path)
    }

    pub fn retain(&mut self, keep: &HashSet<String>) {
        self.staging.retain(|k, _| keep.contains(k));
    }
}

impl Default for Prefetcher {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn decode(path: &str) -> Result<Decoded, String> {
    // 魔数嗅探：容忍扩展名与真实格式不符的素材（image::open 只看扩展名，
    // SDL2_image 时代按魔数嗅探——不能比旧引擎更严格）
    use image::ImageReader;
    let img = ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(Decoded { w, h, pixels: rgba.into_raw() })
}
