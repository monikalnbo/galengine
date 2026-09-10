# 开场起始剧本
n 欢迎使用 galengine 视觉小说引擎！
n 这是一个开箱即用的新手示范剧本。
n 引擎代码零改动，修改 game/data/ 目录下的资源和剧本即可创造属于你的世界。

choice 你接下来想了解什么？
  「查看剧本核心语法」|*syntax
  「了解创作工作流」|*workflow
  「开始自由创作」|*author
endchoice

*syntax
name 引擎小助手 剧本采用简明易读的 DSL：
name 引擎小助手 - bg 素材 [淡入ms]：切换背景
name 引擎小助手 - char 0-2 素材 [位置]：立绘登场
name 引擎小助手 - choice ... endchoice：分支选项
name 引擎小助手 - set / if：变量与好感度分支
jump *finish

*workflow
name 引擎小助手 创作只需 3 步：
name 引擎小助手 1. 把背景放进 game/data/bg/，立绘放进 game/data/char/
name 引擎小助手 2. 用记事本或 VSCode 编辑 game/data/scenario/start.ks
name 引擎小助手 3. 双击 galengine.exe 立即看效果！
jump *finish

*author
name 引擎小助手 祝你的故事创作顺利，打动每一位玩家！
jump *finish

*finish
n 详细创作教程与 26 条指令说明见 GitHub 仓库的 docs/game-authoring.md。
end
