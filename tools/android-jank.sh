#!/usr/bin/env bash
# 真机顿挫速测（Flutter 示例 App，Android）：一个场景只跑一次，几十秒出结论。
#
#   bash tools/android-jank.sh <场景> [install]
#     场景：index-slow | wxss-slow | wxss-fastslow | index-fastslow | http-slow | http-fastslow | resume | sheet
#           enter-query | enter-feedback | enter-storage | enter-form（进页面 + 返回，各两遍）
#           query-slow | query-fastslow | swiper-slow | swiper-fastslow（在子页面里滑）
#           perf-idle | perf-slow（综合场景页：CSS 动画 + 每秒 setData；idle = 不碰屏幕 3s，看小核上够不够）
#
# 判据只看两样：
#   ① 屏上：SurfaceFlinger `--latency` 量引擎那一层每次上屏的时刻，「重复」= 相邻两次上屏隔了一个以上的显示器周期
#      （不靠 App 内日志：ColorOS 上 App 日志量大了会整段静默）；
#   ② 引擎：`debug.mini.damage_log` 里超过 8ms 的帧（哪一类、哪一段贵）。
# 坐标按 1080×2354 的示例首页写死。设备序列号用 ANDROID_SERIAL。
set -u
cd "$(dirname "$0")/.."
SERIAL="${ANDROID_SERIAL:-3B669L00NA000000}"
D(){ "$HOME/Library/Android/sdk/platform-tools/adb" -s "$SERIAL" "$@"; }
F(){ D shell dumpsys window </dev/null | grep -m1 mCurrentFocus | sed 's/.*\///'; }
PKG=dev.minirender.mini_render_example
SC="${1:-wxss-slow}"
OUT=target/_jank; mkdir -p $OUT
APK=sdk/flutter/example/build/app/outputs/flutter-apk/app-release.apk
if [ "${2:-}" = install ]; then
  # APK 里的引擎库必须是刚编的那份：引擎库与 `flutter build apk` 并行跑时，gradle 会把**旧库**合进去，
  # 量出来的是上一版（踩过：两次「对比」其实是同一个库，差异全是噪声）。比可执行段的偏移与大小
  RE=$(ls "$HOME"/Library/Android/sdk/ndk/*/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-readelf 2>/dev/null | tail -1)
  if [ -n "$RE" ]; then
    a=$(unzip -p $APK lib/arm64-v8a/libmini_render.so > $OUT/apk_lib.so && $RE -lW $OUT/apk_lib.so | grep "R E"); rm -f $OUT/apk_lib.so
    b=$($RE -lW sdk/android/src/main/jniLibs/arm64-v8a/libmini_render.so | grep "R E")
    [ "$a" = "$b" ] || { echo "⚠ APK 里的引擎库不是 jniLibs 里那份（先单独重跑 flutter build apk）"; exit 1; }
  fi
  D install -r $APK | tail -1
fi
D shell setprop debug.mini.damage_log 1
D shell am force-stop $PKG
D shell monkey -p $PKG -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1
for i in $(seq 1 30); do sleep 0.3; F | grep -q "$PKG" && break; done
sleep 1.3; D shell input tap 500 400; sleep 2.5
F | grep -q "$PKG" || { echo "⚠ 前台不是示例 App"; D shell setprop debug.mini.damage_log '""'; exit 1; }
# 子页面：首页先滑到底（位置固定），再点那一行。首页目录（utils/catalog.js）一改，这两个坐标就要跟着改 ——
# 进没进对页面按 flutter 日志里的 `page <路由>` 判，点歪了直接报错，不会量错页面
case "$SC" in
  wxss-*) GO_Y=522; GO_PAGE=pages/wxss ;;
  http-*) GO_Y=1150; GO_PAGE=pages/http ;;
  perf-*) GO_Y=1670; GO_PAGE=pages/perf ;;
  *) GO_Y="" ;;
esac
# 进页面（enter-<页>）：先到入口所在的 tab，量「点入口 → 转场 → 返回」两遍
ENTER_Y=""
case "$SC" in
  enter-query) TAB_X=675; ENTER_Y=1410; GO_PAGE=pages/query ;;
  enter-feedback) TAB_X=675; ENTER_Y=730; GO_PAGE=pages/feedback ;;
  enter-storage) TAB_X=675; ENTER_Y=1080; GO_PAGE=pages/storage ;;
  enter-form) TAB_X=405; ENTER_Y=1070; GO_PAGE=pages/form ;;
esac
if [ -n "$ENTER_Y" ]; then
  D shell input tap $TAB_X 2270; sleep 1.5
fi
# 在子页面里滑：query =「节点与滚动」（onPageScroll / IntersectionObserver 回调改数据），swiper =「轮播」（自动播放）
IN_TAB=""
case "$SC" in
  query-*) IN_TAB=675; IN_Y=1410; IN_PAGE=pages/query ;;
  swiper-*) IN_TAB=405; IN_Y=1410; IN_PAGE=pages/swiper ;;
esac
if [ -n "$IN_TAB" ]; then
  D shell input tap $IN_TAB 2270; sleep 1.5; D shell input tap 540 $IN_Y; sleep 2
  D logcat -d -s flutter:I | grep -q "page ${IN_PAGE}" || { echo "⚠ 没进到 ${IN_PAGE}"; exit 1; }
fi
if [ -n "$GO_Y" ]; then
  for i in 1 2 3; do D shell input swipe 540 1900 540 400 120; sleep 0.5; done; sleep 1.5; D shell input tap 540 $GO_Y; sleep 2
  D logcat -d -s flutter:I | grep -q "page $GO_PAGE" || { echo "⚠ 没进到 ${GO_PAGE}（首页目录变了？改上面的坐标）"; exit 1; }
fi
layers() { D shell dumpsys SurfaceFlinger --list </dev/null | grep "SurfaceView\[$PKG" | grep BLAST | sed 's/^RequestedLayerState{//; s/ parentId=.*$//; s/ relativeParentId=.*$//' | tr -d '\r'; }
layers > $OUT/layers.txt
while read -r L; do D shell "dumpsys SurfaceFlinger --latency-clear '$L'" >/dev/null </dev/null; done < $OUT/layers.txt
D logcat -c
# SurfaceFlinger 只留最近 127 帧：动作期间每 0.6s 取一次，事后按上屏时刻去重拼起来
rm -f $OUT/sf_*.txt
( k=0; while [ ! -f $OUT/stop ]; do k=$((k+1)); i=0; while read -r L; do i=$((i+1)); D shell "dumpsys SurfaceFlinger --latency '$L'" > $OUT/sf_${i}_$k.txt </dev/null; done < $OUT/layers.txt; sleep 0.6; done ) &
POLL=$!
# 任何原因退出（含 set -u 报错）都要停掉后台轮询，否则它一直跑、脚本卡住
trap 'touch $OUT/stop' EXIT
rm -f $OUT/stop
case "$SC" in
  *-slow) D shell input swipe 540 1800 540 700 2500; sleep 0.5; D shell input swipe 540 700 540 1800 2500; sleep 0.5 ;;
  *-fastslow)
    for i in 1 2 3; do D shell input swipe 540 1800 540 600 90; sleep 0.25; done
    D shell input swipe 540 1700 540 900 2000; sleep 0.4
    for i in 1 2; do D shell input swipe 540 600 540 1800 90; sleep 0.25; done
    D shell input swipe 540 900 540 1700 2000; sleep 0.5 ;;
  sheet) D shell input tap 385 1330; sleep 0.8; D shell input tap 540 500; sleep 0.8 ;;
  *-idle) sleep 3 ;;
  enter-*)
    for k in 1 2; do D shell input tap 540 $ENTER_Y; sleep 1.6; D shell input keyevent KEYCODE_BACK; sleep 1.2; done
    D logcat -d -s flutter:I | grep -q "page $GO_PAGE" || echo "⚠ 没进到 ${GO_PAGE}（入口坐标变了？）" ;;
  resume)
    # 回桌面再点图标回来：量回来之后第一秒
    D shell input keyevent KEYCODE_HOME; sleep 1.5
    layers > $OUT/layers.txt
    D logcat -c
    T0=$(python3 -c 'import time; print(time.time())')
    D shell monkey -p $PKG -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1
    sleep 1.2
    layers > $OUT/layers.txt ;;
esac
touch $OUT/stop; wait $POLL 2>/dev/null; rm -f $OUT/stop
best=""
i=0
while read -r L; do
  i=$((i+1)); D shell "dumpsys SurfaceFlinger --latency '$L'" > $OUT/sf_${i}_last.txt </dev/null
  r=$(python3 - $OUT/sf_${i}_ <<'EOF'
import sys, statistics, glob
P = set()
for f in glob.glob(sys.argv[1] + '*.txt'):
    L = [l.split() for l in open(f) if l.strip()]
    P |= {int(x[1]) for x in L[1:] if len(x) == 3 and 0 < int(x[1]) < 2**62}
P = sorted(P)
if len(P) < 4: sys.exit()
g = [(b - a) / 1e6 for a, b in zip(P, P[1:])]
# 动作期间：去掉动作之间的长空档（>150ms 是没在动）
gg = [x for x in g if x < 150]
v = statistics.median(gg)
rep = sum(round(x / v) - 1 for x in gg if x > v * 1.5)
print(f"{len(P)} 帧 周期 {v:.2f}ms 屏上重复 {rep} 次（{100*rep/max(1,len(gg)+rep):.1f}%） 最长 {max(gg):.0f}ms")
# 每个空档的起止（单调钟 ns），给下面对时间线用
with open(sys.argv[1] + 'gaps.tsv', 'w') as f:
    for a, b in zip(P, P[1:]):
        if v * 1.5 < (b - a) / 1e6 < 150:
            f.write(f"{a}\t{b}\t{P[0]}\n")
EOF
)
  [ ${#r} -gt ${#best} ] && { best="$r"; bestgaps=$OUT/sf_${i}_gaps.tsv; }
done < $OUT/layers.txt
echo "【${SC}】屏上：${best:-没上屏}"
D logcat -d -v raw -s mini-render:I > $OUT/log.txt
# 空档时间线：每个「屏上重复」前后 引擎在干什么（logcat 单调钟与 SurfaceFlinger 同一个钟）。
# 空档里一行引擎日志都没有 = 引擎那几帧没出画面（没被叫醒 / 没有新内容），不是画得慢
D logcat -d -v monotonic -s mini-render:I flutter:I > $OUT/log_mono.txt
[ -n "${bestgaps:-}" ] && [ -f "$bestgaps" ] && python3 - "$bestgaps" $OUT/log_mono.txt <<'EOF'
import re, sys
gaps = [tuple(int(x) for x in l.split()) for l in open(sys.argv[1]) if l.strip()]
ev = []
for l in open(sys.argv[2], encoding='utf-8', errors='replace'):
    m = re.match(r'\s*(\d+)\.(\d+)\s+\d+\s+\d+\s+\w\s+[\w-]+\s*:\s*(.*)', l)
    if m:
        ev.append((int(m.group(1)) * 10**9 + int(m.group(2).ljust(9, '0')[:9]), m.group(3)))
for a, b, t0 in gaps[:8]:
    strip = lambda x: re.sub(r'滚动 [\d.-]+ 条带 [\d.]+\.\.\d+  ', '', x)[:90]
    near = [f"{(t - a) / 1e6:+.0f}ms {strip(x)}" for t, x in ev if a - 12e6 <= t <= b + 2e6]
    print(f"  空档 @{(a - t0) / 1e9:6.2f}s  {(b - a) / 1e6:4.0f}ms  " + (" | ".join(near[:3]) if near else "（这段引擎没日志）"))
EOF
python3 - $OUT/log.txt <<'EOF'
import re, sys, collections
n = collections.Counter(); slow = collections.Counter(); worst = {}
lines = []
for l in open(sys.argv[1], encoding='utf-8', errors='replace'):
    if '🩹' not in l: continue
    m = re.search(r'🩹 \[SDK\] (\S+).*?([\d.]+)ms\s*$', l)
    if not m: continue
    k, t = m.group(1), float(m.group(2))
    n[k] += 1
    if t > 8.0:
        slow[k] += 1
        lines.append((t, l.strip()))
print("引擎：" + "，".join(f"{k} {n[k]} 帧（>8ms {slow[k]}）" for k in n))
for t, l in sorted(lines, reverse=True)[:5]:
    l = re.sub(r'.*🩹 \[SDK\] ', '', l)
    l = re.sub(r'滚动 [\d.-]+ 条带 [\d.]+\.\.\d+', '', l)
    print("  " + l[:170])
EOF
# 转场：起步一步 / 最大一步与中位一步之比（tools/route-steps.py）
python3 tools/route-steps.py $OUT/log.txt
D shell setprop debug.mini.damage_log '""'
