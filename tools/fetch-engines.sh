#!/usr/bin/env bash
# 兼容矩阵（game-doc/04 G9.1）用的第三方引擎：从 npm 拉固定版本，sha256 写死，放进各夹具的 libs/（不入库）。
#   three.js r160（three.cjs；Chrome 对照页用 three.module.js）、PixiJS 7.4.2、Phaser 3.80.1
# Cocos Creator / LayaAir 的导出物要编辑器生成，这里没有（见 game-doc/验收/G9.md）。
set -euo pipefail
cd "$(dirname "$0")/.."
TMP=$(mktemp -d); trap 'rm -rf "$TMP"' EXIT
get() { # 包 版本 包内路径 sha256 目标
    local tgz="$TMP/$1-$2.tgz"
    [ -f "$tgz" ] || curl -sSL -o "$tgz" "https://registry.npmjs.org/$1/-/$(basename "$1")-$2.tgz"
    mkdir -p "$TMP/x-$1" "$(dirname "$5")"
    tar xzf "$tgz" -C "$TMP/x-$1" "package/$3"
    echo "$4  $TMP/x-$1/package/$3" | shasum -a 256 -c - >/dev/null || { echo "❌ $1 $3 校验和不符"; exit 1; }
    cp "$TMP/x-$1/package/$3" "$5"
}
get three 0.160.0 build/three.cjs b2350ed1fdef4cbb22daffb337187e86f09652198f0d3a35a613d9ebe97b30f6 sample-game/e-three/libs/three.js
get three 0.160.0 build/three.module.js 76dea8151bc9352aef3528b4262e249b2604f62543828328db978d060d61a495 target/engine-ref/three.module.js
get pixi.js 7.4.2 dist/pixi.min.js 9ddba9cd78bc8610a1d445ec939393888be83925c78e40d66d9a17e98450228d sample-game/e-pixi/libs/pixi.js
# PixiJS 的 UMD 只声明 var PIXI：补一行导出给 require 用
grep -q "module.exports = PIXI" sample-game/e-pixi/libs/pixi.js || printf '\nmodule.exports = PIXI;\n' >> sample-game/e-pixi/libs/pixi.js
get phaser 3.80.1 dist/phaser.min.js 62081f6a1b51d040473f919ffedef9009953c20833518e146da5a8d1c9405ea8 sample-game/e-phaser/libs/phaser.js
for d in e-three e-pixi e-phaser; do mkdir -p "sample-game/$d"; done
echo "✓ 引擎就绪"
