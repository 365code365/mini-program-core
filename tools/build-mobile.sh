#!/usr/bin/env bash
# 把引擎编成移动端可用的库。
#
#   bash tools/build-mobile.sh ios       # → sdk/ios/MiniRender.xcframework
#   bash tools/build-mobile.sh android   # → sdk/android/src/main/jniLibs/<abi>/libmini_render.so
#   bash tools/build-mobile.sh check     # 只做交叉编译检查（不产出）
#
# 两个**必须**的构建参数，缺一个都编不过：
#   --no-default-features  关掉桌面依赖。`winit` 在 android 目标上必须选一个 activity
#                          后端特性否则直接编译失败；`arboard`（剪贴板）只支持
#                          X11/macOS/Windows；`rodio` 要打开系统声卡设备。
#                          移动端这些都由宿主负责，引擎不该编进去。
#   --features bindgen     `rquickjs-sys` 只为少数目标预生成了 C 绑定（**没有**
#                          iOS/Android/鸿蒙），不开这个会报
#                          「couldn't read .../bindings/aarch64-apple-ios.rs」。
set -eu
cd "$(dirname "$0")/.."
MODE="${1:-check}"
FLAGS=(--release --lib --no-default-features --features bindgen)
# iOS 的部署目标必须显式给：rustc 默认按 iOS 10 链接，而 C 依赖（QuickJS / openh264）
# 是按 SDK 的新版本编的，两边不一致会缺 `___chkstk_darwin` 直接链不过。
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-13.0}"

# ─────────────────────────────── Android ───────────────────────────────
setup_ndk() {
    # 允许外部指定；否则在 SDK 目录里挑一个最新的
    if [ -z "${ANDROID_NDK_HOME:-}" ]; then
        local base="${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}/ndk"
        [ -d "$base" ] || { echo "❌ 找不到 NDK，请设 ANDROID_NDK_HOME"; exit 1; }
        ANDROID_NDK_HOME="$base/$(ls "$base" | sort -V | tail -1)"
    fi
    local host
    case "$(uname -s)" in
        Darwin) host=darwin-x86_64 ;;
        Linux)  host=linux-x86_64 ;;
        *) echo "❌ 不支持的构建主机"; exit 1 ;;
    esac
    TC="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$host"
    [ -d "$TC" ] || { echo "❌ NDK 工具链不存在: $TC"; exit 1; }
    echo "🔧 NDK: $ANDROID_NDK_HOME"
}

android_target() {
    local target="$1" abi="$2" api="${3:-24}"
    local pfx
    case "$target" in
        aarch64-linux-android)   pfx="aarch64-linux-android$api" ;;
        armv7-linux-androideabi) pfx="armv7a-linux-androideabi$api" ;;
        x86_64-linux-android)    pfx="x86_64-linux-android$api" ;;
        *) echo "❌ 未知 target $target"; exit 1 ;;
    esac
    local up
    up="$(echo "$target" | tr 'a-z-' 'A-Z_')"
    # cc-rs 认下划线形式的变量名；openh264 是 C++，CXX 也要给（少了会报
    # 「failed to find tool "aarch64-linux-android-clang++"」）
    export "CC_$(echo "$target" | tr - _)=$TC/bin/$pfx-clang"
    export "CXX_$(echo "$target" | tr - _)=$TC/bin/$pfx-clang++"
    export "AR_$(echo "$target" | tr - _)=$TC/bin/llvm-ar"
    export "CARGO_TARGET_${up}_LINKER=$TC/bin/$pfx-clang"
    # bindgen 要 sysroot 才找得到 android 的头文件
    export "BINDGEN_EXTRA_CLANG_ARGS_$(echo "$target" | tr - _)=--sysroot=$TC/sysroot"
    rustup target add "$target" >/dev/null 2>&1 || true
    echo "▶ $target ($abi)"
    if [ "$MODE" = check ]; then
        cargo check "${FLAGS[@]}" --target "$target"
    else
        cargo build "${FLAGS[@]}" --target "$target"
        # 放进 Gradle 库模块的 jniLibs，`./gradlew publishToMavenLocal` 打进 AAR
        local dst="sdk/android/src/main/jniLibs/$abi"
        mkdir -p "$dst"
        cp "target/$target/release/libmini_render.so" "$dst/"
        # strip 一下：未 strip 的 arm64 是 41MB，strip 后 35MB
        "$TC/bin/llvm-strip" "$dst/libmini_render.so" 2>/dev/null || true
    fi
}

build_android() {
    setup_ndk
    android_target aarch64-linux-android   arm64-v8a
    android_target armv7-linux-androideabi armeabi-v7a
    android_target x86_64-linux-android    x86_64
    [ "$MODE" = check ] || echo "✅ sdk/android/src/main/jniLibs/*/libmini_render.so（下一步：cd sdk/android && ./gradlew publishToMavenLocal）"
}

# ───────────────────────────────── iOS ─────────────────────────────────
ios_target() {
    local target="$1"
    rustup target add "$target" >/dev/null 2>&1 || true
    echo "▶ $target"
    if [ "$MODE" = check ]; then
        cargo check "${FLAGS[@]}" --target "$target"
    else
        cargo build "${FLAGS[@]}" --target "$target"
    fi
}

build_ios() {
    ios_target aarch64-apple-ios
                                                                                                                                                                                                                                                                                                                                                                                                                                            xcframework
    rm -rf "$out"
    xcodebuild -create-xcframework \
        -library target/aarch64-apple-ios/release/libmini_render.a     -headers "$hdr" \
        -library target/aarch64-apple-ios-sim/release/libmini_render.a -headers "$hdr" \
        -output "$out"
    echo "✅ $out（Swift 里 import MiniRenderFFI）"
}

case "$MODE" in
    android) build_android ;;
    ios)     build_ios ;;
    check)   echo "== 交叉编译检查 =="; build_ios; build_android; echo "✅ iOS / Android 均可编译" ;;
    *) echo "用法: build-mobile.sh [ios|android|check]"; exit 1 ;;
esac
