# galengine 游戏创作手册

给游戏作者：不碰引擎代码，只写数据侧。换游戏 = 换一个 `data/` 目录。

---

## 1. 目录契约（引擎只认这些位置）

```
game/data/
├── config.json        游戏配置（标题/字体/对话框几何/角色色/文案）——见 §5
├── scenario/*.ks      剧本（正文在这里）
├── bgimage/           背景 + CG，jpg|png，1280×720 整幅
│   ├── cg_*.jpg       ⚠ CG 必须用 cg_ 前缀命名，才会进鉴赏目录
│   └── title.jpg      可选：开始菜单背景（jpg|png；没有则深色渐变）
├── fgimage/*.png      立绘，1280×720 整幅透明画布（人物外全透明）
├── bgm/               BGM（wav/ogg/mp3），文件名 = 剧本里 bgm 后的名
├── sound/             音效 SE，文件名 = 剧本里 se 后的名
└── system/font.ttf    可选：随包中文字体（没有则回退 Linux Noto / Windows 雅黑）
```

素材统一 **1280×720 整幅画布**（立绘画在哪层就站在哪，不需要坐标）。

## 2. 剧本语法（26 条指令）

- `#` 行首注释；`*label` 定义跳转锚点；一行一条指令，首词=指令，其余=参数
- 中文报错自带 `文件:行号`

### 演出
| 指令 | 说明 |
|---|---|
| `bg 素材 [淡入ms]` | 背景；**bg 变更=切场景**，旧图回收 |
| `char 0-2 素材\|hide` | 立绘三层（0 左 1 中 2 右），crossfade |
| `cg 素材 [ms]` / `cg hide [ms]` | CG 全屏；**cg_* 自动解锁进鉴赏** |
| `bgm 名` / `se 名` | BGM 循环 / SE 一次；缺文件静默；BGM 随存档 |
| `clear` | 清空对话框 |
| `wait ms` | 强制等待，**玩家不可跳过**（快进也无效） |

### 文本
| 指令 | 说明 |
|---|---|
| `n 台词` | 旁白（无名字牌） |
| `name 名字 台词` | 对话；名字颜色查 config 的 name_colors |
| `{hero}` / `{you}` | 运行时替换 → f.heroName / sf.playerName（含 desktop_write 内容） |

### 分支与变量
| 指令 | 说明 |
|---|---|
| `flag 变量 +2` / `flag 变量 -1` | 好感度加减 |
| `set 变量 = 表达式` | `+ - * /`、`??`（空值合并）、引号字符串 |
| `if 变量 == 值 *label` | `== != >= <= > <`；成立则跳 |
| `jump *label` / `jump 文件.ks *label` | 跳转（可跨文件） |
| `choice 提示`<br>`　选项文字|*label`<br>`endchoice` | 选项块；↑↓+回车 和 鼠标双通道 |
| `input 变量 提示\|宽\|默认值` | 名字输入（支持输入法 + 预设名按钮兜底） |

变量空间：`f.*` 随存档走；`sf.*` 全局（退出即写 `savedata/global.json`）。
引擎保留名：`sf.playerName` `sf.cgs` `sf.volBgm` `sf.volSe` `sf.textSpeed`。

### meta 演出（打破第四面墙）
| 指令 | 说明 |
|---|---|
| `meta_fake_save 日期\|时间\|图` | 存档列表尾部插入「不明存档」（不可读） |
| `meta_corrupt N` / `meta_corrupt all` | 槽位标「破損」，读取被拒 |
| `meta_delete_last` | 玩家亲手三重确认 → 渐白 → 真删最新档（自动备份 backup/） |
| `title_evolve 阶段` | 标题演化：`fade`（褪色）/`green`（变绿）/`small`（小字残题） |
| `reach 素材 [ms] [倍率]` / `reach hide` | 图像放大**伸出屏幕画进黑边** |
| `window_fx shake ms` | 窗口抖动 |
| `window_fx title 文字` / `window_fx title restore` | 改窗口标题 / 复原 |
| `shutdown [秒]` | 晚安关机：自绘倒计时+可取消；Windows 真关机，其余环境日志 |
| `desktop_write 文件\|内容` / `desktop_open 文件` | 往玩家桌面写/打开文本（防路径穿越） |

### 流程
| 指令 | 说明 |
|---|---|
| `end` | 剧本终了（回标题可再来） |

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
input f.heroName 你的名字|320|拓海
n 从今天起，你就是{hero}了。
n 而「{you}」，一直是被注视着的那一方。
```

## 4. 开发循环

```bash
# ① 手动看（cwd = 引擎仓）
ES_DATA_DIR=/root/galgame/game/data ./target/release/galengine

# ② 指定某一段排练（label 级）
ES_SCRIPT=20_mio.ks ES_START_LABEL=*mio_scene2 ES_DATA_DIR=... ./galengine

# ③ 无头冒烟：自动点击跑完，看 ended
ES_SCRIPT=xx.ks ES_DEBUG=1 ES_DEBUG_AUTOCLICK_MS=400 ES_DATA_DIR=... \
  xvfb-run -a ./target/release/galengine 2>&1 | grep ended

# ④ 重置进度：删 savedata/（存档/global.json/meta.json 全在里面）
```

操作速查：点击/空格推进｜A 自动｜按住 Ctrl 快进｜Esc 菜单｜F9/F10 快存快读｜F5-F8 音量。

## 5. config.json（游戏侧唯一配置源）

```json
{
  "title": "你的游戏名",
  "script_start": "10_common.ks",
  "save_dir": "savedata",
  "save_slots": 9,
  "fonts": ["game/data/system/font.ttf", "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", "C:\\Windows\\Fonts\\msyh.ttc"],
  "dialog": { "box_rect": [56, 514, 1168, 172], "text_x": 104, "text_y": 542,
              "font_size": 30, "name_font_size": 30, "lines_per_page": 3 },
  "typewriter_ms": 30,
  "auto_delay_ms": 1000,
  "audio": { "bgm_volume": 80, "se_volume": 90 },
  "game": {
    "hero_default": "拓海", "you_default": "你",
    "input_presets": { "heroName": ["拓海", "拓人", "海斗"], "playerName": ["你", "读者", "旅行者"] },
    "name_colors": { "澪": [150,214,255], "时雨": [186,164,255] },
    "shutdown_message": "晚安。",
    "ritual_texts": ["真的要删除吗？", "再想一下，删除就回不来了。", "最后确认。亲手，删掉它。"]
  }
}
```

- `input_presets` 的键 = input 指令的变量基名（`f.heroName` → `heroName`）
- `ritual_texts` 三条 = meta_delete_last 的三重确认文案（游戏语气自己定）

## 6. 发布

```bash
cargo build --release           # Windows: 用 windows target 交叉构建或 CI
galengine.exe + game/data/      # exe <2MB，数据同级，双击即玩
```

## 7. 从其他引擎迁移

从 KAG/吉里吉里系（`@image [l][r]` 标签语法）迁移的剧本**不能直接运行**，两条路：
1. **按本 DSL 重写**（推荐）——语法量小，一页速查；大纲与文案直接搬，只换写法
2. 编写旧语法 → 本 DSL 的转换脚本（按需自建）

可运行的范本在引擎仓 `testdata/game/data/scenario/`（a3 演出链 / a4 跨文件 / a5 分支输入 / a6 全功能），可直接当写法模板。
