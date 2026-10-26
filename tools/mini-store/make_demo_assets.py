#!/usr/bin/env python3
"""生成演示接口用的图片与音频（本机跑一次，产物随 deploy.sh 一起上传；服务器上不装 Pillow）。

    python3 tools/mini-store/make_demo_assets.py          # 输出到 tools/mini-store/demo_assets/

图片的标题、配色、尺寸都从 demo_movies / demo_music / demo_photos 里读 —— 接口返回的
width/height 与这里画出来的 JPEG 必须一致（瀑布流按它排版），所以只有一份来源。
画法只用渐变 + 几何图形 + 大字（不用照片素材：没有版权问题，压缩后也小），JPEG quality 80。
同样的输入画出同样的图（随机源都带固定种子），重跑不会让 ETag 无谓地变。
"""
import array
import math
import os
import shutil
import sys
import wave

from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import demo_common as C  # noqa: E402
import demo_movies  # noqa: E402
import demo_music  # noqa: E402
import demo_photos  # noqa: E402

OUT = os.path.join(HERE, 'demo_assets')
FONT = os.path.join(os.path.dirname(os.path.dirname(HERE)), 'assets', 'SourceHanSansSC-Regular.otf')
Q = 80


def rgb(h):
    h = h.lstrip('#')
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def font(size):
    return ImageFont.truetype(FONT, size)


def gradient(w, h, c0, c1, angle=90):
    """两色线性渐变：angle=90 从上到下，0 从左到右，45 斜向"""
    # 不用 rotate：旋转后四角会露出黑底。改成「横向 + 纵向」两张渐变按角度混合
    v = Image.linear_gradient('L').resize((w, h))
    hz = Image.linear_gradient('L').rotate(90).resize((w, h))
    if angle > 90:
        hz = hz.transpose(Image.FLIP_LEFT_RIGHT)
    a = math.radians(angle if angle <= 90 else 180 - angle)
    g = Image.blend(hz, v, math.sin(a) ** 2)
    return Image.composite(Image.new('RGB', (w, h), rgb(c1)), Image.new('RGB', (w, h), rgb(c0)), g)


def overlay(img, draw_fn, blur=0):
    """在半透明图层上画（Pillow 的 ImageDraw 直接画 RGB 没有 alpha 混合）"""
    layer = Image.new('RGBA', img.size, (0, 0, 0, 0))
    draw_fn(ImageDraw.Draw(layer))
    if blur:
        layer = layer.filter(ImageFilter.GaussianBlur(blur))
    return Image.alpha_composite(img.convert('RGBA'), layer).convert('RGB')


def text(d, xy, s, size, fill=(255, 255, 255), bold=True, anchor='la', spacing=0):
    f = font(size)
    if spacing:
        x, y = xy
        for ch in s:
            d.text((x, y), ch, font=f, fill=fill, anchor=anchor, stroke_width=1 if bold else 0, stroke_fill=fill)
            x += d.textlength(ch, font=f) + spacing
        return
    d.text(xy, s, font=f, fill=fill, anchor=anchor, stroke_width=max(1, size // 30) if bold else 0, stroke_fill=fill)


def fit_size(d, s, max_w, start):
    size = start
    while size > 14 and d.textlength(s, font=font(size)) > max_w:
        size -= 2
    return size


def save(img, kind, name):
    os.makedirs(os.path.join(OUT, kind), exist_ok=True)
    img.save(os.path.join(OUT, kind, name + '.jpg'), 'JPEG', quality=Q, optimize=True, progressive=True)


# ─────────────── 电影：海报 / 剧照 ───────────────

def poster(m):
    w, h = 360, 540
    r = C.rng('poster', m['id'])
    c0, c1 = m['colors']
    img = gradient(w, h, c0, c1, 90)

    def deco(d):
        # 大光斑 + 斜向光束，像海报上的光效
        for _ in range(4):
            x, y, rad = r.randint(-60, w), r.randint(0, h - 160), r.randint(60, 180)
            d.ellipse((x - rad, y - rad, x + rad, y + rad), fill=(255, 255, 255, r.randint(18, 46)))
        x0 = r.randint(40, 300)
        d.polygon([(x0, 0), (x0 + 60, 0), (x0 - 160, h), (x0 - 260, h)], fill=(255, 255, 255, 26))
    img = overlay(img, deco, blur=6)

    def scene(d):
        # 近景剪影：起伏的地平线 + 一个人影
        pts = [(0, h)]
        for i in range(9):
            pts.append((i * w / 8, h - 170 + r.randint(-36, 30)))
        pts.append((w, h))
        d.polygon(pts, fill=(0, 0, 0, 120))
        px = r.randint(110, 250)
        d.ellipse((px - 11, h - 262, px + 11, h - 240), fill=(0, 0, 0, 170))
        d.polygon([(px - 18, h - 236), (px + 18, h - 236), (px + 26, h - 160), (px - 26, h - 160)], fill=(0, 0, 0, 170))
        d.rectangle((0, h - 150, w, h), fill=(0, 0, 0, 90))
    img = overlay(img, scene)
    d = ImageDraw.Draw(img)
    title = m['title']
    size = fit_size(d, title, w - 48, 58)
    text(d, (24, h - 142), title, size)
    text(d, (26, h - 62), m['enTitle'].upper(), 15, fill=(255, 255, 255), bold=False, spacing=1)
    text(d, (26, h - 36), f"{m['genreText']} · {m['releaseDate'][:4]}", 15, fill=(230, 230, 230), bold=False)
    text(d, (24, 22), '光影放映厅', 15, fill=(255, 255, 255), bold=False)
    if m['score']:
        d.rounded_rectangle((w - 92, 18, w - 18, 50), 8, fill=(255, 180, 0))
        text(d, (w - 55, 34), f"{m['score']:.1f}", 20, fill=(40, 30, 0), anchor='mm')
    save(img, 'poster', f"m{m['id']:02d}")


def still(i):
    w, h = 640, 360
    r = C.rng('still', i)
    film = demo_movies.FILMS[(i * 7) % len(demo_movies.FILMS)]
    c0, c1 = film[4]
    img = gradient(w, h, c0, c1, r.choice([60, 90, 120]))

    def scene(d):
        sx, sy, sr = r.randint(80, 560), r.randint(60, 160), r.randint(28, 60)
        d.ellipse((sx - sr * 2, sy - sr * 2, sx + sr * 2, sy + sr * 2), fill=(255, 240, 200, 40))
        d.ellipse((sx - sr, sy - sr, sx + sr, sy + sr), fill=(255, 236, 190, 200))
        for layer, alpha in ((0, 90), (1, 150), (2, 210)):
            base = 230 + layer * 40
            pts = [(0, h)] + [(x, base - r.randint(0, 70 - layer * 15)) for x in range(0, w + 80, 80)] + [(w, h)]
            d.polygon(pts, fill=(10, 12, 20, alpha))
        d.rectangle((0, 0, w, 26), fill=(0, 0, 0, 255))
        d.rectangle((0, h - 26, w, h), fill=(0, 0, 0, 255))
    img = overlay(img, scene)
    d = ImageDraw.Draw(img)
    text(d, (20, h - 52), f'《{film[0]}》剧照 {i:02d}', 16, fill=(240, 240, 240), bold=False)
    save(img, 'still', f's{i:02d}')


# ─────────────── 音乐：封面 ───────────────

def music_cover(i, label, c0, c1):
    s = 300
    r = C.rng('cover', i)
    img = gradient(s, s, c0, c1, 45)

    def deco(d):
        style = i % 3
        if style == 0:      # 黑胶
            d.ellipse((60, 60, 300 + 40, 300 + 40), fill=(20, 20, 24, 230))
            for k in range(6):
                rr = 40 + k * 18
                d.ellipse((200 - rr, 200 - rr, 200 + rr, 200 + rr), outline=(255, 255, 255, 30), width=2)
            d.ellipse((176, 176, 224, 224), fill=(*rgb(c0), 255))
        elif style == 1:    # 几何块
            for _ in range(5):
                x, y, a = r.randint(-40, 260), r.randint(-40, 200), r.randint(60, 150)
                d.rectangle((x, y, x + a, y + a), fill=(255, 255, 255, r.randint(30, 70)))
        else:               # 波形
            for k in range(28):
                bh = abs(math.sin(k * 0.55 + i)) * 90 + 10
                d.rounded_rectangle((20 + k * 9.5, 190 - bh, 26 + k * 9.5, 190 + bh * 0.4), 3, fill=(255, 255, 255, 150))
    img = overlay(img, deco)
    d = ImageDraw.Draw(img)
    size = fit_size(d, label, s - 40, 44)
    text(d, (22, 30), label, size)
    text(d, (24, 30 + size + 10), 'CLOUD MUSIC', 13, bold=False, spacing=2)
    save(img, 'cover', f'c{i + 1:02d}')


def avatar(i, ch, color):
    s = 96
    img = gradient(s, s, color, '#ffffff', 45)
    img = Image.blend(Image.new('RGB', (s, s), rgb(color)), img, 0.25)
    d = ImageDraw.Draw(img)
    text(d, (s / 2, s / 2 + 2), ch, 44, anchor='mm')
    save(img, 'avatar', f'a{i + 1:02d}')


# ─────────────── 图库照片：按主标签画不同的画面 ───────────────

def photo(name, w, h, tag, title, colors):
    r = C.rng('photo-img', name)
    c0, c1 = colors
    img = gradient(w, h, c0, c1, 90 if tag not in ('极简', '人像') else 135)

    def draw(d):
        if tag in ('风光', '旅行', '植物'):
            sx, sy = r.randint(100, 500), r.randint(60, h // 3)
            d.ellipse((sx - 46, sy - 46, sx + 46, sy + 46), fill=(255, 245, 220, 210))
            for k, a in enumerate((80, 140, 210)):
                base = h * (0.55 + k * 0.12)
                pts = [(0, h)] + [(x, base - r.randint(0, int(h * 0.18))) for x in range(0, w + 100, 100)] + [(w, h)]
                d.polygon(pts, fill=(20, 40, 50, a) if tag != '植物' else (20, 80, 40, a))
            if tag == '旅行':
                d.polygon([(w / 2 - 12, h * 0.62), (w / 2 + 12, h * 0.62), (w / 2 + 170, h), (w / 2 - 170, h)], fill=(60, 60, 70, 230))
                for k in range(6):
                    y = h * 0.66 + k * k * 9
                    d.rectangle((w / 2 - 2 - k, y, w / 2 + 2 + k, y + 6 + k * 2), fill=(255, 255, 255, 220))
            if tag == '植物':
                for _ in range(14):
                    x, y = r.randint(0, w), r.randint(int(h * 0.3), h)
                    d.ellipse((x - 26, y - 10, x + 26, y + 10), fill=(120, 200, 110, 150))
        elif tag in ('城市', '夜景', '建筑'):
            night = tag == '夜景'
            if night:
                for _ in range(60):
                    x, y = r.randint(0, w), r.randint(0, h // 2)
                    d.ellipse((x, y, x + 2, y + 2), fill=(255, 255, 255, r.randint(120, 255)))
                d.ellipse((440, 50, 510, 120), fill=(255, 250, 220, 230))
            x = 0
            while x < w:
                bw, bh = r.randint(50, 110), r.randint(int(h * 0.25), int(h * 0.7))
                shade = r.randint(20, 60) if night else r.randint(50, 110)
                d.rectangle((x, h - bh, x + bw - 4, h), fill=(shade, shade, shade + 20, 235))
                for wy in range(h - bh + 12, h - 10, 18):
                    for wx in range(x + 8, x + bw - 12, 14):
                        if r.random() < (0.45 if night else 0.25):
                            col = (255, 214, 120, 230) if night else (220, 235, 255, 160)
                            d.rectangle((wx, wy, wx + 6, wy + 9), fill=col)
                x += bw
            if tag == '建筑':
                for k in range(3):
                    cx = 120 + k * 180
                    d.pieslice((cx - 70, h * 0.25, cx + 70, h * 0.25 + 140), 180, 360, fill=(255, 255, 255, 60))
        elif tag == '人像':
            cx, cy = w / 2 + r.randint(-60, 60), h * 0.42
            d.ellipse((cx - 70, cy - 90, cx + 70, cy + 60), fill=(60, 40, 50, 200))
            d.ellipse((cx - 160, cy + 70, cx + 160, cy + 420), fill=(60, 40, 50, 200))
            d.ellipse((cx - 200, 40, cx + 200, 440), outline=(255, 255, 255, 70), width=6)
        elif tag == '美食':
            cx, cy = w / 2, h / 2
            d.ellipse((cx - 210, cy - 210, cx + 210, cy + 210), fill=(255, 255, 255, 230))
            d.ellipse((cx - 170, cy - 170, cx + 170, cy + 170), fill=(250, 245, 235, 255))
            for _ in range(9):
                a, rr = r.uniform(0, math.tau), r.uniform(20, 110)
                x, y, s = cx + math.cos(a) * rr, cy + math.sin(a) * rr, r.randint(22, 48)
                d.ellipse((x - s, y - s, x + s, y + s), fill=r.choice([(231, 76, 60, 230), (241, 196, 15, 230), (46, 204, 113, 230), (230, 126, 34, 230)]))
        elif tag == '动物':
            cx, cy = w / 2, h * 0.55
            d.ellipse((cx - 150, cy - 120, cx + 150, cy + 140), fill=(245, 166, 35, 240))
            d.polygon([(cx - 140, cy - 60), (cx - 110, cy - 190), (cx - 40, cy - 110)], fill=(245, 166, 35, 240))
            d.polygon([(cx + 140, cy - 60), (cx + 110, cy - 190), (cx + 40, cy - 110)], fill=(245, 166, 35, 240))
            for ex in (-55, 55):
                d.ellipse((cx + ex - 16, cy - 30, cx + ex + 16, cy + 6), fill=(40, 30, 20, 255))
            d.polygon([(cx - 14, cy + 30), (cx + 14, cy + 30), (cx, cy + 46)], fill=(230, 100, 120, 255))
        else:  # 极简
            cx, cy, rr = r.randint(150, 450), r.randint(int(h * 0.3), int(h * 0.6)), r.randint(60, 130)
            d.ellipse((cx - rr, cy - rr, cx + rr, cy + rr), fill=(255, 120, 90, 220))
            d.line((0, h * 0.75, w, h * 0.75), fill=(60, 70, 90, 200), width=3)
    img = overlay(img, draw)
    d = ImageDraw.Draw(img)
    img = overlay(img, lambda dd: dd.rectangle((0, h - 70, w, h), fill=(0, 0, 0, 70)))
    d = ImageDraw.Draw(img)
    text(d, (22, h - 36), title, 26, anchor='lm')
    text(d, (w - 22, h - 36), f'#{tag}', 18, fill=(235, 235, 235), bold=False, anchor='rm')
    save(img, 'photo', name)


def banner(b):
    w, h = 750, 300
    r = C.rng('banner', b['id'])
    c0, c1 = b['colors']
    img = gradient(w, h, c0, c1, 0)

    def deco(d):
        for _ in range(6):
            x, y, rr = r.randint(300, 760), r.randint(-40, 320), r.randint(40, 150)
            d.ellipse((x - rr, y - rr, x + rr, y + rr), fill=(255, 255, 255, r.randint(25, 60)))
    img = overlay(img, deco, blur=3)
    d = ImageDraw.Draw(img)
    text(d, (48, 92), b['title'], fit_size(d, b['title'], 560, 64))
    text(d, (50, 186), b['subtitle'], 26, fill=(255, 255, 255), bold=False)
    d.rounded_rectangle((50, 228, 186, 268), 20, outline=(255, 255, 255), width=2)
    text(d, (118, 248), '立即查看', 20, anchor='mm')
    save(img, 'banner', b['image'].rsplit('/', 1)[1][:-4])


# ─────────────── 音频：简单旋律 ───────────────

def melody(n):
    """20 秒的小旋律：五声音阶主旋律 + 根音低音，加谐波与包络，听起来不像测试音"""
    rate, secs = 16000, C.AUDIO_SECONDS
    r = C.rng('melody', n)
    root = [261.63, 293.66, 220.0][(n - 1) % 3]
    scale = [0, 2, 4, 7, 9, 12, 14, 16]
    bpm = [96, 112, 84][(n - 1) % 3]
    beat = 60 / bpm
    notes = []
    t = 0.0
    while t < secs:
        dur = beat * r.choice([0.5, 0.5, 1, 1, 1.5, 2])
        notes.append((t, dur, root * 2 ** (r.choice(scale) / 12)))
        t += dur
    buf = array.array('h', [0]) * (rate * secs)
    for start, dur, f in notes:
        s0, s1 = int(start * rate), min(len(buf), int((start + dur) * rate))
        for i in range(s0, s1):
            x = (i - s0) / rate
            env = min(1, x / 0.02) * math.exp(-x * 2.6)
            v = math.sin(math.tau * f * x) + 0.35 * math.sin(math.tau * 2 * f * x) + 0.12 * math.sin(math.tau * 3 * f * x)
            buf[i] = int(buf[i] + 7000 * env * v)
    bass_len = int(beat * 2 * rate)
    for b0 in range(0, len(buf), bass_len):
        f = root / 2 * 2 ** (r.choice([0, 5, 7, 0]) / 12)
        for i in range(b0, min(len(buf), b0 + bass_len)):
            x = (i - b0) / rate
            v = buf[i] + int(5000 * math.exp(-x * 1.5) * math.sin(math.tau * f * x))
            buf[i] = max(-32767, min(32767, v))
    fade = rate
    for i in range(fade):          # 尾部 1 秒淡出，循环播放时接缝不爆音
        buf[-1 - i] = int(buf[-1 - i] * i / fade)
    os.makedirs(os.path.join(OUT, 'audio'), exist_ok=True)
    with wave.open(os.path.join(OUT, 'audio', f'{n}.wav'), 'wb') as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(buf.tobytes())


def main():
    if os.path.isdir(OUT):
        shutil.rmtree(OUT)
    for m in demo_movies.MOVIES:
        poster(m)
    for i in range(1, C.N_STILLS + 1):
        still(i)
    for i, (label, c0, c1) in enumerate(demo_music.cover_specs()[:C.N_COVERS]):
        music_cover(i, label, c0, c1)
    for i, (ch, color) in enumerate(C.AVATAR_SPECS[:C.N_AVATARS]):
        avatar(i, ch, color)
    for spec in demo_photos.photo_specs():
        photo(*spec)
    for b in demo_movies.banners() + demo_music.banners() + demo_photos.banners():
        banner(b)
    for n in range(1, C.N_AUDIO + 1):
        melody(n)
    total = 0
    for dirpath, _, files in os.walk(OUT):
        sub = sum(os.path.getsize(os.path.join(dirpath, f)) for f in files)
        total += sub
        if files:
            print(f'{os.path.relpath(dirpath, OUT):8s} {len(files):3d} 个  {sub / 1024:7.0f} KB')
    print(f'合计 {total / 1048576:.2f} MB → {OUT}')


if __name__ == '__main__':
    main()
