#!/usr/bin/env bash
# 小游戏长稳（game-doc/04 A9.2，05 §7）：真机连续跑一个夹具 N 分钟（默认 30），每 5 分钟采 PSS / RSS，
# 头尾各 60 秒量引擎每帧工作时间（「⏱ [GAME] 工作」，扣掉等缓冲）。结果写 target/_game_soak/<夹具>.txt。
#   bash tools/android-game-soak.sh <夹具> [分钟] [query]
# 判据：后 20 分钟的 RSS 斜率折算 ≤ 20MB / 20 分钟、进程没换（没崩）、尾段 p99 不劣于头段（容差 1ms）。
set -u
cd "$(dirname "$0")/.."
SERIAL="${ANDROID_SERIAL:-3B669L00NA000000}"
A="$HOME/Library/Android/sdk/platform-tools/adb"
D() { "$A" -s "$SERIAL" "$@"; }
GAME="${1:-e-phaser}"; MIN="${2:-30}"; QUERY="${3:-}"
PKG=dev.minirender.example
OUT=target/_game_soak; mkdir -p "$OUT"; R="$OUT/$GAME.txt"; : > "$R"
D shell setprop debug.mini.game_log 1
D shell am force-stop $PKG
D shell "am start -W -n $PKG/.GameActivity --es game '$GAME' --es query '${QUERY:-max=1000000000}'" >/dev/null
sleep 3
PID=$(D shell pidof $PKG | tr -d '\r')
echo "start pid=$PID game=$GAME min=$MIN $(date +%T)" >> "$R"
window() { # 名字
    D logcat -c; sleep 60
    D logcat -d -s mini-render:V | grep -oE '工作 [0-9.]+ms' | grep -oE '[0-9.]+' > "$OUT/$1.work"
    python3 -c "
import sys,statistics
v=sorted(float(x) for x in open('$OUT/$1.work') if x.strip())
print('$1', 'frames', len(v), 'median %.2f p99 %.2f max %.2f >8ms %d' % (statistics.median(v), v[int(len(v)*.99)], v[-1], sum(1 for x in v if x>8)) if v else 'no data')" >> "$R"
}
sample() {
    local p; p=$(D shell pidof $PKG | tr -d '\r')
    local pss rss; pss=$(D shell dumpsys meminfo $PKG | grep -m1 "TOTAL PSS" | awk '{print $3}'); rss=$(D shell "grep VmRSS /proc/$p/status" | awk '{print $2}')
    echo "t=$1min pid=$p pss_kb=$pss rss_kb=$rss fg=$(D shell dumpsys window | grep -m1 mCurrentFocus | grep -c GameActivity)" >> "$R"
}
sample 0
window head
start=$(date +%s)
for m in $(seq 5 5 "$MIN"); do
    while [ $(( $(date +%s) - start )) -lt $(( m * 60 - 60 )) ]; do sleep 10; done
    sample $m
done
window tail
sample end
D shell setprop debug.mini.game_log '""'
echo "done $(date +%T)" >> "$R"
