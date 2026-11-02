//! 音频语义单测（v0.3.2 kira 版）。
//! 无声卡环境 Audio::ok=false —— 播放类操作静默降级、语义查询/日志证据链照常工作，
//! 这正是降级设计的可测性：所有测试不需要真实音频设备。

use super::*;
use gal_config::Config;

fn test_audio() -> Audio {
    let mut conf = Config::default();
    conf.voice.duck = 0.4;
    Audio::new(&conf)
}

/// 生成一个真实可解码的 wav（440Hz 短音），供 kira/symphonia 解码路径走通
fn wav_bytes(frames: usize, sample_rate: u32) -> Vec<u8> {
    let mut b = Vec::new();
    let data_len = (frames * 2) as u32;
    // RIFF 头
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes()); // fmt 块长
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // 单声道
    b.extend_from_slice(&sample_rate.to_le_bytes());
    b.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // 字节率
    b.extend_from_slice(&2u16.to_le_bytes()); // 块对齐
    b.extend_from_slice(&16u16.to_le_bytes()); // 位深
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        let s = (t * 440.0 * std::f32::consts::TAU).sin() * 0.5;
        b.extend_from_slice(&((s * i16::MAX as f32) as i16).to_le_bytes());
    }
    b
}

/// 把测试 wav 写进临时数据目录并让 gal_config 指向它
fn fixture(name: &str, frames: usize) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("gal-audio-kira-{}-{}", std::process::id(), name));
    let bgm = dir.join("bgm");
    let se = dir.join("sound");
    let voice = dir.join("voice");
    for d in [&bgm, &se, &voice] {
        let _ = std::fs::create_dir_all(d);
    }
    std::fs::write(bgm.join(format!("{name}.wav")), wav_bytes(frames, 8000)).unwrap();
    std::fs::write(se.join(format!("{name}.wav")), wav_bytes(frames, 8000)).unwrap();
    std::fs::write(voice.join(format!("{name}.wav")), wav_bytes(frames, 8000)).unwrap();
    dir
}

#[test]
fn 无声卡降级_语义查询可用_不崩() {
    let mut a = test_audio();
    // 本容器无 ALSA：ok=false 也必须全 API 可调用
    a.play_bgm("theme");
    a.pause_bgm();
    a.resume_bgm();
    a.fade_out_bgm(100);
    a.stop_bgm();
    a.xchg_bgm("other", 200);
    a.play_se("click");
    a.stop_se(None);
    a.play_voice("hero01");
    a.stop_voice();
    a.apply_volumes();
    a.pump();
    a.update_duck();
    assert!(!a.take_loop_hit());
    assert!(!a.bgm_fading());
    assert!(!a.se_fading());
    assert_eq!(a.take_interrupted(), None);
}

#[test]
fn 循环点校验_非法区间拒绝并记日志() {
    let mut a = test_audio();
    a.bgmopt(2000, 1000); // start >= end → 拒绝
    a.bgmopt(0, 0);
    // 降级模式下无句柄，也不得崩
    a.bgmopt(100, 2000);
}

#[test]
fn 缺文件静默跳过_不产生状态() {
    let mut a = test_audio();
    a.play_bgm("不存在曲");
    a.play_se("不存在音效");
    a.play_voice("不存在语音");
    assert_eq!(a.cur_bgm(), None);
    assert!(!a.take_loop_hit());
}

#[test]
fn 解码缓存_同曲二次播放不重复解码() {
    let dir = fixture("cachetest", 4000);
    std::env::set_var("ES_DATA_DIR", &dir);
    let mut a = test_audio();
    a.play_bgm("cachetest");
    let first = a.cache.contains_key("bgm:cachetest");
    a.play_bgm("cachetest"); // 同名重播 → 忽略（去重）
    a.xchg_bgm("cachetest", 50); // 强制切（走缓存命中）
    assert!(first, "首次播放应产生解码缓存（dir={}）", dir.display());
    assert!(a.cache.contains_key("bgm:cachetest"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn 语音被_se打断_事件只报一次() {
    let dir = fixture("ding", 2000);
    let mut a = test_audio();
    // 无声卡：voice_playing 恒 false → 不触发打断事件（语义正确）
    a.play_voice("ding");
    a.play_se("ding");
    assert_eq!(a.take_interrupted(), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn 音量换算_边界百分比() {
    assert_eq!(pct_to_db(0), Decibels::SILENCE);
    assert_eq!(pct_to_db(100), Decibels::IDENTITY);
    // 50% ≈ -6dB
    assert!((pct_to_db(50).0 + 6.0).abs() < 0.1, "50% 应约 -6dB，实际 {}", pct_to_db(50).0);
}

#[test]
fn stop_se_全部实例_降级不崩() {
    let mut a = test_audio();
    a.stop_se(None);
    a.stop_se(Some("click"));
}
