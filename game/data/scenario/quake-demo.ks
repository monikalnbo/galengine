*start
bg classroom
n 震动开始（quake 600ms）
quake 600
wq
n 震动结束（wq 放行）
wait 300
n 再震一次然后中途取消
quake 3000
wait 400
stopquake
wq
n 已取消（stopquake 后 wq 立即放行）
n 指纹锁演示
locksnapshot 封印中
wait 500
unlocksnapshot
wc
n 点击推进（wc）
end
