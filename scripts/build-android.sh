#!/usr/bin/env bash
# 构建 Android SDK：为 4 个 ABI 生成 libmini_render.so，输出到 jniLibs 目录结构
#
# 前置条件：
#   1) 安装 Android NDK，并设置 ANDROID_NDK_HOME 环境变量
#   2) cargo install cargo-ndk
#   3) rustup target add aarch64-linux-android armv7-linux-androideabi \
#          x86_64-linux-android i686-linux-android
#   4) 见 README「移动端精简构建」：当前默认构建包含桌面端依赖
#      (winit/softbuffer/rodio/openh264/arboard)，需按说明启用 rquickjs 的
#      bindgen 特性并裁剪桌面依赖后，移动端才能成功交叉编译。
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "缺少 cargo-ndk，请先执行: cargo install cargo-ndk" >&2
  exit 1
fi

OUT="target/android/jniLibs"
mkdir -p "$OUT"

# arm64-v8a / armeabi-v7a / x86_64 / x86
cargo ndk \
  -t arm64-v8a -t armeabi-v7a -t x86_64 -t x86 \
  -o "$OUT" \
  build --release --lib

echo "✅ Android SDK 产物 (.so) 位于 $OUT/{arm64-v8a,armeabi-v7a,x86_64,x86}/libmini_render.so"
echo "   将 jniLibs 目录放入 Android 工程 src/main/ 下，通过 JNI 调用 mr_* 接口。"
