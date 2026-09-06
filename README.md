# eighth-summer-engine

《第八个夏天》自建 galgame 引擎（Rust + SDL2）。引擎独立仓库；游戏资产（剧本/美术/文档）在 `monikalnbo/eighth-summer`。

## 架构（分层）

```
src/
  main.rs        入口（薄）
  config.rs      顶层总配置（data/config.json 唯一配置源，ES_DATA_DIR 可重定向数据目录）
  app.rs         组装+主循环（事件按解释器状态分发）
  gfx/           渲染层：renderer(letterbox+离屏) / assets(纹理库+场景回收) / stage(演出层crossfade) / prefetch(后台预解码缓存层)
  text/          font(字体库+纹理缓存) / layout(中文断行禁则) / writer(打字机状态机)
  script/        剧本语法体系：lexer / command(26条指令) / interp(状态机) / vars(f.* sf.*) / expr(表达式)
  ui/            dialog(对话框) / choice(选项) / inputbox(名字输入)
```

- 逻辑分辨率 1280×720，窗口 letterbox 16:9 居中
- 场景级资源管理：背景变更=切场景，回收图片/文字纹理；后台线程预解码消除卡帧
- sf.* 全局变量退出即写盘（savedata/global.json）

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
