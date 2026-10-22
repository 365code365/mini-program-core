#!/usr/bin/env bash
# 后台跑全量回归（tools/regress.sh），结果落进一个带时间戳的批次目录，方便回溯优化过程。
#
#   bash tools/regress-async.sh [标签] [regress.sh 的参数…]   # 立刻返回，打印批次目录
#   bash tools/regress-async.sh --status                       # 看所有批次（最新在最后）
#   tail -f target/test_batches/<批次>/run.log                  # 实时看
#
# 批次目录 target/test_batches/<YYYYmmdd-HHMMSS>[-标签]/：
#   meta.txt     开跑时刻、git HEAD、未提交改动的 diff --stat 与 diff 的 sha（同一份代码跑两次能认出来）
#   run.log      regress.sh 的全部输出，每行前面加 [HH:MM:SS]（实时写）
#   status       RUNNING → PASS / FAIL（退出码）；结束时刻
#   summary.txt  regress.sh 的分步汇总（每步耗时与结论）
#   logs/        每一步的完整日志（拷自 target/_regress/）
# target/test_batches/index.tsv 每个批次一行：开始 / 结束 / 结论 / 标签 / HEAD / diff sha —— 时间线。
#
# 用法约定（steering 30）：先定向速测、先修发现的问题，**再**发起这个全量；它跑的时候别改 Rust 源码
# （跑的就是工作区本身，改了后半程测的就是另一份代码 —— meta.txt 里的 diff sha 只记开跑那一刻）。
# clean-target.sh 不删 test_batches（白名单），要清就手动删旧批次。
set -u
cd "$(dirname "$0")/.."
ROOT=target/test_batches
mkdir -p "$ROOT"
if [ "${1:-}" = "--status" ]; then
    [ -f "$ROOT/index.tsv" ] && column -t -s $'\t' "$ROOT/index.tsv" || echo "还没有批次"
    for d in "$ROOT"/*/; do
        [ -f "$d/status" ] && grep -q RUNNING "$d/status" && echo "进行中：${d}（$(tail -1 "$d/run.log" | cut -c1-100)）"
    done
    exit 0
fi
LABEL=""
if [ $# -gt 0 ] && [ "${1#--}" = "$1" ]; then LABEL="-$1"; shift; fi
TS=$(date +%Y%m%d-%H%M%S)
B="$ROOT/$TS$LABEL"
mkdir -p "$B/logs"
HEAD=$(git rev-parse --short HEAD)
DIFF_SHA=$(git diff HEAD -- . ':(exclude)dist-*' | shasum | cut -c1-10)
{
    echo "开跑    $(date '+%F %T')"
    echo "HEAD    $HEAD"
    echo "diff    ${DIFF_SHA}（未提交改动的 sha，排除 dist-*）"
    echo "参数    ${*:-（无）}"
    echo "负载    $(uptime | sed 's/.*load averages*: //')"
    echo
    git diff HEAD --stat -- . ':(exclude)dist-*' | tail -40
} > "$B/meta.txt"
echo "RUNNING $(date '+%F %T')" > "$B/status"
START=$(date '+%F %T')
(
    bash tools/regress.sh "$@" 2>&1 | while IFS= read -r line; do printf '[%s] %s\n' "$(date +%H:%M:%S)" "$line"; done > "$B/run.log"
    rc=${PIPESTATUS[0]}
    cp target/_regress/summary.txt "$B/" 2>/dev/null
    cp target/_regress/*.log "$B/logs/" 2>/dev/null
    v=$([ "$rc" = 0 ] && echo PASS || echo FAIL)
    echo "$v rc=$rc $(date '+%F %T')" > "$B/status"
    [ -f "$ROOT/index.tsv" ] || printf '开始\t结束\t结论\t批次\tHEAD\tdiff\n' > "$ROOT/index.tsv"
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$START" "$(date '+%F %T')" "$v" "$TS$LABEL" "$HEAD" "$DIFF_SHA" >> "$ROOT/index.tsv"
) > /dev/null 2>&1 &
disown 2>/dev/null || true
echo "全量回归已在后台开跑：$B"
echo "  实时：tail -f $B/run.log    结论：cat $B/status    全部批次：bash tools/regress-async.sh --status"
