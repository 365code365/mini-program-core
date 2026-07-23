#!/usr/bin/env bash
# 构建 Linux SDK：生成 .so 动态库与 .a 静态库
# 在 Linux 主机执行；交叉编译到 aarch64 需安装对应 gcc 工具链
set -euo pipefail
cd "$(dirname "$0")/.."

TARGET="${1:-$(rustc -vV | sed -n 's/host: //p')}"

echo "==> 目标: $TARGET"
rustup target add "$TARGET" || true
cargo build --release --lib --target "$TARGET"

OUT="target/linux/$TARGET"
mkdir -p "$OUT"
cp "target/$TARGET/release/libmini_render.so" "$OUT/" 2>/dev/null || true
cp "target/$TARGET/release/libmini_render.a"  "$OUT/" 2>/dev/null || true
cp include/mini_render.h "$OUT/"

echo "✅ Linux SDK 产物位于 $OUT"
echo "   常见目标: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu"
echo "   交叉到 aarch64 示例:"
echo "     rustup target add aarch64-unknown-linux-gnu"
echo "     sudo apt-get install gcc-aarch64-linux-gnu"
echo "     CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \\"
echo "       scripts/build-linux.sh aarch64-unknown-linux-gnu"
