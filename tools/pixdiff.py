#!/usr/bin/env python3
"""两张 PNG 的像素差异摘要：差异像素数、占比、包围盒、最大通道差。

用法: python3 tools/pixdiff.py <a.png> <b.png>
用于「改动前后逐页快照」的回归判定：0 差异 = 无回归；
有差异时给出包围盒，便于判断是真回归还是时钟/动画相位噪声。
"""
import sys
from PIL import Image

a = Image.open(sys.argv[1]).convert("RGB")
b = Image.open(sys.argv[2]).convert("RGB")
if a.size != b.size:
    print(f"尺寸不同: {a.size} vs {b.size}")
    raise SystemExit(2)
la, lb = a.load(), b.load()
w, h = a.size
n = 0
x0, y0, x1, y1 = w, h, -1, -1
worst = 0
for y in range(h):
    for x in range(w):
        pa, pb = la[x, y], lb[x, y]
        if pa != pb:
            n += 1
            d = max(abs(pa[i] - pb[i]) for i in range(3))
            worst = max(worst, d)
            x0, y0 = min(x0, x), min(y0, y)
            x1, y1 = max(x1, x), max(y1, y)
total = w * h
if n == 0:
    print("像素完全一致")
else:
    print(
        f"差异像素 {n}/{total} ({n / total * 100:.3f}%)  "
        f"包围盒 x[{x0},{x1}] y[{y0},{y1}]  最大通道差 {worst}"
    )
