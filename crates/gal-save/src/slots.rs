//! 存档槽：save{N}.json（位置+f.*+演出层快照+剧本指纹+缩略图 PNG）。
//! 读档校验指纹，不匹配中文拒读；删档前自动备份 backup/。

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hasher};
use std::path::Path;

use serde::{Deserialize, Serialize};

use gal_script::stage::StageSnap;
use gal_script::vars::Value;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveEntry {
    pub slot: usize,
    /// 场景描述（最近台词截断）
    pub title: String,
    /// 时间戳 2026-09-06 12:34
    pub stamp: String,
    pub file: String,
    /// 恢复行（等待点所在行号）
    pub line: usize,
    pub fvars: HashMap<String, Value>,
    pub snap: StageSnap,
    /// 存档时刻的 BGM 名（读档恢复播放；旧档无此字段=None 不播）
    #[serde(default)]
    pub bgm: Option<String>,
    /// 剧本文件内容指纹（读档校验）
    pub fingerprint: u64,
    /// 缩略图 PNG（320×180）
    pub thumb: Vec<u8>,
}

pub fn slot_path(save_dir: &str, slot: usize) -> String {
    format!("{save_dir}/save{slot}.json")
}

/// 剧本内容指纹（同版本引擎内稳定；跨引擎版本可能变化，属已知简化）
pub fn fingerprint(path: &str) -> u64 {
    let mut h = DefaultHasher::new();
    if let Ok(bytes) = std::fs::read(path) {
        h.write(&bytes);
    }
    h.finish()
}

/// 写入槽位（原子替换：先写临时文件再改名，避免写一半损坏）
pub fn save(dir: &str, entry: &SaveEntry) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("创建存档目录失败：{dir}（{e}）"))?;
    let path = slot_path(dir, entry.slot);
    let tmp = format!("{path}.tmp");
    let text = serde_json::to_string(entry).map_err(|e| format!("存档序列化失败：{e}"))?;
    std::fs::write(&tmp, text).map_err(|e| format!("存档写入失败：{path}（{e}）"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("存档替换失败：{path}（{e}）"))?;
    Ok(())
}

pub fn load(dir: &str, slot: usize) -> Option<SaveEntry> {
    let text = std::fs::read_to_string(slot_path(dir, slot)).ok()?;
    serde_json::from_str(&text).ok()
}

/// 读档校验：指纹不符返回中文错误（规格 §2.4）
pub fn load_checked(dir: &str, slot: usize, data_dir: &str) -> Result<SaveEntry, String> {
    let e = load(dir, slot).ok_or(format!("存档槽 {slot} 是空的"))?;
    let script_path = format!("{data_dir}/scenario/{}", e.file);
    if !Path::new(&script_path).exists() {
        return Err(format!("存档槽 {slot} 对应的剧本已不存在：{}", e.file));
    }
    if fingerprint(&script_path) != e.fingerprint {
        return Err(format!("存档槽 {slot} 与当前剧本版本不一致，无法读取"));
    }
    Ok(e)
}

/// 最新槽位（mtime 最大者）
pub fn latest_slot(dir: &str, slots: usize) -> Option<usize> {
    let mut best: Option<(std::time::SystemTime, usize)> = None;
    for slot in 1..=slots {
        let p = slot_path(dir, slot);
        if let Ok(meta) = std::fs::metadata(&p) {
            if let Ok(t) = meta.modified() {
                if best.is_none_or(|(bt, _)| t > bt) {
                    best = Some((t, slot));
                }
            }
        }
    }
    best.map(|(_, s)| s)
}

/// 真删最新槽：删前备份到 backup/；返回被删槽号
pub fn delete_latest(dir: &str, slots: usize) -> Option<usize> {
    let slot = latest_slot(dir, slots)?;
    let path = slot_path(dir, slot);
    let backup_dir = format!("{dir}/backup");
    let _ = std::fs::create_dir_all(&backup_dir);
    let stamp = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
    let stamp = stamp.map(|t| unix_to_stamp(unix_secs(t))).unwrap_or_else(|| "unknown".into());
    let _ = std::fs::rename(&path, format!("{backup_dir}/save{slot}_{stamp}.json"));
    Some(slot)
}

fn unix_secs(t: std::time::SystemTime) -> i64 {
    t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub fn now_stamp() -> String {
    unix_to_stamp(unix_secs(std::time::SystemTime::now()))
}

/// unix 秒 → "YYYY-MM-DD HH:MM"（民用历算法，免 chrono 依赖）
fn unix_to_stamp(secs: i64) -> String {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (h, m) = (rem / 3600, rem % 3600 / 60);
    // days-from-civil 逆运算（Howard Hinnant 算法）
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mth = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mth <= 2 { y + 1 } else { y };
    format!("{y:04}-{mth:02}-{d:02} {h:02}:{m:02}")
}

/// 离屏 RGBA（1280×720）→ 320×180 PNG 缩略图
pub fn thumb_png(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    use image::{codecs::png::PngEncoder, imageops::FilterType, ImageEncoder, RgbaImage};
    if w == 0 || h == 0 || rgba.len() != (w * h * 4) as usize {
        return Vec::new();
    }
    let img = RgbaImage::from_raw(w, h, rgba.to_vec()).unwrap_or_default();
    let small = image::imageops::resize(&img, 320, 180, FilterType::Triangle);
    let mut out = Vec::new();
    if PngEncoder::new(&mut out)
        .write_image(small.as_raw(), 320, 180, image::ExtendedColorType::Rgba8)
        .is_ok()
    {
        out
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(slot: usize) -> SaveEntry {
        SaveEntry {
            slot,
            title: "夏夜测试".into(),
            stamp: "2026-09-06 12:00".into(),
            file: "a.ks".into(),
            line: 3,
            fvars: HashMap::from([("aff".into(), Value::Int(2))]),
            snap: StageSnap::default(),
            bgm: Some("theme_test".into()),
            fingerprint: 42,
            thumb: vec![1, 2, 3],
        }
    }

    fn tmpdir(tag: &str) -> String {
        let d = std::env::temp_dir().join(format!("galengine_slots_{tag}"));
        let _ = std::fs::remove_dir_all(&d);
        d.to_str().unwrap().to_string()
    }

    #[test]
    fn 槽位往返() {
        let dir = tmpdir("round");
        save(&dir, &entry(1)).unwrap();
        let e = load(&dir, 1).unwrap();
        assert_eq!(e.title, "夏夜测试");
        assert_eq!(e.fvars["aff"], Value::Int(2));
        assert_eq!(e.thumb, vec![1, 2, 3]);
        assert_eq!(e.bgm.as_deref(), Some("theme_test"));
    }

    #[test]
    fn 旧档无bgm字段兼容() {
        let dir = tmpdir("old");
        std::fs::create_dir_all(&dir).unwrap();
        // 旧版格式：无 bgm 字段
        let json = r#"{"slot":1,"title":"旧档","stamp":"t","file":"a.ks","line":1,"fvars":{},"snap":{"bg":null,"chars":[null,null,null],"cg":null},"fingerprint":1,"thumb":[]}"#;
        std::fs::write(format!("{dir}/save1.json"), json).unwrap();
        let e = load(&dir, 1).expect("旧档应可解析");
        assert_eq!(e.bgm, None);
    }

    #[test]
    fn 指纹校验拒读() {
        let dir = tmpdir("fp");
        let data = tmpdir("fpdata");
        std::fs::create_dir_all(format!("{data}/scenario")).unwrap();
        std::fs::write(format!("{data}/scenario/a.ks"), "n v1\nend").unwrap();
        let mut e = entry(2);
        e.fingerprint = fingerprint(&format!("{data}/scenario/a.ks"));
        save(&dir, &e).unwrap();
        assert!(load_checked(&dir, 2, &data).is_ok());
        std::fs::write(format!("{data}/scenario/a.ks"), "n 改过的剧本\nend").unwrap();
        let err = load_checked(&dir, 2, &data).unwrap_err();
        assert!(err.contains("不一致"), "{err}");
    }

    #[test]
    fn 最新槽与备份删除() {
        let dir = tmpdir("del");
        save(&dir, &entry(1)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        save(&dir, &entry(3)).unwrap();
        assert_eq!(latest_slot(&dir, 9), Some(3));
        assert_eq!(delete_latest(&dir, 9), Some(3));
        assert!(load(&dir, 3).is_none());
        assert!(load(&dir, 1).is_some());
        assert!(Path::new(&format!("{dir}/backup")).exists());
    }

    #[test]
    fn 时间戳格式() {
        assert_eq!(unix_to_stamp(0), "1970-01-01 00:00");
        assert_eq!(unix_to_stamp(1725609600), "2024-09-06 08:00"); // UTC
    }
}
