#!/usr/bin/env bash
# 真机验宿主原生组件：<map>（SDK 自带的瓦片地图，同层渲染 + 手势 + MapContext）、<web-view>（系统 WebView + JSSDK）、
# <live-player>（系统播放器拉 HLS，同层渲染）。内置示例的三页带 `?auto=1&chain=1` 时自己跑一遍、每页打一行 `NC-CHECK …`。
#
#   bash tools/android-native-components.sh            # 原生示例（dev.minirender.example，先装好）
#   MINI_APP=flutter bash tools/android-native-components.sh
#       # Flutter 示例：要先编进起始页 `flutter build apk --release --dart-define=MINI_ROUTE='pages/map/map?auto=1&chain=1'` 装上
#
# 判据（全过才 exit 0）：
#   - 原生示例先在地图页上拖一下、点一下：日志里有 `regionchange begin causedBy=gesture` → `end causedBy=drag` 与 `tap <经纬度>`
#     （按在地图上的整串触摸交给了地图，不是页面在滚）；
#   - NC-CHECK map：getCenterLocation / moveToLocation / getRegion / includePoints / toScreenLocation / translateMarker 都回了对的值；
#   - NC-CHECK webview：bindload 到了、网页 postMessage 两条在返回时一次送达、getEnv 是 miniprogram；
#   - NC-CHECK live：拉到流（2004）、snapshot 拿到图、mute / pause / resume 成功。
# 等待一律按墙上时钟轮询日志（logcat 流写进本地文件再 grep，一加的 logcat 环几十秒就冲掉）。
set -u
cd "$(dirname "$0")/.."
ADB=${ADB:-adb}
APP=${MINI_APP:-native}
OUT=target/_native_components
mkdir -p "$OUT"
LOG="$OUT/$APP.log"
if [ "$APP" = flutter ]; then PKG=dev.minirender.mini_render_example; else PKG=dev.minirender.example; fi
fail=0

wait_for() { # 等日志里出现 $1，最多 $2 秒
    local end=$(( $(date +%s) + $2 ))
    while [ "$(date +%s)" -lt "$end" ]; do grep -q "$1" "${LOG}" && return 0; sleep 0.5; done
    return 1
}

"$ADB" shell am force-stop $PKG
"$ADB" logcat -c
"$ADB" logcat -v time > "${LOG}" 2>&1 &
LC=$!
trap 'kill $LC 2>/dev/null' EXIT

if [ "$APP" = native ]; then
    # ── 手势：拖地图、点地图 ──
    "$ADB" shell am start -n $PKG/.MainActivity --es route pages/map/map >/dev/null
    wait_for "updated（地图画好了）" 30 || { echo "❌ 地图没画出来（30s 内没有 bindupdated）"; exit 1; }
    read -r W H < <("$ADB" shell wm size | awk '{print $NF}' | tr 'x' ' ' | tr -d '\r')
    # 地图在导航栏下面、占屏高 0.10~0.44：在 0.25 那一行横着拖，再点一下 0.32 那一行
    "$ADB" shell input swipe $((W * 7 / 10)) $((H * 25 / 100)) $((W * 3 / 10)) $((H * 28 / 100)) 400
    if wait_for "regionchange end causedBy=drag" 8 && grep -q "regionchange begin causedBy=gesture" "${LOG}"; then
        echo "✅ 拖地图：regionchange begin(gesture) → end(drag)"
    else
        echo "❌ 拖地图没有 regionchange（按在地图上的触摸没交给地图？）"; fail=1
    fi
    sleep 1
    "$ADB" shell input tap $((W / 2)) $((H * 32 / 100))
    if wait_for '\[map\] tap 3' 5 || wait_for '\[map\] markertap' 1 || wait_for '\[map\] callouttap' 1; then
        echo "✅ 点地图：$(grep -oE '\[map\] (tap|markertap|callouttap)[^"]*' "${LOG}" | tail -1)"
    else
        echo "❌ 点地图没有 tap / markertap"; fail=1
    fi
    "$ADB" shell am force-stop $PKG
    # 整条命令交给手机上的 shell 时 `&` 要在引号里：不然它把后半截当成后台命令，chain=1 就丢了
    "$ADB" shell "am start -n $PKG/.MainActivity --es route 'pages/map/map?auto=1&chain=1'" >/dev/null
else
    "$ADB" shell monkey -p $PKG -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1
fi

# ── 三页自测串着跑 ──
if ! wait_for "NC-CHECK done" 120; then
    echo "❌ 120s 内没跑完（看 ${LOG}）"; fail=1
fi
for k in map webview live; do
    line=$(grep -oE "NC-CHECK $k \{[^}]*(\}[^\"]*)?" "${LOG}" | tail -1 | sed 's/\\"/"/g')
    if [ -z "$line" ]; then echo "❌ $k：没有结果"; fail=1
    elif echo "$line" | grep -q '"ok":true'; then echo "✅ $line"
    else echo "❌ $line"; fail=1; fi
done
grep -E "🗺 同层组件" "${LOG}" | tail -2
exit $fail
