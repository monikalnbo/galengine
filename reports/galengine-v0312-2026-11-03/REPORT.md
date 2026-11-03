# galengine v0.3.12 完工报告 —— 存档控制标签批（9 标签全链路）

日期：2026-11-03 ｜ 版本：0.3.11 → **0.3.12** ｜ backlog 项：#52/54/58/59/60/61/62/64/65

## 交付内容

1. **9 个 KAG3 存档控制标签**：
   - `s`：停止快进/自动（SysEvent::StopSkip → 引擎态翻转 auto/ctrl_hold）。
   - `disablestore [t/f]`：禁存开关（`Interp.store_disabled`；`can_save()` 拦截，存档界面/autosave 一并生效）。
   - `tempsave` / `tempload`：临时存档 `{save_dir}/temp.json`（结构同普通档、缩略图留空、slot=0，不占玩家槽位不可见；tempload 含 BGM 随档恢复）。
   - `copybookmark 源 目标` / `erasebookmark 槽号`：gal-save 新增 `copy_slot`（整档复制改 slot 号）；erasebookmark 直删 + `refresh_saves`；空槽中文留证不崩。
   - `record`：`Overlay::Gallery` 回顾界面；`goback`：读 `latest_slot` 回退（无档 WARN 留证）。
   - `waittrig 名`：等触发器 `f.trig_<名>` truthy；已置位立即放行，`AudioWait::Trig` 引擎两路轮询接线。
2. **gal-save 新 API**：`copy_slot` / `save_temp` / `load_temp` / `temp_path`（存档槽位 JSON 结构零改动）。
3. **验收剧本** `game/data/scenario/savectrl-demo.ks`（waittrig 置位/放行、disablestore 翻转、s、tempsave 恢复点、record 全链路；tempload 环形回溯按 KAG3 语义在验收中改为恢复点后置演示，避免无头回归死循环——tempsave→tempload 相邻行会无限往返）。

## 门禁留证

| 门禁 | 结果 |
|---|---|
| cargo fmt | ✅ |
| clippy --workspace -D warnings | ✅ 0 错误 |
| cargo test --workspace | ✅ **150 passed / 0 failed**（+4：v0312 存档控制执行 1 / waittrig 等待放行 1 / copy_slot+temp 往返 1（gal-save 侧）/ 解析 1） |
| 无头回归 | ✅ **十剧本 ALL_PASS**（含新增 savectrl-demo.ks；v0.3.2~v0.3.11 全时代剧本零改动通过） |
| 存档格式 | ✅ 槽位 JSON 零改动（temp.json 为新文件不进槽位体系；global.json 结构不变，仅 readLines 运行期自然追加） |
| 版本铁律 | ✅ 0.3.11 → 0.3.12（+0.01） |

## 152 口径更新

- v0.3.12 时点：**有 80 / 部分 21 / 缺 51 / 别名 1**（v0.3.11 为 71/21/60）。
- 缺→有：#52 waittrig、#54 s、#58 disablestore、#59 tempsave、#60 tempload、#61 copybookmark、#62 erasebookmark、#64 goback、#65 record。已回写 `/root/.hermes-web-ui/person/1/workspace/reports/galengine-krkr-parity-2026-11-02.md`。

## 教训

- **tempload 紧跟 tempsave 是无头回归死循环陷阱**：tempload 恢复到 tempsave 时刻的行号，若两标签相邻（或中间行数少于推进步长）会无限往返。验收剧本须把恢复点设计在收尾处，或让 tempload 后不再回到 tempsave 之前的行。
- **磁盘满会让 `cargo build --release --quiet` 静默失败**，run_tests.sh 里的旧二进制会以「全 FAIL」假象误导排查；遇到全量 FAIL 先 `df -h`。

## 产物

- commit + tag `v0.3.12`；CHANGELOG.md v0.3.12 段；本报告；rolling 文件同步。
