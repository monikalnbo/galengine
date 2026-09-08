# galengine 重构需求书 v1.1

> 本文是引擎的唯一需求来源与设计记录。

---

## 0. 项目定位（拍板，不可改）

- **通用 galgame 引擎**，单二进制，服务多部游戏。换游戏=换 `data/` 目录+`game.yaml`，引擎代码零改动
- 技术栈：**Rust + SDL2**（用户指定；Windows 玩家双击 exe，体积 <10MB）
- 游戏内容（角色名颜色/预设名/文案/立绘/剧本）**只存在于数据侧**，引擎里禁止出现任何游戏特定字符串
- 引擎仓（本仓）与游戏仓分离；引擎自带 `testdata/` 自包含可测

## 1. 分层架构（用户拍板：每功能一模块，文件百行量级；workspace 10 crate）

```
L1  gal-engine crate    组装门面：app(主循环) / input(路由：choice/input/台词三态) /
                       frame(帧组装) / systems(系统动作) / exts(拓展注册)
L2  gal-script crate    lexer / command(全指令枚举) / interp(状态机) / expr / vars(f.* sf.*) / stage
    gal-ui crate        dialog / choice / inputbox / title / menu / savemenu / gallery /
                       settings / bottombar / layers(图层叠加) / cursor(两态鼠标) /
                       overlay(覆盖层状态机) / ritual / restui
L3  gal-render crate    renderer(1280×720离屏+letterbox+越界层) / stage_draw(crossfade) /
                       assets(纹理库) / prefetch(后台解码) / blur(虚化) / grade(降饱和)
    gal-text crate      font(字体库+纹理缓存) / layout(中文断行禁则) / writer(打字机)
    gal-audio crate     SDL2_mixer 封装（无声卡降级静音）
    gal-save crate      slots(存档槽+指纹) / meta(meta状态持久化)
L4  gal-config crate    顶层唯一配置（data/game.yaml）
    gal-platform crate  关机/桌面文件（Windows真执行，验收环境仅日志）
    gal-ext crate       Extension 契约（tick/draw/on_key/on_click）+ timer_choice 参考实现
```

规则：只准向下依赖；L2 不碰 SDL 事件泵；L3 互相独立；所有报错中文（含剧本文件:行号）。
游戏只依赖 gal-engine 一个 crate（`run_with(Boot)`）。

## 2. 功能需求清单（全部必须有）

### 2.1 渲染
- 1280×720 逻辑分辨率，窗口自由缩放 16:9 letterbox 居中黑边
- 层序：背景→立绘×3→CG→对话框→系统UI；素材统一 1280×720 整幅画布
- **越界层 reach**：图片放大伸出游戏区画进黑边（打破第四面墙），`reach hide` 收回
- 场景级资源管理：背景变更=切场景，回收不用的图片/文字纹理；后台线程预解码（消除切图卡帧）

### 2.2 剧本 DSL（自有语法生态，26+条指令）
```
bg 素材 [ms]          char 0-2 素材|hide [x y|位置名]    cg 素材 [ms] / cg hide
bgm 名 / bgm stop / bgm fadeout ms / se 名（缺文件静默）   clear   wait ms(不可跳过)
n 旁白                name 名字 台词
flag 变量 +2|-1       set 变量 = 表达式(?? + - * / 引号字符串)
jump [*文件] *label   if 变量 == 值 *label（== != >= <= > <）
choice 提示\n 项|*目标 ... endchoice
input 变量 提示|宽|默认值
meta_fake_save 日期|时间|图   meta_corrupt N|all   meta_delete_last
title_evolve 阶段    reach 素材 [ms] [倍率] / reach hide
window_fx shake ms / window_fx title 文字 / window_fx title restore
shutdown [秒]        desktop_write 文件|内容    desktop_open 文件
end
```
- `{hero}`→f.heroName、`{you}`→sf.playerName 运行时替换（台词/desktop_write/window_fx title）
- 变量：f.* 随存档；sf.* 存 `savedata/global.json` **退出即写盘**；未赋值变量数值比较按 0（免疫崩溃）
- UTF-8 全链路（读取层剥 BOM）

### 2.3 交互
- 点击/空格推进；打字机默认 30ms/字（可调）；点击=补全→翻页→下一句
- 选项：↑↓+回车 **和** 鼠标 hover+点击双通道
- 名字输入：SDL_TEXTINPUT（IME 透传）+ **预设名按钮兜底**
- 自动模式 A 键（行完延迟 1000ms 推进+AUTO角标）；**快进按住 Ctrl**（wait/meta 段不可跳）

### 2.4 存档/meta 演出（本作灵魂）
- F9 快存/F10 快读 + 9 槽界面（缩略图+时间）；存档=位置+f.*+演出层快照+剧本指纹
- 读档校验剧本指纹，不匹配中文拒读
- meta_fake_save：列表插入「不明存档」不可读；meta_corrupt：槽位标「破損」
- meta_delete_last：**玩家亲手**三重确认→全屏渐白光页→真删文件（删前自动备份 backup/）
- title_evolve：标题画面按阶段演化（褪色/变绿/小字）
- CG 鉴赏：cg 指令自动解锁记 sf.cgs，网格+锁定剪影+全屏翻页
- 音量：BGM/SE/文字速度三滑条+F5-F8 快捷键，存 sf.*

### 2.5 打破第四面墙（Windows 目标群体）
- shutdown N：自绘倒计时 UI+取消按钮；Windows 真执行 `shutdown /s /t N`，验收环境只写日志
- desktop_write/open：向玩家桌面写/打开文本文件（文件名防路径穿越）
- window_fx：窗口抖动/改标题/恢复

## 3. 配置（data/game.yaml 唯一配置源）
meta{title,version,script_start,donation_*} / display{窗口,letterbox,bg_saturation} / ui{dialog 几何字号,choice,bottombar,title,menu,input,cursor 两态鼠标,button,layers 图层叠加} / sprites{layers,default_fade_ms,positions 预设位置,chibi Q版名单} / audio{音量,目录,fade_ms} / fonts[]（候选链：随包→Linux Noto→Windows msyh） / save{dir,slots,auto_save_ms} / auto{delay_ms} / game{hero_default,you_default,input_presets,name_colors,shutdown_message,ritual_texts,skip_read_only,menu_items,title_items} / extensions[]{name,拓展自解释字段}
缺失字段自动回退内置默认（零配置可跑）；全字段示例见创作手册 §5。

## 4. 验收标准（每步完成必须过）
1. `run_tests.sh`：xvfb 三剧本回归（a3 演出链/a4 跨文件/a5 分支输入）全 ended
2. 关键帧 ffmpeg x11grab 截图 + VLM 判读（VLM 是眼睛不是法官，关键结论 PIL 像素交叉验证）
3. `cargo test --workspace` 纯逻辑单测 + SDL dummy 驱动 E2E（expr/lexer/vars/slots/…）
4. CI（fmt --check + clippy -D warnings + 全量编译测试）；打 tag 交叉出包
4. 验收辅助环境变量：ES_SCRIPT / ES_START_LABEL / ES_DATA_DIR / ES_DEBUG / ES_DEBUG_AUTOCLICK_MS
5. 通过才 commit+push；**永不覆盖正式文件**（测试走副本）

## 5. 已踩坑（必读，来自旧实现错题本）
- rust-sdl2 `load_font_at_index(path, index, point_size)` 参数序与 C API **相反**
- render target 只有闭包式 `with_texture_canvas`，无裸 set_render_target
- 无 GPU（xvfb）canvas 必须回退 software；截图要 sleep≥2s（软件渲染首帧慢）
- ffmpeg 逐次启动有 ~0.4s 开销，连拍用单进程 `-framerate`
- 首次背景淡入：crossfade 计时不能依赖「有旧图」
- 字体 TTC 面序：Noto CJK 的 SC=索引2，用 face_family_name 探测勿硬编码
- JSON 配置不允许注释行；读取剥 BOM
- 每帧构建 Ctx 时禁止文件 I/O（曾每帧读 9 个存档 JSON）

## 6. 编码纪律（Karpathy+Ponytail）
先想后写/假设说出来；200 行能 50 行就重写；单实现不建接口；外科手术式修改（不顺手改无关代码）；每步可验证；**没有根因禁止修复**（先建红绿循环）；纯逻辑先补单测；已知简化天花板标 `// ponytail:` 注释；文件名全英文；产出后跑 simplify 四维自查（复用/质量/效率/深度）。

## 7. 未完成清单
- app.rs 仍在瘦身（目标 ≤350 行）；overlay.rs key/click 重复模式可再合并
- F9/F10 全链路、音频真实播放、Rest 取消、删档「回头」路径**未实测**
- kslint 静态检查工具（label/变量/资源存在性）未写
- Windows CI 未建；IME/DPI/雅黑/关机需真机回归
