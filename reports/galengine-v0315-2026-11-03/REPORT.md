# galengine v0.3.15 完工报告 —— 超链接批（5 标签）

日期：2026-11-03 ｜ 版本：0.3.14 → **0.3.15** ｜ backlog 项：P1-6 邻近（152 #12 link / #13 endlink / #14 button / #32 locklink / #33 unlocklink）

## 交付内容

1. **5 个 KAG3 超链接标签**：
   - `link storage=*标签 / exp=表达式`：行内超链接开始。到配对 `endlink` 的文本可点，
     点击后跳转标签（jump_label + run_until_wait）或求值赋值（expr::eval_assign）。
     parse 层守卫：storage/exp 至少一项，否则「link 用法」中文报错。
   - `endlink`：闭区间成区。无配对报「endlink：没有配对的 link」；空区间报
     「endlink：link 与 endlink 之间没有文本」。
   - `button graphic=素材 storage=|exp=`：图形按钮。image/ 素材整幅可点（水平居中、
     垂直 3/4 处，素材原尺寸），悬停放大 6% 提示，点击触发动作；进入新等待态
     `RunState::WaitButton`，点空白不推进（不吃输入）。缺素材文件不绘制不崩。
   - `locklink` / `unlocklink`：链接锁。`link_at()` 锁定期恒返回 None（不可点），
     绘制同步置灰 RGB(120,124,134)；解锁恢复可点（激活青 RGB(96,200,255)）。
2. **基建**：
   - `LinkRegion { start, end, action }` + `LinkAction::{Jump,Exp}` + `ButtonSpec`。
   - 区间定位按打字机字符计数（换行计 1、ruby 组计 1）；link…endlink 组装期走
     `pending_links`，`apply_pending` 段落落地时转入 `links`、随段落替换自然清空；
     `indent` 期间区间随全角缩进右移 2。
   - `gal_ui::dialog::link_rects`：按当前揭示状态逐行算链接命中矩形（逻辑坐标）；
     `draw_units_colored`：逐单元着色绘制（ruby/内嵌图同步支持）。
   - `gal_ui::textbutton`：按钮落位/绘制/命中；鼠标悬停检测依赖 `Game::mouse_logical`
     （MouseMotion 逻辑坐标持续更新）。
   - 无头驱动：`auto_step` 补 `WaitButton => activate_button()`。

## 门禁留证

| 门禁 | 结果 |
|---|---|
| cargo fmt | ✅ |
| clippy --workspace --all-targets -D warnings | ✅ 0 错误 |
| cargo test --workspace | ✅ **170 passed / 0 failed**（+10：link 用法解析守卫 1 / 成区+嵌套+空区间+无配对 4 / 锁定命中 1 / activate_link Jump+Exp 2 / WaitButton 激活 2） |
| 无头回归 | ✅ **14 剧本 ALL_PASS**（含新增 link-demo.ks：jump 跳转路径 + exp 求值路径 + lock/unlock；v0.3.2~v0.3.14 全时代剧本零改动通过） |
| 存档格式 | ✅ 零改动（Interp 纯新增字段不落盘；savedata/global.json 旧文件载入实测） |
| 版本铁律 | ✅ 0.3.14 → 0.3.15（+0.01） |

## 验收剧本 link-demo.ks

- `link storage=*linked` + `hch 点我跳转` + `endlink` → 无头 auto 驱动点击命中 → 跳转 `*linked`。
- `link exp="f.gold = f.gold + 100"` → 点击后 `f.gold == 100` 断言（if/else 双路径留证）。
- `locklink` / `unlocklink` 标志位与置灰口径演示。
- 与 `l` 等待态交互：链接命中优先于普通 click 推进；锁定期点击落普通推进。

## 152 口径更新

- 消息/文本组：20（5 简化）/ 5 / 7 → **25（10 简化）/ 5 / 2**（缺→有：#12 link、#13 endlink、#14 button、#32 locklink、#33 unlocklink）。
- 总口径：100 / 21 / 31 → **105 / 21 / 26**。
- 已回写 `/root/.hermes-web-ui/person/1/workspace/reports/galengine-krkr-parity-2026-11-02.md`。

## 简化口径说明（诚实记录）

- KAG3 link 支持的名字/目标槽位 hint（name=、target=）、按钮的 caption/副作用属性
  未收；本批 `storage`/`exp` 两动作 + 命中/置灰/悬停是核心通道，152 表按「有（简化）」登记。
- `button` 素材仅支持 image/ 整幅点击（无切分 4 态贴图）。

## 下批建议（backlog §6）

- 消息层多文字层批：#20 current / #21 position 补全 / #31 glyph。
- 图形化输入批（建议单列）：#15 edit 补全 / #16 checkbox / #17 commit。
