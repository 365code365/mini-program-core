#!/usr/bin/env bash
# macOS 一条龙：编引擎库 → 生成工程 → 编「灵犀 for Mac」→ 启动，日志落 target/mac-debug/<时刻>/。
#
#   bash tools/mac-debug.sh                 # 编库 + 编 App + 启动
#   bash tools/mac-debug.sh --no-lib        # 只改了 Swift：不重编引擎库
#   bash tools/mac-debug.sh -- -open cinema # `--` 之后的参数原样交给 App（-open <AppID|id> / -demo 1 / -tab hub / -shot <目录>）
#
# 引擎库默认调试档（不做 LTO、开增量，同 ios-debug.sh）；LTO=1 与发版同档。
set -eu
cd "$(dirname "$0")/.."
. "$HOME/.cargo/env" 2>/dev/null || true
BUILD_LIB=1
APP_ARGS=()
while [ $# -gt 0 ]; do
    case "$1" in
        --no-lib) BUILD_LIB=0 ;;
        --) shift; APP_ARGS=("$@"); break ;;
        *) echo "未知参数 $1"; exit 2 ;;
    esac
    shift
done
RUN="target/mac-debug/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN"
ln -sfn "$(basename "$RUN")" target/mac-debug/latest
log() { echo "[$(date +%H:%M:%S)] $*" | tee -a "$RUN/steps.log"; }
die() { log "❌ $*"; exit 1; }

if [ "$BUILD_LIB" = 1 ] || [ ! -d sdk/macos/MiniRender.xcframework ]; then
    lto=false; [ "${LTO:-0}" = 1 ] && lto=thin
    log "编引擎库（LTO=${lto}）…"
    MOBILE_LTO=$lto bash tools/build-mobile.sh macos >"$RUN/lib.log" 2>&1 || die "引擎库构建失败，见 $RUN/lib.log"
fi
command -v xcodegen >/dev/null || brew install --formula xcodegen >/dev/null || die "缺 xcodegen"
(cd sdk/macos/example && xcodegen generate --spec project.yml --quiet) || die "xcodegen 失败"
log "编 App…"
(cd sdk/macos/example && xcodebuild -project MiniRenderMac.xcodeproj -scheme LingxiMac -configuration Debug \
    -derivedDataPath build/dd build) >"$RUN/xcode.log" 2>&1 || \
    die "App 构建失败，见 $RUN/xcode.log（$(grep -m3 'error:' "$RUN/xcode.log" | tr '\n' ' ')）"
pkill -x 灵犀 2>/dev/null || true
BIN="sdk/macos/example/build/dd/Build/Products/Debug/灵犀.app/Contents/MacOS/灵犀"
"$BIN" ${APP_ARGS[@]+"${APP_ARGS[@]}"} >"$RUN/app.log" 2>&1 &
log "✅ 已启动（pid $!）。日志：$RUN/app.log（[事件] / [商店] / 🎨 后端）"
log "   Swift 断点：Xcode 打开 sdk/macos/example/MiniRenderMac.xcodeproj → Run"
python3 tools/prune-target.py --quiet >>"$RUN/steps.log" 2>&1 || true
