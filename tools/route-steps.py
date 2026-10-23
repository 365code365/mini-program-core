"""android-jank.sh 用：从引擎日志里取每一次页面转场的 t 序列（🩹 行尾「转场 0.123」），
看起步那一步、最大那一步是中位一步的几倍（理想都 ≈1；起步那一步明显大就是「点进页面一顿」）。

    python3 tools/route-steps.py target/_jank/log.txt
"""
import re
import statistics
import sys

runs, cur = [], []
for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
    if "🩹" not in line:
        continue
    m = re.search(r"转场 ([\d.]+)", line)
    if m:
        cur.append(float(m.group(1)))
    elif cur:
        runs.append(cur)
        cur = []
if cur:
    runs.append(cur)
for k, ts in enumerate(runs, 1):
    d = [b - a for a, b in zip(ts, ts[1:])]
    moving = [x for x in d if x > 0]
    if len(moving) < 3:
        continue
    med = statistics.median(moving)
    print(
        f"  转场 {k}：{len(ts)} 帧，原地 {len(d) - len(moving)} 帧，"
        f"起步一步 {moving[0] / med:.2f}× 中位，最大一步 {max(moving) / med:.2f}×（中位 Δt {med:.3f}）"
    )
