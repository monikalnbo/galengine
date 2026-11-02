//! 无障碍朗读（TTS）：**不引入任何新依赖**，走系统朗读命令。
//!
//! - Windows：PowerShell `System.Speech.Synthesis.SpeechSynthesizer`（系统自带，离线）
//! - macOS：`say`
//! - Linux：`espeak-ng` / `espeak` / `spd-say`（装了哪个用哪个）
//!
//! 找不到任何后端时返回 false，调用方把设置项灰掉——**优雅降级，不影响游戏**。
//! 每次朗读启动一个短命子进程，句与句之间不需要排队；停止用进程句柄 kill。

use std::process::{Child, Command, Stdio};

/// 后端枚举（设置面板显示/报告用）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    WindowsSapi,
    MacSay,
    EspeakNg,
    Espeak,
    SpdSay,
}

impl Backend {
    pub fn command(self) -> &'static str {
        match self {
            Backend::WindowsSapi => "powershell",
            Backend::MacSay => "say",
            Backend::EspeakNg => "espeak-ng",
            Backend::Espeak => "espeak",
            Backend::SpdSay => "spd-say",
        }
    }
}

/// 探测第一个可用后端
pub fn detect() -> Option<Backend> {
    if cfg!(windows) {
        return Some(Backend::WindowsSapi); // 系统自带，不做探测（调用失败会自然静默）
    }
    let list: &[Backend] = if cfg!(target_os = "macos") {
        &[Backend::MacSay, Backend::EspeakNg, Backend::Espeak]
    } else {
        &[Backend::EspeakNg, Backend::Espeak, Backend::SpdSay]
    };
    list.iter().copied().find(|b| which(b.command()).is_some())
}

fn which(cmd: &str) -> Option<()> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(cmd)).find(|p| p.is_file()).map(|_| ()).or_else(
        || {
            // Windows 可执行带扩展名
            std::env::split_paths(&path)
                .map(|d| d.join(format!("{cmd}.exe")))
                .find(|p| p.is_file())
                .map(|_| ())
        },
    )
}

/// 启动朗读；`rate` 为相对语速（-10..10，0 = 默认）
pub fn speak(backend: Backend, text: &str, rate: i32) -> Option<Child> {
    if text.trim().is_empty() {
        return None;
    }
    let mut c = match backend {
        Backend::WindowsSapi => {
            // 单引号转义：PowerShell 里 '' = '
            let safe = text.replace('\'', "''");
            let mut c = Command::new("powershell");
            c.args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!(
                    "Add-Type -AssemblyName System.Speech; \
                     $s=New-Object System.Speech.Synthesis.SpeechSynthesizer; \
                     $s.Rate={rate}; $s.Speak('{safe}');"
                ),
            ]);
            c
        }
        Backend::MacSay => {
            let mut c = Command::new("say");
            c.args(["-r", &(180 + rate * 15).to_string(), text]);
            c
        }
        Backend::EspeakNg | Backend::Espeak => {
            let mut c = Command::new(backend.command());
            // espeak 语速词/分：默认 175，rate -10..10 → 80..300
            c.args(["-s", &(175 + rate * 12).to_string(), text]);
            c
        }
        Backend::SpdSay => {
            let mut c = Command::new("spd-say");
            c.args(["-r", &(rate * 10).to_string(), text]);
            c
        }
    };
    c.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok()
}

/// 停止朗读（kill 子进程；已结束返回 false）
pub fn stop(child: &mut Option<Child>) -> bool {
    match child.take() {
        Some(mut c) => {
            let alive = c.try_wait().ok().flatten().is_none();
            let _ = c.kill();
            let _ = c.wait();
            alive
        }
        None => false,
    }
}

/// 子进程是否还在读（自然结束则清理句柄）
pub fn alive(child: &mut Option<Child>) -> bool {
    let Some(c) = child.as_mut() else { return false };
    match c.try_wait() {
        Ok(Some(_)) => {
            *child = None;
            false
        }
        Ok(None) => true,
        Err(_) => false,
    }
}
