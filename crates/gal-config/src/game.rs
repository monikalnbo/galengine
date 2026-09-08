//! config/game.rs —— 游戏数据配置（角色色/预设名/仪式文案/游戏规则）

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct GameCfg {
    pub hero_default: String,
    pub you_default: String,
    pub shutdown_message: String,
    /// 名字输入预设（键=input 变量基名）
    pub input_presets: HashMap<String, Vec<String>>,
    /// 角色名 → 名字牌颜色 RGB
    pub name_colors: HashMap<String, [u8; 3]>,
    /// meta_delete_last 三重确认文案（3条）
    pub ritual_texts: Vec<String>,
    /// 快进是否跳过已读文本（true=只跳已读，false=全部可跳）
    pub skip_read_only: bool,
    /// Esc 菜单项（自定义菜单文字）
    pub menu_items: Vec<String>,
    /// 标题菜单项
    pub title_items: Vec<String>,
}
impl Default for GameCfg {
    fn default() -> Self {
        Self {
            hero_default: "他".into(),
            you_default: "你".into(),
            shutdown_message: "晚安。".into(),
            input_presets: HashMap::new(),
            name_colors: HashMap::new(),
            ritual_texts: vec![
                "真的要删除吗？".into(),
                "再想一下，删掉就回不来了。".into(),
                "最后确认。亲手，删掉它。".into(),
            ],
            skip_read_only: false,
            menu_items: vec![
                "继续".into(),
                "存档".into(),
                "读取存档".into(),
                "CG 鉴赏".into(),
                "设置".into(),
                "回到标题".into(),
                "退出游戏".into(),
            ],
            title_items: vec![
                "开始游戏".into(),
                "继续游戏".into(),
                "读取存档".into(),
                "CG 鉴赏".into(),
                "设置".into(),
                "退出游戏".into(),
            ],
        }
    }
}
