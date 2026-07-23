#!/usr/bin/env bash
# 构建 macOS SDK：生成 Apple Silicon + Intel 通用(universal)动态库与静态库
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> 添加 macOS 目标"
rustup target add aarch64-apple-darwin x86_64-apple-darwin

echo "==> 编译 (aarch64 / x86_64)"
cargo build --release --lib --target aarch64-apple-darwin
cargo build --release --lib --target x86_64-apple-darwin

OUT="target/macos"
mkdir -p "$OUT"

echo "==> lipo 合并为通用二进制"
lipo -create \
  target/aarch64-apple-darwin/release/libmini_render.dylib \
  target/x86_64-apple-darwin/release/libmini_render.dylib \
  -output "$OUT/libmini_render.dylib"

lipo -create \
  target/aarch64-apple-darwin/release/libmini_render.a \
  target/x86_64-apple-darwin/release/libmini_render.a \
  -output "$OUT/libmini_render.a"

cp include/mini_render.h "$OUT/"

echo "✅ macOS SDK 产物:"
echo "   $OUT/libmini_render.dylib   (通用动态库)"
echo "   $OUT/libmini_render.a       (通用静态库)"
echo "   $OUT/mini_render.h          (C 头文件)"
lipo -info "$OUT/libmini_render.dylib"
