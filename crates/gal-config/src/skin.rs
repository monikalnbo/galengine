//! config/skin.rs —— UI 皮肤（对话框盒/底栏配色，作者多套 + 玩家切换）
//!
//! 一套皮肤 = 对话框盒（图/填充/边框/文字色）+ 底栏按钮配色。
//! 作者在 `ui.skin` 里给多套（`{"name": ..., "scheme": ...}`），玩家在设置面板切换
//! （`sf.skinIndex`）；未配置 = 内置「夜色」一套。

use serde::Deserialize;

/// 一套配色（对话框 + 底栏）
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct SkinCfg {
    /// 皮肤名（设置面板显示）
    pub name: String,
    /// 对话框填充 RGBA
    pub dialog_fill: [u8; 4],
    /// 对话框边框 RGBA
    pub dialog_border: [u8; 4],
    /// 对话框正文字色 RGB（null = 跟随 text.color）
    pub dialog_text: Option<[u8; 3]>,
    /// 底栏按钮常态填充 RGBA
    pub bar_fill: [u8; 4],
    /// 底栏按钮高亮填充 RGBA
    pub bar_accent: [u8; 4],
    /// 底栏按钮常态边框 RGBA
    pub bar_border: [u8; 4],
    /// 底栏按钮文字色 RGB
    pub bar_text: [u8; 3],
}
impl Default for SkinCfg {
    fn default() -> Self {
        Self {
            name: "夜色".into(),
            dialog_fill: [10, 12, 26, 205],
            dialog_border: [255, 255, 255, 60],
            dialog_text: None,
            bar_fill: [28, 34, 58, 215],
            bar_accent: [52, 74, 128, 235],
            bar_border: [120, 138, 185, 150],
            bar_text: [205, 212, 230],
        }
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct SkinListCfg {
    /// 可选皮肤列表（空 = 只用内置「夜色」）
    pub schemes: Vec<SkinCfg>,
}

impl SkinListCfg {
    /// 实际可用皮肤列表（至少一套内置）
    pub fn effective(&self) -> Vec<SkinCfg> {
        if self.schemes.is_empty() {
            vec![SkinCfg::default()]
        } else {
            self.schemes.clone()
        }
    }

    /// 按序号取皮肤（越界回落到第 0 套）
    pub fn at(&self, index: usize) -> SkinCfg {
        let list = self.effective();
        list.get(index).cloned().unwrap_or_else(|| list[0].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 空列表回落内置皮肤() {
        let list = SkinListCfg::default();
        assert_eq!(list.effective().len(), 1);
        assert_eq!(list.at(5), SkinCfg::default()); // 越界 → 第 0 套
    }

    #[test]
    fn 多套皮肤按序取用() {
        let warm =
            SkinCfg { name: "暖夜".into(), dialog_fill: [46, 20, 20, 210], ..Default::default() };
        let list = SkinListCfg { schemes: vec![SkinCfg::default(), warm.clone()] };
        assert_eq!(list.at(1), warm);
        assert_eq!(list.at(9), SkinCfg::default());
    }
}
