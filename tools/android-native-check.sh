#!/usr/bin/env bash
# 原生集成（sdk/android/example：一个 Activity + MiniProgramView，不经 Flutter）的真机验收。
#
#   bash tools/android-native-check.sh [all|pixel|pages|func] [install]
#     pixel  画面上屏：视图画的内容 vs 屏幕截图逐像素（去掉系统画的状态栏 / 导航条）+ 颜色通道
#     pages  26 个页面逐个深链冷启动：进对页面、进程活着、各存一张截图（target/_native/pages/）
#     func   交互：tabBar、navigateTo + 返回键、滚动、定时器续帧、Toast / Modal、storage 落盘、
#            前后台、转屏改尺寸、导航栏返回箭头、custom 页胶囊
#   DEVICE=emulator-5554 bash tools/android-native-check.sh pages     # 默认一加 PLY110 真机
#
# 先编：bash tools/build-android-abi.sh && (cd sdk/android/example && ./gradlew assembleDebug)
# —— 一条一条串着跑（并行时 gradle 会打进旧库，见 .kiro/steering/30-test-and-cleanup.md）。
#
# 两条跟别的检查共用设备时的规矩：
# - **不清 logcat**（`logcat -c`）：别的脚本（flutter-android-check / android-jank）正等着自己那一行，
#   清掉它就永远等不到。这里按启动时刻 `-T` 只读之后的日志；
# - 进对没进对按 `[事件] page <路由>` 判，前台不是本示例就报错退出，不会拿别人的画面当结果。
#
# 坐标都是**逻辑像素**（按 MINI_GEOM 的 density 换成设备像素），按 423×923 的一加 PLY110 首页写死；
# 别的机型宽度不同（rpx 跟着缩放）可能点歪 —— 点歪了会报「没进到 …」，不会量错页面。
set -u
cd "$(dirname "$0")/.."
DEV="${DEVICE:-3B669L00NA000000}"
PKG=dev.minirender.example
APK=sdk/android/example/app/build/outputs/apk/debug/app-debug.apk
OUT=target/_native
MODE="${1:-all}"
mkdir -p "$OUT"
D() { "$HOME/Library/Android/sdk/platform-tools/adb" -s "$DEV" "$@"; }
FAIL=0
ok() { echo "  ✅ $*"; }
bad() { echo "  ❌ $*"; FAIL=$((FAIL + 1)); }

if [ "${2:-}" = install ]; then
    # APK 里的引擎库必须是 jniLibs 里刚编的那份（比可执行段的偏移与大小）
    RE=$(ls "$HOME"/Library/Android/sdk/ndk/*/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-readelf 2>/dev/null | tail -1)
    if [ -n "$RE" ]; then
        unzip -p "$APK" lib/arm64-v8a/libmini_render.so > "$OUT/apk_lib.so"
        a=$("$RE" -lW "$OUT/apk_lib.so" | grep "R E"); rm -f "$OUT/apk_lib.so"
        b=$("$RE" -lW sdk/android/src/main/jniLibs/arm64-v8a/libmini_render.so | grep "R E")
        [ "$a" = "$b" ] || { echo "⚠ APK 里的引擎库不是 jniLibs 里那份（单独重跑 ./gradlew assembleDebug）"; exit 1; }
    fi
    D install -r "$APK" | tail -1
fi

# ─────────────────────────── 公用 ───────────────────────────
T0=""
# 设备上的时刻（logcat -T 的格式）。整条命令要作为一个字符串交给远端 shell：格式里有空格，
# 拆成多个参数的话远端 date 报「Max 1 argument」、-T 拿到空串退成「最近 1 行」
mark() { T0=$(D shell "date +'%m-%d %H:%M:%S.000'" | tr -d '\r'); }
# 自 mark 以来本示例的日志（不清缓冲区）
log() { D logcat -d -T "$T0" -s mini-example:I 2>/dev/null; }
front() { D shell dumpsys window </dev/null | grep -m1 mCurrentFocus | grep -q "$PKG/"; }
# 等一行日志出现（正则），最多 $2 秒；打印匹配到的最后一行
wait_log() {
    local re="$1" n=$(( ${2:-20} * 4 )) l=""
    for _ in $(seq 1 "$n"); do
        l=$(log | grep -E "$re" | tail -1)
        [ -n "$l" ] && { echo "$l"; return 0; }
        sleep 0.25
    done
    return 1
}
GEOM=""
DPR=""
# 冷启动到某一页（route 为空 = 首页），等到它的 page 事件；额外参数原样交给 am start
launch() {
    local route="$1"; shift
    D shell am force-stop "$PKG"
    mark
    if [ -n "$route" ]; then D shell am start -n "$PKG/.MainActivity" --es route "$route" "$@" >/dev/null
    else D shell am start -n "$PKG/.MainActivity" "$@" >/dev/null; fi
    local want="${route%%\?*}"
    [ -n "$want" ] || want="pages/index/index"
    wait_log "page $want " 30 >/dev/null || { bad "冷启动 ${want}：30 秒内没有 page 事件"; return 1; }
    GEOM=$(log | grep -m1 MINI_GEOM)
    DPR=$(echo "$GEOM" | sed 's/.*density=\([0-9.]*\).*/\1/')
    sleep 1.2
    front || { bad "前台不是本示例（别的检查在用这台设备？）"; return 1; }
}
g() { echo "$GEOM" | sed "s/.* $1=\([0-9]*\).*/\1/"; }
# 逻辑坐标 → 设备像素
px() { python3 -c "print(round($1*$DPR))"; }
tap() { D shell input tap "$(px "$1")" "$(px "$2")"; }
swipe() { D shell input swipe "$(px "$1")" "$(px "$2")" "$(px "$3")" "$(px "$4")" "${5:-300}"; }
shot() { D exec-out screencap -p > "$OUT/$1.png"; }
# 两张截图在某个逻辑矩形里的差异像素占比（%），用来判「画面动了没有」
changed() {
    python3 - "$OUT/$1.png" "$OUT/$2.png" "$DPR" "$3" <<'EOF'
import sys
from PIL import Image, ImageChops
a, b = Image.open(sys.argv[1]).convert('RGB'), Image.open(sys.argv[2]).convert('RGB')
d = float(sys.argv[3]); x0, y0, x1, y1 = [round(float(v) * d) for v in sys.argv[4].split(',')]
diff = ImageChops.difference(a.crop((x0, y0, x1, y1)), b.crop((x0, y0, x1, y1))).convert('L')
n = sum(1 for v in diff.getdata() if v > 8)
print(f"{100 * n / max(1, (x1 - x0) * (y1 - y0)):.2f}")
EOF
}
crashed() {
    D logcat -d -T "$T0" -b crash 2>/dev/null | grep -E "$PKG|mini_render" | head -3
}

# ─────────────────────────── pixel ───────────────────────────
check_pixel() {
    echo "【上屏】视图画的内容 vs 屏幕截图"
    D shell rm -f "/sdcard/Android/data/$PKG/files/engine_frame.png"
    launch "pages/detail/detail?from=dump" --ez dump true || return
    local l
    l=$(wait_log MINI_DUMP 20) || { bad "20 秒内没等到 MINI_DUMP"; return; }
    shot screen
    D pull "/sdcard/Android/data/$PKG/files/engine_frame.png" "$OUT/engine.png" >/dev/null
    python3 - "$OUT" "$l" <<'EOF'
import re, sys, subprocess
from PIL import Image
out, l = sys.argv[1], sys.argv[2]
g = lambda k: int(re.search(rf' {k}=(\d+)', l).group(1))
x, y, w, h, sb, bot = g('x'), g('y'), g('w'), g('h'), g('statusBar'), g('bottom')
e = Image.open(f'{out}/engine.png').convert('RGB')
s = Image.open(f'{out}/screen.png').convert('RGB').crop((x, y, x + w, y + h))
e.save(f'{out}/engine_rgb.png'); s.save(f'{out}/screen_crop.png')
# 状态栏（时间 / 图标）与底部导航条（手势横条 / 三键）是系统画在画面上面的，不算
box = f'0,{sb},{w - 1},{h - 1 - bot}'
r = subprocess.run(['python3', 'tools/pixdiff.py', f'{out}/engine_rgb.png', f'{out}/screen_crop.png', '--box', box],
                   capture_output=True, text=True).stdout.strip()
print(f'  视图 {w}x{h}@({x},{y})，比较区 y[{sb},{h - 1 - bot}]：{r}')
sys.exit(0 if r == '像素完全一致' else 1)
EOF
    [ $? = 0 ] && ok "逐像素一致" || bad "上屏有差异（看 $OUT/engine_rgb.png 与 screen_crop.png）"
    # 颜色通道：详情页的两个按钮是 #07c160（微信绿），R/B 反了会是 (96,193,7)。按颜色数像素，不依赖按钮位置
    local c
    c=$(python3 -c "
from PIL import Image
s=list(Image.open('$OUT/screen_crop.png').getdata())
print(s.count((7, 193, 96)), s.count((96, 193, 7)))")
    set -- $c
    [ "$1" -gt 10000 ] && [ "$2" = 0 ] && ok "颜色通道：#07c160 的像素 ${1} 个、R/B 反了的 0 个" \
        || bad "颜色通道：#07c160 有 ${1} 个、(96,193,7) 有 ${2} 个"
}

# ─────────────────────────── pages ───────────────────────────
PAGES="index components apis host basic form picker swiper scroll media canvas custom wxml wxss event animation
feedback route storage network query navcustom perf http dashboard detail"
check_pages() {
    echo "【页面】逐页深链冷启动"
    mkdir -p "$OUT/pages"
    local n=0
    for p in $PAGES; do
        launch "pages/$p/$p" || continue
        D shell pidof "$PKG" >/dev/null || { bad "${p}：进程没了"; continue; }
        local c; c=$(crashed)
        [ -z "$c" ] || { bad "${p}：崩溃 $c"; continue; }
        D exec-out screencap -p > "$OUT/pages/$p.png"
        n=$((n + 1))
    done
    ok "$n 个页面进对、无崩溃（截图 $OUT/pages/）"
}

# 交互那一组单独一个文件（这个文件只放公用函数与上屏 / 逐页两组）
. tools/android-native-func.sh

case "$MODE" in
    pixel) check_pixel ;;
    pages) check_pages ;;
    func) check_func ;;
    all) check_pixel; check_pages; check_func ;;
    *) echo "用法: android-native-check.sh [all|pixel|pages|func] [install]"; exit 1 ;;
esac
echo
[ "$FAIL" = 0 ] && echo "✅ 全部通过" || echo "❌ $FAIL 项不通过"
exit "$FAIL"
