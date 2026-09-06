# galengine

数据驱动的通用 galgame / 视觉小说引擎。**Rust + SDL2，单二进制约 1.4MB，MIT 协议。**

换一部游戏 = 换一个 `data/` 目录 + `config.json`，引擎代码零改动。游戏内容（剧本、立绘、音乐、角色配色、UI 文案）全部在数据侧，引擎内不含任何具体作品的信息。

## 特性

- **1280×720 离屏渲染**，窗口自由缩放 16:9 letterbox 居中；图片资源后台线程预解码，切场景不卡帧
- **26 条剧本 DSL**：背景/立绘/CG/音频、变量与表达式（`?? + - * /`）、跨文件跳转、选项分支、名字输入
- **打字机文本**：中文断行禁则、30ms/字可调、点击三段语义（补全→翻页→推进）
- **双通道交互**：键盘（↑↓+回车）与鼠标（悬停+点击）全程等价；SDL 文本输入透传 IME，预设名按钮兜底
- **存档系统**：9 槽 + 快存快读（F9/F10），缩略图 + 演出层快照 + 剧本指纹校验（不匹配中文拒读）
- **meta 演出（打破第四面墙）**：不明存档、破損标记、三重确认删档、标题画面演化、图像越出屏幕画进黑边、窗口抖动、向玩家桌面写文件、晚安关机
- **CG 鉴赏**：`cg` 指令自动解锁，锁定剪影 + 全屏翻页
- **环境设置**：BGM / SE / 文字速度三滑条 + 快捷键，全局持久化
- **可验收**：全中文报错（含 文件:行号）、无头自动回归、单测齐全

## 剧本长什么样

```text
bg bg_station_rain 800
bgm rain_theme
name 时雨 ……你，也会来这里啊。
char 0 shigure_neutral

flag aff_shigure +1
choice 雨快停了，要说什么？
  「一起走吧」|*walk
  「再见」|*bye
endchoice

*walk
flag aff_shigure +2
if aff_shigure >= 3 *shigure_route
```

## 快速开始

```bash
git clone https://github.com/monikalnbo/galengine
cd galengine
cargo build --release
ES_DATA_DIR=testdata/game/data ./target/release/galengine   # 跑自带演示数据
```

操作：点击/空格推进｜↑↓+回车或鼠标选项｜A 自动模式｜按住 Ctrl 快进｜Esc 菜单｜F9/F10 快存读｜F5-F8 音量。

## 📚 文档（怎么用、怎么写、怎么改）

| 文档 | 给谁 | 内容 |
|---|---|---|
| **[创作手册](docs/game-authoring.md)** | 编剧 / 美术 / 音频 / 策划 | **操作规范 + 教程**：目录契约（素材放哪、怎么命名）、26 条指令逐条说明、完整小样剧本、config.json 全字段、开发循环（怎么看效果/排练单段/无头冒烟/重置进度）、发布打包、旧引擎语法迁移 |
| **[错题本](docs/mistakes.md)** | 引擎维护者 / 做验收的人 | 五类 32 条真实踩坑（引擎代码 / Rust-sdl2 API / 像素验收 / 测试自身 / 工程流程）+ 8 条防复发规则 |
| **[设计文档](docs/rebuild-spec.md)** | 想理解引擎为什么这样设计的人 | 分层架构（L1-L4）、功能需求全清单、配置规范、验收标准 |

**做一部游戏只需要读第一篇。** 照目录摆素材、抄 config、按指令表写剧本，即可完整跑起来。

## 架构（每文件百行量级，只准向下依赖）

```
L1  app/input/render/systems   主循环 · SDL 事件路由 · 帧组装 · 系统动作
L2  script/ ui/                剧本(词法/指令/解释器/表达式/变量) · 界面(12 个组件)
L3  gfx/ text/ audio save/     letterbox渲染 · 演出层crossfade · 预解码 · 字体断行打字机 · 混音 · 存档
L4  platform.rs config.rs      关机/桌面文件 · 唯一配置源 data/config.json
```

## 测试与验收

```bash
cargo test       # 18 项：纯逻辑单测 + SDL dummy 驱动存读档端到端
./run_tests.sh   # xvfb 三剧本回归（演出链 / 跨文件 / 分支输入），全部跑至 ended
```

视觉验收流程：ffmpeg x11grab 连拍关键帧 + PIL 像素断言交叉验证（标题背景、名字牌配色、选项高亮、越界层黑边覆盖等）。

## License

[MIT](LICENSE)
