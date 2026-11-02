# galengine 游戏创作手册

给游戏作者：不碰引擎代码，只写数据侧。换游戏 = 换一个 `data/` 目录。

---

## 1. 目录契约（引擎只认这些位置）

```
game/data/
├── game.yaml          游戏配置（标题/字体/对话框几何/角色色/文案/图层/拓展）——见 §5
├── scenario/*.ks      剧本（正文在这里）
├── bgimage/           背景 + CG，jpg|png，1280×720 整幅
│   ├── cg_*.jpg       ⚠ CG 必须用 cg_ 前缀命名，才会进鉴赏目录
│   └── title.jpg      可选：开始菜单背景（jpg|png；没有则深色渐变）
├── fgimage/*.png      立绘，1280×720 整幅透明画布（人物外全透明）
├── bgm/               BGM（wav/ogg/mp3），文件名 = 剧本里 bgm 后的名
├── sound/             音效 SE，文件名 = 剧本里 se 后的名
├── voice/             语音（可选；`voice 角色/文件名` 或 `voice 文件名`）
└── system/font.ttf    可选：随包中文字体（没有则回退 Linux Noto / Windows 雅黑）
```

素材统一 **1280×720 整幅画布**（立绘画在哪层就站在哪；位置微调用 char 的坐标或预设位）。

## 2. 剧本语法（26+ 条指令）

- `#` 行首注释；`*label` 定义跳转锚点；一行一条指令，首词=指令，其余=参数
- 中文报错自带 `文件:行号`

### 演出
| 指令 | 说明 |
|---|---|
| `bg 素材 [淡入ms]` | 背景；**bg 变更=切场景**，旧图回收 |
| `char 0-2 素材\|hide [x y\|位置名]` | 立绘三层，crossfade；末参数可写坐标，或 `sprites.positions` 里的预设名（内置 `left` `center` `right`，可在 game.yaml 扩展） |
| `cg 素材 [ms]` / `cg hide [ms]` | CG 全屏；**cg_* 自动解锁进鉴赏** |
| `bgm 名` / `bgm stop` / `bgm fadeout ms` | BGM 循环 / 停止 / 淡出；缺文件静默；BGM 随存档 |
| `se 名` | 音效一次 |
| `voice 名` / `voice stop` | 语音一次；查找 `voice/<角色>/<名>` 或 `voice/<名>`；播放时按 `voice.duck` 压低 BGM/SE |
| `clear` | 清空对话框 |
| `wait ms` | 强制等待，**玩家不可跳过**（快进也无效） |

### 文本
| 指令 | 说明 |
|---|---|
| `n 台词` | 旁白（无名字牌） |
| `name 名字 台词` | 对话；名字颜色查 game.yaml 的 name_colors |
| `{hero}` / `{you}` | 运行时替换 → f.heroName / sf.playerName（台词、desktop_write、window_fx title 均生效） |

### 分支与变量
| 指令 | 说明 |
|---|---|
| `flag 变量 +2` / `flag 变量 -1` | 好感度加减 |
| `set 变量 = 表达式` | `+ - * /`、`??`（空值合并）、引号字符串 |
| `if 变量 == 值 *label` | `== != >= <= > <`；成立则跳；未赋值变量按 0 比较，不崩 |
| `jump *label` / `jump 文件.ks *label` | 跳转（可跨文件） |
| `choice 提示`<br>`　选项文字|*label`<br>`endchoice` | 选项块；↑↓+回车 和 鼠标双通道 |
| `input 变量 提示\|宽\|默认值` | 名字输入（支持输入法 + 预设名按钮兜底） |

变量空间：`f.*` 随存档走；`sf.*` 全局（退出即写 `savedata/global.json`）。
引擎保留名：`sf.playerName` `sf.cgs` `sf.volBgm` `sf.volSe` `sf.volVoice` `sf.textSpeed` `sf.autoDelay` `sf.textSize` `sf.lineSpacing` `sf.waitMarker` `sf.outline` `sf.fontFamily` `sf.skipRead` `sf.transOn` `sf.skinIndex` `sf.keymap.<动作>`。
剧本里 `set sf.textSize = 34` 这类写法会在下一帧立即生效（与设置面板同一套值）。

### meta 演出（打破第四面墙）
| 指令 | 说明 |
|---|---|
| `meta_fake_save 日期\|时间\|图` | 存档列表尾部插入「不明存档」（不可读） |
| `meta_corrupt N` / `meta_corrupt all` | 槽位标「破損」，读取被拒 |
| `meta_delete_last` | 玩家亲手三重确认 → 渐白 → 真删最新档（自动备份 backup/） |
| `title_evolve 阶段` | 标题演化：`fade`（褪色）/`green`（变绿）/`small`（小字残题） |
| `reach 素材 [ms] [倍率]` / `reach hide` | 图像放大**伸出屏幕画进黑边** |
| `window_fx shake ms` | 窗口抖动 |
| `window_fx title 文字` / `window_fx title restore` | 改窗口标题 / 复原（文字支持 `{hero}`/`{you}`） |
| `shutdown [秒]` | 晚安关机：自绘倒计时+可取消；Windows 真关机，其余环境日志 |
| `desktop_write 文件\|内容` / `desktop_open 文件` | 往玩家桌面写/打开文本（防路径穿越；文件名与内容支持 `{hero}`；内容 `\n` 换行） |

### 流程
| 指令 | 说明 |
|---|---|
| `end` | 剧本终了（回标题可再来） |

### 拓展指令（extensions 声明后生效）
在 game.yaml `extensions:` 段声明拓展后，其行为自动挂进主循环。自带 `timer_choice`：
**倒计时选项**——任何 `choice` 进入选项态即起表，顶部画剩余时间条，超时自动选含 `pick` 文字的选项（默认「迟疑」，找不到回退第 0 项）。倒计时毫秒与超时挑选项都在 game.yaml 配。

## 3. 完整小样（开场 → 分支 → meta）

```text
# 第一幕：转学
bg bg_station_rain 800
bgm rain_theme
n 六月的雨，把站台洗成了一种透明的颜色。
name 时雨 ……你，也会来这里啊。
char 0 shigure_neutral
name 时雨 今天，雨停得很早。
char 0 shigure_smile

flag aff_shigure +1
choice 雨快停了，要说什么？
  「一起走吧」|*walk
  「再见」|*bye
endchoice

*walk
flag aff_shigure +2
name 时雨 ……嗯。
jump *after
*bye
n 她的伞尖，在积水里点了一下。
jump *after

*after
if aff_shigure >= 3 *shigure_route
n 蝉声还没开始，但夏天已经在咬合了。
end

*shigure_route
bg cg_first_meet 1200
n ——那条路线的门，开了。
end
```

玩家名替换示例：

```text
input f.heroName 你的名字|320|阿岚
n 从今天起，你就是{hero}了。
n 而「{you}」，一直是被注视着的那一方。
```

## 4. 开发循环

```bash
# ① 手动看（cwd = 引擎仓；DATA=你的数据目录）
ES_DATA_DIR=<你的>/game/data ./target/release/galengine

# ② 指定某一段排练（label 级）
ES_SCRIPT=20_first.ks ES_START_LABEL=*scene2 ES_DATA_DIR=<你的>/game/data ./target/release/galengine

# ③ 无头冒烟：自动点击跑完，看 ended
ES_SCRIPT=xx.ks ES_DEBUG=1 ES_DEBUG_AUTOCLICK_MS=400 ES_DATA_DIR=<你的>/game/data \
  xvfb-run -a ./target/release/galengine 2>&1 | grep ended

# ④ 重置进度：删 savedata/（存档/global.json/meta.json 全在里面）
```

操作速查：点击/空格推进｜A 自动｜按住 Ctrl 快进｜Esc 菜单｜F9/F10 快存快读｜F5-F8 音量。

## 5. game.yaml（游戏侧唯一配置源）

只写想覆盖的字段，缺省自动回退内置默认值（**零配置也能跑**）。config.json 已废除。
**文档里出现的字段全部真实生效**；已废弃/从未实现的字段（`letterbox`、`box_image`、
`opacity`、`item_image`、`bottombar.enabled`、`button.radius`、`menu_gap` 等）已从引擎
删除，旧 game.yaml 里残留它们不会报错，会被直接忽略。

```yaml
meta:
  title: 你的游戏名            # 窗口标题
  version: 1.0.0
  script_start: 10_common.ks   # 入口剧本
  donation_url: ""             # 打赏链接（空=设置菜单不显示打赏按钮）
  donation_text: ""

display:
  width: 1280                  # 窗口初始尺寸（逻辑分辨率恒 1280×720，按比例缩放）
  height: 720
  resizable: true              # 窗口能否被玩家拉伸
  bg_saturation: 1.0           # 背景饱和度（<1 降饱和护眼，仅作用 bgimage）

ui:
  dialog:                      # 对话框（盒体配色由 ui.skin 决定）
    box_rect: [56, 514, 1168, 172]
    text_x: 104
    text_y: 542
    font_size: 30              # 基准字号（text.size 优先）
    name_font_size: 30
    lines_per_page: 3
    typewriter_ms: 30          # 每字毫秒（玩家「文字速度」覆盖）
  choice:                      # 选项列表
    item_width: 760
    item_height: 66
    gap: 18
    top_y: 236
    font_size: 28
  bottombar:                   # 对话框底功能条
    buttons: [自动, 快进, 存档, 读档, 设置]   # 空列表 = 不显示底栏
    button_width: 92
    button_height: 28
    gap: 10
    y: 689                     # 底栏顶边 y（逻辑坐标；默认贴底）
    right_align: 1180
    font_size: 20
  title:                       # 标题画面
    background: bgimage/title.jpg   # 或 null=渐变
    title_text_y: 100
    title_font_size: 96
    hint_text: "— select —"
    hint_y: 238
    menu_top_y: 270
  menu:                        # Esc 系统菜单
    item_width: 380
    item_height: 62
    gap: 14
    top_y: 100
    font_size: 30
  input:                       # 名字输入页（y 坐标 + 压暗纱透明度）
    prompt_y: 180
    box_y: 270
    preset_y: 400
    confirm_y: 480
    hint_y: 575
    veil_alpha: 160
    max_len: 12                # 名字最大字符数
  cursor:                      # 自定义鼠标两态（32×32 透明 PNG；null=系统默认）
    default: image/cursor.png
    click: image/cursor_click.png
    hotspot: [0, 0]
  button:                      # 通用按钮配色（选项/菜单/底栏/输入页/存读档共用）
    fill_normal: [28, 34, 58, 215]
    fill_accent: [52, 74, 128, 235]
    border_normal: [130, 150, 200, 150]
    border_accent: [150, 200, 255, 220]
    text_normal: [210, 216, 232]
    text_accent: [255, 255, 255]
    border_width: 1
  skin:                        # 可切换皮肤：对话框盒 + 底栏配色（玩家设置里切）
    schemes:
      - name: 夜色              # 未配置 = 只用这一套内置皮肤
        dialog_fill: [10, 12, 26, 205]
        dialog_border: [255, 255, 255, 60]
        dialog_text: null      # null = 跟随 text.color
        bar_fill: [28, 34, 58, 215]
        bar_accent: [52, 74, 128, 235]
        bar_border: [120, 138, 185, 150]
        bar_text: [205, 212, 230]
      - name: 暖夜
        dialog_fill: [46, 20, 20, 210]
        dialog_border: [200, 120, 90, 220]
  layers:                      # 图层叠加：任意界面的多层组合定义
    title_extra:               # 名字可自定义，界面代码按名取用
      - image: bgimage/title_glow.png
        rect: [0, 0, 1280, 720]
      - fill: [0, 0, 0, 120]   # image 为空时纯色层
        rect: [0, 600, 1280, 120]
      - text: "{meta.title}"   # 文字层（支持 {meta.title} 等引用）
        rect: [0, 60, 1280, 80]
        font_size: 48
        color: [255, 255, 255]

text:                          # 正文排版作者默认值（玩家可在设置里覆盖）
  size: 0                      # 0 = 跟随 ui.dialog.font_size
  line_spacing: 0              # 像素，叠加在字体推荐行高之上
  color: [242, 242, 246]
  outline: 0                   # 描边宽度（SDL_ttf 描边与字同色）
  shadow_offset: [0, 0]        # [dx, dy]；0,0 = 无阴影
  shadow_color: [0, 0, 0]
  wait_marker: true            # 页满是否画「▼」
  families:                    # 设置面板「字体」下拉的候选（空 = 只用 fonts 候选链）
    - name: Noto Sans CJK SC
      path: /usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc

voice:
  dir: voice                   # 语音目录（相对数据目录）
  volume: 100                  # 默认音量 0-100（玩家 sf.volVoice 覆盖）
  duck: 0.45                   # 语音播放时 BGM/SE 压到的比例（1.0 = 不压低）

keymap:                        # 键位作者默认；玩家在设置「键位」组重绑
  bindings:
    next: [Return, KpEnter, Space]
    auto: [A]
    skip: [LCtrl, RCtrl]
    backlog: [PageUp]
    menu: [Escape]
    save: [F9]
    load: [F10]
    fullscreen: [F11]

sprites:
  layers: 3                    # 立绘层数
  default_fade_ms: 500
  positions:                   # 预设位置（char 指令按名引用）
    left: { x: 0, y: 0 }
    center: { x: 320, y: 0 }
    right: { x: 640, y: 0 }
    far_left: { x: -160, y: 0 }
  chibi: [chibi_girl]          # Q版立绘名单（出场时背景自动虚化突出角色）

audio:
  bgm_dir: bgm                 # BGM 子目录（相对数据目录）
  se_dir: sound                # SE 子目录
  bgm_volume: 80               # 0-100（玩家可在设置改，全局持久化）
  se_volume: 90
  fade_ms: 800                 # 切 BGM 时的淡入时长

fonts:                         # 字体候选链，逐个尝试
  - game/data/system/font.ttf
  - /usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc
  - C:\Windows\Fonts\msyh.ttc

save:
  dir: savedata
  slots: 9
  auto_save_ms: 0              # 自动存档间隔（0=关；>0 时按此间隔写最近槽位）

auto:
  delay_ms: 1000               # 自动模式翻页等待

game:
  hero_default: 阿岚           # input 未填时的兜底名
  you_default: 你
  shutdown_message: 晚安。
  input_presets:               # 键 = input 变量基名（f.heroName → hero）
    hero: [阿岚, 小舟, 阿远]
    player: [你, 读者, 旅行者]
  name_colors:                 # 名字牌配色（缺省白）
    澪: [150, 214, 255]
    时雨: [186, 164, 255]
  ritual_texts:                # meta_delete_last 三重确认文案（3 条）
    - 真的要删除吗？
    - 再想一下，删掉就回不来了。
    - 最后确认。亲手，删掉它。
  skip_read_only: false        # true=快进只跳已读
  menu_items: [继续, 存档, 读取存档, CG 鉴赏, 设置, 回到标题, 退出游戏]
  title_items: [开始游戏, 继续游戏, 读取存档, CG 鉴赏, 设置, 退出游戏]

extensions:                    # 拓展声明（见 §2 拓展指令）
  - name: timer_choice         # 倒计时选项
    default_ms: 7000
    pick: 迟疑                 # 超时自动选含此文字的选项
```

### 玩家设置面板（`savedata/global.json`）

Esc →「设置」打开，分页显示，改动**立刻生效并立即写盘**，重启仍在。玩家值存在
`global.json` 的 `sf.*` 里，作者默认值（上面的 game.yaml）不会因此被改动；
「恢复默认设置」= 回到作者默认值。

| 组 | 项目 | 存储键 | 生效范围 |
|---|---|---|---|
| 文字 | 文字速度 | `sf.textSpeed` | 打字机每字毫秒 |
| 文字 | 自动播放间隔 | `sf.autoDelay` | 自动模式翻页等待 |
| 文字 | 字号 | `sf.textSize` | 正文字号（当前台词立即重排） |
| 文字 | 行距 | `sf.lineSpacing` | 行高追加像素 |
| 文字 | 等待标记 ▼ | `sf.waitMarker` | 页满标记开/关 |
| 文字 | 文字描边 | `sf.outline` | 描边宽度 0-4px |
| 文字 | 字体 | `sf.fontFamily` | 切到 `text.families` 第 N 项 |
| 文字 | 跳过已读 | `sf.skipRead` | 快进只跳已读行 |
| 显示 | 全屏 | 窗口状态 | 立即切全屏/窗口 |
| 显示 | 转场动画 | `sf.transOn` | 关=转场直接切图 |
| 显示 | 界面皮肤 | `sf.skinIndex` | 换 `ui.skin.schemes` 第 N 套 |
| 音频 | BGM / 音效 / 语音音量 | `sf.volBgm` / `sf.volSe` / `sf.volVoice` | 三通道即时调音量 |
| 键位 | 8 个动作重绑 | `sf.keymap.<动作>` | 按键即绑，Esc 取消 |
| 其他 | 恢复默认设置 / 打赏 | — | 重置全部玩家值 / 打开 donation_url |

## 6. 发布

```bash
cargo build --release           # Windows: 用 windows target 交叉构建或 CI
galengine.exe + game/data/      # exe <2MB，数据同级，双击即玩
```

## 7. 从其他引擎迁移

从 KAG/吉里吉里系（`@image [l][r]` 标签语法）迁移的剧本**不能直接运行**，两条路：
1. **按本 DSL 重写**（推荐）——语法量小，一页速查；大纲与文案直接搬，只换写法
2. 编写旧语法 → 本 DSL 的转换脚本（按需自建）

引擎仓不附带示例数据（保持纯净）。写法以上文第 3 节的完整小样为准，复制进你自己的 `scenario/` 即可运行。
