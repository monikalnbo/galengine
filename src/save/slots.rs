//! 存档槽：9 槽 JSON + 缩略图 + 快速存读档 + 删档备份 + 剧本指纹校验。

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::script::vars::Value;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveData {
    pub file: String,
    /// 恢复点：当前台词行的解析行号（重跑=重打当前句）
    pub pc: usize,
    pub f_vars: HashMap<String, Value>,
    /// 演出层快照（背景/立绘/CG 路径）
    pub stage: StageSnap,
    /// 缩略图相对路径（thumbs/slot1.png）
    pub thumb: String,
    /// 保存时间（本地格式化）
    pub saved_at: String,
    /// 剧本内容指纹（改版检测：不一致=另一个夏天的记录）
    pub digest: u64,
    pub chapter: String,
}

/// 演出层快照（路径）
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StageSnap {
    pub bg: Option<String>,
    pub chars: [Option<String>; 3],
    pub cg: Option<String>,
}

pub struct SlotStore {
    pub dir: String,
    pub slots: usize,
    /// list 缓存（任何 save/delete 失效；避免每帧 9 次文件 I/O）
    list_cache: Option<(u64, Vec<Option<SaveData>>)>,
    dirty_serial: u64,
}

impl SlotStore {
    pub fn new(dir: &str, slots: usize) -> Self {
        Self {
            dir: dir.to_string(),
            slots,
            list_cache: None,
            dirty_serial: 0,
        }
    }

    fn bump(&mut self) {
        self.dirty_serial += 1;
    }

    pub fn slot_path(&self, n: usize) -> PathBuf {
        PathBuf::from(&self.dir).join(format!("save{n}.json"))
    }

    fn thumb_path(&self, name: &str) -> PathBuf {
        PathBuf::from(&self.dir).join(name)
    }

    /// 写存档：JSON + 缩略图 PNG
    pub fn save(&mut self, n: usize, data: &SaveData, thumb_png: &[u8]) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("创建存档目录失败（{e}）"))?;
        if !thumb_png.is_empty() {
            let tp = self.thumb_path(&data.thumb);
            std::fs::write(&tp, thumb_png).map_err(|e| format!("缩略图写入失败（{e}）"))?;
        }
        let json =
            serde_json::to_string_pretty(data).map_err(|e| format!("存档序列化失败（{e}）"))?;
        std::fs::write(self.slot_path(n), json).map_err(|e| format!("存档写入失败（{e}）"))?;
        self.bump();
        Ok(())
    }

    pub fn load(&self, n: usize) -> Result<SaveData, String> {
        let p = self.slot_path(n);
        let text = std::fs::read_to_string(&p)
            .map_err(|_| format!("存档不存在：{}", p.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("存档损坏，无法读取（{e}）"))
    }

    /// 全槽位快照（缓存；写入/删除后自动失效）
    pub fn list(&mut self) -> Vec<Option<SaveData>> {
        if let Some((serial, cached)) = &self.list_cache {
            if *serial == self.dirty_serial {
                return cached.clone();
            }
        }
        let fresh: Vec<Option<SaveData>> = (1..=self.slots).map(|n| self.load(n).ok()).collect();
        self.list_cache = Some((self.dirty_serial, fresh.clone()));
        fresh
    }

    /// 删除（删档仪式用；删前自动备份）
    pub fn delete(&mut self, n: usize) -> Result<PathBuf, String> {
        let p = self.slot_path(n);
        if !p.exists() {
            return Err("这一格已经是空的了".into());
        }
        // 备份（对玩家隐藏）
        let backup_dir = PathBuf::from(&self.dir).join("backup");
        std::fs::create_dir_all(&backup_dir).map_err(|e| format!("备份目录创建失败（{e}）"))?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let bak = backup_dir.join(format!("save{n}_{stamp}.json"));
        std::fs::copy(&p, &bak).map_err(|e| format!("备份失败（{e}）"))?;
        std::fs::remove_file(&p).map_err(|e| format!("删除失败（{e}）"))?;
        self.bump();
        Ok(bak)
    }

    /// 最后一个非空槽位（meta_delete_last 目标）
    pub fn last_used(&self) -> Option<usize> {
        (1..=self.slots).rev().find(|&n| self.slot_path(n).exists())
    }
}

/// FNV-1a 内容指纹（剧本改版检测）
pub fn digest_of(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// 当前时间戳（存档显示用）
pub fn now_str() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    // 无 chrono：UTC 秒转简易日期（galgame 存档显示足够）
    let secs = d.as_secs();
    let days = secs / 86400 + 719_162; // 1970-01-01 起的天数偏移到儒略日
    let (y, m, dd) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{dd:02} {:02}:{:02}", (secs / 3600) % 24, (secs / 60) % 60)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
