#!/usr/bin/env bash
# 构建 iOS SDK：生成包含真机(arm64)与模拟器(arm64+x86_64)的 XCFramework
#
# 前置条件：
#   1) 安装 Xcode 与命令行工具
#   2) rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
#   3) 见 README「移动端精简构建」：rquickjs-sys 对 iOS 目标需启用 bindgen
#      生成绑定，且需裁剪桌面端依赖(winit/softbuffer/rodio/openh264/arboard)。
set -euo pipefail
cd "$(dirname "$0")/.."

LIB="libmini_render.a"

echo "==> 编译 iOS 真机 + 模拟器"
cargo build --release --lib --target aarch64-apple-ios
cargo build --release --lib --target aarch64-apple-ios-sim
cargo build --release --lib --target x86_64-apple-ios

echo "==> 合并模拟器架构 (arm64-sim + x86_64)"
mkdir -p target/ios/sim
lipo -create \
  target/aarch64-apple-ios-sim/release/$LIB \
  target/x86_64-apple-ios/release/$LIB \
  -output target/ios/sim/$LIB

echo "==> 生成 XCFramework"
rm -rf target/ios/MiniRender.xcframework
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/$LIB -headers include \
  -library target/ios/sim/$LIB -headers include \
  -output target/ios/MiniRender.xcframework

echo "✅ iOS SDK 产物: target/ios/MiniRender.xcframework"
echo "   拖入 Xcode 工程，桥接头引入 mini_render.h 即可调用 mr_* 接口。"
