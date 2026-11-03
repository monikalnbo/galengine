# v0.3.9 条件标签收尾验收剧本（无头 CI 走 ES_SCRIPT=layopt-demo.ks）
# 覆盖：layopt(visible/opacity)/laycount/stoptrans/move 缓动尾参

*start
n 条件标签收尾验收开始。

# —— layopt：隐藏立绘层 1 ——
char 0 alice_normal
n 立绘已上场。
layopt 1 visible=false
n 层 1 已隐藏（layopt visible=false）。
layopt 1 visible=true
n 层 1 已恢复。
layopt 1 opacity=128
n 层 1 半透明（opacity=128）。
layopt 1 opacity=255
n 层 1 恢复不透明。

# —— laycount：层数记录 ——
laycount 5
n laycount 5 已记录。

# —— stoptrans：中止在途转场停在目标态 ——
backlay 0
bg school
trans 5000 crossfade
stoptrans
n 转场已中止，背景停在目标图（stoptrans）。

# —— move 缓动尾参 ——
move 5 200,0@600 easeinout
wa
n easeinout 位移动画完成。
move 5 -200,0@600 easeout
wa
n easeout 位移动画完成。
# 旧写法零改动：无尾参 = linear（v0.3.6 剧本原样）
move 5 0,0@400
wa
n 无尾参 move（linear）完成。

n 条件标签收尾验收完毕。
end
