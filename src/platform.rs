//! 平台层：关机调度（晚安演出）。
//! Windows 真执行 shutdown；Linux/验收环境只记日志——绝不可能关掉开发机。

use std::process::Command;

pub fn schedule_shutdown(secs: u32, message: &str) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        Command::new("shutdown")
            .args(["/s", "/t", &secs.to_string(), "/c", message])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("设定关机失败（{e}）"))
    } else {
        eprintln!("[platform] 验收环境不执行关机（模拟：shutdown /s /t {secs}）");
        Ok(())
    }
}

pub fn cancel_shutdown() -> Result<(), String> {
    if cfg!(target_os = "windows") {
        Command::new("shutdown")
            .arg("/a")
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("取消关机失败（{e}）"))
    } else {
        eprintln!("[platform] 验收环境（模拟：shutdown /a）");
        Ok(())
    }
}

// —— 桌面能力（meta 演出：向玩家桌面写信/打开文件）——

/// 桌面目录：Windows（USERPROFILE\Desktop，兼容 OneDrive 重定向）/ Linux（~/Desktop）
pub fn desktop_dir() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    if cfg!(target_os = "windows") {
        let home = std::env::var("USERPROFILE").ok()?;
        let d = PathBuf::from(&home).join("Desktop");
        if d.is_dir() {
            return Some(d);
        }
        let one = PathBuf::from(&home).join("OneDrive").join("Desktop");
        if one.is_dir() {
            return Some(one);
        }
        None
    } else {
        let home = std::env::var("HOME").ok()?;
        let d = PathBuf::from(home).join("Desktop");
        if d.is_dir() {
            Some(d)
        } else {
            None // 无桌面概念（无头/验收环境）
        }
    }
}

/// 文件名安全检查：拒绝路径穿越/分隔符
fn safe_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 120
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
    {
        Err(format!("文件名不合法：{name}"))
    } else {
        Ok(())
    }
}

/// 在桌面创建/追加文本文件（UTF-8）；返回完整路径
pub fn desktop_write(name: &str, content: &str) -> Result<String, String> {
    safe_name(name)?;
    let dir = desktop_dir().ok_or("找不到桌面目录")?;
    let path = dir.join(name);
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("写入桌面文件失败（{e}）"))?;
    f.write_all(format!("{content}\n").as_bytes())
        .map_err(|e| format!("写入失败（{e}）"))?;
    Ok(path.to_string_lossy().into_owned())
}

/// 用系统默认程序打开文件
pub fn open_file(path: &str) -> Result<(), String> {
    if !std::path::Path::new(path).exists() {
        return Err(format!("文件不存在：{path}"));
    }
    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/c", "start", "", path])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("打开失败（{e}）"))
    } else {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("打开失败（{e}）——验收环境无 xdg-open 属正常"))
    }
}
