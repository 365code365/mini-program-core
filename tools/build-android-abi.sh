#!/usr/bin/env bash
# 只编一个 Android ABI 的 libmini_render.so（调试 Flutter 插件 / 真机时用，省掉另外两个 ABI 的时间）。
#
#   bash tools/build-android-abi.sh                 # arm64-v8a（真机与 Apple 芯片上的模拟器）
#   bash tools/build-android-abi.sh x86_64          # x86_64 模拟器
#   LTO=1 bash tools/build-android-abi.sh           # 开回 thin LTO（与发版同一档，慢）
#
# 默认**不做 LTO、开增量**：这是真机调试用的，改一行引擎之后重编几十秒；thin LTO 每次都要
# 把整张依赖图重新优化一遍（几分钟）。像素不受影响（Cargo.toml 的 release 注释里有实测）。
# 发版用 build-mobile.sh android（三个 ABI 都开 thin LTO）。
#
# 工具链设置与 tools/build-mobile.sh 同一份（直接取那边的函数），产物同样放进
# sdk/android/src/main/jniLibs/<abi>/，Flutter 插件与原生 SDK 都从那里拿。
set -eu
cd "$(dirname "$0")/.."
ABI="${1:-arm64-v8a}"
case "$ABI" in
    arm64-v8a)   TARGET=aarch64-linux-android ;;
    armeabi-v7a) TARGET=armv7-linux-androideabi ;;
    x86_64)      TARGET=x86_64-linux-android ;;
    *) echo "用法: build-android-abi.sh [arm64-v8a|armeabi-v7a|x86_64]"; exit 1 ;;
esac
MODE=android
if [ "${LTO:-0}" = 1 ]; then MOBILE_LTO=thin; else MOBILE_LTO=false; fi
# 与 build-mobile.sh 同一套参数（默认带 gpu，`ANDROID_GPU=0` 关掉）
if [ "${ANDROID_GPU:-1}" = 0 ]; then
    ANDROID_FLAGS=(--release --lib --no-default-features --features bindgen)
else
    ANDROID_FLAGS=(--release --lib --no-default-features --features bindgen,gpu)
fi
# 只取 build-mobile.sh 里的两个函数定义，不执行它末尾的 case
eval "$(sed -n '/^setup_ndk() {/,/^}/p;/^android_target() {/,/^}/p' tools/build-mobile.sh)"
setup_ndk
android_target "$TARGET" "$ABI"
echo "✅ sdk/android/src/main/jniLibs/$ABI/libmini_render.so"
