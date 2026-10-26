#!/bin/bash
# Mini Render Engine 构建脚本

set -e

echo "🔨 构建 Mini Render Engine..."

# 构建 Rust 库
cargo build --release

echo "✅ 动态库构建完成！"
# cargo 不回收旧产物：剪掉换下来的那几份（判据见 tools/prune-target.py 文件头）
python3 "$(dirname "$0")/tools/prune-target.py" --quiet || true

# 显示生成的库文件
echo ""
echo "📦 生成的库文件:"
if [[ "$OSTYPE" == "darwin"* ]]; then
    ls -la target/release/libmini_render.dylib 2>/dev/null || true
elif [[ "$OSTYPE" == "linux"* ]]; then
    ls -la target/release/libmini_render.so 2>/dev/null || true
fi

echo ""
echo "📄 C 头文件: include/mini_render.h"
echo ""
echo "🚀 运行 Rust 示例:"
echo "   cargo run --example demo"
echo ""
echo "🔗 C 程序链接示例 (macOS):"
echo "   clang examples/demo.c -L target/release -lmini_render -o demo_c"
echo "   DYLD_LIBRARY_PATH=target/release ./demo_c"
