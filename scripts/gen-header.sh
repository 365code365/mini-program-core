#!/usr/bin/env bash
# 用 cbindgen 从 FFI 源码重新生成 C 头文件 include/mini_render.h
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v cbindgen >/dev/null 2>&1; then
  echo "缺少 cbindgen，请先执行: cargo install cbindgen" >&2
  exit 1
fi

cbindgen --config cbindgen.toml --crate mini-render --output include/mini_render.h
echo "✅ 已生成 include/mini_render.h"
