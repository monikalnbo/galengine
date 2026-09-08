//! 变量系统：f.* 随存档；sf.* 存 savedata/global.json，退出即写盘。

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
    pub fn as_f64(&self) -> f64 {
        match self {
            Value::Int(i) => *i as f64,
            Value::Float(f) => *f,
            Value::Str(s) => s.parse().unwrap_or(0.0),
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

/// global.json 落盘格式（只存 sf.*；f.* 随存档走）
#[derive(Serialize, Deserialize)]
struct GlobalFile {
    sf: HashMap<String, Value>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Vars {
    pub f: HashMap<String, Value>,
    pub sf: HashMap<String, Value>,
}

impl Vars {
    pub fn get(&self, name: &str) -> Option<&Value> {
        match name.split_once('.') {
            Some(("sf", k)) => self.sf.get(k),
            _ => self.f.get(name.strip_prefix("f.").unwrap_or(name)),
        }
    }

    pub fn set(&mut self, name: &str, v: Value) {
        match name.split_once('.') {
            Some(("sf", k)) => {
                self.sf.insert(k.to_string(), v);
            }
            _ => {
                self.f.insert(name.strip_prefix("f.").unwrap_or(name).to_string(), v);
            }
        }
    }

    pub fn get_or(&self, name: &str) -> Value {
        self.get(name).cloned().unwrap_or(Value::Int(0))
    }

    pub fn load_global(path: &str) -> Vars {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str::<GlobalFile>(&text)
                .map(|g| Vars { f: HashMap::new(), sf: g.sf })
                .unwrap_or_default(),
            Err(_) => Vars::default(),
        }
    }

    pub fn save_global(&self, path: &str) -> Result<(), String> {
        if let Some(dir) = Path::new(path).parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("创建存档目录失败：{path}（{e}）"))?;
        }
        // 只持久化 sf.*；f.* 随存档走
        let file = GlobalFile { sf: self.sf.clone() };
        let text =
            serde_json::to_string_pretty(&file).map_err(|e| format!("全局变量序列化失败：{e}"))?;
        std::fs::write(path, text).map_err(|e| format!("全局变量写盘失败：{path}（{e}）"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 命名空间路由() {
        let mut v = Vars::default();
        v.set("f.a", Value::Int(1));
        v.set("sf.b", Value::Str("x".into()));
        v.set("c", Value::Int(3)); // 裸名默认 f
        assert_eq!(v.get("f.a"), Some(&Value::Int(1)));
        assert_eq!(v.get("a"), Some(&Value::Int(1)));
        assert_eq!(v.get("sf.b"), Some(&Value::Str("x".into())));
        assert_eq!(v.get("c"), Some(&Value::Int(3)));
        assert_eq!(v.get_or("nope"), Value::Int(0));
    }

    #[test]
    fn global持久化往返() {
        let dir = std::env::temp_dir().join("galengine_test_vars");
        let path = dir.join("global.json");
        let mut v = Vars::default();
        v.set("sf.playerName", Value::Str("测试者".into()));
        v.set("f.heroName", Value::Str("不该持久化".into()));
        v.save_global(path.to_str().unwrap()).unwrap();
        let loaded = Vars::load_global(path.to_str().unwrap());
        assert_eq!(loaded.sf.get("playerName"), Some(&Value::Str("测试者".into())));
        assert!(loaded.f.is_empty());
    }
}
