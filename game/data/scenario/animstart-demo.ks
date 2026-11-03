*start
bg classroom
cg cg_alice 0
n 自由层载入动画档（copylay 而来的 cg 内容）
copylay 4 5 0
animstart 5 swing
wa
n 档位播完（位移+透明度双轨道）
wait 300
animstop 5
layer 5 hide 0
bg bg_space 500 crossfade
wait 600
n 再来一遍
animstart 5 swing
wa
wait 200
animstop 5
layer 5 hide 0
end
