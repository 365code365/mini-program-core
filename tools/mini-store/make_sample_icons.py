#!/usr/bin/env python3
"""给四个联网示例（movie-online / music-online / photo-wall / mini-hub）画商店图标与 tabBar 图标。

    python3 tools/mini-store/make_sample_icons.py

商店图标（icon.png，144×144）复用 make_icon.py 的画法；tabBar 图标 81×81 透明 PNG，
线条图标按名字画（未选中灰、选中用主题色），4 倍画布画完再缩小，边缘才平滑。
"""
import math
import os
import sys

from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import make_icon  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(HERE))
S, K = 81, 4                 # 输出边长、放大倍数
GRAY = '#9aa0ab'

APPS = {
    'movie-online': ('影', '#ff7a8a', '#e8243f', '#ff4e6a', [('home', 'tab-home'), ('mine', 'tab-mine')]),
    'music-online': ('听', '#ff8a6b', '#e0314f', '#f04c4c', [('disc', 'tab-disc'), ('chart', 'tab-chart'), ('mine', 'tab-mine')]),
    'photo-wall': ('拾光', '#60a5fa', '#2563eb', '#3b82f6', [('grid', 'tab-grid'), ('heart', 'tab-heart')]),
    'mini-hub': ('跳', '#9d8bff', '#4f46e5', '#6b5cff', [('link', 'tab-link'), ('form', 'tab-form')]),
}


def draw_icon(kind, color):
    n = S * K
    img = Image.new('RGBA', (n, n), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    w = 6 * K               # 线宽
    c = n / 2

    def box(x0, y0, x1, y1):
        return (x0 * K, y0 * K, x1 * K, y1 * K)
    if kind == 'home':
        d.line([(14 * K, 38 * K), (c, 14 * K), (67 * K, 38 * K)], fill=color, width=w, joint='curve')
        d.rounded_rectangle(box(22, 36, 59, 68), radius=4 * K, outline=color, width=w)
        d.rounded_rectangle(box(35, 48, 46, 68), radius=2 * K, fill=color)
    elif kind == 'mine':
        d.ellipse(box(28, 12, 53, 37), outline=color, width=w)
        d.pieslice(box(14, 42, 67, 96), 180, 360, outline=color, width=w)
    elif kind == 'disc':
        d.ellipse(box(12, 12, 69, 69), outline=color, width=w)
        d.ellipse(box(32, 32, 49, 49), fill=color)
        d.arc(box(21, 21, 60, 60), 200, 280, fill=color, width=w // 2)
    elif kind == 'chart':
        for i, h in enumerate((22, 40, 30)):
            x = 16 + i * 18
            d.rounded_rectangle(box(x, 66 - h, x + 11, 66), radius=3 * K, fill=color)
        d.line([(12 * K, 70 * K), (69 * K, 70 * K)], fill=color, width=w // 2)
    elif kind == 'grid':
        for (x, y, h) in ((14, 13, 30), (43, 13, 18), (14, 48, 20), (43, 36, 32)):
            d.rounded_rectangle(box(x, y, x + 24, y + h), radius=5 * K, outline=color, width=w)
    elif kind == 'heart':
        pts = []
        for i in range(120):
            t = i / 120 * math.tau
            x = 16 * math.sin(t) ** 3
            y = -(13 * math.cos(t) - 5 * math.cos(2 * t) - 2 * math.cos(3 * t) - math.cos(4 * t))
            pts.append((c + x * 1.9 * K, c + 2 * K + y * 1.9 * K))
        d.line(pts + [pts[0]], fill=color, width=w, joint='curve')
    elif kind == 'link':
        d.rounded_rectangle(box(10, 28, 44, 52), radius=12 * K, outline=color, width=w)
        d.rounded_rectangle(box(37, 28, 71, 52), radius=12 * K, outline=color, width=w)
    elif kind == 'form':
        d.rounded_rectangle(box(16, 10, 65, 71), radius=7 * K, outline=color, width=w)
        for y in (26, 40, 54):
            d.line([(26 * K, y * K), ((55 if y != 54 else 44) * K, y * K)], fill=color, width=w)
    return img.resize((S, S), Image.LANCZOS)


def main():
    for app, (glyph, c0, c1, theme, tabs) in APPS.items():
        base = os.path.join(ROOT, 'sample', app)
        make_icon.make(os.path.join(base, 'icon.png'), glyph, c0, c1)
        os.makedirs(os.path.join(base, 'images'), exist_ok=True)
        for kind, name in tabs:
            draw_icon(kind, GRAY).save(os.path.join(base, 'images', name + '.png'), optimize=True)
            draw_icon(kind, theme).save(os.path.join(base, 'images', name + '-on.png'), optimize=True)
        print('✓', app)


if __name__ == '__main__':
    main()
