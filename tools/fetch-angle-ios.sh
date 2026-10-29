#!/usr/bin/env bash
# 拉 iOS 上 WebGL 用的 ANGLE（Metal 后端）动态框架到 sdk/ios/Angle/（不入库，同 MiniRender.xcframework）。
#
# 来源：github.com/levinli303/ANGLESwiftUI 里的 libEGL.xcframework / libGLESv2.xcframework
#（从官方 ANGLE 编的，带 EGL_ANGLE_create_context_webgl_compatibility；libEGL 改成到 App 的 Frameworks/ 里找
# libGLESv2.framework）。ANGLE 是 BSD-3，许可证随框架放一份。提交号写死，框架的 sha256 也写死。
#
# 只留 iOS 两片（ios-arm64 真机、ios-arm64_x86_64-simulator），用 xcodebuild 重新打成 xcframework；
# 框架的最低系统是 iOS 15（再低的机型 dlopen 失败，getContext('webgl') 返回 null，2D 游戏不受影响）。
#
#   bash tools/fetch-angle-ios.sh          # 已就绪就跳过
#   bash tools/fetch-angle-ios.sh --force  # 重拉
set -euo pipefail
cd "$(dirname "$0")/.."
REPO=https://github.com/levinli303/ANGLESwiftUI.git
COMMIT=3a8cd236db963e294474ac996827835e0ab97b6d
OUT=sdk/ios/Angle
# 真机片的二进制（校验用：换了来源要一起改）
SHA_EGL=4711ed517d52b2a6d55a5ffd304cef74eb051d7cfb8ca79b8afd194a06860d8b
SHA_GLES=7e6bf51eaa846ed5cab9e1042a9126870e756fa78c82bc8f10fff349d831aa63
if [ "${1:-}" != --force ] && [ -d "$OUT/libEGL.xcframework" ] && [ -d "$OUT/libGLESv2.xcframework" ]; then
    for n in libEGL libGLESv2; do ln -sfn "../../ios/Angle/$n.xcframework" "sdk/flutter/ios/$n.xcframework"; done
    echo "✓ $OUT 已就绪"
    exit 0
fi
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
git -c advice.detachedHead=false clone -q --filter=blob:none --no-checkout "$REPO" "$TMP/src"
git -C "$TMP/src" checkout -q "$COMMIT"
S="$TMP/src/ANGLESwiftUI"
check() {
    local got
    got=$(shasum -a 256 "$1" | awk '{print $1}')
    [ "$got" = "$2" ] || { echo "❌ 校验和不符：$1（$got）"; exit 1; }
}
check "$S/libEGL.xcframework/ios-arm64/libEGL.framework/libEGL" "$SHA_EGL"
check "$S/libGLESv2.xcframework/ios-arm64/libGLESv2.framework/libGLESv2" "$SHA_GLES"
rm -rf "$OUT"
mkdir -p "$OUT"
for n in libEGL libGLESv2; do
    xcodebuild -create-xcframework \
        -framework "$S/$n.xcframework/ios-arm64/$n.framework" \
        -framework "$S/$n.xcframework/ios-arm64_x86_64-simulator/$n.framework" \
        -output "$OUT/$n.xcframework" >/dev/null
done
curl -sSL -o "$OUT/LICENSE.angle" https://raw.githubusercontent.com/google/angle/main/LICENSE || echo "⚠ ANGLE 许可证没拉到（BSD-3，https://github.com/google/angle/blob/main/LICENSE）"
# Flutter 插件的 podspec 用同一份（CocoaPods 的 vendored 路径不能越出 podspec 所在目录；软链整个 Angle 目录它不认，逐个链）
for n in libEGL libGLESv2; do ln -sfn "../../ios/Angle/$n.xcframework" "sdk/flutter/ios/$n.xcframework"; done
du -sh "$OUT"/*.xcframework
echo "✅ $OUT"
