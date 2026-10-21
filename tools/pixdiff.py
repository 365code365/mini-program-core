#!/usr/bin/env python3
"""两张 PNG 的像素差异摘要：差异像素数、占比、包围盒、最大通道差。
用法: python3 tools/pixdiff.py <a.png> <b.png> [--box x0,y0,x1,y1]
用于「改动前后逐页快照」的回归判定：0 差异 = 无回归；
有差异时给出包围盒，便于判断是真回归还是时钟/动画相位噪声。

`--box` 只比这个矩形（含端点，像素坐标），占比也按矩形面积算。页面上有按墙上时钟走的
GIF / 倒计时时，整页比较会被噪声淹没，交互检查用它只看被操作的那一块。
"""
import sys
from PIL import Image
args = [a for a in sys.argv[1:]]
box = None
if "--box" in args:
    i = args.index("--box")
    box = [int(v) for v in args[i + 1].split(",")]
    del args[i:i + 2]
a = Image.open(args[0]).convert("RGB")
b = Image.open(args[1]).convert("RGB")
if a.size != b.size:
    print(f"尺寸不同: {a.size} vs {b.size}")
    raise SystemExit(2)
la, lb = a.load(), b.load()
w, h = a.size
bx0, by0, bx1, by1 = box if box else (0, 0, w - 1, h - 1)
bx0, by0 = max(bx0, 0), max(by0, 0)
bx1, by1 = min(bx1, w - 1), min(by1, h - 1)
n = 0
x0, y0, x1, y1 = w, h, -1, -1
worst = 0
for y in range(by0, by1 + 1):
    for x in range(bx0, bx1 + 1):
        pa, pb = la[x, y], lb[x, y]
        if pa != pb:
            n += 1
            d = max(abs(pa[i] - pb[i]) for i in range(3))
            worst = max(worst, d)
            x0, y0 = min(x0, x), min(y0, y)
            x1, y1 = max(x1, x), max(y1, y)
total = (bx1 - bx0 + 1) * (by1 - by0 + 1)
if n == 0:
    print("像素完全一致")
else:
    print(
        f"差异像素 {n}/{total} ({n / total * 100:.3f}%)  "
        f"包围盒 x[{x0},{x1}] y[{y0},{y1}]  最大通道差 {worst}"
    )
