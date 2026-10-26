#!/usr/bin/env bash
# iOS 一条龙：等 Xcode → 补环境 → 编引擎库 → 起模拟器 → `flutter run` 挂上调试 → 抓日志与截图。
#
#   bash tools/ios-debug.sh              # 全流程（Xcode 没装就打开 App Store 页面并等着）
#   bash tools/ios-debug.sh --no-lib     # 只改了 Dart / Swift：不重编引擎库
#   bash tools/ios-debug.sh --stop       # 停掉调试会话（flutter run 与日志采集）
#   bash tools/ios-debug.sh --shot       # 给正在跑的会话补一张截图
#   LTO=1 bash tools/ios-debug.sh        # 引擎库开 thin LTO（与发版同档，慢）
#   IOS_TARGET=sim bash tools/ios-debug.sh    # 插着真机也走模拟器（默认：插着真机就用真机）
#   IOS_APP=native bash tools/ios-debug.sh    # Swift 原生示例（sdk/ios/example）而不是 Flutter 示例
#   IOS_APP=native IOS_AUTO_OPEN=1 …          #   启动后直接进示例小程序（模拟器）
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
        [ "$(cat "$STATE/kind" 2>/dev/null)" = device ] && { echo "真机截图请在手机上按键截，或 Xcode → Devices"; exit 1; }
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
    log "xcode-select 指着 $(xcode-select -p)，本次用 DEVELOPER_DIR=${DEVELOPER_DIR}（不改系统设置）"
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
# 真机模式不要模拟器运行时（8GB），别让 ios-setup 去下
IOS_SKIP_RUNTIME=1 bash tools/ios-setup.sh >"$RUN/setup.log" 2>&1 || true
grep -E '❌' "$RUN/setup.log" | grep -v 'xcode-select\|首启' | while read -r l; do log "  setup: $l"; done
command -v pod >/dev/null || die "缺 CocoaPods（brew install cocoapods）"

# ── 3. 目标设备：插着真机就用真机，否则模拟器（IOS_TARGET=device / sim 强制） ──
# flutter 的设备 id 是 UDID（00008140-…），与 devicectl 的 CoreDevice 标识不是一个东西，以 flutter 为准
DEVICE=""
if [ "${IOS_TARGET:-auto}" != sim ]; then
    DEVICE=$(flutter devices --machine 2>/dev/null | python3 -c '
import json, sys
try:
    ds = json.load(sys.stdin)
except ValueError:
    ds = []
real = [d for d in ds if d.get("targetPlatform") == "ios" and not d.get("emulator")]
print(real[0]["id"] if real else "")')
    [ -z "$DEVICE" ] && [ "${IOS_TARGET:-auto}" = device ] && die "没找到真机（数据线插好、手机上点「信任」、设置里打开开发者模式）"
fi
if [ -n "$DEVICE" ]; then
    UDID=$DEVICE
    echo "$UDID" > "$STATE/udid"; echo device > "$STATE/kind"
    log "真机：$(flutter devices 2>/dev/null | grep "$UDID" | sed 's/^ *//' | tr -s ' ')"
    # 签名：用 Runner 工程里写的 DEVELOPMENT_TEAM；本机得有这个 team 的有效开发证书
    TEAM=$(grep -m1 -oE 'DEVELOPMENT_TEAM = [A-Z0-9]+' "$EXAMPLE/ios/Runner.xcodeproj/project.pbxproj" | awk '{print $3}')
    if ! security find-identity -v -p codesigning 2>/dev/null | grep -q "Apple Development"; then
        die "本机没有 Apple Development 证书：Xcode → Settings → Accounts 登录 Apple ID 后再跑"
    fi
    log "签名 team：${TEAM:-未写}（Runner.xcodeproj）"
else
    echo sim > "$STATE/kind"
    if ! xcrun simctl list runtimes 2>/dev/null | grep -q '^iOS'; then
        log "下载 iOS 模拟器运行时（约 8GB）"
        xcodebuild -downloadPlatform iOS >"$RUN/runtime.log" 2>&1 || die "运行时下载失败，见 $RUN/runtime.log"
    fi
    # 内嵌的 Python 顶格写：缩进了就是 IndentationError
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
        log "新建模拟器 mini-iPhone（$dt / ${rt}）"
    fi
    echo "$UDID" > "$STATE/udid"
    xcrun simctl boot "$UDID" 2>/dev/null || true
    open -a Simulator --args -CurrentDeviceUDID "$UDID"
    xcrun simctl bootstatus "$UDID" -b >/dev/null 2>&1
    log "模拟器：$(xcrun simctl list devices | grep "$UDID" | sed 's/^ *//')"

fi

# ── 4. 引擎库（XCFramework：真机 + 模拟器两片） ───────────────────────
if [ "$BUILD_LIB" = 1 ] || [ ! -d sdk/ios/MiniRender.xcframework ]; then
    lto=false; [ "${LTO:-0}" = 1 ] && lto=thin
    log "编引擎库（LTO=${lto}）…"
    t0=$(date +%s)
    MOBILE_LTO=$lto bash tools/build-mobile.sh ios >"$RUN/lib.log" 2>&1 || die "引擎库构建失败，见 $RUN/lib.log（末尾：$(tail -3 "$RUN/lib.log" | tr '\n' ' ')）"
    log "引擎库完成 $(( $(date +%s) - t0 ))s"
fi
[ -e sdk/flutter/ios/MiniRender.xcframework ] || ln -sfn ../../ios/MiniRender.xcframework sdk/flutter/ios/MiniRender.xcframework

# ── 5. flutter run（后台常驻：VM Service、热重载、print 都在 flutter.log） ─
stop_session
command -v flutter >/dev/null || die "缺 flutter"
(cd "$EXAMPLE" && flutter pub get) >"$RUN/pub.log" 2>&1 || die "flutter pub get 失败，见 $RUN/pub.log"

# 真机但没装 iOS 平台组件（Xcode 26 起它和模拟器运行时是同一个 ~8.5GB 包）：
# 带 scheme 的 xcodebuild（flutter run / flutter build ios 都是）一律报
# 「iOS 26.5 is not installed … Ineligible destinations」，连真机也不让编。
# 绕法：按 target 编（-project -target 不走 destination 解析，只认 -sdk），devicectl 装机启动。
# 代价：只能是 Profile（AOT）—— Debug 包在 iOS 14+ 上没挂调试器会被系统拒绝 JIT；
# 所以没有热重载 / Dart 断点，日志与崩溃照样有（native.log）。平台组件装好后自动走 flutter run。
platform_ready() { xcrun simctl list runtimes 2>/dev/null | grep -q "^iOS $(xcrun --sdk iphoneos --show-sdk-version | cut -d. -f1-2)"; }
# 已经下好（哪怕还没挂载）就别再发起下载：2026-10-26 实测，下完再跑一次 downloadPlatform 会把同一份
# 资源再登记一遍，`simctl runtime list` 里冒出两条「Duplicate of …」。**别去 `simctl runtime delete`
# 那两条重复的**：它们指向同一个 dmg，删「重复」会把唯一那份一起删掉（当场丢了 8.5GB，只能重下）。
platform_image_present() { xcrun simctl runtime list 2>/dev/null | grep -qE "^iOS $(xcrun --sdk iphoneos --show-sdk-version | cut -d. -f1-2).*\((Ready|Mounting|Verifying)"; }
device_profile_run() {
    local sym="$ROOT/$EXAMPLE/build/xsym" app
    log "没有 iOS $(xcrun --sdk iphoneos --show-sdk-version) 平台组件：改走 target 构建 + devicectl（Profile 包）"
    pgrep -f 'downloadPlatform iOS' >/dev/null || platform_image_present || {
        nohup xcodebuild -downloadPlatform iOS >"$STATE/platform.log" 2>&1 &
        log "  已在后台下载平台组件（$STATE/platform.log），装好后再跑本脚本即可用 flutter run 调试"
    }
    (cd "$EXAMPLE" && flutter build ios --config-only --profile) >"$RUN/config.log" 2>&1 || die "flutter build ios --config-only 失败，见 $RUN/config.log"
    (cd "$EXAMPLE/ios" && xcodebuild -project Pods/Pods.xcodeproj -alltargets -configuration Profile \
        -sdk iphoneos SYMROOT="$sym" build) >"$RUN/pods.log" 2>&1 || die "Pods 构建失败，见 $RUN/pods.log"
    # 签名要 Xcode 里登录过 Apple ID（「No Accounts」）：打开 Xcode 等你登录，每 30s 重试一次
    local tries=0 team_args=()
    # IOS_TEAM=<Team ID> 覆盖工程里写的 team（命令行的构建设置优先级最高，不用改 pbxproj）
    [ -n "${IOS_TEAM:-}" ] && team_args=("DEVELOPMENT_TEAM=$IOS_TEAM")
    until (cd "$EXAMPLE/ios" && xcodebuild -project Runner.xcodeproj -target Runner -configuration Profile \
            -sdk iphoneos SYMROOT="$sym" -allowProvisioningUpdates -allowProvisioningDeviceRegistration \
            ${team_args[@]+"${team_args[@]}"} build) >"$RUN/xcode.log" 2>&1; do
        if grep -q 'No Accounts' "$RUN/xcode.log"; then
            [ "$tries" = 0 ] && { log "签名要 Apple ID：Xcode → Settings → Accounts 点 + 登录（team ${TEAM:-?}）。登录后自动继续"; open -a Xcode; }
            tries=$((tries + 1)); [ "$tries" -ge 120 ] && die "等了一小时还没登录 Apple ID"
            sleep 30; continue
        fi
        # 登录的 Apple ID 不在工程写的 team 里：在 Xcode 的 Runner → Signing & Capabilities 选自己的 team
        # （会改 pbxproj 的 DEVELOPMENT_TEAM），或 IOS_TEAM=<Team ID> 重跑。改了就自动继续
        if grep -qE 'No Account for Team|requires a development team|is not available|cannot be registered' "$RUN/xcode.log"; then
            if [ "$tries" = 0 ]; then
                log "签名 team 不可用（$(grep -m1 -oE 'No Account for Team[^.]*|requires a development team|is not available[^.]*|cannot be registered[^.]*' "$RUN/xcode.log")）"
                log "  Xcode 已打开 Runner 工程 → 选 Runner target → Signing & Capabilities → Team 选你自己的"
                log "  （个人免费账号要是提示 Bundle Identifier 被占用，把它改成独一无二的，比如加上你的名字缩写）。改完自动继续"
                open "$ROOT/$EXAMPLE/ios/Runner.xcworkspace"
            fi
            tries=$((tries + 1)); [ "$tries" -ge 120 ] && die "等了一小时 team 还没选好"
            sleep 30; continue
        fi
        die "Runner 构建失败，见 $RUN/xcode.log（$(grep -m3 'error:' "$RUN/xcode.log" | tr '\n' ' ')）"
    done
    app="$sym/Profile-iphoneos/Runner.app"
    [ -d "$app" ] || die "没找到 $app"
    # bundle id 以编出来的包为准：个人免费 team 常要在 Xcode 里改成独一无二的
    BUNDLE=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app/Info.plist" 2>/dev/null || echo "$BUNDLE")
    log "装机…"
    xcrun devicectl device install app --device "$UDID" "$app" >"$RUN/install.log" 2>&1 || \
        die "装机失败，见 $RUN/install.log（手机解锁、设置 → 隐私与安全 → 开发者模式打开；首次要在 设置 → 通用 → VPN与设备管理 信任开发者）"
    xcrun devicectl device process launch --device "$UDID" --console --terminate-existing "$BUNDLE" \
        >"$RUN/native.log" 2>&1 </dev/null &
    echo $! > "$STATE/native.pid"
    sleep 8
    kill -0 "$(cat "$STATE/native.pid")" 2>/dev/null || die "启动失败，见 $RUN/native.log（$(tail -3 "$RUN/native.log" | tr '\n' ' ')）"
    log "✅ 已在真机上跑起来（${BUNDLE}，Profile）"
    log "   日志（print / NSLog / Rust 日志 / 崩溃）：$RUN/native.log"
    log "   停：bash tools/ios-debug.sh --stop"
    python3 tools/prune-target.py --quiet >>"$RUN/steps.log" 2>&1 || true
    exit 0
}
# Xcode 26.6 的 iOS SDK 是 26.5.1（23F81a），而 downloadPlatform 下来的运行时是 23F77：
# 默认匹配挑 23F81a、找不到，就报「Platform Not Installed」（运行时明明 Ready）。钉到已装的那个版本
sdk_major=$(xcrun --sdk iphoneos --show-sdk-version | cut -d. -f1-2)
rt_build=$(xcrun simctl runtime list 2>/dev/null | grep -E "^iOS ${sdk_major} .*\(Ready\)" | head -1 | grep -oE '\(([0-9A-Z]+)\)' | head -1 | tr -d '()')
[ -n "$rt_build" ] && xcrun simctl runtime match set "iphoneos${sdk_major}" "$rt_build" >/dev/null 2>&1
[ -n "$DEVICE" ] && ! platform_ready && [ "${IOS_APP:-flutter}" != native ] && device_profile_run

# ── 5'. IOS_APP=native：Swift 原生示例（sdk/ios/example），不经 Flutter ─────────
# XcodeGen 按 project.yml 生成工程 → xcodebuild → simctl / devicectl 装机启动，`--console` 把 NSLog 接到 native.log。
# 没有 Dart，所以也没有热重载；Swift 断点在 Xcode 里开 sdk/ios/example/MiniRenderExample.xcodeproj 直接 Run
native_run() {
    local dir="$ROOT/sdk/ios/example" dd="$ROOT/sdk/ios/example/build/dd" bundle=dev.minirender.example.native
    command -v xcodegen >/dev/null || brew install --formula xcodegen >/dev/null || die "缺 xcodegen（brew install xcodegen）"
    (cd "$dir" && xcodegen generate --spec project.yml --quiet) || die "xcodegen 生成工程失败"
    local dest sdkdir team_args=()
    if [ -n "$DEVICE" ]; then
        dest="id=$UDID"; sdkdir=Debug-iphoneos
        team_args=("DEVELOPMENT_TEAM=${IOS_TEAM:-$TEAM}" -allowProvisioningUpdates -allowProvisioningDeviceRegistration)
    else
        dest="id=$UDID"; sdkdir=Debug-iphonesimulator
    fi
    log "编 Swift 原生示例（$([ -n "$DEVICE" ] && echo 真机 || echo 模拟器)）…"
    (cd "$dir" && xcodebuild -project MiniRenderExample.xcodeproj -scheme MiniRenderExample -configuration Debug \
        -destination "$dest" -derivedDataPath "$dd" ${team_args[@]+"${team_args[@]}"} build) >"$RUN/xcode.log" 2>&1 || \
        die "原生示例构建失败，见 $RUN/xcode.log（$(grep -m3 'error:' "$RUN/xcode.log" | tr '\n' ' ')）"
    local app="$dd/Build/Products/$sdkdir/MiniRenderExample.app"
    if [ -n "$DEVICE" ]; then
        xcrun devicectl device install app --device "$UDID" "$app" >"$RUN/install.log" 2>&1 || die "装机失败，见 $RUN/install.log"
        xcrun devicectl device process launch --device "$UDID" --console --terminate-existing "$bundle" \
            >"$RUN/native.log" 2>&1 </dev/null &
    else
        xcrun simctl install "$UDID" "$app" || die "装进模拟器失败"
        xcrun simctl launch --console-pty --terminate-running-process "$UDID" "$bundle" ${IOS_AUTO_OPEN:+-autoOpen 1} \
            >"$RUN/native.log" 2>&1 </dev/null &
    fi
    echo $! > "$STATE/native.pid"
    sleep 8
    [ -z "$DEVICE" ] && xcrun simctl io "$UDID" screenshot "$RUN/screen.png" >/dev/null 2>&1
    log "✅ Swift 原生示例已在$([ -n "$DEVICE" ] && echo 真机 || echo 模拟器)上跑起来（${bundle}）"
    log "   日志（[事件] / mini-render / 崩溃）：$RUN/native.log   截图：$RUN/screen.png"
    log "   Swift 断点：Xcode 打开 sdk/ios/example/MiniRenderExample.xcodeproj → Run"
    log "   停：bash tools/ios-debug.sh --stop"
    exit 0
}
[ "${IOS_APP:-flutter}" = native ] && native_run
log "flutter run -d ${UDID}（首次要 pod install + 全量编，几分钟）"
(cd "$EXAMPLE" && exec flutter run -d "$UDID" --debug </dev/null) >"$RUN/flutter.log" 2>&1 &
echo $! > "$STATE/flutter.pid"
if [ -z "$DEVICE" ]; then
    xcrun simctl spawn "$UDID" log stream --style compact --level debug \
        --predicate 'process == "Runner"' >"$RUN/native.log" 2>&1 &
    echo $! > "$STATE/native.pid"
fi

waited=0
until grep -qE 'Dart VM Service|Flutter run key commands|Debug service listening' "$RUN/flutter.log"; do
    if ! kill -0 "$(cat "$STATE/flutter.pid")" 2>/dev/null; then
        die "flutter run 退出了，见 $RUN/flutter.log（末尾：$(grep -E 'Error|error:|Failed' "$RUN/flutter.log" | tail -3 | tr '\n' ' ')）"
    fi
    sleep 5; waited=$((waited + 5))
    [ "$waited" -ge "${RUN_WAIT:-1200}" ] && die "等了 ${waited}s 还没起来，见 $RUN/flutter.log"
done
sleep 5
# 真机截图 simctl 管不了；真机的 NSLog / print 都在 flutter.log 里
[ -z "$DEVICE" ] && xcrun simctl io "$UDID" screenshot "$RUN/screen.png" >/dev/null 2>&1
VM=$(grep -oE 'http://127\.0\.0\.1:[0-9]+/[^ ]*' "$RUN/flutter.log" | head -1)
log "✅ 已在$([ -n "$DEVICE" ] && echo 真机 || echo 模拟器)上跑起来（${BUNDLE}）"
log "   Dart VM Service：${VM:-见 flutter.log}（VS Code「Flutter: Attach to Process」或 DevTools 连它）"
log "   日志：$RUN/flutter.log  $RUN/native.log   截图：$RUN/screen.png"
log "   停：bash tools/ios-debug.sh --stop    补截图：bash tools/ios-debug.sh --shot"
# 收尾剪一次交叉编译目录（同 build-mobile.sh）
python3 tools/prune-target.py --quiet >>"$RUN/steps.log" 2>&1 || true
