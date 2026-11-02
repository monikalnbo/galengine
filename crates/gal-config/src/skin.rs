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

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SkinListCfg {
    /// 可选皮肤列表（空 = 只用内置「夜色」）
    pub schemes: Vec<SkinCfg>,
    /// 主题包目录（相对数据目录，默认 `skins`）；目录下每个 `*.yaml|*.yml`
    /// 可以是一套皮肤（含 `name:`）或 `schemes:` 列表，按文件名排序追加到列表尾部。
    pub dir: String,
}

impl Default for SkinListCfg {
    fn default() -> Self {
        Self { schemes: Vec::new(), dir: "skins".into() }
    }
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

    /// 合并主题包后的完整列表（同名的后出现者覆盖前者）
    pub fn with_packs(&self, packs: Vec<SkinCfg>) -> Vec<SkinCfg> {
        let mut list = self.effective();
        for p in packs {
            match list.iter_mut().find(|x| !p.name.is_empty() && x.name == p.name) {
                Some(slot) => *slot = p,
                None => list.push(p),
            }
        }
        list
    }

    /// 按序号取皮肤（越界回落到第 0 套）
    pub fn at(&self, index: usize) -> SkinCfg {
        let list = self.effective();
        list.get(index).cloned().unwrap_or_else(|| list[0].clone())
    }
}

/// 扫描主题包目录：每个 `.yaml|.yml` 可以是一套皮肤（顶层 `name:`）
/// 或 `schemes:` 列表（便于一个包放多套）。文件名排序、坏文件跳过并提示。
pub fn scan_packs(dir: &str) -> Vec<SkinCfg> {
    let root = if std::path::Path::new(dir).is_absolute() {
        dir.to_string()
    } else {
        format!("{}/{}", crate::data_dir(), dir)
    };
    let Ok(rd) = std::fs::read_dir(&root) else { return Vec::new() };
    let mut files: Vec<_> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| matches!(p.extension().and_then(|s| s.to_str()), Some("yaml" | "yml")))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for p in files {
        let Ok(text) = std::fs::read_to_string(&p) else { continue };
        // 先按列表包解析（有 schemes 且非空 → 视为多套包）
        if let Ok(list) = serde_yaml::from_str::<SkinListCfg>(&text) {
            if !list.schemes.is_empty() {
                out.extend(list.schemes);
                continue;
            }
        }
        match serde_yaml::from_str::<SkinCfg>(&text) {
            Ok(s) if !s.name.trim().is_empty() => out.push(s),
            Ok(_) => eprintln!("[skin] 跳过无名主题包 {}", p.display()),
            Err(e) => eprintln!("[skin] 跳过坏主题包 {}：{e}", p.display()),
        }
    }
    out
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
    fn 主题包扫描_单套与多套兼容_坏文件跳过() {
        let dir = std::env::temp_dir().join(format!("gal-skins-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a_single.yaml"), "name: 单套\ndialog_fill: [1, 2, 3, 200]\n")
            .unwrap();
        std::fs::write(dir.join("b_multi.yml"), "schemes:\n  - name: 多一\n  - name: 多二\n")
            .unwrap();
        std::fs::write(dir.join("c_bad.yaml"), "name: [坏").unwrap();
        let packs = scan_packs(dir.to_str().unwrap());
        let names: Vec<&str> = packs.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["单套", "多一", "多二"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 多套皮肤按序取用() {
        let warm =
            SkinCfg { name: "暖夜".into(), dialog_fill: [46, 20, 20, 210], ..Default::default() };
        let list =
            SkinListCfg { schemes: vec![SkinCfg::default(), warm.clone()], ..Default::default() };
        assert_eq!(list.at(1), warm);
        assert_eq!(list.at(9), SkinCfg::default());
    }
}
