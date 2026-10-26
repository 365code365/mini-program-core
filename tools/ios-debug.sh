#!/usr/bin/env bash
# iOS 一条龙：等 Xcode → 补环境 → 编引擎库 → 起模拟器 → `flutter run` 挂上调试 → 抓日志与截图。
#
#   bash tools/ios-debug.sh              # 全流程（Xcode 没装就打开 App Store 页面并等着）
#   bash tools/ios-debug.sh --no-lib     # 只改了 Dart / Swift：不重编引擎库
#   bash tools/ios-debug.sh --stop       # 停掉调试会话（flutter run 与日志采集）
#   bash tools/ios-debug.sh --shot       # 给正在跑的会话补一张截图
#   LTO=1 bash tools/ios-debug.sh        # 引擎库开 thin LTO（与发版同档，慢）
#
# 产物落 target/ios-debug/<时刻>/（`latest` 软链指向最新一次）：
#   steps.log   每一步的时刻与结论          lib.log     引擎库构建
#   flutter.log flutter run 的全部输出（含 Dart VM Service 地址，DevTools / 断点从这里连）
#   native.log  模拟器里 Runner 进程的系统日志（Swift / Rust 的 NSLog、崩溃）
#   screen.png  起来之后的截图
#
# 只有两步要人：Xcode 本体要 Apple ID（脚本打开 App Store 页面，点「获取」）；
# 首次启动 Xcode 要接受许可、输开机密码（脚本检测到就打开 Xcode，等你点完）。其余全自动。
#
# 不用 sudo：xcode-select 还指着 CommandLineTools 时，用 DEVELOPER_DIR 指向 Xcode
# （xcrun / xcodebuild / cargo 的 cc 都认它），不改系统设置。
#
# 引擎库默认**不做 LTO、开增量**（调试档，同 build-android-abi.sh）：改一行引擎重编几十秒。
# 发版仍用 `bash tools/build-mobile.sh ios`（thin LTO）。
set -u
cd "$(dirname "$0")/.."
ROOT=$(pwd)
. "$HOME/.cargo/env" 2>/dev/null || true
EXAMPLE=sdk/flutter/example
BUNDLE=dev.minirender.miniRenderExample
STATE=target/ios-debug
mkdir -p "$STATE"

stop_session() {
    for f in "$STATE"/flutter.pid "$STATE"/native.pid; do
        [ -f "$f" ] && kill "$(cat "$f")" 2>/dev/null
        rm -f "$f"
    done
}
udid_of_session() { cat "$STATE/udid" 2>/dev/null; }

case "${1:-}" in
    --stop) stop_session; echo "已停掉调试会话"; exit 0 ;;
    --shot)
        u=$(udid_of_session); [ -n "$u" ] || { echo "没有在跑的会话"; exit 1; }
        out="$STATE/latest/screen-$(date +%H%M%S).png"
        xcrun simctl io "$u" screenshot "$out" >/dev/null && echo "$out"; exit $? ;;
esac
BUILD_LIB=1
[ "${1:-}" = "--no-lib" ] && BUILD_LIB=0

RUN="$STATE/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN"
ln -sfn "$(basename "$RUN")" "$STATE/latest"
log() { echo "[$(date +%H:%M:%S)] $*" | tee -a "$RUN/steps.log"; }
die() { log "❌ $*"; exit 1; }

# ── 1. Xcode ───────────────────────────────────────────────────────────
find_xcode() {
    for app in /Applications/Xcode.app /Applications/Xcode-*.app /Applications/Xcode_*.app; do
        [ -x "$app/Contents/Developer/usr/bin/xcodebuild" ] && { echo "$app"; return 0; }
    done
    return 1
}
XCODE=$(find_xcode)
if [ -z "$XCODE" ]; then
    log "没有 Xcode：打开 App Store 的 Xcode 页面，请点「获取 / 安装」（约 12GB 下载）。装好后自动继续"
    open "macappstore://apps.apple.com/app/id497799835" 2>/dev/null
    waited=0
    until XCODE=$(find_xcode); do
        sleep 30; waited=$((waited + 30))
        [ $((waited % 600)) = 0 ] && log "  还在等 Xcode（${waited}s，空闲盘 $(df -h / | awk 'NR==2{print $4}')）"
        [ "$waited" -ge "${XCODE_WAIT:-14400}" ] && die "等了 ${waited}s 还没装好 Xcode"
    done
    # App Store 先放出 .app 再补完内容：等体积稳定再往下走
    prev=0
    while :; do
        cur=$(du -sk "$XCODE" 2>/dev/null | awk '{print $1}')
        [ "$cur" = "$prev" ] && break
        prev=$cur; sleep 20
    done
fi
log "Xcode：$XCODE"
if [ "$(xcode-select -p 2>/dev/null)" != "$XCODE/Contents/Developer" ]; then
    export DEVELOPER_DIR="$XCODE/Contents/Developer"
    log "xcode-select 指着 $(xcode-select -p)，本次用 DEVELOPER_DIR=$DEVELOPER_DIR（不改系统设置）"
fi

# 许可与首启组件：没完成时 xcrun 下的一切都会拒绝工作
if ! xcodebuild -license check >/dev/null 2>&1 || ! xcodebuild -checkFirstLaunchStatus >/dev/null 2>&1; then
    if sudo -n true 2>/dev/null; then
        sudo -n xcodebuild -license accept && sudo -n xcodebuild -runFirstLaunch
    else
        log "Xcode 首启：打开 Xcode，请同意许可并输开机密码装组件。完成后自动继续"
        open "$XCODE"
        until xcodebuild -license check >/dev/null 2>&1 && xcodebuild -checkFirstLaunchStatus >/dev/null 2>&1; do
            sleep 15
        done
    fi
fi
log "Xcode $(xcodebuild -version | head -1)，iPhoneOS SDK $(xcrun --sdk iphoneos --show-sdk-version)"

# ── 2. 其余工具（rust 目标 / brew 工具 / CocoaPods），已装的跳过 ─────────
bash tools/ios-setup.sh >"$RUN/setup.log" 2>&1 || true
grep -E '❌' "$RUN/setup.log" | grep -v 'xcode-select\|首启' | while read -r l; do log "  setup: $l"; done
command -v pod >/dev/null || die "缺 CocoaPods（brew install cocoapods）"

# ── 3. 模拟器运行时 + 设备 ─────────────────────────────────────────────
if ! xcrun simctl list runtimes 2>/dev/null | grep -q '^iOS'; then
    log "下载 iOS 模拟器运行时（约 8GB）"
    xcodebuild -downloadPlatform iOS >"$RUN/runtime.log" 2>&1 || die "运行时下载失败，见 $RUN/runtime.log"
fi
UDID=$(xcrun simctl list devices available -j | python3 -c '
import json, sys, re
devs = json.load(sys.stdin)["devices"]
def ver(rt):
    m = re.search(r"iOS-(\d+)-(\d+)", rt)
    return (int(m.group(1)), int(m.group(2))) if m else (0, 0)
cands = [(d["state"] == "Booted", ver(rt), d["name"], d["udid"])
         for rt, ds in devs.items() if "iOS" in rt for d in ds if d["name"].startswith("iPhone")]
# 已经开着的优先，其次最新运行时上的 Pro 机型
cands.sort(key=lambda c: (c[0], c[1], "Pro" in c[2] and "Max" not in c[2]), reverse=True)
print(cands[0][3] if cands else "")')
if [ -z "$UDID" ]; then
    rt=$(xcrun simctl list runtimes -j | python3 -c 'import json,sys; r=[x["identifier"] for x in json.load(sys.stdin)["runtimes"] if x["platform"]=="iOS" and x["isAvailable"]]; print(r[-1] if r else "")')
    dt=$(xcrun simctl list devicetypes -j | python3 -c 'import json,sys; d=[x["identifier"] for x in json.load(sys.stdin)["devicetypes"] if x["name"].startswith("iPhone") and "Pro" in x["name"]]; print(d[-1] if d else "")')
    [ -n "$rt" ] && [ -n "$dt" ] || die "没有可用的 iOS 运行时 / iPhone 机型"
    UDID=$(xcrun simctl create "mini-iPhone" "$dt" "$rt")
    log "新建模拟器 mini-iPhone（$dt / $rt）"
fi
echo "$UDID" > "$STATE/udid"
xcrun simctl boot "$UDID" 2>/dev/null || true
open -a Simulator --args -CurrentDeviceUDID "$UDID"
xcrun simctl bootstatus "$UDID" -b >/dev/null 2>&1
log "模拟器：$(xcrun simctl list devices | grep "$UDID" | sed 's/^ *//')"

# ── 4. 引擎库（XCFramework：真机 + 模拟器两片） ───────────────────────
if [ "$BUILD_LIB" = 1 ] || [ ! -d sdk/ios/MiniRender.xcframework ]; then
    lto=false; [ "${LTO:-0}" = 1 ] && lto=thin
    log "编引擎库（LTO=$lto）…"
    t0=$(date +%s)
    MOBILE_LTO=$lto bash tools/build-mobile.sh ios >"$RUN/lib.log" 2>&1 || die "引擎库构建失败，见 $RUN/lib.log（末尾：$(tail -3 "$RUN/lib.log" | tr '\n' ' ')）"
    log "引擎库完成 $(( $(date +%s) - t0 ))s"
fi
[ -e sdk/flutter/ios/MiniRender.xcframework ] || ln -sfn ../../ios/MiniRender.xcframework sdk/flutter/ios/MiniRender.xcframework

# ── 5. flutter run（后台常驻：VM Service、热重载、print 都在 flutter.log） ─
stop_session
command -v flutter >/dev/null || die "缺 flutter"
(cd "$EXAMPLE" && flutter pub get) >"$RUN/pub.log" 2>&1 || die "flutter pub get 失败，见 $RUN/pub.log"
log "flutter run -d $UDID（首次要 pod install + 全量编，几分钟）"
(cd "$EXAMPLE" && exec flutter run -d "$UDID" --debug </dev/null) >"$RUN/flutter.log" 2>&1 &
echo $! > "$STATE/flutter.pid"
xcrun simctl spawn "$UDID" log stream --style compact --level debug \
    --predicate 'process == "Runner"' >"$RUN/native.log" 2>&1 &
echo $! > "$STATE/native.pid"

waited=0
until grep -qE 'Dart VM Service|Flutter run key commands|Debug service listening' "$RUN/flutter.log"; do
    if ! kill -0 "$(cat "$STATE/flutter.pid")" 2>/dev/null; then
        die "flutter run 退出了，见 $RUN/flutter.log（末尾：$(grep -E 'Error|error:|Failed' "$RUN/flutter.log" | tail -3 | tr '\n' ' ')）"
    fi
    sleep 5; waited=$((waited + 5))
    [ "$waited" -ge "${RUN_WAIT:-1200}" ] && die "等了 ${waited}s 还没起来，见 $RUN/flutter.log"
done
sleep 5
xcrun simctl io "$UDID" screenshot "$RUN/screen.png" >/dev/null 2>&1
VM=$(grep -oE 'http://127\.0\.0\.1:[0-9]+/[^ ]*' "$RUN/flutter.log" | head -1)
log "✅ 已在模拟器上跑起来（$BUNDLE）"
log "   Dart VM Service：${VM:-见 flutter.log}（VS Code「Flutter: Attach to Process」或 DevTools 连它）"
log "   日志：$RUN/flutter.log  $RUN/native.log   截图：$RUN/screen.png"
log "   停：bash tools/ios-debug.sh --stop    补截图：bash tools/ios-debug.sh --shot"
# 收尾剪一次交叉编译目录（同 build-mobile.sh）
python3 tools/prune-target.py --quiet >>"$RUN/steps.log" 2>&1 || true
