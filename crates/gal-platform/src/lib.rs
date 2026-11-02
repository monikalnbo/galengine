//! 平台层：关机/桌面文件/URL打开/无障碍朗读。Windows 真执行；验收环境仅写日志。

pub mod tts;

use std::path::PathBuf;
use std::process::Command;

// ============================================================
// 关机
// ============================================================

pub fn arm_shutdown(secs: u32) {
    if cfg!(windows) {
        let _ = Command::new("shutdown").args(["/s", "/t", &secs.to_string()]).spawn();
    } else {
        eprintln!("[platform] shutdown /s /t {secs}（验收环境仅日志）");
    }
}

pub fn cancel_shutdown() {
    if cfg!(windows) {
        let _ = Command::new("shutdown").arg("/a").spawn();
    } else {
        eprintln!("[platform] shutdown /a（验收环境仅日志）");
    }
}

// ============================================================
// 桌面文件
// ============================================================

/// 文件名防路径穿越：只取 basename，拒绝 .. 与空
fn sanitize(file: &str) -> Result<String, String> {
    let name = file.rsplit(['/', '\\']).next().unwrap_or("");
    if name.is_empty() || name.contains("..") {
        return Err(format!("文件名不合法：{file}"));
    }
    Ok(name.to_string())
}

fn desktop_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(home) = std::env::var_os("USERPROFILE") {
            return PathBuf::from(home).join("Desktop");
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let d = PathBuf::from(home).join("Desktop");
        if d.is_dir() {
            return d;
        }
    }
    PathBuf::from(".")
}

pub fn desktop_write(file: &str, content: &str) -> Result<(), String> {
    let name = sanitize(file)?;
    let path = desktop_dir().join(&name);
    std::fs::write(&path, content)
        .map_err(|e| format!("桌面文件写入失败：{}（{e}）", path.display()))?;
    eprintln!("[platform] desktop_write -> {}", path.display());
    Ok(())
}

pub fn desktop_open(file: &str) -> Result<(), String> {
    let name = sanitize(file)?;
    let path = desktop_dir().join(&name);
    if !path.exists() {
        return Err(format!("桌面文件不存在：{}", path.display()));
    }
    let r = if cfg!(windows) {
        Command::new("cmd").args(["/c", "start", "", &name]).current_dir(desktop_dir()).spawn()
    } else {
        Command::new("xdg-open").arg(&path).spawn()
    };
    match r {
        Ok(_) => {
            eprintln!("[platform] desktop_open -> {}", path.display());
            Ok(())
        }
        Err(e) => Err(format!("打开桌面文件失败：{e}")),
    }
}

// ============================================================
// URL 打开（打赏按钮用）
// ============================================================

pub fn desktop_open_url(url: &str) -> Result<(), String> {
    if cfg!(windows) {
        // Windows：使用 rundll32 调用系统协议处理器，不走 cmd.exe，防范 & 符号命令拼接注入与参数截断
        Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .map_err(|e| format!("打开URL失败：{e}"))?;
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn().map_err(|e| format!("打开URL失败：{e}"))?;
    } else {
        Command::new("xdg-open").arg(url).spawn().map_err(|e| format!("打开URL失败：{e}"))?;
    }
    Ok(())
}

// ============================================================
// 系统字体（运行时加载任意字体；不新增依赖，只做目录枚举）
// ============================================================

/// 本机常见字体目录（存在的才返回）
pub fn font_dirs() -> Vec<String> {
    let mut dirs: Vec<String> = Vec::new();
    if cfg!(windows) {
        if let Some(windir) = std::env::var_os("WINDIR") {
            dirs.push(format!("{}/Fonts", PathBuf::from(windir).display()));
        }
        dirs.push("C:/Windows/Fonts".into());
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(format!("{}/Microsoft/Windows/Fonts", PathBuf::from(local).display()));
        }
    } else if cfg!(target_os = "macos") {
        dirs.push("/System/Library/Fonts".into());
        dirs.push("/Library/Fonts".into());
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(format!("{}/Library/Fonts", PathBuf::from(home).display()));
        }
    } else {
        dirs.push("/usr/share/fonts".into());
        dirs.push("/usr/local/share/fonts".into());
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(format!("{}/.local/share/fonts", PathBuf::from(home).display()));
        }
        dirs.push(format!("{}/.fonts", std::env::var("HOME").unwrap_or_default()));
    }
    dirs.retain(|d| std::path::Path::new(d).is_dir());
    dirs.dedup();
    dirs
}

/// 目录（递归一层子目录）下的字体文件，按路径排序
pub fn list_font_files(dir: &str) -> Vec<String> {
    fn one(dir: &std::path::Path, out: &mut Vec<String>, depth: u8) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if depth > 0 {
                    one(&p, out, depth - 1);
                }
            } else if matches!(
                p.extension().and_then(|s| s.to_str()).map(|s| s.to_ascii_lowercase()).as_deref(),
                Some("ttf" | "otf" | "ttc")
            ) {
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
    let mut out = Vec::new();
    one(std::path::Path::new(dir), &mut out, 1);
    out.sort();
    out
}
