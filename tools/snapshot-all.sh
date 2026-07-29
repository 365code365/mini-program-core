#!/usr/bin/env bash
# 把一个小程序的所有页面逐页截图到指定目录（compare 的 --rust-from 输入）。
#
# 用法: bash tools/snapshot-all.sh <app> <输出目录> [--settle 秒] [--time 秒]
#   bash tools/snapshot-all.sh sample-app       target/fin2_sample
#   bash tools/snapshot-all.sh sample/news-app  target/fin2_news
#
# 注意：跑之前一定要清 target/mini-storage。无头点击/输入会把数据写进
# `target/mini-storage/<app>.json`，下次启动被读回来，页面内容就和基线不一样了
# （曾让 sample 的购物车页从 1.51% 假崩到 10.05%）。
set -u
BIN=./target/release/mini-app-window
APP="${1:?用法: snapshot-all.sh <app> <输出目录>}"
OUT="${2:?用法: snapshot-all.sh <app> <输出目录>}"
shift 2
EXTRA=("$@")
if [ ${#EXTRA[@]} -eq 0 ]; then EXTRA=(--settle 1 --time 2); fi
if [ ! -x "$BIN" ]; then echo "先 cargo build --release"; exit 1; fi

# 解析 app.json 的 pages 列表（纯配置读取）
APP_DIR="$APP"
[ -d "$APP_DIR" ] || APP_DIR="sample/$APP"
[ -f "$APP_DIR/app.json" ] || { echo "找不到 $APP_DIR/app.json"; exit 1; }
PAGES=$(python3 -c "
import json,sys
d=json.load(open('$APP_DIR/app.json'))
print('\n'.join(d.get('pages',[])))
")

rm -rf "$OUT"
n=0
for p in $PAGES; do
    rm -rf target/mini-storage
    if "$BIN" "$APP" --route "$p" "${EXTRA[@]}" --snapshot "$OUT" >/dev/null 2>&1; then
        if [ -f "$OUT/$p/rust.png" ]; then n=$((n+1)); else echo "  ⚠️  $p 没出图"; fi
    else
        echo "  ❌ $p 截图失败"
    fi
done
rm -rf target/mini-storage
echo "$APP -> $OUT : $n/$(echo "$PAGES" | wc -l | tr -d ' ') 页"
