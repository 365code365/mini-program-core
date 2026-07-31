#!/usr/bin/env bash
# SDK 宿主（MiniEngine，移动端那条路）与桌面窗体的像素一致性对比。
#
#   bash tools/sdk-parity.sh                  # sample-app，对照 target/fin2_sample
#   bash tools/sdk-parity.sh sample/news-app target/fin2_news
#
# 判据：**逐像素完全一致**。两条路共用 host:: 下的渲染与合成，只有帧调度是各自的，
# 所以只要哪天开始分叉，这里立刻能看出来在哪一页。
set -u
# 快照回归统一用**固定日期**：有页面用 `new Date()` 高亮当天（news-app 的签到日历），
# 不固定的话基线跨天就失效 —— 会在日历那一格上冒出 0.1~0.8% 的差异，看着像渲染坏了。
# 想临时换日期：`MINI_FAKE_NOW=2025-01-01 bash tools/xxx.sh`
export MINI_FAKE_NOW="${MINI_FAKE_NOW:-2024-03-15}"
APP="${1:-sample-app}"
BASE="${2:-target/fin2_sample}"
OUT=target/_sdk_parity
# SDK 侧的数据目录（storage / 图片缓存）在 examples/sdk_parity.rs 里指到了
# `target/sdk-parity/`，与桌面的 `target/mini-storage` 不是一处。两边都要清 ——
# 只清一处的下场：news-app 的「已读 N 篇」在 SDK 侧一轮轮攒下去，
# 下次对比就冒出个 0.07% 的假差异（第一次遇到时以为是渲染改坏了）。
rm -rf "$OUT" target/mini-storage target/sdk-parity
cargo run --release --example sdk_parity -- "$APP" "$OUT" >/dev/null 2>&1 || {
    echo "❌ 渲染失败，单独跑一次看报错：cargo run --release --example sdk_parity -- $APP $OUT"; exit 1; }

same=0; diff=0; miss=0
for f in $(find "$BASE" -name rust.png | sort); do
    rel="${f#"$BASE"/}"
    mine="$OUT/$rel"
    page="$(dirname "$rel")"
    if [ ! -f "$mine" ]; then
        printf "  ⚠️  %-30s SDK 侧没出图\n" "$page"; miss=$((miss+1)); continue
    fi
    d=$(python3 tools/pixdiff.py "$f" "$mine" 2>&1 | tail -1)
    case "$d" in
        *完全一致*) same=$((same+1)) ;;
        *) printf "  ✗ %-30s %s\n" "$page" "$d"; diff=$((diff+1)) ;;
    esac
done
echo "一致 $same / 不一致 $diff / 缺图 $miss"
[ "$diff" = 0 ] && [ "$miss" = 0 ] && echo "✅ SDK 宿主与桌面像素一致" || exit 1
