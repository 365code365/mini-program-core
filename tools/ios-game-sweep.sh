#!/usr/bin/env bash
# iOS 真机小游戏逐个速测：每个游戏开 N 秒（-gameStats 1），汇总帧率、真掉帧、pump 分位、自适应降帧、报错。
#
#   bash tools/ios-game-sweep.sh                       # 商店里全部小游戏，每个 40 秒
#   bash tools/ios-game-sweep.sh 30 play-3d-runner farm001   # 指定秒数与游戏
#
# 前提：原生示例已装在手机上（bash tools/ios-debug.sh 装过），手机解锁、联网（游戏从商店拉）。
# 启动参数统一带 `?auto=1`：认的游戏（跑酷、萌宠小岛 …）自动玩，不认的停在首屏（也照样量帧）。
# 判据（steering 10，不看平均帧率）：掉帧 = 上屏间隔超过 1.5 倍目标间隔（按 vsync 时刻，GameFrameStats.swift）；
# pump 的 p95 超过 16.7ms 就是 60fps 撑不住了。结果落 target/ios-game-sweep/<时刻>/（summary.txt + 每个游戏的日志）
set -u
cd "$(dirname "$0")/.."
SECS="${1:-40}"; shift || true
UDID="${IOS_UDID:-$(xcrun devicectl list devices 2>/dev/null | awk '/physical/ && /connected/ {for (i=1;i<=NF;i++) if ($i ~ /^[0-9A-F]{8}-[0-9A-F]{16}$/) print $i}' | head -1)}"
[ -n "$UDID" ] || { echo "没有连着的 iPhone"; exit 1; }
STORE="${MINI_STORE_URL:-https://mini-program.51aihelp.com}"
if [ $# -gt 0 ]; then GAMES="$*"; else
    GAMES=$(curl -s -m 15 "$STORE/api/apps?kind=all&limit=200" | python3 -c "import json,sys; print(' '.join(a['id'] for a in json.load(sys.stdin)['apps'] if a.get('kind')=='game'))")
fi
OUT="target/ios-game-sweep/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$OUT"
for g in $GAMES; do
    log="$OUT/$g.log"
    ( timeout $((SECS + 15)) xcrun devicectl device process launch --console --terminate-existing --device "$UDID" \
        dev.minirender.example.native -- -open "$g" -route "?auto=1" -gameStats 1 ${IOS_ARGS:-} >"$log" 2>&1 & )
    sleep $((SECS + 17))
    python3 - "$g" "$log" <<'PY' | tee -a "$OUT/summary.txt"
import re, sys, statistics as st
g, path = sys.argv[1], sys.argv[2]
t = open(path, errors='ignore').read()
rows = re.findall(r'上屏 (\d+) 帧（([\d.]+) fps，目标 (\d+)）间隔中位 ([\d.]+) 最大 ([\d.]+)ms 掉帧 (\d+)；pump 中位 ([\d.]+) p95 ([\d.]+) 最大 ([\d.]+)ms', t)
rows = rows[1:]  # 第一段含建实例、编着色器
if not rows:
    print(f"{g:18} ⚠ 没有帧统计（没打开 / 没装最新的示例 / 锁屏）"); sys.exit()
fps = [float(r[1]) for r in rows]; drops = sum(int(r[5]) for r in rows); worst = max(float(r[4]) for r in rows)
p50 = st.median(float(r[6]) for r in rows); p95 = max(float(r[7]) for r in rows)
gov = len(re.findall('自适应帧率 60 → 30', t)); errs = len(re.findall(r'⚠️ \[mini-game\]|\"type\":\"error\"', t))
print(f"{g:18} fps 中位 {st.median(fps):5.1f} 最低 {min(fps):5.1f} | 掉帧 {drops:3d} 最长间隔 {worst:6.1f}ms | pump 中位 {p50:5.1f} p95 峰 {p95:5.1f}ms | 降到 30 {gov} 次 | 报错 {errs}")
PY
done
echo "结果：$OUT/summary.txt"
