# 更新日志

遵循 [语义化版本](https://semver.org/lang/zh-CN/)。旧 `game.yaml`、旧存档、旧 `savedata/global.json` 均可直接使用。

## v0.3.8 — 2026-11-03

**视频子系统批**（backlog P2-4）：KAG3 视频标签组 8 项 + `wv` 变真等待。
选型：ffmpeg-sidecar（CLI 子进程 rawvideo 管道，MIT，零 FFI 静态包——对比评估见 reports/galengine-video-eval-2026-11-03/）。

### 新增

- **feature `video`（默认关）**：`cargo build --features video` 开启；关闭时全部视频标签 WARN/DEBUG 放行不崩（三路日志方针）。
- 8 个标签：`video`（参数记录）/ `playvideo 名称 [ms] [skip]` / `stopvideo` / `openvideo 名称` / `pausevideo` / `resumevideo` / `preparevideo 名称` / `rewindvideo`。
- `wv`：feature 开启时轮询播放器在播态（真等待）；关闭时立即放行（v0.3.3 降级口径不变）。
- 实现：ffmpeg 子进程 → rgb24 管道 → SDL 纹理逐帧上传，等比铺满窗口；找不到 ffmpeg 或子进程失败 → WARN 降级为未在播，演出不中断。
- 片源探测：`data/video/`、`data/movie/` 目录，mp4/webm/mkv/avi 扩展名（`gal-config::paths` `video` kind）。
- 已知口径：resumevideo/rewindvideo 以重播实现（无播放头记忆）；音频轨暂不混音（KAG3 惯例配 playbgm）；视频层盖在演出层之上。

### 兼容性

- 默认构建（不开 feature）行为与 v0.3.7 完全一致；存档格式零改动。
- 旧剧本/旧配置/旧存档零改动兼容（v0.3.2 时代数据目录实测验证，savedata 字节级不变）。

## v0.3.7 — 2026-11-03

**双页原子转场批**：KAG3 `forelay` / 无前缀 `trans`（backlog P1-2 后半，消费 v0.3.6 back 页）。
「后页准备（backlay）→ 一次翻上（trans/forelay）」核心演出形态补齐。
存档格式零改动（在途转场照旧不进存档，fore 页立即等于转场后状态）。旧剧本/旧配置/旧存档零改动兼容（实测验证）。

### 新增标签（2 个形态）

- `forelay [层] [ms] [method]`：back 页内容一次翻上到 fore 页（层号缺省=0 背景；
  ms 缺省 700；method 同 trans 方法族 universal/crossfade/slide/scroll）。
- 无前缀 `trans [ms] [method]`：把已备好的 back 页翻上（与 forelay 同语义，KAG3 原生参数形态；
  首参不是合法层号时按此形态解析，`trans 500 crossfade` / `trans 400` 均可）。
- 空后页 = 淡出到空（KAG3 口径）；`wt` 照旧等在途转场；玩家关转场（sf.transOn=0）时直切。

### 兼容性

- `Layer.back` 数据模型不变；转场后 back 页清空（KAG3 语义：翻上即消费）。
- 旧「trans 层号 素材」语法全部保留（`trans 0 bg/b 500 crossfade` 照旧），消歧规则见解析层注释。
- 存档快照结构零改动（backs 字段 v0.3.6 已有）。

## v0.3.6 — 2026-11-02

**双页模型 + 动画层批**：KAG3 fore/back 双页与 move 位移动画（backlog P1-2/P1-3 前半）。
存档快照新增 `backs` 字段（`#[serde(default)]`，旧档读出为全空，恢复行为与旧版一致）。

### 新增标签（5 个）

- `backlay [层] [hide]`：fore 页内容复制到 back 页（层号缺省=0 背景；`hide` 清空 back 页）。
- `copylay 源层 目标层 [ms]`：源层 fore 页复制到目标层 fore 页（ms 缺省 0 直切，可淡入）。
- `move 层 x,y@ms …`：关键帧位移动画（帧间线性插值；缺 `@ms` 延续上一帧时间；
  缺 t=0 帧自动补当前 offset——从当前位置起步）。各层独立在途，重复调用覆盖同层动画。
- `wa`：等全部在途位移动画结束（无在途立即放行）。
- `animstop [层]`：取消在途动画（无参=全部层；offset 停在当前位置，DEBUG 留证）。

### 兼容性

- back 页平时不绘制（渲染零变化）；`Layer`/`Stage` 加字段均为新增，旧剧本/旧存档零改动。
- 在途动画是瞬时状态不进存档（存档 = 当时 offset 定格，恢复后不再动）。
- `StageSnap` 新增 `backs` 字段；v0.2.2 以来全部旧档可直接读。

## v0.3.5 — 2026-11-02

**宏系统批**：KAG3 宏语义对标子集（backlog #43-45、#38/39、#87/88）。
宏展开是运行期拦截（调用处「未知指令」命中宏表 → 属性文本层代入 → 压宏帧逐行执行），
等待点落宏体里时存档位置记主剧本调用行——**存档格式零改动**。旧剧本/旧配置/旧存档零改动兼容（实测验证）。

### 新增标签（7 个）

- `macro 名` … `endmacro`：宏定义（宏体行独立行空间，load 阶段注册进宏表）。
- 宏调用「宏名 属性…」：属性 `key=value` / `key="带 空格"` / `key='单引'` / 裸词布尔属性；
  宏体内 `%key` 文本层代入（最长键优先防 %hp/%hpmax 混淆，`%%` 转义字面 %）。
- 属性缺省：未传的 `%key` 按空串代入（KAG3 口径，不崩演出），DEBUG 留证落 engine.log。
- 嵌套宏调用与宏内 if 块（宏体行空间独立：块配对挂 `macro:宏名` 键，branch_taken 按作用域隔离）。
- 递归防护：`MAX_MACRO_DEPTH=16`（宏「名」嵌套超限中文报错，参照 call 栈防线）。
- `erasemacro 宏名`：删除宏（不存在时 DEBUG 留证并忽略）。
- `ignore` … `endignore`：解析期注释块，lexer 层整段跳过（块内任何内容不进指令流）。
- `clearvar` / `clearsysvar`：清 f.* / sf.*（global.json 结构不变，清空≠删字段，INFO 留证）。

### 兼容性

- 宏/ignore 全是既有文法超集，旧剧本不含这些指令名，行为零变化。
- 存档：宏帧是运行期状态不进存档；等待点记调用行，旧存档格式照常读写。
- 宏体内台词行的 %代入结果整体进台词文本（不拆指令名）。

## v0.3.4 — 2026-11-02

**表达式求值器批（用户已定档的单独一大批）**：选型对照 evalexpr / exmex / rhai 后按「效率优先 + 多库少自研」方针复核——
三者均无完整的 TJS 语义桥接层（f./sf. 变量映射、Int/Float 混算、空串=0 兼容、未赋值裸词按 0），
桥接代码量 ≥ 剩余自研量，故**自研扩展现有递归下降求值器**（零新依赖）。
求值器落稳后按 152 对照总表落地直接解锁的标签组。旧剧本/旧配置/旧存档零改动兼容（实测验证）。

### 求值器扩展（零新依赖）

- 三元运算符 `cond ? a : b`（右结合，惰性求值）。
- 数组字面量 `[a, b, c]`、下标 `arr[i]`（越界中文报错）。
- 字典字面量 `%[ key: value, ... ]`（裸键按字符串）、成员访问 `dict.key`、`.length`。
- 容器真值：空数组/空字典为假；逻辑运算贯通容器。
- 赋值表达式 `eval_assign`：`f.a = 5`、`f.a += 1`（含 `-= *= /= %=`）、`f.arr[0] = x`、`f.dict.k = v`。

### 新增标签（4 个）+ 存量升级

- `elsif 条件`：KAG3 elsif 链（多分支 if 块，配对表 load 阶段一次算好）。
- `emb 表达式`：求值结果嵌入当前台词（KAG3 emb）。
- `eval 表达式`：求值（可含赋值表达式），不产生输出（KAG3 eval）。
- `trace 文本`：调试输出落 engine.log（`script` 子系统 INFO），支持 `%表达式` 内联求值；
  求不出按原样保留 `%`（诊断通道绝不中断演出）。
- `else`/`endif` 存量实现补齐配对语义（elsif 链尾正确跳转，已命中分支栈防 fall-through 误入）。

### 明确不做（本批裁量）

- iscript/endscript（TJS 全语言内嵌，单独批次）。
- macro/endmacro/erasemacro（宏系统，依赖表达式参数代入，随 macro 批）。
- clearvar/clearsysvar（单标签低收益，随宏批顺手）。

## v0.3.3 — 2026-11-02

**等待标签收尾 + 转场扩展批**：KAG3 等待标签 `wm`/`wv`/`wp` 落地（v0.3.2 的 `WaitAudio` 轮询地基上增量实现）；
转场方法族在 crossfade/universal 之外新增 **slide（四向滑动）/ scroll（四向卷动）** 两家族共 8 个方法名。
旧 `game.yaml`、旧剧本、旧存档零改动兼容（实测验证）；静默失败留证方针延续（wv 无视频子系统 → 立即放行 + engine.log WARN）。

### 新增标签（3 个）

- `wm`：等 BGM 播完（含淡出落停；无声卡立即放行）。
- `wv`：等视频播完（本引擎暂无视频子系统 → 立即放行 + WARN 留证）。
- `wp`：等全部声音（BGM/SE/语音）播完。

### 转场扩展（8 个方法名）

- `slide l/r/u/d`：胶片式滑动——旧内容沿方向滑出、新内容整屏跟进。
- `scroll l/r/u/d`：擦除式卷动——旧内容原位、新内容从方向侧推入覆盖。
- 纯几何实现（零新依赖、零 shader）；未知方法名照旧退化交叉淡化。

## v0.3.2 — 2026-11-02

**音频库化批**：gal-audio 手写混音器整体退役，换用 [kira 0.10](https://github.com/tazz4843/kira)（内部 cpal 输出 + symphonia 解码）；
全新 KAG3 音频标签组（约 18 个标签）落地。旧 `game.yaml`、旧剧本（`bgm`/`se`/`voice`）、旧存档零改动兼容；
无声卡/缺文件照旧静默降级，且新增 gal-audit 多路日志证据链（`{数据目录}/log/engine.log` + `trace.log`）。

### 换库（自研 → 成熟库）

- **解码**：手写 ogg/PCM 解码 → kira 内置 symphonia（ogg/mp3/flac/wav）。
- **输出**：手写 cpal 流 + 环形缓冲混音 → kira AudioManager（cpal 后端）。
- **淡入淡出**：手写线性包络 → kira Tween（fade_in_tween / set_volume / stop / pause 均可挂时长）。
- **循环点**：手写回卷计数 → kira loop_region（命中检测仍按播放头跳变判定，供 `wl` 等待）。
- 手写保留：通道映射（BGM A/B 双缓冲、SE 多实例、语音独占）、KAG 等待语义轮询、缺文件查找、gal-audit 日志/trace 埋点。

### 新增 KAG3 音频标签组（旧 `bgm/se/voice` 语法原样可用）

- BGM：`playbgm [ms]`（双缓冲淡入）/ `pausebgm` / `resumebgm` / `fadeinbgm 名称 ms` /
  `fadeoutbgm ms` / `fadepausebgm ms`（淡出暂停，播放头停在淡出点）/ `fadebgm 百分比 ms` /
  `xchgbgm 名称 ms`（双缓冲交叉淡化无缝切歌）/ `bgmopt 起ms 止ms`（循环点）。
- SE：`playse 名称` / `stopse` / `fadeinse 名称 ms` / `fadeoutse 名称 ms` / `fadese 百分比 ms` / `seopt 名称`。
- 等待：`wb`（BGM 淡入淡出结束）/ `wl`（到达循环点）/ `ws`（SE 全部播完）/ `wf`（SE 淡入淡出结束）——
  无声卡降级环境立即放行，不卡演出。

### 多路日志（gal-audit）

- `{数据目录}/log/engine.log`：分级日志（`ES_LOG_LEVEL` 过滤），运行时失败全量留证。
- `{数据目录}/log/trace.log`：每个音频动作一条 `tag=k=v` 追踪记录（`[相对秒] subsys tag ...`）。
- stderr：WARN+ 镜像（兼容旧终端习惯），`ES_LOG_STDERR=1` 全量；日志自身失败自动降级为仅 stderr。

### 工程

- `run_tests.sh`：无头回归提速（autoclick 600ms→250ms）并放宽单剧本超时到 45s。
- 演示剧本 `scenario/audio-demo.ks` + 合成音频资产（bgm/theme.ogg、bgm/next.ogg、sound/click.ogg）。

## v0.3.1 — 2026-11-02

**功能批（补丁版号）**：清掉 v0.3.0 遗留的四项，并补齐四项 P2 能力（主题包 / 注音 / 多语言 / 无障碍朗读）。
旧 `game.yaml`、旧存档、旧 `savedata/global.json` 全部照常加载（v0.3.0 模板数据与旧 settings 已实测）。

### 遗留项清零

- **`ui.layers[*].anim` 真正生效**：`fade:<ms>` / `slide:<ms>:<dx,dy>` / `zoom:<ms>:<倍率>`，
  从图层组出现时刻起算、缓出入场；未写 `anim` 的画面与旧版逐像素一致。
- **语音被音效打断的提示**：`se` 播放时若有语音在放，左下角淡入提示「♪ 语音被音效「X」打断」1.5s。
- **任意系统字体**：设置面板「文字 → 系统字体 → 载入…」扫描系统字体目录（最多 16 族），
  载入后下一帧即可在「字体」行切换；选择写入 `savedata/fonts.json`，重启自动恢复。
- **设置面板分页重构**：行按组（文字/显示/音频/无障碍/键位/其他）统一排序、每页 11 行，
  新增项不再挤爆硬编码的每页 10 项。

### P2 功能

- **主题包**：`ui.skin.dir`（默认 `skins`）下每个 `*.yaml|*.yml` 可写单套皮肤（顶层 `name:`）
  或 `schemes:` 列表；按文件名排序追加，与 `ui.skin.schemes` 按名合并（同名后者覆盖），
  坏包跳过并提示，不影响启动。
- **注音（ruby）**：剧本 `[基底]^(注音)`，基底 1-8 字、注音 1-16 字；整组不可断行、
  宽度取基底与注音较大值；仅含注音的页额外预留行高（旧排版零变化）；TTS 朗读自动去注音。
- **多语言**：`lang/<code>.yaml`（`name/code/strings`）+ 剧本/选项/菜单/标题菜单中的
  `{@键}`、`{@键|兜底}`；设置「文字 → 语言」切换（`sf.language`，0=作者原文），
  当前台词/选项/UI 立即重排；未写宏的旧剧本逐字节不变。
- **无障碍朗读（TTS）**：不新增依赖，调用系统朗读命令（PowerShell SAPI / macOS `say` /
  `espeak-ng` / `espeak` / `spd-say`）；设置「无障碍 → 朗读台词 / 朗读语速」，
  无可用后端时该行标注「本机无可用后端」并自动关闭，不影响游戏。

### 兼容与体积

- 单二进制 1,610,520 → 1,722,824 字节（+6.97%）。
- v0.3.0 模板数据（无 `lang`/`skins`/`tts` 段）无头跑通；旧 `global.json` 缺新键时按作者默认。

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
