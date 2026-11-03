
*start
n 消息层批 v0.3.16 演示
n 默认消息层位置，接下来用 position 改到左下。

position left=80 top=480 width=900 height=200
n 消息层已移到左下（left=80 top=480 900x200）。

position opacity=128
n 透明度降到一半，看得见背后了吧？

position visible=false
n 这行字你应该看不见消息层框（visible=false）。

position visible=true opacity=255
n 框回来了。

current layer=message0
n current 只是记录当前输出层（单消息层引擎，DEBUG 留证），演出不变。

position left=160 top=520 width=1000 height=180
n 再挪一次收尾。演示完毕。

if sf.msgLeft == 160
n 断言通过：sf.msgLeft=160 已落变量。
else
n 断言失败：sf.msgLeft 不对！
endif

jump *end

*end
n 演示结束。
