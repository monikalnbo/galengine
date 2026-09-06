//! 变量系统：f.* 周目变量（随存档）+ sf.* 全局变量（独立文件持久化，退出即写盘）。
//! A3：存储与读写；A5：表达式求值；A6：f.* 并入存档。

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
}

impl Value {
    pub fn as_i64(&self) -> i64 {
        match self {
            Value::Int(i) => *i,
            Value::Float(f) => *f as i64,
            Value::Str(s) => s.parse().unwrap_or(0),
        }
    }
    pub fn as_str(&self) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Str(s) => s.clone(),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct Vars {
    /// 周目变量（f.*），随存档
    pub f: HashMap<String, Value>,
    /// 全局变量（sf.*），savedata/global.json 持久化
    pub sf: HashMap<String, Value>,
}

impl Vars {
    pub fn new() -> Self {
        Self::default()
    }

    /// 读取全局存档；不存在返回全新实例（首次启动）
    pub fn load_global(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::new(),
        }
    }

    /// 退出即写盘（旧引擎 sf 不持久化的死穴，docs/20 §3.2）
    pub fn save_global(&self, path: &str) -> Result<(), String> {
        if let Some(dir) = Path::new(path).parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("创建存档目录失败：{path}（{e}）"))?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| format!("全局变量序列化失败：{e}"))?;
        std::fs::write(path, text).map_err(|e| format!("全局变量写盘失败：{path}（{e}）"))
    }

    /// 读：名字形如 "f.aff_mio" / "sf.kakusei" / 裸名（默认 f.）
    pub fn get(&self, name: &str) -> Option<&Value> {
        let (ns, key) = split_ns(name);
        match ns {
            "sf" => self.sf.get(key),
            _ => self.f.get(key),
        }
    }

    pub fn set(&mut self, name: &str, v: Value) {
        let (ns, key) = split_ns(name);
        match ns {
            "sf" => {
                self.sf.insert(key.to_string(), v);
            }
            _ => {
                self.f.insert(key.to_string(), v);
            }
        }
    }

    pub fn get_or(&self, name: &str) -> Value {
        self.get(name).cloned().unwrap_or(Value::Int(0))
    }
}

fn split_ns(name: &str) -> (&str, &str) {
    match name.split_once('.') {
        Some(("sf", k)) => ("sf", k),
        Some(("f", k)) => ("f", k),
        _ => ("f", name),
    }
}
