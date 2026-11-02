//! gal-audit —— 引擎多路日志：分级文件日志 + trace 追踪 + stderr 镜像。
//!
//! 方针（v0.3.2 定档）：运行时失败对玩家一律静默降级（不崩、不弹窗、不中断演出），
//! 但必须留下完整证据链——日志记录 + trace 追踪记录 + 多路日志，供作者事后排错。
//!
//! - 分级：TRACE < DEBUG < INFO < WARN < ERROR（ES_LOG_LEVEL=trace|debug|info|warn|error 过滤文件日志）
//! - 引擎日志：`{数据目录}/log/engine.log`（全部级别，追加写）
//! - trace 追踪：`{数据目录}/log/trace.log`（只收 `trace!` 事件：每条音频动作的 tag=k=v 记录）
//! - stderr：默认只镜像 WARN+（保持 v0.3.x 终端行为）；`ES_LOG_STDERR=1` 全量镜像
//! - 无头/无声卡环境照常工作：日志只落文件，不依赖音频设备或交互
//! - 日志自身失败（磁盘满/只读）自动降级为仅 stderr，绝不影响游戏演出

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// 日志级别（数值越小越详细）
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Level {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl Level {
    fn tag(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO ",
            Self::Warn => "WARN ",
            Self::Error => "ERROR",
        }
    }
    fn from_env() -> Level {
        match std::env::var("ES_LOG_LEVEL").as_deref() {
            Ok("trace") => Self::Trace,
            Ok("info") => Self::Info,
            Ok("warn") => Self::Warn,
            Ok("error") => Self::Error,
            _ => Self::Debug,
        }
    }
}

struct Sink {
    /// 引擎日志（全级别）
    file: Option<File>,
    /// trace 追踪日志（tag 事件）
    trace: Option<File>,
    /// 文件日志过滤级别
    min: Level,
    /// stderr 全量镜像（默认只 WARN+）
    stderr_all: bool,
    /// 写失败只提醒一次（不刷屏）
    warned: bool,
    /// 进程启动时刻（trace 相对时间戳）
    t0: Instant,
}

static SINK: OnceLock<Mutex<Sink>> = OnceLock::new();

fn sink() -> &'static Mutex<Sink> {
    SINK.get_or_init(|| {
        Mutex::new(Sink {
            file: None,
            trace: None,
            min: Level::from_env(),
            stderr_all: std::env::var("ES_LOG_STDERR").is_ok(),
            warned: false,
            t0: Instant::now(),
        })
    })
}

/// 初始化日志（幂等；在拿到数据目录后尽早调用）。
/// 目录/文件创建失败 = 仅 stderr 模式（记一次 WARN，不重复提醒）。
pub fn init(dir: &str) {
    let log_dir = format!("{dir}/log");
    let r = std::fs::create_dir_all(&log_dir).and_then(|()| {
        Ok((
            OpenOptions::new().create(true).append(true).open(format!("{log_dir}/engine.log"))?,
            OpenOptions::new().create(true).append(true).open(format!("{log_dir}/trace.log"))?,
        ))
    });
    let mut s = sink().lock().unwrap_or_else(|e| e.into_inner());
    match r {
        Ok((f, t)) => {
            let first = s.file.is_none();
            s.file = Some(f);
            s.trace = Some(t);
            drop(s); // log() 内部会再锁 sink：先放锁防自锁
            if first {
                log(Level::Info, "log", &format!("日志系统就绪：{log_dir}/engine.log + trace.log"));
            }
        }
        Err(e) => {
            if !s.warned {
                s.warned = true;
                let _ = writeln!(
                    std::io::stderr(),
                    "[gal-audit] 日志目录不可用（{e}），降级为仅终端输出"
                );
            }
        }
    }
}

fn epoch_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// 写一条日志（内部 API；宏入口）
pub fn log(level: Level, subsys: &str, msg: &str) {
    let mut s = sink().lock().unwrap_or_else(|e| e.into_inner());
    if level >= s.min {
        let line = format!("[{:.3}] {} {} | {msg}", epoch_secs(), level.tag(), subsys);
        if let Some(f) = &mut s.file {
            if f.write_all(line.as_bytes()).and_then(|()| f.write_all(b"\n")).is_err() && !s.warned
            {
                s.warned = true;
                let _ = writeln!(std::io::stderr(), "[gal-audit] 日志写入失败，降级为仅终端输出");
            }
        }
    }
    // stderr：WARN+ 恒镜像（兼容 v0.3.x 终端排错习惯）；ES_LOG_STDERR=1 全量
    if level >= Level::Warn || s.stderr_all {
        let _ = writeln!(std::io::stderr(), "[{}] {} | {}", subsys, level.tag(), msg);
    }
}

/// 写一条 trace 追踪事件（只进 trace.log；引擎动作的完整证据链）
pub fn trace(subsys: &str, tag: &str, fields: &[(&str, String)]) {
    let mut s = sink().lock().unwrap_or_else(|e| e.into_inner());
    let mut line = format!("[{:>9.3}s] {} {}", s.t0.elapsed().as_secs_f64(), subsys, tag);
    for (k, v) in fields {
        line.push_str(&format!(" {k}={v}"));
    }
    if let Some(f) = &mut s.trace {
        let _ = f.write_all(line.as_bytes());
        let _ = f.write_all(b"\n");
    }
}

/// 日志宏：`audit!(Warn, "bgm", "加载失败：{path}（{e}）")`
#[macro_export]
macro_rules! audit {
    ($lvl:ident, $subs:expr, $($arg:tt)*) => {
        $crate::log::log($crate::log::Level::$lvl, $subs, &format!($($arg)*))
    };
}

/// trace 追踪宏：`audit_trace!("bgm", "playbgm", "name" => name, "fade_ms" => fade_ms)`
#[macro_export]
macro_rules! audit_trace {
    ($subs:expr, $tag:expr $(, $k:expr => $v:expr)* $(,)?) => {
        $crate::log::trace($subs, $tag, &[ $( ($k, $v.to_string()) ),* ])
    };
}

/// trace 追踪宏（无字段）：`audit_trace0!("bgm", "stop")`
#[macro_export]
macro_rules! audit_trace0 {
    ($subs:expr, $tag:expr) => {
        $crate::log::trace($subs, $tag, &[])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 初始化与写日志_无目录权限也不崩() {
        // 不存在的目录：init 失败 → 仅 stderr 模式，log/trace 均不得 panic
        init("/proc/definitely/not/writable/galengine");
        log(Level::Warn, "test", "静默降级验证");
        trace("test", "probe", &[("k", "v".into())]);
    }

    #[test]
    fn 正常目录_两级日志文件都生成() {
        let dir = std::env::temp_dir().join(format!("gal-audit-test-{}", std::process::id()));
        init(dir.to_str().unwrap());
        log(Level::Info, "test", "写入引擎日志");
        trace("test", "probe", &[("name", "theme".into()), ("ms", "800".into())]);
        assert!(std::path::Path::new(&dir).join("log/engine.log").exists());
        assert!(std::path::Path::new(&dir).join("log/trace.log").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
