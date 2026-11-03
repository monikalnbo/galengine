# v0.3.8 KAG3 视频标签组验收剧本（无头 CI 走 ES_SCRIPT=video-demo.ks）
# 覆盖：video/playvideo(+ms/skip)/wv/stopvideo/pausevideo/resumevideo/
#       openvideo/preparevideo/rewindvideo
# 片源：video/testclip.mp4（ffmpeg 合成 1 秒纯色片，随仓库走）
# 注意：无头 CI 的引擎构建默认不开 video feature → 全部 WARN 放行（本剧本同时
#       验证「feature 关闭不崩」）；开启 video feature 时走真实播放路径。

*start
n 视频标签组验收开始。

# —— 参数设置（记录不生效口径）——
video mode=1
n video 参数已记录。

# —— playvideo + wv：主路径 ——
playvideo testclip 1000
wv
n playvideo 已启动（或降级放行），wv 已放行。

# —— skip 可点击跳过 ——
playvideo testclip skip
wv
n skip 版 playvideo 放行。

# —— open/prepare/pause/resume/rewind ——
openvideo testclip
wv
n openvideo 放行。
preparevideo testclip
n preparevideo 已记录。
pausevideo
n pausevideo 已执行。
resumevideo
n resumevideo 已执行。
rewindvideo
n rewindvideo 已执行。

# —— stopvideo 立即收尾 ——
playvideo testclip
stopvideo
n stopvideo 已停止。
wv
n 无在播视频时 wv 直通放行。

n 视频标签组验收完毕。
end
