#!/usr/bin/env bash
# 小游戏分支「纯搬迁 / 不影响小程序」的等价证明（game-doc/04 §0.1）：
# 与 G0.1 在未改动的 main 上生成的 target/baseline_main_* 逐字节比较。
#
#   bash tools/game-equiv.sh            # 快照 md5 + 交互 + damage + gallery + tabBar + SDK 一致性
#
# 退出码：0 = 全部一致；1 = 有不一致（看输出里标 ✗ 的那一行）；2 = 缺基线（先按 04 G0.1 生成）
set -u
cd "$(dirname "$0")/.."
. "$HOME/.cargo/env" 2>/dev/null || true
# 环境里残留的 CARGO_* 会让工具用到旧二进制（steering 30），一律清掉
for v in $(env | grep -oE '^CARGO_(TARGET_DIR|PROFILE_[A-Z_]+|BUILD_[A-Z_]+)=' | tr -d '='); do unset "$v"; done
for d in target/baseline_main_sample target/baseline_main_news target/baseline_main_ia; do
    [ -d "$d" ] || { echo "缺基线 ${d}（先按 game-doc/04 G0.1 生成）"; exit 2; }
done
FAIL=0
OUT=target/_game_equiv
rm -rf "$OUT"; mkdir -p "$OUT"
clear_storage() { rm -rf target/mini-storage target/sdk-parity/mini-storage; }

# ① 逐页快照 md5
cmp_snap() {
    local app="$1" base="$2" out="$3" n=0 bad=0 f rel
    clear_storage
    bash tools/snapshot-all.sh "$app" "$out" --settle 0 --time 2 > "$out.log" 2>&1 || { echo "✗ 快照失败 ${app}（$out.log）"; FAIL=1; return; }
    while IFS= read -r f; do
        rel=${f#"$base"/}; n=$((n + 1))
        if ! cmp -s "$f" "$out/$rel"; then
            bad=$((bad + 1)); echo "  ✗ $rel"; python3 tools/pixdiff.py "$f" "$out/$rel" 2>/dev/null | tail -1
        fi
    done < <(find "$base" -name '*.png' | sort)
    [ "$bad" = 0 ] && echo "✓ 快照 ${app}：$n/$n 逐字节一致" || { echo "✗ 快照 ${app}：$bad/$n 不一致"; FAIL=1; }
}
cmp_snap sample-app target/baseline_main_sample "$OUT/sample"
cmp_snap sample/news-app target/baseline_main_news "$OUT/news"

# ② 交互（与基线逐张比较，脚本自带空转守卫）
clear_storage
# showcase 页的 loading.gif 按墙上时钟取帧（steering 30 的已知例外），偶发差一块约 82×84 设备像素、包围盒 x≈616~698。
# 不一致时重跑一次：重跑一致就按时钟噪声报，仍不一致才算失败（两次都落在 GIF 上的概率很低，真回归两次都会错）
if bash tools/interaction-check.sh "$OUT/ia" target/baseline_main_ia > "$OUT/ia.log" 2>&1; then echo "✓ 交互：$(tail -1 "$OUT/ia.log")"
else
    first=$(grep -E '✗' "$OUT/ia.log" | head -1)
    rm -rf "$OUT/ia"; clear_storage
    if bash tools/interaction-check.sh "$OUT/ia" target/baseline_main_ia > "$OUT/ia2.log" 2>&1; then
        echo "✓ 交互：重跑一致（第一次的差异按 GIF 时钟噪声处理：${first}）"
    else echo "✗ 交互，重跑仍不一致（$OUT/ia.log、$OUT/ia2.log）"; FAIL=1; fi
fi

# ③ 增量 == 整帧
clear_storage
if bash tools/damage-check.sh > "$OUT/damage.log" 2>&1; then echo "✓ damage：$(tail -1 "$OUT/damage.log")"; else echo "✗ damage（$OUT/damage.log）"; FAIL=1; fi

# ④ gallery 与 HEAD
clear_storage
./target/release/examples/gallery > "$OUT/gallery.log" 2>&1 || { echo "✗ gallery 运行失败"; FAIL=1; }
changed=$(git status --porcelain doc/gallery | grep -v '35_gif_frame4' || true)
[ -z "$changed" ] && echo "✓ gallery 与 HEAD 一致（35_gif_frame4 按已知噪声除外）" || { echo "✗ gallery 有变化："; echo "$changed"; FAIL=1; }

# ⑤ tabBar 点击（--touch）
clear_storage
if bash tools/tab-click-check.sh > "$OUT/tab.log" 2>&1; then echo "✓ tabBar：$(tail -1 "$OUT/tab.log")"; else echo "✗ tabBar（$OUT/tab.log）"; FAIL=1; fi

# ⑥ SDK 与桌面逐像素（用本次快照作参照，等价于与基线比：①已证明本次快照 = 基线）
clear_storage
rm -rf target/fin2_sample target/fin2_news
cp -R "$OUT/sample" target/fin2_sample; cp -R "$OUT/news" target/fin2_news
if bash tools/sdk-parity.sh > "$OUT/sdk1.log" 2>&1 && bash tools/sdk-parity.sh sample/news-app target/fin2_news > "$OUT/sdk2.log" 2>&1; then
    echo "✓ SDK 一致性：$(tail -1 "$OUT/sdk1.log") / $(tail -1 "$OUT/sdk2.log")"
else echo "✗ SDK 一致性（$OUT/sdk*.log）"; FAIL=1; fi
rm -rf target/fin2_sample target/fin2_news "$OUT/sample" "$OUT/news" "$OUT/ia"
clear_storage
python3 tools/prune-target.py --quiet >/dev/null 2>&1 || true
[ "$FAIL" = 0 ] && echo "全部一致 ✅" || echo "有不一致 ❌"
exit $FAIL
