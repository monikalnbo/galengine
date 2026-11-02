# v0.3.4 表达式求值器 + 条件分支验收：elsif/emb/eval/trace + 数组/字典/三元
*start
eval f.score = 80
eval f.tags = ['combat', 'romance']
eval f.cfg = %[ name: '夏', hp: 100 ]

# eval 赋值与复合运算
eval f.score += 5
trace 分数=%f.score，首标签=%f.tags[0]，姓名=%f.cfg.name
if f.score > 100
n 满分线之上。
elsif f.score == 85
n 恰好 85 分。
else
n 不到 85。
endif

# elsif 链多分支
eval f.lv = 2
if f.lv > 3
n 高级。
elsif f.lv > 1
n 中级。
elsif f.lv > 0
n 初级。
endif

# emb：表达式结果嵌入台词
n 玩家设定演示开始。
emb 'HP=' + f.cfg.hp
n 上面一行是 emb 输出。

# 三元 + 成员访问 + 数组 length
n 判定结果看 trace 日志。
trace 数组长度=%f.tags.length，hp 加成=%f.cfg.hp + 20
end
