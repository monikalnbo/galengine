# v0.3.7 双页原子转场验收剧本：backlay → 无前缀 trans / forelay → wt
*start
# ── 1. 无前缀 trans：备好 back 页 → 一次翻上 → wt 等转场完 ──
bg classroom
backlay 0
bg bg_space
trans 500 crossfade
wt
n 双页转场翻上（corridor 换 classroom 的后页）

# ── 2. forelay 指定层 + slide 方法 ──
backlay 0
bg school
forelay 0 400 slide-l
wt
n forelay 滑动翻上

# ── 3. 空后页 = 淡出到空（KAG3 口径） ──
trans 400
wt
n 淡出完毕
end
