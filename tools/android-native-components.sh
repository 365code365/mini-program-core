#!/usr/bin/env bash
# 真机验宿主原生组件：<map>（SDK 自带的瓦片地图，同层渲染 + 手势 + MapContext）、<web-view>（系统 WebView + JSSDK）、
# <live-player>（系统播放器拉 HLS，同层渲染）、<camera>（Camera2，同层渲染 + CameraContext）、<live-pusher>（RTMP 推流）。
# 内置示例的五页带 `?auto=1&chain=1` 时自己跑一遍、每页打一行 `NC-CHECK …`。
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
#   - NC-CHECK live：拉到流（2004）、snapshot 拿到图、mute / pause / resume 成功；
#   - NC-CHECK camera：initdone 的 maxZoom 是 number、onCameraFrame 收到 RGBA 帧、setZoom / takePhoto / 录 2.5 秒的 mp4 都成；
#   - NC-CHECK pusher：1003 / 1007 / 1008 / 1001 / 1002 / 1009 都报了、netstatus 码率 > 0、snapshot 成；
#     推流收在本机 ffmpeg（`-listen 1`，`adb reverse tcp:1935`），收到的文件 ffprobe 要有 h264 与 aac 两路；
#     pusher 页还开一段怀旧滤镜 + 背景音、再进一次小窗：bgmstart / bgmprogress / enter / leavepictureinpicture 都要到，
#     收端比滤镜前后 (R−B) 均值（大 4 以上）、背景音前后 RMS（高 10dB 以上）、小窗那段画面不断（帧间隔 < 0.5s）。
# 相机 / 麦克风权限先 `pm grant`（不弹框）。
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

# 系统的权限框（ColorOS 上 `pm grant` 要开发者选项里关掉权限监控，多半不行）：弹出来就点「使用时允许」那一类按钮
allow_permission() {
    "$ADB" shell dumpsys window 2>/dev/null </dev/null | grep -q "mCurrentFocus.*GrantPermissionsActivity" || return 0
    "$ADB" shell uiautomator dump /sdcard/_nc_ui.xml >/dev/null 2>&1 </dev/null
    local b
    b=$("$ADB" shell cat /sdcard/_nc_ui.xml 2>/dev/null </dev/null | tr '>' '\n' | grep -E 'text="(仅在使用中允许|使用时允许|仅使用期间允许|始终允许|允许|While using the app|Allow)"' | head -1 \
        | sed -E 's/.*bounds="\[([0-9]+),([0-9]+)\]\[([0-9]+),([0-9]+)\]".*/\1 \2 \3 \4/')
    [ -n "$b" ] || return 0
    set -- $b
    "$ADB" shell input tap $(( ($1 + $3) / 2 )) $(( ($2 + $4) / 2 )) </dev/null
    echo "（点了权限框的允许）"
}

wait_for() { # 等日志里出现 $1，最多 $2 秒；等的时候顺手点掉权限框
    local end=$(( $(date +%s) + $2 )) n=0
    while [ "$(date +%s)" -lt "$end" ]; do
        grep -q "$1" "${LOG}" && return 0
        n=$((n + 1)); [ $((n % 4)) -eq 0 ] && allow_permission
        sleep 0.5
    done
    return 1
}

"$ADB" shell am force-stop $PKG
# 相机 / 麦克风：先授权（真机上弹权限框会把自测卡住）
"$ADB" shell pm grant $PKG android.permission.CAMERA 2>/dev/null
"$ADB" shell pm grant $PKG android.permission.RECORD_AUDIO 2>/dev/null
# 推流的收端：本机 ffmpeg 当 RTMP 服务器，手机经 adb reverse 连 127.0.0.1:1935
FLV="$OUT/pusher_$APP.flv"
rm -f "$FLV"
FF=""
if command -v ffmpeg >/dev/null; then
    "$ADB" reverse tcp:1935 tcp:1935 >/dev/null
    ffmpeg -nostdin -loglevel error -listen 1 -i rtmp://127.0.0.1:1935/live/nc -c copy -f flv "$FLV" > "$OUT/ffmpeg_$APP.log" 2>&1 &
    FF=$!
fi
"$ADB" logcat -c
"$ADB" logcat -v time > "${LOG}" 2>&1 &
LC=$!
trap 'kill $LC 2>/dev/null; [ -n "$FF" ] && kill $FF 2>/dev/null; "$ADB" reverse --remove tcp:1935 >/dev/null 2>&1' EXIT

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
if ! wait_for "NC-CHECK done" 180; then
    echo "❌ 180s 内没跑完（看 ${LOG}）"; fail=1
fi
for k in map webview live camera pusher; do
    line=$(grep -oE "NC-CHECK $k \{[^}]*(\}[^\"]*)?" "${LOG}" | tail -1 | sed 's/\\"/"/g')
    if [ -z "$line" ]; then echo "❌ $k：没有结果"; fail=1
    elif echo "$line" | grep -q '"ok":true'; then echo "✅ $line"
    else echo "❌ $line"; fail=1; fi
done
grep -E "🗺 同层组件" "${LOG}" | tail -2
# 收到的流：要有 h264 与 aac（ffmpeg 在推流端断开后自己退出、把文件写完）
if [ -n "$FF" ]; then
    for _ in $(seq 1 20); do kill -0 $FF 2>/dev/null || break; sleep 0.5; done
    kill $FF 2>/dev/null; FF=""
    streams=$(ffprobe -v error -show_entries stream=codec_name,width,height -of csv=p=0 "$FLV" 2>/dev/null | tr '\n' ' ')
    dur=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$FLV" 2>/dev/null)
    if echo "$streams" | grep -q h264 && echo "$streams" | grep -q aac; then echo "✅ 收到的推流：$streams 时长 ${dur}s（${FLV}）"
    else echo "❌ 收到的推流不对：「$streams」（${FLV}，ffmpeg 日志 $OUT/ffmpeg_$APP.log）"; fail=1; fi
    # 美颜 / 滤镜 / 背景音 / 小窗（pusher.js 的时间线：开推 2.5s 后开怀旧滤镜 + 背景音 2s，全关 2s，再小窗 2.5s）：
    #   画面：滤镜那段 (R−B) 的均值比开之前大（怀旧偏棕）；声音：背景音那段的 RMS 比开之前高 10dB 以上；
    #   小窗那段画面不断（相邻两帧的时间戳差 < 0.5s）
    python3 - "$FLV" <<'PY' || fail=1
import subprocess, sys, json
flv = sys.argv[1]
def frame_rb(t):
    raw = subprocess.run(["ffmpeg", "-nostdin", "-loglevel", "error", "-ss", str(t), "-i", flv, "-frames:v", "1", "-vf", "scale=90:160",
                          "-f", "rawvideo", "-pix_fmt", "rgb24", "-"], capture_output=True).stdout
    n = len(raw) // 3
    return sum(raw[i * 3] - raw[i * 3 + 2] for i in range(n)) / max(n, 1) if n else None
def rms_db(a, d):
    out = subprocess.run(["ffmpeg", "-nostdin", "-ss", str(a), "-t", str(d), "-i", flv, "-af", "astats=metadata=0", "-f", "null", "-"],
                         capture_output=True, text=True).stderr
    vals = [float(l.split(":")[-1]) for l in out.splitlines() if "RMS level dB" in l and "inf" not in l]
    return vals[-1] if vals else -120.0
pts = subprocess.run(["ffprobe", "-v", "error", "-select_streams", "v", "-show_entries", "packet=pts_time", "-of", "csv=p=0", flv],
                     capture_output=True, text=True).stdout.split()
pts = sorted(float(p) for p in pts if p.strip() and p != "N/A")
gap = max((b - a for a, b in zip(pts, pts[1:])), default=99)
before, during = frame_rb(1.5), frame_rb(3.6)
qa, qb = rms_db(0.5, 1.5), rms_db(2.9, 1.5)
ok_fx = before is not None and during is not None and during - before > 4
ok_bgm = qb - qa > 10
ok_gap = pts and pts[-1] > 9 and gap < 0.5
print(("✅" if ok_fx else "❌") + f" 怀旧滤镜：R−B 均值 {before} → {during}")
print(("✅" if ok_bgm else "❌") + f" 背景音混进推流：RMS {qa:.1f} → {qb:.1f} dB")
print(("✅" if ok_gap else "❌") + f" 小窗时推流不断：画面到 {pts[-1] if pts else 0:.1f}s，最大帧间隔 {gap:.2f}s")
sys.exit(0 if ok_fx and ok_bgm and ok_gap else 1)
PY
fi
exit $fail
