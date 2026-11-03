*start
n 超链接演示开始（v0.3.15 link 族）
link storage=*linked
hch 点我跳转
endlink
l
n 链接区已登记（无头回归由点击命中驱动）
n 普通推进到跳转前的最后一句
l
jump *after
*linked
n 已通过链接跳到这里
jump *after
*after
locklink
n locklink 后链接置灰不可点
unlocklink
n unlocklink 恢复；下面演示 exp 链接
set f.gold = 0
emb "金币初始=#{f.gold}"
l
link exp="f.gold = f.gold + 100"
hch 点我加币
endlink
l
emb "点击链接后金币=#{f.gold}"
l
if f.gold == 100
n exp 链接已生效（回归断言点）
else
n exp 链接未生效（异常路径）
endif
l
n 链接演示结束
end
