#!/usr/bin/env bash
# A8.2：three.js 同一个场景，小游戏（ANGLE + SwiftShader，无头）对 Chrome（--use-angle=swiftshader）逐像素比。
# 阈值 0.1%（game-doc/验收/G8.md）。先要 tools/fetch-angle.sh 与 tools/fetch-engines.sh
set -euo pipefail
cd "$(dirname "$0")/.."
. "$HOME/.cargo/env"
R=$PWD/target/engine-ref
cargo test --release --offline --test game_e2e -- e1_three -q >/dev/null
cp sample-game/e-three/scene.js sample-game/e-three/ref.html "$R/"
PORT=${PORT:-8799}
python3 -m http.server "$PORT" --directory "$R" >/dev/null 2>&1 & S=$!
trap 'kill $S 2>/dev/null' EXIT
sleep 1.5
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --use-angle=swiftshader --enable-unsafe-swiftshader --hide-scrollbars \
    --window-size=375,667 --force-device-scale-factor=1 --virtual-time-budget=5000 --screenshot="$R/three-chrome.png" "http://localhost:$PORT/ref.html?frames=30" 2>/dev/null
OUT=$(python3 tools/pixdiff.py "$R/three-chrome.png" "$R/three-mini.png" | tail -1)
echo "$OUT"
PCT=$(echo "$OUT" | sed -E 's/.*\(([0-9.]+)%\).*/\1/')
python3 -c "import sys; sys.exit(0 if float('$PCT') <= 0.1 else 1)" && echo "✅ three.js 与 Chrome 差异 ≤ 0.1%" || { echo "❌ 超过 0.1%"; exit 1; }
