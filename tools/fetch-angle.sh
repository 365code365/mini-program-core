#!/usr/bin/env bash
# 拉 WebGL 用的 ANGLE / SwiftShader 预编译库到 third_party/angle/<平台>/（design-webgl.md §6；不入库）。
#
# - libEGL / libGLESv2：PyPI 的 comfy-angle 0.1.1（取自 Electron 的 ANGLE，BSD-3），sha256 写死；
# - SwiftShader（Vulkan 软件实现，无头测试的确定性后端）：从本机 Chrome 拷 libvk_swiftshader.dylib 与 ICD 描述。
# 只做 macOS arm64（开发机）；Android 走系统 EGL（见 egl.rs 的候选库顺序）。
set -euo pipefail
cd "$(dirname "$0")/.."
D=third_party/angle/macos-arm64
WHL_URL=https://files.pythonhosted.org/packages/9f/0d/44c35dbe55cacd4b7683d83ab00aacc2c136268e2545a425a8af4c2c8b0e/comfy_angle-0.1.1-py3-none-macosx_11_0_arm64.whl
WHL_SHA=c0af17e17070cf7af5dbd66c51a1b6cca2820c8611e487e7bd398006a1447124
if [ -f "$D/libEGL.dylib" ] && [ -f "$D/libGLESv2.dylib" ] && [ -f "$D/libvk_swiftshader.dylib" ]; then echo "✓ $D 已就绪"; exit 0; fi
mkdir -p "$D"
TMP=$(mktemp -d); trap 'rm -rf "$TMP"' EXIT
curl -sSL -o "$TMP/a.whl" "$WHL_URL"
echo "$WHL_SHA  $TMP/a.whl" | shasum -a 256 -c - >/dev/null || { echo "❌ comfy-angle 校验和不符"; exit 1; }
unzip -q -o "$TMP/a.whl" -d "$TMP/w"
cp "$TMP/w/comfy_angle/libs/libEGL.dylib" "$TMP/w/comfy_angle/libs/libGLESv2.dylib" "$D/"
cp "$TMP/w/comfy_angle/electron-LICENSE" "$D/LICENSE.electron"
CH=$(ls -d "/Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/"*/Libraries 2>/dev/null | sort -V | tail -1 || true)
if [ -n "$CH" ] && [ -f "$CH/libvk_swiftshader.dylib" ]; then
    cp "$CH/libvk_swiftshader.dylib" "$CH/vk_swiftshader_icd.json" "$D/"
else
    echo "⚠️ 没找到本机 Chrome 的 SwiftShader：无头 WebGL 用例会回退到 Metal（不逐字节确定）"
fi
ls -la "$D"
