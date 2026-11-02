//! 防回归：随包模板 game.yaml 必须能被当前 Config 解析（作者照文档配必真实生效的前提）。

#[test]
fn 模板_game_yaml_可被当前配置解析() {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../template/game/data/game.yaml");
    let s = std::fs::read_to_string(p).expect("读模板 game.yaml");
    let cfg: gal_config::Config = serde_yaml::from_str(&s).expect("模板 game.yaml 解析失败");
    assert_eq!(cfg.meta.script_start, "start.ks");
    assert!(cfg.display.resizable);
    assert!(!cfg.ui.bottombar.buttons.is_empty());
}
