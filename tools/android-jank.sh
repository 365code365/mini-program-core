#!/usr/bin/env bash
# 真机顿挫速测（Flutter 示例 App，Android）：一个场景只跑一次，几十秒出结论。
#
#   bash tools/android-jank.sh <场景> [install]
#     场景：index-slow | wxss-slow | wxss-fastslow | index-fastslow | http-slow | http-fastslow | resume | sheet
#           enter-query | enter-feedback | enter-storage | enter-form | enter-skyline（进页面 + 返回，各两遍）
#           query-slow | query-fastslow | swiper-slow | swiper-fastslow（在子页面里滑）
#           preview-swipe（图片预览：开、左右滑、下拉关、点关闭键）
#           perf-idle | perf-slow（综合场景页：CSS 动画 + 每秒 setData；idle = 不碰屏幕 3s，看小核上够不够）
#
# 判据只看两样：
#   ① 屏上：SurfaceFlinger `--latency` 量引擎那一层每次上屏的时刻，「重复」= 相邻两次上屏隔了一个以上的显示器周期
#      （不靠 App 内日志：ColorOS 上 App 日志量大了会整段静默）；
#   ② 引擎：`debug.mini.damage_log` 里超过 8ms 的帧（哪一类、哪一段贵）。
# 坐标按 1080×2354 的示例首页写死。设备序列号用 ANDROID_SERIAL。
#
# `STORE_APP=<id>`（原生与 Flutter 示例都行）：量商店里下载来的小程序，如 `STORE_APP=news-app MINI_APP=native … store-long`
#   （`*-slow` / `*-fastslow` / `*-long` 这几类只在当前页上下滑，任何小程序都能用）。
# `MINI_APP=native`：量纯原生示例（sdk/android/example，MiniProgramView），同一个演示小程序、同一组坐标。
# APK 用 `cd sdk/android/example && ./gradlew assembleRelease` 出的那份（debug 包是 DEBUGGABLE，量出来偏慢）
set -u
cd "$(dirname "$0")/.."
SERIAL="${ANDROID_SERIAL:-3B669L00NA000000}"
ADB_BIN="$HOME/Library/Android/sdk/platform-tools/adb"
# 脚本里的点击 / 滑动坐标按 1080 宽的屏写死（一加 PLY110 设了 `wm size 1080x2354`）。屏幕分辨率被改回原生
# （1272×2772）之后，按宽度等比换算 —— 页面按 dp 排版、两档 dp 宽只差 1%，换算后落在同一个控件上。
# 不换算的话 tabBar 点不到、「没进到 pages/query」，整段量的是首页
SCREEN_W=$("$ADB_BIN" -s "$SERIAL" shell wm size 2>/dev/null | awk -F'[ x]' '/Override/{w=$3} /Physical/{if(!w)w=$3} END{print w}')
TAP_K=$(python3 -c "print(${SCREEN_W:-1080}/1080)")
D(){
  if [ "$1" = shell ] && [ "$2" = input ] && { [ "$3" = tap ] || [ "$3" = swipe ]; } && [ "$TAP_K" != "1.0" ]; then
    local op=$3; shift 3
    local args; args=$(python3 -c "import sys; k=$TAP_K; a=sys.argv[1:]; n=4 if '$op'=='swipe' else 2; print(' '.join([str(round(float(v)*k)) for v in a[:n]]+a[n:]))" "$@")
    "$ADB_BIN" -s "$SERIAL" shell input $op $args
    return
  fi
  "$ADB_BIN" -s "$SERIAL" "$@"
}
F(){ D shell dumpsys window </dev/null | grep -m1 mCurrentFocus | sed 's/.*\///'; }
SC="${1:-wxss-slow}"
OUT=target/_jank; mkdir -p $OUT
if [ "${MINI_APP:-flutter}" = native ]; then
  PKG=dev.minirender.example
  APK=sdk/android/example/app/build/outputs/apk/release/app-release.apk
  # 宿主日志（`[事件] page <路由>`）；原生示例一启动就是小程序首页，没有 Flutter 那层入口列表
  HOSTLOG=mini-example:I; OPEN_TAP=""
  # 桌面图标进的是入口页（StoreActivity，服务器上的小程序列表），量的是内置示例：直接起 MainActivity。
  # 回桌面再回来也用同一条 —— 意图与任务的根意图一样，系统只把任务带回前台，不会再起一个
  LAUNCH(){ D shell am start -n $PKG/.MainActivity >/dev/null 2>&1; }
  # `STORE_APP=news-app`：量从服务器下载的小程序（与入口页点图标同一条路：MainActivity 带 storeApp 起来，
  # 盖 loading 层、下载解包、打开）。列表从商店服务取（STORE_URL 可换），已是最新版就不再下载
  if [ -n "${STORE_APP:-}" ]; then
    STORE_URL="${STORE_URL:-http://64.83.22.217:9630}"
    STORE_JSON=$(curl -s -m 8 "$STORE_URL/api/apps" | python3 -c "import json,sys; a=[x for x in json.load(sys.stdin)['apps'] if x['id']=='$STORE_APP']; print(json.dumps(a[0],ensure_ascii=True,separators=(',',':')) if a else '')")
    [ -n "$STORE_JSON" ] || { echo "⚠ 商店里没有 $STORE_APP（或连不上 $STORE_URL）"; exit 1; }
    # `STORE_PATH=pages/detail/detail?id=101`：直接打开到这一页（深链同一条路：storeLink 带 path / query）
    if [ -n "${STORE_PATH:-}" ]; then
      LINK_JSON=$(STORE_PATH="$STORE_PATH" STORE_JSON="$STORE_JSON" python3 -c "
import json, os, urllib.parse
app = json.loads(os.environ['STORE_JSON']); p = os.environ['STORE_PATH']; path, _, q = p.partition('?')
print(json.dumps({'key': app.get('appId') or app['id'], 'path': path, 'query': dict(urllib.parse.parse_qsl(q)), 'scene': 1065}, separators=(',', ':')))")
      LAUNCH(){ D shell "am start -n $PKG/.MainActivity --es storeLink '$LINK_JSON' --es storeUrl '$STORE_URL'" >/dev/null 2>&1; }
    else
      LAUNCH(){ D shell "am start -n $PKG/.MainActivity --es storeApp '$STORE_JSON' --es storeUrl '$STORE_URL'" >/dev/null 2>&1; }
    fi
  fi
else
  PKG=dev.minirender.mini_render_example
  APK=sdk/flutter/example/build/app/outputs/flutter-apk/app-release.apk
  HOSTLOG=flutter:I; OPEN_TAP="500 400"
  LAUNCH(){ D shell monkey -p $PKG -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1; }
  # Flutter 示例也能量商店里的小程序：按 AppID 走深链（minirender://open，与外部 App / H5 拉起同一条路）
  if [ -n "${STORE_APP:-}" ]; then
    STORE_URL="${STORE_URL:-http://64.83.22.217:9630}"
    STORE_KEY=$(curl -s -m 8 "$STORE_URL/api/apps/$STORE_APP" | python3 -c "import json,sys; print(json.load(sys.stdin).get('appId',''))")
    [ -n "$STORE_KEY" ] || { echo "⚠ 商店里没有 $STORE_APP（或连不上 $STORE_URL）"; exit 1; }
    OPEN_TAP=""
    SP=$(STORE_PATH="${STORE_PATH:-}" python3 -c "import os,urllib.parse; p=os.environ['STORE_PATH']; print('&path='+urllib.parse.quote(p,safe='') if p else '')")
    LAUNCH(){ D shell "am start -a android.intent.action.VIEW -d 'minirender://open?appid=$STORE_KEY$SP' $PKG" >/dev/null 2>&1; }
  fi
fi
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
D logcat -c
LAUNCH
for i in $(seq 1 30); do sleep 0.3; F | grep -q "$PKG" && break; done
sleep 1.3; [ -n "$OPEN_TAP" ] && D shell input tap $OPEN_TAP; sleep 2.5
# 商店小程序：第一次要下载（几 MB），等到第一条 page 事件（loading 层收起）再量
if [ -n "${STORE_APP:-}" ]; then
  for i in $(seq 1 60); do D logcat -d -s $HOSTLOG </dev/null | grep -q "\[事件\] page " && break; sleep 0.5; done
  sleep 1.5
fi
F | grep -q "$PKG" || { echo "⚠ 前台不是示例 App"; D shell setprop debug.mini.damage_log '""'; exit 1; }
# `ENTER_TAP="x y"`：先点一下进子页面再量（如首页点一篇新闻进详情：首页的自动轮播还在底下翻，
# 直接深链到详情页量不出这种「被盖住那一页逼着重画」的顿）
if [ -n "${ENTER_TAP:-}" ]; then
  D logcat -c; D shell input tap $ENTER_TAP; sleep 2.5
  D logcat -d -s $HOSTLOG </dev/null | grep "\[事件\] page " | tail -1
fi
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
  # 最重的一页（movable / 手势 / grid-view / sheet）：组件 tab 滑到底，倒数第二行
  enter-skyline) TAB_X=405; ENTER_Y=1858; GO_PAGE=pages/skyline/skyline; ENTER_BOTTOM=1 ;;
esac
if [ -n "$ENTER_Y" ]; then
  D shell input tap $TAB_X 2270; sleep 1.5
  if [ -n "${ENTER_BOTTOM:-}" ]; then
    for i in 1 2 3; do D shell input swipe 540 1900 540 400 120; sleep 0.5; done; sleep 1.5
  fi
fi
# 在子页面里滑：query =「节点与滚动」（onPageScroll / IntersectionObserver 回调改数据），swiper =「轮播」（自动播放）
IN_TAB=""
case "$SC" in
  query-*) IN_TAB=675; IN_Y=1410; IN_PAGE=pages/query ;;
  swiper-*) IN_TAB=405; IN_Y=1410; IN_PAGE=pages/swiper ;;
  # 图片预览（wx.previewImage）：「媒体」页底部那排缩略图，点第一张打开
  preview-*) IN_TAB=405; IN_Y=1692; IN_PAGE=pages/media/media ;;
esac
# 宿主页（host-*）：地图那一段（同层渲染的 <map>：宿主画、引擎画进页面）。拖动不从地图上开始（盖上去的地图会吃掉触摸）
case "$SC" in
  host-*)
    D shell input tap 945 2270; sleep 1.5
    for i in 1 2; do D shell input swipe 540 1150 540 350 600; sleep 0.4; done; sleep 1
    D logcat -d -s $HOSTLOG | grep -q "page pages/host/host" || { echo "⚠ 没进到宿主页"; exit 1; } ;;
esac
if [ -n "$IN_TAB" ]; then
  D shell input tap $IN_TAB 2270; sleep 1.5; D shell input tap 540 $IN_Y; sleep 2
  D logcat -d -s $HOSTLOG | grep -q "page ${IN_PAGE}" || { echo "⚠ 没进到 ${IN_PAGE}"; exit 1; }
fi
if [ -n "$GO_Y" ]; then
  for i in 1 2 3; do D shell input swipe 540 1900 540 400 120; sleep 0.5; done; sleep 1.5; D shell input tap 540 $GO_Y; sleep 2
  D logcat -d -s $HOSTLOG | grep -q "page $GO_PAGE" || { echo "⚠ 没进到 ${GO_PAGE}（首页目录变了？改上面的坐标）"; exit 1; }
fi
# 引擎那一层：SurfaceView 的 BLAST 层；没有 SurfaceView（老的 Bitmap + onDraw 上屏，画在窗口里）就量窗口那一层
layers() {
  local all sv
  all=$(D shell dumpsys SurfaceFlinger --list </dev/null | tr -d '\r')
  sv=$(echo "$all" | grep "SurfaceView\[$PKG" | grep BLAST)
  [ -z "$sv" ] && sv=$(echo "$all" | grep -E "^RequestedLayerState\{$PKG/[^ ]+#[0-9]+ parentId")
  echo "$sv" | sed 's/^RequestedLayerState{//; s/ parentId=.*$//; s/ relativeParentId=.*$//' | grep -v '^$'
}
layers > $OUT/layers.txt
while read -r L; do D shell "dumpsys SurfaceFlinger --latency-clear '$L'" >/dev/null </dev/null; done < $OUT/layers.txt
D logcat -c
# SurfaceFlinger 只留最近 127 帧：动作期间每 0.6s 取一次，事后按上屏时刻去重拼起来
rm -f $OUT/sf_*.txt
# 停止标记要在起轮询**之前**删：上一次退出时 trap 留下了它，晚删的话轮询一起来就看见、立刻退出，
# 整段只剩最后一次 dump 的 127 帧（踩过：好几轮的「屏上重复」都只量了最后 1.4 秒）
rm -f $OUT/stop
( k=0; while [ ! -f $OUT/stop ]; do k=$((k+1)); i=0; while read -r L; do i=$((i+1)); D shell "dumpsys SurfaceFlinger --latency '$L'" > $OUT/sf_${i}_$k.txt </dev/null; done < $OUT/layers.txt; sleep 0.6; done ) &
POLL=$!
# 任何原因退出（含 set -u 报错）都要停掉后台轮询，否则它一直跑、脚本卡住
trap 'touch $OUT/stop' EXIT
case "$SC" in
  *-slow) D shell input swipe 540 1800 540 700 2500; sleep 0.5; D shell input swipe 540 700 540 1800 2500; sleep 0.5 ;;
  *-fastslow)
    for i in 1 2 3; do D shell input swipe 540 1800 540 600 90; sleep 0.25; done
    D shell input swipe 540 1700 540 900 2000; sleep 0.4
    for i in 1 2; do D shell input swipe 540 600 540 1800 90; sleep 0.25; done
    D shell input swipe 540 900 540 1700 2000; sleep 0.5 ;;
  sheet) D shell input tap 385 1330; sleep 0.8; D shell input tap 540 500; sleep 0.8 ;;
  # 「滑一会儿才顿」：同一页连续甩 + 慢拖 40 秒左右，看后半段是不是比前半段差（每轮一行，见下面的分段统计）
  *-long)
    for k in $(seq 1 10); do
      date +%s.%N > $OUT/round_$k
      for i in 1 2; do D shell input swipe 540 1800 540 600 90; sleep 0.3; done
      D shell input swipe 540 1700 540 900 1500; sleep 0.3
      for i in 1 2; do D shell input swipe 540 600 540 1800 90; sleep 0.3; done
      D shell input swipe 540 900 540 1700 1500; sleep 0.3
    done ;;
  *-idle) sleep 3 ;;
  # 打开预览 → 左右各滑三张（快甩 + 慢拖）→ 下拉关闭 → 再开 → 点右上角关闭
  preview-swipe)
    D shell input tap 206 2230; sleep 1
    for i in 1 2 3; do D shell input swipe 900 1200 200 1200 150; sleep 0.6; done
    for i in 1 2 3; do D shell input swipe 200 1200 900 1200 500; sleep 0.6; done
    D shell input swipe 540 1000 560 1700 400; sleep 0.8
    D shell input tap 206 2230; sleep 1; D shell input tap 1007 205; sleep 0.8 ;;
  enter-*)
    # 返回点导航栏的返回箭头，不发 KEYCODE_BACK：没有触摸时 ColorOS 把屏降到 60Hz，
    # 退场转场量出来一半是「重复」，可那是系统降频不是引擎慢（真人返回都是点 / 侧滑）
    for k in 1 2; do D shell input tap 540 $ENTER_Y; sleep 1.6; D shell input tap 50 190; sleep 1.2; done
    D logcat -d -s $HOSTLOG | grep -q "page $GO_PAGE" || echo "⚠ 没进到 ${GO_PAGE}（入口坐标变了？）" ;;
  resume)
    # 回桌面再点图标回来：量回来之后第一秒
    D shell input keyevent KEYCODE_HOME; sleep 1.5
    layers > $OUT/layers.txt
    D logcat -c
    T0=$(python3 -c 'import time; print(time.time())')
    LAUNCH
    sleep 1.2
    layers > $OUT/layers.txt ;;
  resume-scroll)
    # 「回桌面再进来，一滑就顿」：先在首页滑一段，回桌面停 3 秒，回来立刻甩 + 慢拖
    D shell input swipe 540 1800 540 700 600; sleep 0.5
    D shell input keyevent KEYCODE_HOME; sleep 3
    LAUNCH
    sleep 0.8; layers > $OUT/layers.txt
    while read -r L; do D shell "dumpsys SurfaceFlinger --latency-clear '$L'" >/dev/null </dev/null; done < $OUT/layers.txt
    D logcat -c
    D shell input swipe 540 1800 540 600 90; sleep 0.3
    D shell input swipe 540 600 540 1800 90; sleep 0.3
    D shell input swipe 540 1700 540 900 1500; sleep 0.3
    D shell input swipe 540 900 540 1700 1500; sleep 0.5 ;;
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
# 前后两半各自的重复（「滑一会儿才顿」看后半段是不是变差）
half = len(g) // 2
for name, part in (("前半", g[:half]), ("后半", g[half:])):
    pp = [x for x in part if x < 150]
    r = sum(round(x / v) - 1 for x in pp if x > v * 1.5)
    print(f"  {name}：{len(pp)} 帧 重复 {r} 次（{100*r/max(1,len(pp)+r):.1f}%）")
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
D logcat -d -v monotonic -s mini-render:I $HOSTLOG > $OUT/log_mono.txt
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
# 整个 pump 慢的那几次（⏱ 行：含重画之前的换页、事件回调、失效分析）
grep -a "⏱ \[SDK\] pump" $OUT/log.txt | sed 's/.*⏱ \[SDK\] //; s/（含重画前的换页 \/ 事件回调 \/ 失效分析；*/（/' | sort -t' ' -k2 -rn | head -6 | sed 's/^/  /' | cut -c1-300
D shell setprop debug.mini.damage_log '""'
