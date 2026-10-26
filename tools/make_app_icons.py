"""两个演示 App 的启动图标（Pillow 画，矢量感的几何图形，不依赖设计稿）。

    python3 tools/make_app_icons.py

- Flutter 示例「灵犀」：靛蓝 → 青的渐变底，白色的「双环」（灵犀一点通：两个小程序连在一起）+ 一颗星芒；
- 原生示例「灵犀原生」：同一个标志，微信绿 → 青蓝的底（一眼分得出是哪一个）。

产物：
- Android：mipmap-*/ic_launcher.png（老系统用，圆角方形）、ic_launcher_round.png，
  mipmap-anydpi-v26/ic_launcher.xml（自适应图标：前景 / 背景两层，前景只画在中间 66/108 的安全区里，
  各家桌面裁成圆 / 方 / 水滴都不会切到标志）；
- iOS（只有 Flutter 示例有 iOS 工程）：AppIcon.appiconset 里的 19 张（1024 那张不带透明通道，App Store 要求）。
先在 1024 上画、再缩（LANCZOS），小尺寸不糊。
"""
import math
import os

from PIL import Image, ImageDraw, ImageFilter

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
S = 1024

THEMES = {
    'flutter': ((0x4F, 0x46, 0xE5), (0x06, 0xB6, 0xD4)),
    'native': ((0x07, 0xC1, 0x60), (0x10, 0x8E, 0xE9)),
}


def gradient(c0, c1, size=S):
    """左上 → 右下的斜向渐变"""
    img = Image.new('RGB', (size, size))
    px = img.load()
    for y in range(size):
        for x in range(size):
            t = (x + y) / (2 * (size - 1))
            px[x, y] = tuple(int(c0[i] + (c1[i] - c0[i]) * t) for i in range(3))
    return img


def emblem(size=S, scale=1.0):
    """白色标志（RGBA，透明底）：两个叠在一起的圆环 + 右上一颗四角星芒。`scale`：相对整张图的大小"""
    big = size * 2  # 两倍尺寸画、再缩：圆环边缘平滑
    layer = Image.new('RGBA', (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    c = big / 2
    r = big * 0.19 * scale
    w = int(big * 0.062 * scale)
    gap = r * 0.62
    left = (c - gap - r, c - r, c - gap + r, c + r)
    right = (c + gap - r, c - r, c + gap + r, c + r)
    d.ellipse(left, outline=(255, 255, 255, 255), width=w)
    d.ellipse(right, outline=(255, 255, 255, 255), width=w)
    # 两环直接叠在一起：试过在交点擦出缝做「相扣」，缩到 48px 以下只剩一块看不清的豁口，不如干净的双环
    # 星芒：四角星
    sx, sy, sr = c + gap + r * 0.95, c - r * 1.05, r * 0.42
    pts = []
    for k in range(8):
        a = math.pi / 4 * k - math.pi / 2
        rr = sr if k % 2 == 0 else sr * 0.28
        pts.append((sx + rr * math.cos(a), sy + rr * math.sin(a)))
    d.polygon(pts, fill=(255, 255, 255, 255))
    return layer.resize((size, size), Image.LANCZOS)


def shadow(layer, blur=18, alpha=70):
    """标志下面一层淡淡的投影（让白色标志在亮色渐变上也立得住）"""
    a = layer.split()[3].point(lambda v: v * alpha // 255)
    sh = Image.new('RGBA', layer.size, (0, 0, 40, 0))
    sh.putalpha(a)
    sh = sh.filter(ImageFilter.GaussianBlur(blur))
    out = Image.new('RGBA', layer.size, (0, 0, 0, 0))
    out.alpha_composite(sh, (0, int(blur * 0.6)))
    out.alpha_composite(layer)
    return out


def rounded_mask(size, radius):
    m = Image.new('L', (size * 2, size * 2), 0)
    ImageDraw.Draw(m).rounded_rectangle((0, 0, size * 2 - 1, size * 2 - 1), radius=radius * 2, fill=255)
    return m.resize((size, size), Image.LANCZOS)


def full_icon(theme, shape='square'):
    """完整图标（1024，RGBA）：渐变底 + 标志；shape = square（iOS / 老安卓圆角方形）/ round / flat（iOS 1024，不透明）"""
    bg = gradient(*THEMES[theme]).convert('RGBA')
    # 左上一点高光：不是一块死板的渐变
    glow = Image.new('RGBA', (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(glow).ellipse((S * 0.02, S * 0.0, S * 0.62, S * 0.5), fill=(255, 255, 255, 40))
    bg.alpha_composite(glow.filter(ImageFilter.GaussianBlur(120)))
    bg.alpha_composite(shadow(emblem(S, 1.0)))
    if shape == 'flat':
        return bg.convert('RGB')
    if shape == 'round':
        m = Image.new('L', (S, S), 0)
        ImageDraw.Draw(m).ellipse((0, 0, S - 1, S - 1), fill=255)
    else:
        m = rounded_mask(S, int(S * 0.225))
    out = Image.new('RGBA', (S, S), (0, 0, 0, 0))
    out.paste(bg, (0, 0), m)
    return out


ANDROID_DPI = {'mdpi': 48, 'hdpi': 72, 'xhdpi': 96, 'xxhdpi': 144, 'xxxhdpi': 192}


def android(theme, res):
    sq, rd = full_icon(theme), full_icon(theme, 'round')
    # 自适应图标的两层：108dp 画布，标志缩到 66dp 的安全区里（66/108 ≈ 0.61）
    bg = gradient(*THEMES[theme]).convert('RGBA')
    fg = shadow(emblem(S, 0.61 * 1.25))
    for dpi, px in ANDROID_DPI.items():
        d = os.path.join(res, f'mipmap-{dpi}')
        os.makedirs(d, exist_ok=True)
        sq.resize((px, px), Image.LANCZOS).save(os.path.join(d, 'ic_launcher.png'))
        rd.resize((px, px), Image.LANCZOS).save(os.path.join(d, 'ic_launcher_round.png'))
        layer = int(px * 108 / 48)
        bg.resize((layer, layer), Image.LANCZOS).save(os.path.join(d, 'ic_launcher_background.png'))
        fg.resize((layer, layer), Image.LANCZOS).save(os.path.join(d, 'ic_launcher_foreground.png'))
    d = os.path.join(res, 'mipmap-anydpi-v26')
    os.makedirs(d, exist_ok=True)
    xml = ('<?xml version="1.0" encoding="utf-8"?>\n'
           '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n'
           '    <background android:drawable="@mipmap/ic_launcher_background" />\n'
           '    <foreground android:drawable="@mipmap/ic_launcher_foreground" />\n'
           '    <monochrome android:drawable="@mipmap/ic_launcher_foreground" />\n'
           '</adaptive-icon>\n')
    for name in ('ic_launcher.xml', 'ic_launcher_round.xml'):
        with open(os.path.join(d, name), 'w') as f:
            f.write(xml)


def ios(theme, appiconset):
    import json
    flat = full_icon(theme, 'flat')
    with open(os.path.join(appiconset, 'Contents.json')) as f:
        spec = json.load(f)
    for im in spec['images']:
        px = round(float(im['size'].split('x')[0]) * int(im['scale'][0]))
        flat.resize((px, px), Image.LANCZOS).save(os.path.join(appiconset, im['filename']))


def main():
    android('flutter', os.path.join(ROOT, 'sdk/flutter/example/android/app/src/main/res'))
    android('native', os.path.join(ROOT, 'sdk/android/example/app/src/main/res'))
    ios('flutter', os.path.join(ROOT, 'sdk/flutter/example/ios/Runner/Assets.xcassets/AppIcon.appiconset'))
    # 官网 / 说明里用的大图
    out = os.path.join(ROOT, 'target', 'app-icons')
    os.makedirs(out, exist_ok=True)
    full_icon('flutter').save(os.path.join(out, 'lingxi-flutter.png'))
    full_icon('native').save(os.path.join(out, 'lingxi-native.png'))
    print('ok')


if __name__ == '__main__':
    main()
