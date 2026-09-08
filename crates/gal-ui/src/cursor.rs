//! 自定义鼠标：conf.ui.cursor 配置 PNG 光标（default/click 两态）。
//! 未配置时零行为（系统光标）；路径相对数据目录。

use sdl2::mouse::Cursor;
use sdl2::surface::Surface;

use gal_config::Config;

pub struct CursorMgr {
    normal: Option<Cursor>,
    click: Option<Cursor>,
}

fn load(path: &str, hotspot: (i32, i32)) -> Option<Cursor> {
    use sdl2::image::LoadSurface;
    let surf: Surface = match Surface::from_file(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[cursor] 光标图加载失败：{path}（{e}）");
            return None;
        }
    };
    match Cursor::from_surface(surf, hotspot.0, hotspot.1) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("[cursor] 光标创建失败：{path}（{e:?}）");
            None
        }
    }
}

fn resolve(path: &str) -> String {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        path.into()
    } else {
        format!("{}/{path}", gal_config::data_dir())
    }
}

impl CursorMgr {
    /// 系统默认光标（未配置时）
    pub fn system() -> Self {
        Self { normal: None, click: None }
    }

    /// 按配置构建；未配置或加载失败 = 系统光标
    pub fn new(conf: &Config) -> Self {
        let c = &conf.ui.cursor;
        let hs = (c.hotspot[0], c.hotspot[1]);
        Self {
            normal: c.default.as_ref().and_then(|p| load(&resolve(p), hs)),
            click: c.click.as_ref().and_then(|p| load(&resolve(p), hs)),
        }
    }

    pub fn set_default(&self) {
        if let Some(c) = &self.normal {
            c.set();
        }
    }

    pub fn set_click(&self) {
        if let Some(c) = &self.click {
            c.set();
        } else {
            self.set_default();
        }
    }
}
