# 更新日志

遵循 [语义化版本](https://semver.org/lang/zh-CN/)。旧 `game.yaml`、旧存档、旧 `savedata/global.json` 均可直接使用。

## v0.3.0 — 2026-11-02

**设置系统补齐**：game.yaml 死字段清零 + 玩家设置从 6 项扩到 25 项，全部即时生效并持久化。

### game.yaml 字段治理（P0）

- **删除**（解析后从未被读取，属文档噪音；旧 YAML 残留这些键仍可静默加载）：
  `display.letterbox`、`ui.dialog.box_image`、`ui.dialog.opacity`、
  `ui.choice.item_image`、`ui.choice.item_hover_image`、`ui.bottombar.enabled`、
  `ui.title.menu_gap`、`ui.menu.item_image`、`ui.button.normal_image`、
  `ui.button.accent_image`、`ui.button.radius`。
- **接通**（此前写了也不生效，现在真实生效）：
  - `ui.bottombar.buttons`（按钮集合与顺序）、`button_width/height/gap/y/right_align/font_size`
  - `ui.choice` / `ui.menu` 的 `item_width/item_height/gap/top_y/font_size`
  - `ui.input` 的 `prompt_y/box_y/preset_y/confirm_y/hint_y/veil_alpha/max_len`
  - `ui.button` 的整套配色与 `border_width`
  - `save.auto_save_ms`（>0 时按间隔自动写最近槽位）
  - `audio.bgm_dir/se_dir/fade_ms`
  - `display.width/height/resizable`
  - `game.menu_items` / `game.title_items`

### 玩家设置面板（P1）

Esc →「设置」现为分组分页（每页 10 项），改动即时生效并立即写入 `savedata/global.json`，重启仍在。

| 组 | 设置项 |
|---|---|
| 文字 | 文字速度、自动播放间隔、字号、行距、等待标记、文字描边、字体、跳过已读 |
| 显示 | 全屏、转场动画、界面皮肤 |
| 音频 | BGM / 音效 / 语音音量 |
| 键位 | 推进、自动、快进、回想、菜单、存档、读档、全屏（8 个动作重绑） |
| 其他 | 恢复默认设置、打赏（配置了链接时显示） |

- 字号 / 行距 / 描边变更后，**当前台词立即按新样式重排**，无需翻页。
- 字体候选来自 `text.families`，缺字体文件时自动回退候选链，不报错。

### 新增游戏侧配置段

- `text:` —— 正文字号/行距/颜色/描边/阴影/等待标记/字体候选（作者默认值，玩家可覆盖）
- `voice:` —— 语音目录、默认音量、duck 压低比例
- `keymap:` —— 8 个动作的作者默认键位
- `ui.skin.schemes` —— 多套对话框+底栏配色，玩家设置里切换

### 音频与剧本

- 新增语音通道：`voice 名` / `voice stop`；播放时按 `voice.duck` 压低 BGM/SE，
  自然播完后自动恢复。
- BGM 切歌按 `audio.fade_ms` 淡入。

### 体积

release 产物改为 `opt-level = "s"`，虽新增整套设置逻辑，二进制仍比 v0.2.3 更小
（1,963,376 B → 1,610,520 B）。

## v0.2.3 — 2026-11-02

脚本层 P0/P1：统一层表与 trans 转场引擎、表达式求值器重写、if 块、call/return。
