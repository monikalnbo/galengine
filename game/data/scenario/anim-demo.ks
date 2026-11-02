# v0.3.6 双页模型 + 动画层验收剧本：backlay/copylay/move/wa/animstop
*start
# ── 1. backlay：fore 页复制到 back 页 ──
bg classroom
backlay 0
n 双页就绪（back 页 = classroom）

# ── 2. copylay：cg 层复制到自由层 ──
cg cg_alice
copylay 4 5
n 复制层完毕

# ── 3. move 关键帧位移 + wa 等待 ──
move 5 0,0@0 120,60@400
wa
n 动画走完
jump *phase2

*phase2
# ── 4. animstop：无 t=0 帧 → 从当前位置起步，随后取消 ──
move 5 200,100@300
animstop 5
n 取消动画
end
