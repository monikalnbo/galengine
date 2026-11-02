# v0.3.5 宏系统验收剧本：定义 / 属性代入 / 缺省 / 嵌套 / 递归防护 / erasemacro / ignore / clearvar
*start
# ── 1. 宏定义与属性代入 ──
macro greet
eval f.greeted = f.greeted + 1
n 问候 %who 次数 %n
endmacro
eval f.greeted = 0
greet who=夏 n=2

# ── 2. 未传属性按空串缺省（KAG3 口径）──
greet

# ── 3. 数值属性进表达式 ──
macro hit
eval f.total = f.total + %dmg
endmacro
eval f.total = 0
hit dmg=7
hit dmg=5
n 合计 %dmg 无关 总计已累计
jump *phase2

# ── 4. 嵌套宏 + 宏内 if 块 ──
*phase2
macro inner
eval f.log = f.log + "i"
endmacro
macro outer
inner
inner
if %flag == 1
eval f.log = f.log + "T"
else
eval f.log = f.log + "F"
endif
endmacro
eval f.log = ""
outer flag=0
n 宏内条件分支完毕

# ── 5. ignore 注释块：块内指令不执行 ──
ignore
eval f.log = f.log + "X"
n 这句不该出现
endignore
n 忽略块已跳过

# ── 6. clearvar 清 f.*（sf 不动）──
eval sf.marker = "kept"
clearvar
n 清变量完毕

# ── 7. erasemacro ──
macro gone
n 旧宏
endmacro
erasemacro gone
n 宏已删除
end
