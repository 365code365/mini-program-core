#!/usr/bin/env bash
# Flutter 插件在 Android 模拟器 / 真机上的像素验收：纹理没被拉伸、通道没反、没有错位。
#
#   bash tools/flutter-android-check.sh            # 用已经编好的 example APK
#   BUILD=1 bash tools/flutter-android-check.sh    # 先编（--dart-define=MINI_AUTODUMP=1）
#   BACKEND=gpu BUILD=1 bash tools/flutter-android-check.sh   # 偏好 GPU（auto / cpu / gpu，默认 cpu）
#
# 做法：例子带 MINI_AUTODUMP 启动时直接打开内置小程序的静态详情页，3 秒后把引擎当前帧
# 存成 PNG 并在日志里打一行 MINI_DUMP（像素尺寸、导航栏底边）。脚本看到这一行立刻截屏，
# 按导航栏底边裁出纹理那一块，与引擎帧逐像素比（tools/pixdiff.py）。
#
# 判据：差异像素为 0。不为 0 时先看包围盒 —— 落在状态栏 / 导航栏里说明裁剪位置算错了，
# 落在页面里才是纹理上屏的问题。
#
# GPU 上屏时（日志 `backend=gpu`）引擎帧是同一组图层的 CPU 合成，屏幕上是 GPU 画的：
# 差异为 0 就是「GPU 输出与 CPU 逐字节一致」在真机链路上的证据。没选上 GPU 时
# `[后端]` 那行写着原因（门槛 / 自检 / 设备）。
set -eu
cd "$(dirname "$0")/.."
ADB="${ADB:-$HOME/Library/Android/sdk/platform-tools/adb}"
DEV="${DEVICE:-emulator-5554}"
PKG=dev.minirender.mini_render_example
OUT=target/flutter-android-check
mkdir -p "$OUT"
a() { "$ADB" -s "$DEV" "$@"; }

if [ "${BUILD:-0}" = 1 ]; then
    (cd sdk/flutter/example && flutter build apk --debug --target-platform android-arm64 \
        --dart-define=MINI_AUTODUMP=1 --dart-define=MINI_BACKEND="${BACKEND:-cpu}")
fi
a install -r sdk/flutter/example/build/app/outputs/flutter-apk/app-debug.apk >/dev/null
a shell am force-stop "$PKG"
a shell rm -f "/sdcard/Android/data/$PKG/files/engine_frame.png"
a logcat -c
a shell monkey -p "$PKG" -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1

line=""
for _ in $(seq 1 60); do
    line=$(a logcat -d | grep -m1 "MINI_DUMP" || true)
    [ -n "$line" ] && break
    sleep 0.5
done
[ -n "$line" ] || { echo "❌ 30 秒内没等到 MINI_DUMP（例子没带 --dart-define=MINI_AUTODUMP=1？）"; exit 1; }
a exec-out screencap -p > "$OUT/screen.png"
a pull "/sdcard/Android/data/$PKG/files/engine_frame.png" "$OUT/engine.png" >/dev/null
echo "$line" | sed 's/.*MINI_DUMP/MINI_DUMP/'
# 后端结论与 GPU 的诊断行（选上 / 没选上的原因、自检、回退）
a logcat -d | grep -E "\[后端\]|🎨|⚠️ GPU|GPU 没选上|GPU 回退|🧪|🧩" | sed 's/^.*: //' | head -12 || true

python3 - "$OUT" "$line" <<'EOF'
import re, sys, subprocess
from PIL import Image
out, line = sys.argv[1], sys.argv[2]
dpr = float(re.search(r'dpr=([\d.]+)', line).group(1))
top = float(re.search(r'bodyTop=([\d.]+)', line).group(1))
engine = Image.open(f'{out}/engine.png').convert('RGB')
screen = Image.open(f'{out}/screen.png').convert('RGB')
y0 = round(top * dpr)
crop = screen.crop((0, y0, engine.width, y0 + engine.height))
crop.save(f'{out}/screen_crop.png')
engine.save(f'{out}/engine_rgb.png')
print(f'纹理 {engine.width}x{engine.height} 屏幕上的位置 (0,{y0})')
r = subprocess.run(['python3', 'tools/pixdiff.py', f'{out}/engine_rgb.png', f'{out}/screen_crop.png'],
                   capture_output=True, text=True)
print(r.stdout.strip() or r.stderr.strip())
EOF
