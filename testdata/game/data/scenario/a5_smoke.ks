# A5 冒烟：input/choice/flag/set/if 全链路
bg bg_classroom 700
n 新学期第一天。在名字被写上黑板之前——
input f.heroName 你的名字|320|拓海
n 好。从今天起，你就是{hero}了。
set aff_mio = 0
choice 教室门被拉开，要看向哪一边？
  窗边的澪|*c_mio
  讲台边的千岁|*c_chi
  门口的夏乃|*c_kan
endchoice
*c_mio
flag aff_mio +2
char 0 mio_happy
name 澪 早上好，{hero}。坐我旁边吧，窗户这边凉快。
jump *after
*c_chi
char 0 chitose_laugh
name 千岁 哟，新面孔！要加入轻音部吗？
jump *after
*c_kan
char 0 kanano_happy
name 夏乃 迟到迟到！啊，是你！我们在花火大会见过的，对吧！
jump *after
*after
if aff_mio >= 3 *mio_route
n 蝉声里，第一节课开始了。这个夏天的齿轮，咬合上了。
end
*mio_route
n 好感旗标判定通过：澪 +3，专属路线入口打开。
end
