#!/usr/bin/env bash
# 无头回归：对外部数据目录跑指定剧本（用法：./run_tests.sh <DATA目录> [剧本...]）
set -e
cd "$(dirname "$0")"
DATA="${1:-${ES_DATA_DIR:-}}"
[ -n "$DATA" ] || { echo "用法: $0 <数据目录> [剧本.ks ...]（目录契约见 docs/game-authoring.md）"; exit 1; }
shift || true
SCRIPTS="${@:-}"
[ -n "$SCRIPTS" ] || SCRIPTS=$(ls "$DATA/scenario/"*.ks | xargs -n1 basename 2>/dev/null || true)
[ -n "$SCRIPTS" ] || { echo "数据目录无剧本：$DATA/scenario"; exit 1; }
cargo build --release --quiet || exit 1
export ES_DATA_DIR="$DATA"
export ES_DEBUG=1
export ES_DEBUG_AUTOCLICK_MS=600
fail=0
for s in $SCRIPTS; do
  ok=$(xvfb-run -a -s "-screen 0 1280x800x24" bash -c \
    "ES_SCRIPT=$s timeout 15 ./target/release/galengine 2>&1 | grep -c ended" || true)
  echo "$s -> ended:$ok"
  [ "$ok" -ge 1 ] || { echo "FAIL: $s"; fail=1; }
done
[ "$fail" = 0 ] && echo ALL_PASS || exit 1
