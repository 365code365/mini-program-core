#!/usr/bin/env bash
# 把移动端产物打成可发布的两个 zip（+ 校验和），供 GitHub Release 上传。
#
#   bash tools/build-mobile.sh android      # 先编，产出 jniLibs/<abi>/libmini_render.so
#   bash tools/build-mobile.sh ios          # 先编，产出 sdk/ios/MiniRender.xcframework
#   bash tools/package-mobile.sh            # → dist/mini-render-{android,ios}-<版本>.zip
#   bash tools/package-mobile.sh 0.1.0      # 指定版本号（默认取 Cargo.toml）
#
# 为什么要打包而不是直接传 .so / .xcframework：
#   - 集成方要的不只是二进制，还有 Kotlin / Swift 那层 View 和 Gradle / SPM / Pod 描述，
#     分开传会对不上版本；
#   - `.xcframework` 是目录，Release 附件只能是单文件；
#   - 有 SHA256SUMS 才能让下载方验证（README 里写了 `shasum -a 256 -c`）。
#
# 产物**不进仓库**：dist/ 已在 .gitignore 里（arm64 的 .so 34MB、XCFramework 358MB）。
set -eu
cd "$(dirname "$0")/.."

VERSION="${1:-$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')}"
OUT="dist"
AND_ZIP="$OUT/mini-render-android-$VERSION.zip"
IOS_ZIP="$OUT/mini-render-ios-$VERSION.zip"
STAGE="$OUT/.stage"

rm -rf "$STAGE"
mkdir -p "$OUT" "$STAGE"

have_android=0
have_ios=0
[ -d sdk/android/src/main/jniLibs ] && have_android=1
[ -d sdk/ios/MiniRender.xcframework ] && have_ios=1

if [ "$have_android" = 0 ] && [ "$have_ios" = 0 ]; then
    echo "❌ 找不到任何产物。先跑："
    echo "   bash tools/build-mobile.sh android"
    echo "   bash tools/build-mobile.sh ios"
    exit 1
fi

# ─────────────────────────────── Android ───────────────────────────────
if [ "$have_android" = 1 ]; then
    D="$STAGE/mini-render-android-$VERSION"
    mkdir -p "$D"
    # Gradle 模块整体带走（jniLibs 在 src/main 下，publishToMavenLocal 会打进 AAR）
    rsync -a --exclude build --exclude .gradle sdk/android/ "$D/android/"
    cp sdk/README.md "$D/SDK-README.md"
    cp LICENSE "$D/" 2>/dev/null || true
    cat > "$D/README.txt" <<EOF
mini-render Android $VERSION

目录:
  android/                       Gradle 库模块（含 Kotlin 的 MiniProgramView / MiniEngine）
  android/src/main/jniLibs/      arm64-v8a / armeabi-v7a / x86_64 的 libmini_render.so（已 strip）

两种用法:
  A. 直接用 .so —— 把 jniLibs/ 拷进你自己的 module 的 src/main/jniLibs/，
     再把 android/src/main/kotlin/ 下的两个 .kt 拷进你的源码树。
  B. 打成 AAR —— cd android && ./gradlew publishToMavenLocal
     然后在业务工程里 implementation("dev.minirender:mini-render:$VERSION")

接入代码见 SDK-README.md。引擎按**目录**读小程序，不解析 .wxapkg。
EOF
    ( cd "$STAGE" && zip -qr "../../$AND_ZIP" "mini-render-android-$VERSION" )
    echo "✅ $AND_ZIP ($(du -h "$AND_ZIP" | cut -f1))"
    for abi in sdk/android/src/main/jniLibs/*/; do
        printf '   %-14s %s\n' "$(basename "$abi")" "$(du -h "$abi/libmini_render.so" | cut -f1)"
    done
fi

# ───────────────────────────────── iOS ─────────────────────────────────
if [ "$have_ios" = 1 ]; then
    D="$STAGE/mini-render-ios-$VERSION"
    mkdir -p "$D"
    rsync -a sdk/ios/MiniRender.xcframework "$D/"
    rsync -a sdk/ios/Sources "$D/"
    rsync -a sdk/ios/ffi-headers "$D/"
    cp sdk/ios/Package.swift sdk/ios/MiniRender.podspec "$D/"
    cp include/mini_render.h "$D/ffi-headers/"
    cp sdk/README.md "$D/SDK-README.md"
    cp LICENSE "$D/" 2>/dev/null || true
    cat > "$D/README.txt" <<EOF
mini-render iOS $VERSION

目录:
  MiniRender.xcframework/   真机 arm64 + 模拟器 arm64 的静态库（含 C 头与 module.modulemap）
  Sources/MiniRender/       Swift 的 MiniProgramView（集成方只用这个）
  Package.swift             Swift Package 描述
  MiniRender.podspec        CocoaPods 描述

两种用法:
  A. Swift Package —— 把本目录当本地包：.package(path: "路径")
  B. CocoaPods    —— pod 'MiniRender', :path => '路径'

Swift 里 import MiniRenderFFI 拿 C ABI；日常只用 MiniProgramView。
链进 App 后死代码消除会大幅缩小体积（静态库本体偏大是正常的）。
接入代码见 SDK-README.md。
EOF
    ( cd "$STAGE" && zip -qr "../../$IOS_ZIP" "mini-render-ios-$VERSION" )
    echo "✅ $IOS_ZIP ($(du -h "$IOS_ZIP" | cut -f1))"
fi

rm -rf "$STAGE"

# ─────────────────────────── 校验和 ───────────────────────────
# macOS 只有 shasum，Linux（CI）通常两个都有 —— 挑存在的那个
if command -v shasum >/dev/null 2>&1; then
    ( cd "$OUT" && shasum -a 256 mini-render-*-"$VERSION".zip > SHA256SUMS )
else
    ( cd "$OUT" && sha256sum mini-render-*-"$VERSION".zip > SHA256SUMS )
fi
echo "✅ $OUT/SHA256SUMS"
cat "$OUT/SHA256SUMS"
echo
echo "上传到 Release（需要 gh 已登录）："
echo "  gh release create v$VERSION $OUT/mini-render-*-$VERSION.zip $OUT/SHA256SUMS \\"
echo "     --repo 365code365/mini-program-core --title \"mini-render v$VERSION\" --notes-file <说明文件>"
