# galengine v0.3.13 完工报告 —— 文本标记批（9 标签）

日期：2026-11-03 ｜ 版本：0.3.12 → **0.3.13** ｜ backlog 项：P1-5（152 #2 graph / #3 hch / #5 ruby / #6 font / #7 deffont / #8 resetfont / #9 style / #10 defstyle / #11 resetstyle）

## 交付内容

1. **9 个 KAG3 文本标记标签**：
   - `graph 素材名`：行内嵌图全链路——`\u{b}名\u{b}` 占位符 → `Unit::Image`（整体不拆行、
     打字机计 0 字符、朗读纯文本自动去图）→ 渲染时经 TextureBank 按行高等比贴图。
     素材探测链：`image/` → `bgimage/` → `cg/`（`paths` 新增 `image` kind）；缺文件跳过不崩。
   - `ruby text= alt=`：转成既有 `[基底]^(注音)` 文法拼进台词，与手写注音文法同效。
   - `hch`：行内样式片段，按普通文本拼进（无行内色通道）+ DEBUG 留证。
   - `font/deffont/resetfont`、`style/defstyle/resetstyle`：属性记录 + DEBUG 留证（演出零变化；
     行内字号/颜色真通道留待后续批次，届时只改这 6 个臂）。
2. **工程**：`TextureBank::size_of`（尺寸查询）、`dialog::draw/draw_units` 加 `bank` 参数、
   `gal-text::plain` 去图、验收剧本 `texttag-demo.ks` + 合成图素材 `data/image/*.png`。

## 门禁留证

| 门禁 | 结果 |
|---|---|
| cargo fmt | ✅ |
| clippy --workspace --all-targets -D warnings | ✅ 0 错误 |
| cargo test --workspace | ✅ **154 passed / 0 failed**（+4：v0313 解析 1 / graph+ruby+style exec 1 / layout 图占位文法 2） |
| 无头回归 | ✅ **十一剧本 ALL_PASS**（含新增 texttag-demo.ks；v0.3.2~v0.3.12 全时代剧本零改动通过） |
| 存档格式 | ✅ 零改动（Unit::Image 纯新增变体，旧文本不含占位符） |
| 版本铁律 | ✅ 0.3.12 → 0.3.13（+0.01） |

## 152 口径更新

- v0.3.13 时点：**有 89 / 部分 21 / 缺 42 / 别名 1**（v0.3.12 为 80/21/51）。
- 缺→有：#2 graph、#3 hch、#5 ruby、#6 font、#7 deffont、#8 resetfont、#9 style、#10 defstyle、#11 resetstyle。
  已回写 `/root/.hermes-web-ui/person/1/workspace/reports/galengine-krkr-parity-2026-11-02.md`。

## 简化口径说明（诚实记录）

- `font/deffont/style` 族本批为**记录 + 留证**，不产生实际样式变化——152 表按「有（简化）」登记，
  真通道（行内字号/颜色/对齐）是后续批次显式课题，不在本批充数。
- `link/button/endlink` 超链接族（#12/13/14）未含本批（依赖点击区域命中，单独批次）。

## 产物

- commit + tag `v0.3.13`；CHANGELOG.md v0.3.13 段；本报告；rolling 文件同步。
