# galengine v0.3.11 完工报告 —— 演出等待收尾批（quake 族 + wc/resetwait + 指纹锁）

日期：2026-11-03 ｜ 版本：0.3.10 → **0.3.11** ｜ backlog 项：P2-3（quake 完整化，S）

## 交付内容

1. **quake 族（P2-3）**：
   - `quake [ms] [最大幅度]`（缺省 300ms/幅度 14）→ `SysEvent::Quake`，复用既有 `ShakeSt` 衰减包络通道（`window_fx shake` 同源）。
   - `stopquake` → `SysEvent::StopQuake`（清除 shake 态）；`wq` → `RunState::WaitAudio(AudioWait::QuakeDone)`，引擎轮询 `sys.shake.is_none()` 放行（feature video 开/关两路均接线）。
2. **`wc`**：等点击一次——进入 `RunState::WaitClick`（KAG3 waitclick 计数形态的一次点击口径）。
3. **`resetwait`**：取消当前 `WaitTimer` 强制等待并立即推进到下一等待点；无等待态静默。
4. **`locksnapshot [名]` / `unlocksnapshot`**：运行期指纹锁状态（`Interp.snapshot_lock`），存档/读档界面标题旁渲染「🔒 名」金色标记；不进存档快照（存档格式零改动）。
5. **验收剧本** `game/data/scenario/quake-demo.ks`（quake+wq、stopquake 中断、指纹锁、wc 全链路）。

## 门禁留证

| 门禁 | 结果 |
|---|---|
| cargo fmt | ✅ |
| clippy --workspace -D warnings | ✅ 0 错误 |
| cargo test --workspace | ✅ **146 passed / 0 failed**（+3：解析 1 / quake族+wc/resetwait 执行 1 / resetwait 取消等待 1） |
| 无头回归 | ✅ **十剧本 ALL_PASS**（含新增 quake-demo.ks；v0.3.2~v0.3.10 全时代剧本零改动通过） |
| 存档格式 | ✅ 零改动（snapshot_lock 为运行期状态） |
| 版本铁律 | ✅ 0.3.10 → 0.3.11（+0.01） |

## 152 口径更新

- v0.3.11 时点：**有 71 / 部分 21 / 缺 60 / 别名 1**（v0.3.10 为 64/21/67）。
- 缺→有：#78 resetwait、#80 wc、#84 wq、#90 locksnapshot、#91 unlocksnapshot；部分→有：#82 quake、#83 stopquake。P2-3 在 backlog §6 标 ✅。

## 教训

- 本批无新教训。

## 产物

- commit + tag `v0.3.11`；CHANGELOG.md v0.3.11 段；本报告；rolling 文件同步。
