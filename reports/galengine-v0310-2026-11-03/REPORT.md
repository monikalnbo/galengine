# galengine v0.3.10 完工报告 —— 动画档批（animstart / .asd）

日期：2026-11-03 ｜ 版本：0.3.9 → **0.3.10** ｜ backlog 项：P2-2（.asd 动画 + animstart/animstop）

## 交付内容

1. **`animstart 层 档名`**：加载 `data/asd/档名.asd` 文本档为多属性关键帧时间轴。
   - `gal-config::paths` 新增 `asd` kind（`{data}/asd/<名>.asd`）。
   - 文件缺失/解析错误 → 中文报错停句；空双轨档报错（演出不静默黑屏）。
2. **.asd 档格式**（`parse_asd`）：行 `time,x,y[,opacity]`——2 列纯 opacity、3 列纯位移、4 列双轨齐写；`#` 注释、空行跳过；时间乱序自动升序 sort；错列数/非数字 → 行号中文报错。
3. **AnimClip**（stage.rs）：pos/opa 双轨线性插值；`segment()` 段查找 + 具体化采样；opacity 轨道直写层不透明度（与 layopt 的 opacity 字段同字段）。
4. **口径合一**：与 move 共用「同层覆盖、wa（`move_left_ms` 取 moves+anims 最大）、animstop/stop_move 停点、在途动画不进存档」。
5. **验收剧本** `game/data/scenario/animstart-demo.ks` + 示例档 `game/data/asd/swing.asd`。

## 门禁留证

| 门禁 | 结果 |
|---|---|
| cargo fmt | ✅ |
| clippy --workspace -D warnings | ✅ 0 错误 |
| cargo test --workspace | ✅ **143 passed / 0 failed**（v0.3.9 为 137，+6：asd 解析×2、AnimClip 插值/覆盖/存档×3、animstart 执行×1；parse 臂并入既有解析测试） |
| 无头回归 | ✅ **九剧本 ALL_PASS**（含新增 animstart-demo.ks；anim-demo v0.3.6 旧剧本零改动通过） |
| 存档格式 | ✅ 零改动（在途动画不进快照；单测覆盖旧档反序列化 + 快照往返） |
| 版本铁律 | ✅ 0.3.9 → 0.3.10（+0.01） |

## 152 口径更新

- v0.3.10 时点：**有 64 / 部分 21 / 缺 67 / 别名 1**（v0.3.9 为 63/21/68）。
- #99 `animstart`：缺 → 有。P2-2 在 backlog §6 标 ✅。
- 已回写 `/root/.hermes-web-ui/person/1/workspace/reports/galengine-krkr-parity-2026-11-02.md`。

## 兼容性

- 存档快照结构零改动；旧剧本（anim-demo/audio-demo 等 v0.3.2~v0.3.6 时代）九剧本全过。
- `move` 无 .asd 剧本零改动；`asd` kind 仅新增路径分支，既有素材探测不变。

## 教训

- 本批无新教训（延续 E9 既有口径）。

## 产物

- commit + tag `v0.3.10`；CHANGELOG.md v0.3.10 段；本报告；rolling 文件同步。
