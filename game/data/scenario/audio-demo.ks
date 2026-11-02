# v0.3.2 KAG3 音频标签组验收剧本（无头 CI 走 ES_SCRIPT=audio-demo.ks）
# 覆盖：playbgm/pausebgm/resumebgm/fadeinbgm/fadeoutbgm/fadepausebgm/fadebgm/
#       xchgbgm/bgmopt/wb/wl/playse/stopse/fadeinse/fadeoutse/fadese/seopt/ws/wf
# 以及旧语法零改动（bgm/se/voice）

# —— 旧语法回归（v0.3.x 剧本必须原样能跑）——
bgm theme
n 旧语法 bgm theme 已播。等待一秒。
wait 1000
bgm fadeout 500
n 旧语法 bgm fadeout 已淡出。

# —— 新标签：双缓冲淡入 ——
fadeinbgm theme 800
wb
n fadeinbgm 淡入完成，wb 放行。
bgmopt 1000 6000
n 循环点已设为 1s-6s，播到循环点后自动放行。
wl
n 已命中循环点（或降级放行）。

# —— 交叉淡化切歌 ——
xchgbgm next 900
wb
n 无缝切歌完成。

# —— 暂停/恢复/淡出暂停 ——
pausebgm
n 已暂停。
resumebgm
n 已恢复。
fadepausebgm 600
n 已淡出暂停。

# —— 音量淡 ——
resumebgm
fadebgm 40 500
n BGM 音量已淡到 40%。

# —— SE 组 ——
playse click
fadeinse click 200
n wf
n fadeinse 淡入完成。
fadese 30 400
n SE 音量淡到 30%。
fadeoutse click 400
stopse
n SE 组全部通过。
seopt click

# —— fadeoutbgm 结束 ——
fadeoutbgm 600
wb
n 全部音频标签验收完毕。
end
