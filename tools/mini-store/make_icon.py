#!/usr/bin/env python3
"""给小程序画一个商店图标（icon.png，144×144）：圆角方块 + 两色渐变 + 白色字。

    python3 tools/mini-store/make_icon.py <小程序目录> <字> <起始色> <结束色>
    python3 tools/mini-store/make_icon.py sample/news-app 头 '#ff5a4e' '#d43c33'

字体用仓库里的 assets/SourceHanSansSC-Regular.otf（加粗靠描边）。已有 icon.png 会被覆盖。
"""
import os
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
FONT = os.path.join(ROOT, 'assets', 'SourceHanSansSC-Regular.otf')
S = 144


def hex_rgb(h):
    h = h.lstrip('#')
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def make(out, glyph, c0, c1):
    a, b = hex_rgb(c0), hex_rgb(c1)
    big = S * 4  # 4 倍画再缩小：圆角与字边缘平滑
    grad = Image.new('RGB', (big, big))
    px = grad.load()
    for y in range(big):
        for x in range(big):
            t = (x + y) / (2 * big - 2)
            px[x, y] = tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3))
    mask = Image.new('L', (big, big), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, big - 1, big - 1), radius=32 * 4, fill=255)
    img = Image.new('RGBA', (big, big), (0, 0, 0, 0))
    img.paste(grad, (0, 0), mask)
    d = ImageDraw.Draw(img)
    n = len(glyph)
    size = int(big * (0.56 if n == 1 else 0.40 if n == 2 else 0.30))
    font = ImageFont.truetype(FONT, size)
    box = d.textbbox((0, 0), glyph, font=font, stroke_width=size // 28)
    w, h = box[2] - box[0], box[3] - box[1]
    d.text(((big - w) / 2 - box[0], (big - h) / 2 - box[1]), glyph, font=font, fill='white',
           stroke_width=size // 28, stroke_fill='white')
    img.resize((S, S), Image.LANCZOS).save(out, optimize=True)


if __name__ == '__main__':
    if len(sys.argv) != 5:
        sys.exit(__doc__)
    make(os.path.join(sys.argv[1], 'icon.png'), sys.argv[2], sys.argv[3], sys.argv[4])
    print('✓', os.path.join(sys.argv[1], 'icon.png'))
