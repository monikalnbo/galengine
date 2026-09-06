# galengine（eighth-summer-engine）

通用 galgame 引擎（Rust + SDL2，单二进制 <10MB）。换游戏=换 `data/` 目录+`config.json`，引擎代码零改动；引擎仓与游戏仓分离（首个客户：《第八个夏天》）。

> 唯一需求来源：[`docs/rebuild-spec.md`](docs/rebuild-spec.md)。
> 2026-09 按需求书从零重构完成（旧实现 git log 可回溯）。

## 分层（只准向下依赖）

```
L1  app/input/render/systems   主循环 · 事件路由 · 帧组装 · 系统动作与状态枢纽
L2  script/ ui/                剧本(lexer/command/interp/expr/vars) · 界面(dialog/choice/inputbox/menu/title/savemenu/gallery/volume/ritual/restui/overlay)
L3  gfx/ text/ audio.rs save/  renderer(letterbox+越界) · stage(crossfade) · assets/prefetch · font/layout/writer · SDL2_mixer · slots/meta
L4  platform.rs config.rs      关机/桌面文件 · 唯一配置源 data/config.json
```

## 功能一览

- 1280×720 离屏 + 16:9 letterbox；reach 越界层画进黑边；window_fx shake/标题
- 26 条剧本 DSL（bg/char/cg/bgm/se/flag/set/jump/if/choice/input/meta 四件套/reach/shutdown/desktop_write…），错误全中文（文件:行号）
- {hero}/{you} 运行时替换；f.* 随存档，sf.* 退出即写盘 global.json
- 双通道选项；SDL_TEXTINPUT（IME）+预设名兜底；A 自动模式；按住 Ctrl 快进（wait 不可跳）
- 9 槽存档（缩略图+演出层快照+剧本指纹，不匹配中文拒读）+F9/F10 快存读
- meta 演出：不明存档/破損标/三重确认删档（备份 backup/）/标题演化
- CG 鉴赏（cg 指令自动解锁记 sf.cgs，目录=bgimage/cg_*，锁定剪影+全屏翻页）
- 音量三滑条（BGM/SE/文字速度，存 sf.*）+F5-F8 快捷键；晚安关机（Windows 真执行，验收环境日志）；desktop_write/open（防路径穿越）

## 运行

```bash
cargo build --release
ES_DATA_DIR=testdata/game/data ./target/release/galengine   # 自带测试数据
ES_DATA_DIR=/root/galgame/game/data ./target/release/galengine  # 游戏仓数据
```

- 操作：点击/空格推进｜↑↓+回车或鼠标选项｜A 自动｜按住 Ctrl 快进｜Esc 菜单｜F9/F10 快存读
- 验收辅助：`ES_SCRIPT` `ES_START_LABEL` `ES_DATA_DIR` `ES_DEBUG=1` `ES_DEBUG_AUTOCLICK_MS=N`

## 测试与验收

```bash
cargo test       # 纯逻辑单测 + SDL dummy 驱动存读档 E2E
./run_tests.sh   # xvfb 三剧本回归（a3 演出链 / a4 跨文件 / a5 分支输入），全 ended
# a6 全功能冒烟（meta/越界/窗口/关机/桌面文件）
ES_DATA_DIR=testdata/game/data ES_SCRIPT=a6_smoke.ks ES_DEBUG=1 ES_DEBUG_AUTOCLICK_MS=400 \
  xvfb-run -a ./target/release/galengine
```

视觉验收：ffmpeg x11grab 关键帧连拍 + PIL 像素交叉验证（标题/名牌/选项高亮/reach 越界黑边均过；VLM 判读待有图像能力会话补跑）。

## 已知简化（ponytail）

- 存档指纹用 DefaultHasher：同版本引擎内稳定，跨版本升级存档会拒读（重打一局）
- 存档时间戳为 UTC
- kslint 静态检查（label/变量/资源存在性）未建；Windows 真机（IME/DPI/关机）待回归
