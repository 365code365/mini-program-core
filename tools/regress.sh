#!/usr/bin/env bash
# 一条命令跑完全量回归（.kiro/steering/30-test-and-cleanup.md 的那几项 + 逐页快照 + 双端对比），
# 每一步单独计时、单独落日志，最后给一张汇总表。
#
#   bash tools/regress.sh            # 全量
#   bash tools/regress.sh --no-h5    # 跳过 H5 双端对比（要起 headless Chrome）
#   bash tools/regress.sh --keep     # 跑完保留快照 / 对比图（默认只留日志与汇总）
#
# 为什么要有它：从前是手敲八九条命令，每条之间等人看结果，一轮几个小时。
# 这里把各步串成一条命令，**每步给出耗时**，哪一步慢一眼可见；
# 开跑前先打一行负载与交换区占用（内存吃紧时换页能把构建拖慢一个数量级）。
#
# 各步之间**不并行**：它们共用 target/mini-storage（无头点击会落盘），
# 并行会读到对方刚写的数据，出来的是假差异。
#
# 退出码：0 = 全部通过；1 = 有步骤失败（看汇总里标 FAIL 的那一行与它的日志）。
# 「看图」= 不判失败但需要人看：gallery 与 HEAD 有差异（附差异占比与包围盒）。
set -u
cd "$(dirname "$0")/.."
. "$HOME/.cargo/env" 2>/dev/null || true
# 各工具脚本写死用 ./target/release/ 下的二进制。环境里残留的 CARGO_TARGET_DIR /
# CARGO_PROFILE_* 会让 cargo 编到别处、或换一套编译参数，而快照用的还是 target/release 里
# **旧的**二进制 —— 测试全绿、gallery 却是几小时前的代码画的（2026-10 真踩过：
# 一条 `export` 留在了 shell 里，后面整轮回归的结论全作废）。这里一律清掉并打出来。
for v in $(env | grep -oE '^CARGO_(TARGET_DIR|PROFILE_[A-Z_]+|BUILD_[A-Z_]+)=' | tr -d '='); do
    echo "⚠️  忽略环境里的 $v=${!v}（回归必须用 Cargo.toml 的 release 配置编进 target/）"
    unset "$v"
done
H5=1
KEEP=0
for a in "$@"; do
    case "$a" in
        --no-h5) H5=0 ;;
        --keep) KEEP=1 ;;
        *) echo "未知参数 $a（可用：--no-h5 --keep）"; exit 2 ;;
    esac
done
OUT=target/_regress
rm -rf "$OUT"
mkdir -p "$OUT"
SUMMARY="$OUT/summary.txt"
: > "$SUMMARY"
FAILED=0
T0=$(date +%s)

# step <名字> <函数>：跑一步，记耗时、退出码，结论取日志最后一行非空行
step() {
    local name="$1" fn="$2" log="$OUT/$1.log" s rc mark verdict
    s=$(date +%s)
    "$fn" > "$log" 2>&1
    rc=$?
    case $rc in
        0) mark=OK ;;
        3) mark=看图 ;;
        *) mark=FAIL; FAILED=$((FAILED + 1)) ;;
    esac
    verdict=$(grep -vE '^[[:space:]]*$' "$log" | tail -1 | cut -c1-110)
    printf '%-12s %5ss  %-4s  %s\n' "$name" "$(( $(date +%s) - s ))" "$mark" "$verdict" | tee -a "$SUMMARY"
}

clear_storage() { rm -rf target/mini-storage target/sdk-parity/mini-storage; }

# ① 构建：一次 cargo 调用编全部目标（lib / bin / example / 测试），cargo 能把它们排满所有核
do_build() {
    clear_storage
    cargo build --release --all-targets && echo "构建完成"
}

# ② 单测 + 集成测试：汇总通过 / 失败数，并列出失败的测试二进制
do_test() {
    clear_storage
    cargo test --release --no-fail-fast
    local rc=$?
    awk '/^test result:/ { for (i = 1; i <= NF; i++) { if ($(i+1) ~ /^passed;/) p += $i; if ($(i+1) ~ /^failed;/) f += $i } }
         END { printf "测试：通过 %d，失败 %d\n", p, f }' "$OUT/test.log"
    return $rc
}

# ③ 增量重绘 == 整帧重绘（逐字节）
do_damage() { clear_storage; bash tools/damage-check.sh; }

# ④ 65 张场景图：与 HEAD 不同的逐张给差异占比与包围盒
do_gallery() {
    clear_storage
    ./target/release/examples/gallery || return 1
    local changed n=0 f
    changed=$(git diff HEAD --name-only -- doc/gallery | grep '\.png$')
    for f in $changed; do
        git show "HEAD:$f" > "$OUT/_head.png" 2>/dev/null || { echo "  $f（HEAD 里没有，新图）"; n=$((n + 1)); continue; }
        printf '  %-36s ' "$(basename "$f")"
        python3 tools/pixdiff.py "$OUT/_head.png" "$f" | tail -1
        n=$((n + 1))
    done
    rm -f "$OUT/_head.png"
    [ "$n" = 0 ] && { echo "gallery 与 HEAD 逐字节一致"; return 0; }
    # 35_gif_frame4 按墙上时钟取 GIF 帧，单独变了算噪声（见 steering 30）
    if [ "$changed" = "doc/gallery/35_gif_frame4.png" ]; then
        echo "只有 35_gif_frame4 变了（GIF 按墙上时钟取帧，已知噪声）"; return 0
    fi
    echo "gallery 有 $n 张与 HEAD 不同（含未提交的有意改动；35_gif_frame4 是时钟噪声）"
    return 3
}

# ⑤ 三个 app 的 tabBar 点击（--touch，走完整指针层）
do_tabclick() { clear_storage; bash tools/tab-click-check.sh; }

# 逐页快照（sdk-parity 与 H5 对比的输入）
do_snapshots() {
    rm -rf target/fin2_sample target/fin2_news
    clear_storage
    bash tools/snapshot-all.sh sample-app target/fin2_sample --settle 0 --time 2 || return 1
    clear_storage
    bash tools/snapshot-all.sh sample/news-app target/fin2_news --settle 0 --time 2
}

# ⑥ 移动端 SDK 与桌面窗体逐像素一致
do_sdkparity() {
    [ -d target/fin2_sample ] || { echo "没有 target/fin2_sample（快照那步失败了？）"; return 1; }
    bash tools/sdk-parity.sh
}

# ⑦ 指针层与覆盖层；有 target/baseline_ia 就同时做前后对比
do_interaction() {
    clear_storage
    if [ -d target/baseline_ia ]; then
        bash tools/interaction-check.sh "$OUT/ia" target/baseline_ia
    else
        bash tools/interaction-check.sh "$OUT/ia"
    fi
}

# 双端对比（原生 vs 编译出的 H5）。--html / --out 都放 target/ 下：
# 不给 --html 会重新编进入库的 dist-html / dist-news，把工作区弄脏
do_h5() {
    local rc=0 a b
    ./target/release/examples/compare --all --html "$OUT/h5_sample" \
        --rust-from target/fin2_sample --out "$OUT/cmp_sample" || rc=1
    ./target/release/examples/compare --all --app sample/news-app --html "$OUT/h5_news" \
        --rust-from target/fin2_news --out "$OUT/cmp_news" || rc=1
    a=$(grep -o '阈值差异像素 [0-9.]*%' "$OUT/h5.log" | sed -n 1p | grep -o '[0-9.]*%')
    b=$(grep -o '阈值差异像素 [0-9.]*%' "$OUT/h5.log" | sed -n 2p | grep -o '[0-9.]*%')
    echo "H5 对比：sample ${a:-?}，news ${b:-?}（基线 4.57% / 6.46%，不得回退）"
    return $rc
}

echo "回归开始 $(date '+%H:%M:%S')，负载 $(uptime | sed 's/.*load averages*: //')，交换区 $(sysctl -n vm.swapusage 2>/dev/null | awk '{print $6"/"$3}')" | tee -a "$SUMMARY"
step build do_build
if [ "$FAILED" = 0 ]; then
    step test do_test
    step damage do_damage
    step gallery do_gallery
    step tabclick do_tabclick
    step snapshots do_snapshots
    step sdkparity do_sdkparity
    step interaction do_interaction
    [ "$H5" = 1 ] && step h5 do_h5
else
    echo "构建失败，后面的步骤不跑了：看 $OUT/build.log" | tee -a "$SUMMARY"
fi
clear_storage

# 收尾：只清本脚本与它调用的工具写出来的东西（不跑 clean-target.sh：
# 那会连别的会话正在用的目录一起删）。日志与汇总留着，很小
rm -rf target/_sdk_parity target/dm_*_on target/dm_*_off /tmp/dm_*_on.log /tmp/dm_*_off.log
if [ "$KEEP" = 0 ]; then
    rm -rf target/fin2_sample target/fin2_news "$OUT/ia" "$OUT"/h5_* "$OUT"/cmp_*
fi
echo "总耗时 $(( $(date +%s) - T0 ))s，失败 $FAILED 步；日志在 $OUT/" | tee -a "$SUMMARY"
[ "$FAILED" = 0 ]
