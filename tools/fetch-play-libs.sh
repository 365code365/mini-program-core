#!/usr/bin/env bash
# 3D 可玩示例（sample-game/play-3d-*）用的第三方库：从 npm 拉固定版本、sha256 写死，放进各包的 libs/（不入库，见 .gitignore）。
#   three.js r160（build/three.cjs，MIT）—— 与 G9.1 兼容矩阵（tools/fetch-engines.sh）同一个版本、同一个校验和
#   cannon-es 0.20.0（dist/cannon-es.cjs.js，MIT）—— 刚体物理（保龄球）
# 包要自包含（能单独上传 mini-store / 微信后台），所以每个包各放一份；上传前先跑这个脚本。
set -euo pipefail
cd "$(dirname "$0")/.."
TMP=$(mktemp -d); trap 'rm -rf "$TMP"' EXIT
get() { # 包 版本 包内路径 sha256 目标…
    local pkg=$1 ver=$2 path=$3 sum=$4; shift 4
    local tgz="$TMP/$(basename "$pkg")-$ver.tgz"
    [ -f "$tgz" ] || curl -sSL -o "$tgz" "https://registry.npmjs.org/$pkg/-/$(basename "$pkg")-$ver.tgz"
    mkdir -p "$TMP/x-$pkg"
    tar xzf "$tgz" -C "$TMP/x-$pkg" "package/$path"
    echo "$sum  $TMP/x-$pkg/package/$path" | shasum -a 256 -c - >/dev/null || { echo "❌ $pkg $path 校验和不符"; exit 1; }
    for dst in "$@"; do mkdir -p "$(dirname "$dst")"; cp "$TMP/x-$pkg/package/$path" "$dst"; done
}
THREE_SUM=b2350ed1fdef4cbb22daffb337187e86f09652198f0d3a35a613d9ebe97b30f6
get three 0.160.0 build/three.cjs $THREE_SUM \
    sample-game/play-3d-stack/libs/three.js sample-game/play-3d-runner/libs/three.js sample-game/play-3d-bowling/libs/three.js \
    sample-game/play-3d-island/libs/three.js
get cannon-es 0.20.0 dist/cannon-es.cjs.js 58255a3214dfcddd41e4d5dbc47e3fb39e5679da67c1f47ca5ce0c788c9eef3f \
    sample-game/play-3d-bowling/libs/cannon-es.js
# cannon-es 的 CJS 版是给 Node 打的：`require('perf_hooks')` 拿 performance（Node 内置模块），小游戏里没有这个模块
# （微信也没有），require 直接抛错、整个包起不来。换成全局 performance（小游戏有，与 rAF 同一时间轴）。改完再校验一次，
# 上游改了这一行的写法就报错，别悄悄留着原样
C=sample-game/play-3d-bowling/libs/cannon-es.js
sed -i.bak "s/^const performance = require('perf_hooks') && require('perf_hooks').performance || {};$/const performance = (typeof globalThis !== 'undefined' \&\& globalThis.performance) || {};/" "$C" && rm -f "$C.bak"
grep -q "require('perf_hooks')" "$C" && { echo "❌ cannon-es 里的 perf_hooks 没替换掉"; exit 1; }
echo "✓ 3D 示例的库就绪"
