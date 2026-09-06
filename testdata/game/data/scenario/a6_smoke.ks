# A6 全功能冒烟：meta/越界/窗口/关机
bg bg_room 700
n 存档系统的深处，有什么东西醒了过来。
meta_fake_save 7月7日|23:59|bgimage/bg_black.jpg
n 一格不明的存档，混进了列表的末尾。
meta_corrupt all
n 而所有的存档，都开始碎裂。
meta_delete_last
n 那么——亲手，把它删掉吧。
n 光翻过了这一页。世界安静了下来。
desktop_write letter.txt|还记得那个夏天吗。{hero}
desktop_open letter.txt
window_fx title 该休息了
reach cg_p01 1500 1.8
wait 1500
n 有什么，伸出了屏幕。
shutdown 60
end
