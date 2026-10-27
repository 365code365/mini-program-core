#!/usr/bin/env bash
# 拉 Khronos WebGL CTS（只要 sdk/tests 里用得到的目录，稀疏克隆，约 5MB）到 /tmp/cts/WebGL，固定到 3a65272e
set -euo pipefail
D=${1:-/tmp/cts}; mkdir -p "$D"; cd "$D"
[ -d WebGL ] || git clone -q --filter=blob:none --sparse https://github.com/KhronosGroup/WebGL.git
cd WebGL
git sparse-checkout set sdk/tests/js sdk/tests/resources sdk/tests/conformance
git checkout -q 3a65272e40f97c82de335825ea77c4da8e77a637 2>/dev/null || true
echo "✓ $D/WebGL/sdk/tests"
