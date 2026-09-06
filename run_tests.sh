#!/usr/bin/env bash
# 三剧本回归：a3 演出链 / a4 跨文件 / a5 分支输入变量
set -e
cd "$(dirname "$0")"
export ES_DATA_DIR=testdata/game/data
export ES_DEBUG=1
export ES_DEBUG_AUTOCLICK_MS=600
for s in a3_smoke.ks a4_main.ks a5_smoke.ks; do
  ok=$(xvfb-run -a -s "-screen 0 1280x800x24" bash -c \
    "timeout 15 ./target/release/galengine 2>&1 | grep -c ended" || true)
  echo "$s -> ended:$ok"
  [ "$ok" -ge 1 ] || { echo "FAIL: $s"; exit 1; }
done
echo ALL_PASS
