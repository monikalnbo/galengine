# eighth-summer-engine

《第八个夏天》自建 galgame 引擎（Rust + SDL2）。引擎独立仓库；游戏资产（剧本/美术/文档）在 `monikalnbo/eighth-summer`。

> **状态：源码已清空，等待按需求书重构**（历史实现在 git log 里可回溯）。
> 唯一需求来源：[`docs/rebuild-spec.md`](docs/rebuild-spec.md)——功能清单/分层架构/剧本 DSL/验收标准/踩坑记录全部在内。

## 剧本语法

见 `docs`（游戏仓 docs/20 第四节）与 `src/script/command.rs`（一处看全 26 条指令）。
示例：`testdata/game/data/scenario/a5_smoke.ks`（input/choice/flag/set/if 全链路）。

## 运行

```bash
cargo build --release
# 用自带测试数据
ES_DATA_DIR=testdata/game/data ./target/release/eighth-summer
# 指向游戏仓数据
ES_DATA_DIR=/root/galgame/game/data ./target/release/eighth-summer
```

- 操作：点击/空格推进｜↑↓+回车或鼠标选项｜A 自动模式｜按住 Ctrl 快进｜Esc 退出
- 验收辅助环境变量：`ES_SCRIPT`（启动剧本）`ES_DEBUG=1`（日志）`ES_DEBUG_AUTOCLICK_MS=N`（自动点击，xvfb 验收）

## 测试

```bash
./run_tests.sh   # xvfb 三剧本回归（a3/a4/a5）
```
