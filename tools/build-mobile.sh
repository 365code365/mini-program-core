#!/usr/bin/env bash
# 把引擎编成移动端可用的库。
#
#   bash tools/build-mobile.sh ios       # → sdk/ios/MiniRender.xcframework
#   bash tools/build-mobile.sh android   # → sdk/android/src/main/jniLibs/<abi>/libmini_render.so
#   bash tools/build-mobile.sh macos     # → sdk/macos/MiniRender.xcframework（Apple 芯片；MAC_UNIVERSAL=1 再带 Intel）
#   bash tools/build-mobile.sh check     # 只做交叉编译检查（不产出）
#
# 三个**必须**的构建参数，缺一个都编不过：
#   --no-default-features   关掉桌面依赖。`winit` 在 android 目标上必须选一个 activity
#                           后端特性否则直接编译失败；`arboard`（剪贴板）只支持
#                           X11/macOS/Windows；`rodio` 要打开系统声卡设备。
#                           移动端这些都由宿主负责，引擎不该编进去。
#   --features bindgen      `rquickjs-sys` 只为少数目标预生成了 C 绑定（**没有**
#                           iOS/Android/鸿蒙），不开这个会报
#                           「couldn't read .../bindings/aarch64-apple-ios.rs」。
#   IPHONEOS_DEPLOYMENT_TARGET  rustc 默认按 iOS 10 链接，而 C 依赖（QuickJS/openh264）
#                           是按 SDK 新版本编的，两边不一致会缺 `___chkstk_darwin`。
#
# 两端都另外编进 `gpu`：
# - Android（wgpu 的 Vulkan / GLES）：`nativeAttachSurface` 交上来的 ANativeWindow 由 GPU 合成器直接上屏；
# - iOS（wgpu 的 Metal）：原生 `MiniProgramView` 把自己的 CAMetalLayer 交给 `mr_app_attach_surface`。
#   Flutter 的 iOS 外部纹理（CVPixelBuffer）用不上它，照旧 CPU。链接时宿主要带 Metal / QuartzCore
#   （Package.swift / podspec 已写上）。
# SDK 默认偏好仍是 cpu，宿主不选 auto/gpu 就不碰 GPU（两个原生视图都选 auto）。
# `ANDROID_GPU=0` / `IOS_GPU=0` 编一份不带 GPU 的（体积小几 MB，attach 恒返回 CPU）。
set -eu
cd "$(dirname "$0")/.."
MODE="${1:-check}"
FLAGS=(--release --lib --no-default-features --features bindgen)
if [ "${ANDROID_GPU:-1}" = 0 ]; then
    ANDROID_FLAGS=("${FLAGS[@]}")
else
    ANDROID_FLAGS=(--release --lib --no-default-features --features bindgen,gpu)
fi
if [ "${IOS_GPU:-1}" = 0 ]; then
    IOS_FLAGS=("${FLAGS[@]}")
else
    IOS_FLAGS=(--release --lib --no-default-features --features bindgen,gpu)
fi
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-13.0}"

# ─────────────────────────────── Android ───────────────────────────────
setup_ndk() {
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
    local snake up
    snake="$(echo "$target" | tr - _)"
    up="$(echo "$target" | tr 'a-z-' 'A-Z_')"
    # cc-rs 认下划线形式的变量名；openh264 是 C++，CXX 也要给
    # （少了会报「failed to find tool "aarch64-linux-android-clang++"」）
    export "CC_$snake=$TC/bin/$pfx-clang"
    export "CXX_$snake=$TC/bin/$pfx-clang++"
    export "AR_$snake=$TC/bin/llvm-ar"
    export "CARGO_TARGET_${up}_LINKER=$TC/bin/$pfx-clang"
    # bindgen 要 sysroot 才找得到 android 的头文件
    export "BINDGEN_EXTRA_CLANG_ARGS_$snake=--sysroot=$TC/sysroot"
    rustup target add "$target" >/dev/null 2>&1 || true
    echo "▶ $target ($abi)"
    if [ "$MODE" = check ]; then
        cargo check "${ANDROID_FLAGS[@]}" --target "$target"
    else
        # 发出去的库要 thin LTO（Cargo.toml 的 release 为了开发构建快关掉了它，见那里的注释）。
        # 写在函数体里而不是脚本开头：tools/build-android-abi.sh 只 eval 这个函数。
        # MOBILE_LTO=false：真机调试用（build-android-abi.sh 的默认）—— 改一行引擎重编从几分钟降到几十秒
        local lto="${MOBILE_LTO:-thin}" incr=false
        [ "$lto" = false ] && incr=true
        CARGO_PROFILE_RELEASE_LTO="$lto" CARGO_PROFILE_RELEASE_INCREMENTAL="$incr" \
            cargo build "${ANDROID_FLAGS[@]}" --target "$target"
        # 放进 Gradle 库模块的 jniLibs，publishToMavenLocal 时打进 AAR
        local dst="sdk/android/src/main/jniLibs/$abi"
        mkdir -p "$dst"
        cp "target/$target/release/libmini_render.so" "$dst/"
        # strip：未 strip 的 arm64 是 41MB，strip 后 35MB
        "$TC/bin/llvm-strip" "$dst/libmini_render.so" 2>/dev/null || true
    fi
}

build_android() {
    setup_ndk
    android_target aarch64-linux-android   arm64-v8a
    android_target armv7-linux-androideabi armeabi-v7a
    android_target x86_64-linux-android    x86_64
    if [ "$MODE" != check ]; then
        echo "✅ sdk/android/src/main/jniLibs/*/libmini_render.so"
        echo "   下一步：cd sdk/android && ./gradlew publishToMavenLocal"
    fi
}

# ───────────────────────────────── iOS ─────────────────────────────────
ios_target() {
    local target="$1"
    rustup target add "$target" >/dev/null 2>&1 || true
    echo "▶ $target"
    if [ "$MODE" = check ]; then
        cargo check "${IOS_FLAGS[@]}" --target "$target"
    else
        # 发出去的库要 thin LTO（同 android_target）；MOBILE_LTO=false 是调试档（tools/ios-debug.sh 的默认）
        local lto="${MOBILE_LTO:-thin}" incr=false
        [ "$lto" = false ] && incr=true
        CARGO_PROFILE_RELEASE_LTO="$lto" CARGO_PROFILE_RELEASE_INCREMENTAL="$incr" \
            cargo build "${IOS_FLAGS[@]}" --target "$target"
    fi
}

build_ios() {
    ios_target aarch64-apple-ios
    if [ "$MODE" = check ]; then return 0; fi
    ios_target aarch64-apple-ios-sim
    # 头文件目录里必须同时放 module.modulemap，否则 Swift 无法 `import MiniRenderFFI`
    # （SPM 的 binaryTarget 就是靠它把 C 头暴露成模块的）
    local hdr=target/ios-headers
    rm -rf "$hdr"
    mkdir -p "$hdr"
    cp include/mini_render.h "$hdr/"
    cp sdk/ios/ffi-headers/module.modulemap "$hdr/"
    local out=sdk/ios/MiniRender.xcframework
    rm -rf "$out"
    xcodebuild -create-xcframework \
        -library target/aarch64-apple-ios/release/libmini_render.a     -headers "$hdr" \
        -library target/aarch64-apple-ios-sim/release/libmini_render.a -headers "$hdr" \
        -output "$out"
    # Flutter 插件（sdk/flutter）的 podspec 用同一份：CocoaPods 的 vendored 路径不能越出
    # podspec 所在目录，所以放一个软链过去（已在 sdk/flutter/.gitignore 里）
    ln -sfn ../../ios/MiniRender.xcframework sdk/flutter/ios/MiniRender.xcframework
    echo "✅ $out"
    echo "   Swift 里 import MiniRenderFFI；SPM 用 sdk/ios/Package.swift，CocoaPods 用 MiniRender.podspec"
    echo "   Flutter 插件：sdk/flutter/ios/MiniRender.xcframework 已软链到这里"
}

# ─────────────────────────────── macOS ───────────────────────────────
# 给 sdk/macos 的 MiniProgramView（AppKit 宿主，PC 微信式示例在 sdk/macos/example）用的静态库。
# 与 iOS 的区别：不要 bindgen（rquickjs-sys 预生成了 darwin 的绑定）；带 h264 / audio（桌面上软解 <video>、
# 自己出声，与 mini-app-window 一样）；不带 desktop（窗口、上屏、剪贴板都归宿主）。
build_macos() {
    local flags=(--release --lib --no-default-features --features gpu,h264,audio)
    local lto="${MOBILE_LTO:-thin}" incr=false
    [ "$lto" = false ] && incr=true
    local targets=(aarch64-apple-darwin)
    [ "${MAC_UNIVERSAL:-0}" = 1 ] && targets+=(x86_64-apple-darwin)
    export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-14.0}"
    for t in "${targets[@]}"; do
        rustup target add "$t" >/dev/null 2>&1 || true
        echo "▶ $t"
        CARGO_PROFILE_RELEASE_LTO="$lto" CARGO_PROFILE_RELEASE_INCREMENTAL="$incr" \
            cargo build "${flags[@]}" --target "$t"
    done
    local lib="target/${targets[0]}/release/libmini_render.a"
    if [ "${#targets[@]}" -gt 1 ]; then
        mkdir -p target/macos-universal
        lipo -create "target/aarch64-apple-darwin/release/libmini_render.a" "target/x86_64-apple-darwin/release/libmini_render.a" \
            -output target/macos-universal/libmini_render.a
        lib=target/macos-universal/libmini_render.a
    fi
    local hdr=target/macos-headers
    rm -rf "$hdr"
    mkdir -p "$hdr"
    cp include/mini_render.h "$hdr/"
    cp sdk/ios/ffi-headers/module.modulemap "$hdr/"
    local out=sdk/macos/MiniRender.xcframework
    rm -rf "$out"
    xcodebuild -create-xcframework -library "$lib" -headers "$hdr" -output "$out"
    echo "✅ $out"
}

case "$MODE" in
    android) build_android ;;
    ios)     build_ios ;;
    macos)   build_macos ;;
    check)   echo "== 交叉编译检查 =="; build_ios; build_android; echo "✅ iOS / Android 均可编译" ;;
    *) echo "用法: build-mobile.sh [ios|android|macos|check]"; exit 1 ;;
esac
# 剪掉交叉编译目录里换下来的旧产物（发版 thin LTO 与调试档哈希不同，来回切会一份份攒下去）
python3 tools/prune-target.py --quiet || true
