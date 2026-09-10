//! slots 单元测试

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
    let backup_dir = format!("{dir}/backup");
    assert!(Path::new(&backup_dir).exists());
    let files: Vec<String> = std::fs::read_dir(&backup_dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(files.len(), 1);
    assert!(files[0].starts_with("save3_"));
    assert!(!files[0].contains(':'));
}

#[test]
fn 时间戳格式() {
    assert_eq!(unix_to_stamp(0), "1970-01-01 00:00");
    assert_eq!(unix_to_stamp(1725609600), "2024-09-06 08:00");
}
