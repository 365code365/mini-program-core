#!/usr/bin/env bash
# 小游戏真机速测（原生示例的 GameActivity，game-doc/05 §5.6）：一个场景几十秒出结论。
#
#   bash tools/android-game-jank.sh <夹具> [秒数] [install] [query]
#     夹具：sample-game/ 下的目录名（g-square、g-bench-sprites …）；query 例 "n=500"
#
# 判据（game-doc/05 §5.1，不看平均帧率）：
#   ① 屏上：SurfaceFlinger --latency 量游戏那一层的上屏时刻；「重复」按 **rAF 目标间隔**（60fps = 16.67ms）算，
#      不按显示器周期 —— 60fps 在 120Hz 上本来就隔一个 vsync；
#   ② rAF：夹具的 T:FRAME 行里的 dt（rAF 参数之差）：中位、峰值、> 1.5 倍目标的次数；
#   ③ 引擎：「⏱ [GAME] 工作」> 8ms 的帧（等缓冲的 vsync 背压已扣掉）。
# 量之前确认前台是 GameActivity（手机是几个会话共用的，steering 30），否则直接报错退出。
set -u
cd "$(dirname "$0")/.."
SERIAL="${ANDROID_SERIAL:-3B669L00NA000000}"
A="$HOME/Library/Android/sdk/platform-tools/adb"
D() { "$A" -s "$SERIAL" "$@"; }
GAME="${1:-g-square}"; SECS="${2:-6}"; QUERY="${4:-}"
PKG=dev.minirender.example
APK=sdk/android/example/app/build/outputs/apk/release/app-release.apk
OUT=target/_game_jank; mkdir -p "$OUT"
if [ "${3:-}" = install ]; then
  [ "$(unzip -p $APK lib/arm64-v8a/libmini_render.so | md5)" = "$(md5 -q sdk/android/src/main/jniLibs/arm64-v8a/libmini_render.so)" ] \
    || { echo "⚠ APK 里的引擎库不是 jniLibs 里那份（先单独重跑 ./gradlew assembleRelease）"; exit 1; }
  D install -r $APK | tail -1
fi
D shell setprop debug.mini.game_log 1
D shell am force-stop $PKG
D logcat -c
# query 里的 & 要在远端 shell 里也引起来（adb shell 把参数拼成一行交给远端 sh，不引的话 & 把命令切成后台）
D shell "am start -W -n $PKG/.GameActivity --es game '$GAME' --es query '${QUERY:-max=1000000000}'" >/dev/null
sleep 1.5
D shell dumpsys window </dev/null | grep -m1 mCurrentFocus | grep -q "$PKG/$PKG.GameActivity" || { echo "⚠ 前台不是 GameActivity"; D shell setprop debug.mini.game_log '""'; exit 1; }
L=$(D shell dumpsys SurfaceFlinger --list </dev/null | tr -d '\r' | grep "SurfaceView\[$PKG/$PKG.GameActivity" | grep BLAST | sed 's/^RequestedLayerState{//; s/ parentId=.*$//' | head -1)
[ -n "$L" ] || { echo "⚠ 找不到游戏的 SurfaceView 层"; exit 1; }
D shell "dumpsys SurfaceFlinger --latency-clear '$L'" >/dev/null </dev/null
D logcat -c
rm -f "$OUT"/sf_*.txt
# SurfaceFlinger 只留最近 127 帧：每 0.8s 取一次，事后按上屏时刻去重拼起来
end=$(( $(date +%s) + SECS )); k=0
while [ "$(date +%s)" -lt "$end" ]; do k=$((k+1)); D shell "dumpsys SurfaceFlinger --latency '$L'" > "$OUT/sf_$k.txt" </dev/null; sleep 0.8; done
D logcat -d -v raw 'mini-game:I' 'mini-render:I' '*:S' > "$OUT/log.txt"
RATE=$(D shell dumpsys display </dev/null | grep -oE 'mActiveRenderFrameRate=[0-9.]+' | head -1 | cut -d= -f2)
D shell setprop debug.mini.game_log '""'
python3 - "$OUT" "$GAME" "${RATE:-?}" <<'PY'
import glob, json, re, statistics, sys
out, game, rate = sys.argv[1:4]
P = set()
for f in glob.glob(out + '/sf_*.txt'):
    L = [l.split() for l in open(f) if l.strip()]
    P |= {int(x[1]) for x in L[1:] if len(x) == 3 and 0 < int(x[1]) < 2**62}
P = sorted(P)
tgt = 1000 / 60
g = [(b - a) / 1e6 for a, b in zip(P, P[1:])]
g = [x for x in g if x < 150]
rep = sum(round(x / tgt) - 1 for x in g if x > tgt * 1.5)
log = open(out + '/log.txt', encoding='utf-8', errors='replace').read().splitlines()
dt = []
for l in log:
    if 'T:FRAME ' in l:
        try: d = json.loads(l.split('T:FRAME ', 1)[1].split(' ', 1)[1]).get('dt', 0)
        except Exception: continue
        if d and d > 0: dt.append(d)
work = [float(m[1]) for m in (re.search(r'工作 ([\d.]+)ms', l) for l in log) if m]
slow = sum(1 for w in work if w > 8)
print(f"【{game}】显示 {rate}Hz  屏上 {len(P)} 帧，间隔中位 {statistics.median(g):.2f}ms 最长 {max(g):.1f}ms，按 60fps 重复 {rep} 次（{100*rep/max(1,len(g)+rep):.1f}%）")
if dt:
    print(f"   rAF {len(dt)} 次：dt 中位 {statistics.median(dt):.2f}ms 峰值 {max(dt):.2f}ms，>25ms {sum(1 for d in dt if d > 25)} 次")
if work:
    s = sorted(work)
    print(f"   引擎工作（扣等缓冲）：中位 {statistics.median(s):.2f} p99 {s[int(len(s)*.99)]:.2f} 最大 {s[-1]:.2f}ms，>8ms {slow} 帧 / {len(s)}")
PY
