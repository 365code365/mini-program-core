#!/usr/bin/env python3
"""把示例小程序的真机截图剪成一段 60 秒、1920×1080 的展示视频（配乐见 music.py）。

    python3 tools/showcase/video.py <截图目录> <配乐.wav> <输出.mp4> [--fps 30] [--preview 秒]

截图文件名是 `<示例目录名>__<路由>.png`（target/_video/capture.sh 截的）。分镜按 120 BPM 对拍（一小节 2 秒）：
- 0~4s   片头：深色渐变 + 流动光点网格，标题逐字亮起，4s 处随鼓点冲击闪白；
- 4~52s  12 个示例各两小节：三部手机从右侧依次滑入、带倾斜与景深，左侧应用名 / 分类 / 交互要点打字出现，
         背景光晕取这个示例截图的主色，拍点上轻微缩放「呼吸」；
- 52~56s 截图墙：全部截图斜着排成几列上下对流；
- 56~60s 片尾：数字统计逐个滚上来，标题收束，淡出。
只用 PIL + numpy 逐帧画，原始帧经管道交给 ffmpeg 编码（libx264 + AAC）。
"""
import argparse
import glob
import math
import os
import subprocess
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
FONT = os.path.join(ROOT, "assets", "SourceHanSansSC-Regular.otf")
W, H = 1920, 1080
BEAT = 0.5

APPS = [
    ("music", "悦听音乐", "娱乐", ["播放进度跟着定时器走", "拖动进度条，歌词滚到当前行", "吸底迷你播放条"]),
    ("train", "畅行火车票", "出行", ["城市互换 · 日期选择", "车次筛选与排序", "选座、乘车人校验、下单确认"]),
    ("recipe", "厨房小当家", "美食", ["触底分页加载", "swiper 步骤 + 倒计时", "份量联动食材用量"]),
    ("notes", "灵感笔记", "工具", ["实时搜索 · 标签筛选", "防抖自动保存", "长按菜单 · 回收站"]),
    ("kanban", "团队看板", "工具", ["横向三列看板", "picker 改优先级 / 负责人 / 截止日", "子任务进度与统计"]),
    ("stocks", "自选股", "金融", ["每秒跳一次行情", "表头排序 · 涨红跌绿", "交易校验与持仓盈亏"]),
    ("reader", "书香阅读", "娱乐", ["书架 · 书城轮播", "阅读器字号 / 主题 / 进度", "书签与阅读时长"]),
    ("smart-home", "智慧家", "生活服务", ["房间 tab 与 swiper 联动", "空调 / 灯光详情", "一键场景批量控制"]),
    ("game-2048", "2048", "娱乐", ["触摸滑动合并", "撤销一步 · 胜负弹窗", "排行榜持久化"]),
    ("calendar", "日程本", "工具", ["6×7 月视图", "重复规则展开", "时间 / 日期选择校验"]),
    ("parking", "停车宝", "出行", ["自定义车牌键盘", "停车计费与封顶", "预约车位 · 优惠券"]),
    ("marathon", "城市马拉松报名", "生活服务", ["四步报名向导", "身份证校验带出生日", "截止倒计时"]),
]

INTRO, APP_LEN, WALL, OUTRO = 4.0, 4.0, 4.0, 4.0


def font(size):
    return ImageFont.truetype(FONT, size)


def ease_out(t):
    t = min(max(t, 0.0), 1.0)
    return 1 - (1 - t) ** 3


def ease_in_out(t):
    t = min(max(t, 0.0), 1.0)
    return t * t * (3 - 2 * t)


def dominant(img):
    """截图的主色（去掉接近白 / 灰的像素之后的平均），没有就给一个蓝"""
    a = np.asarray(img.convert("RGB").resize((60, 130))).reshape(-1, 3).astype(float)
    sat = a.max(1) - a.min(1)
    a = a[(sat > 60) & (a.max(1) > 80)]
    if len(a) < 30:
        return (60, 120, 255)
    c = a.mean(0)
    return tuple(int(v) for v in c)


def rounded(img, r):
    m = Image.new("L", img.size, 0)
    ImageDraw.Draw(m).rounded_rectangle((0, 0, img.size[0] - 1, img.size[1] - 1), r, fill=255)
    out = img.convert("RGBA")
    out.putalpha(m)
    return out


def phone(shot, h=760):
    """截图套一个手机外框（圆角黑边 + 高光），带透明底"""
    s = shot.convert("RGB")
    sw = int(h * s.width / s.height)
    screen = rounded(s.resize((sw, h), Image.LANCZOS), 34)
    pad = 14
    fw, fh = sw + pad * 2, h + pad * 2
    frame = Image.new("RGBA", (fw, fh), (0, 0, 0, 0))
    d = ImageDraw.Draw(frame)
    d.rounded_rectangle((0, 0, fw - 1, fh - 1), 46, fill=(18, 20, 28, 255), outline=(90, 96, 120, 255), width=2)
    frame.alpha_composite(screen, (pad, pad))
    # 斜高光
    gl = Image.new("RGBA", (fw, fh), (0, 0, 0, 0))
    ImageDraw.Draw(gl).polygon([(0, 0), (fw * 0.55, 0), (fw * 0.15, fh), (0, fh)], fill=(255, 255, 255, 18))
    frame.alpha_composite(Image.composite(gl, Image.new("RGBA", (fw, fh)), frame.split()[3]))
    return frame


def shadowed(img, blur=26, alpha=150, off=(0, 24)):
    a = img.split()[3]
    sh = Image.new("RGBA", (img.width + blur * 4, img.height + blur * 4), (0, 0, 0, 0))
    m = Image.new("L", sh.size, 0)
    m.paste(a.point(lambda v: alpha if v > 0 else 0), (blur * 2 + off[0], blur * 2 + off[1]))
    m = m.filter(ImageFilter.GaussianBlur(blur))
    sh = Image.new("RGBA", sh.size, (0, 0, 0, 255))
    sh.putalpha(m)
    sh.alpha_composite(img, (blur * 2, blur * 2))
    return sh


class Background:
    """深色渐变 + 网格 + 漂浮光点；光晕颜色可变"""

    def __init__(self):
        y = np.linspace(0, 1, H)[:, None]
        x = np.linspace(0, 1, W)[None, :]
        base = np.zeros((H, W, 3))
        base[..., 0] = 10 + 14 * y + 0 * x
        base[..., 1] = 12 + 10 * y
        base[..., 2] = 26 + 30 * y
        self.base = base
        self.xx, self.yy = np.meshgrid(np.arange(W), np.arange(H))
        rng = np.random.default_rng(7)
        self.dots = rng.uniform(0, 1, (90, 4))  # x, y, 速度, 大小

    def frame(self, t, glow, glow_pos=(0.62, 0.5), glow_strength=1.0):
        img = self.base.copy()
        gx, gy = glow_pos[0] * W, glow_pos[1] * H
        d2 = ((self.xx - gx) ** 2 + (self.yy - gy) ** 2) / (520.0 ** 2)
        g = np.exp(-d2)[..., None] * glow_strength
        img += g * np.array(glow) * 0.55
        # 第二团冷色光
        d3 = ((self.xx - W * 0.12) ** 2 + (self.yy - H * 0.9) ** 2) / (600.0 ** 2)
        img += np.exp(-d3)[..., None] * np.array((40, 90, 255)) * 0.25
        out = Image.fromarray(np.clip(img, 0, 255).astype(np.uint8), "RGB")
        d = ImageDraw.Draw(out, "RGBA")
        # 透视网格（往下走）
        off = (t * 60) % 60
        for i in range(0, 22):
            yy = H * 0.62 + (i * 60 + off) ** 1.25 * 0.35
            if yy > H:
                break
            d.line([(0, yy), (W, yy)], fill=(120, 150, 255, 26), width=1)
        for i in range(-12, 13):
            d.line([(W / 2 + i * 40, H * 0.62), (W / 2 + i * 260, H)], fill=(120, 150, 255, 22), width=1)
        # 光点
        for x, y, v, s in self.dots:
            px = (x * W + t * (20 + v * 50)) % W
            py = (y * H - t * (10 + v * 25)) % H
            r = 1 + s * 2.5
            a = int(60 + 120 * (0.5 + 0.5 * math.sin(t * 2 + x * 10)))
            d.ellipse((px - r, py - r, px + r, py + r), fill=(200, 220, 255, a))
        return out


def text_reveal(d, xy, s, f, fill, p):
    """逐字出现：p ∈ [0,1]"""
    n = int(round(len(s) * min(max(p, 0), 1)))
    d.text(xy, s[:n], font=f, fill=fill)


def bold_text(d, xy, s, f, fill, stroke=1):
    d.text(xy, s, font=f, fill=fill, stroke_width=stroke, stroke_fill=fill)


def load_shots(shot_dir):
    groups = {}
    # 按截图的先后（capture.sh 里每个示例先截首页）：第一张放中间
    for p in sorted(glob.glob(os.path.join(shot_dir, "*.png")), key=os.path.getmtime):
        app = os.path.basename(p).split("__")[0]
        groups.setdefault(app, []).append(p)
    return groups


def crop_status(img):
    """截掉系统状态栏与导航条，只留小程序画面"""
    w, h = img.size
    top = int(h * 0.035)
    bottom = int(h * 0.955)
    return img.crop((0, top, w, bottom))


def build(args):
    groups = load_shots(args.shots)
    apps = [a for a in APPS if groups.get(a[0])]
    if not apps:
        sys.exit("截图目录里没有可用的截图")
    bg = Background()
    # 预处理：每张截图做成手机、算主色
    phones, colors, raw = {}, {}, []
    for app, *_ in apps:
        imgs = [crop_status(Image.open(p)) for p in groups[app][:3]]
        raw += imgs
        colors[app] = dominant(imgs[0])
        phones[app] = [shadowed(phone(im)) for im in imgs]
    extra = [crop_status(Image.open(p)) for p in groups.get("chat", [])]
    raw += extra
    thumbs = [rounded(im.convert("RGB").resize((220, int(220 * im.height / im.width)), Image.LANCZOS), 18) for im in raw]
    total = INTRO + APP_LEN * len(apps) + WALL + OUTRO
    if args.preview:
        total = min(total, args.preview)
    fps = args.fps
    nframes = int(total * fps)
    f_title, f_sub, f_small = font(88), font(36), font(28)
    f_app, f_cat, f_feat, f_idx = font(76), font(30), font(36), font(150)
    enc = subprocess.Popen(
        ["ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}", "-r", str(fps), "-i", "-",
         "-i", args.audio, "-t", f"{total:.3f}", "-c:v", "libx264", "-preset", "medium", "-crf", "18", "-pix_fmt", "yuv420p",
         "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart", "-shortest", args.out],
        stdin=subprocess.PIPE,
    )
    for fi in range(nframes):
        t = fi / fps
        beat_phase = (t % BEAT) / BEAT
        pulse = math.exp(-beat_phase * 6)  # 每拍开头的一下
        if t < INTRO:
            frame = intro_frame(bg, t, f_title, f_sub, f_small)
        elif t < INTRO + APP_LEN * len(apps):
            k = int((t - INTRO) // APP_LEN)
            lt = t - INTRO - k * APP_LEN
            frame = app_frame(bg, t, lt, k, apps[k], phones, colors, pulse, (f_app, f_cat, f_feat, f_idx, f_small), len(apps))
        elif t < INTRO + APP_LEN * len(apps) + WALL:
            lt = t - INTRO - APP_LEN * len(apps)
            frame = wall_frame(bg, t, lt, thumbs, f_title)
        else:
            lt = t - INTRO - APP_LEN * len(apps) - WALL
            frame = outro_frame(bg, t, lt, f_title, f_sub, f_small, len(apps))
        enc.stdin.write(frame.convert("RGB").tobytes())
        if fi % (fps * 5) == 0:
            print(f"  {t:5.1f}s / {total:.0f}s", flush=True)
    enc.stdin.close()
    enc.wait()
    print(f"{args.out}：{total:.0f}s {W}x{H}@{fps}")


def intro_frame(bg, t, f_title, f_sub, f_small):
    img = bg.frame(t, (80, 120, 255), (0.5, 0.45), 0.6 + 0.4 * ease_out(t / 3))
    d = ImageDraw.Draw(img, "RGBA")
    title = "Mini Render"
    sub = "Rust 自研的小程序渲染引擎 · 对齐微信 Skyline"
    tag = "12 个全新示例小程序 · 真机运行"
    p = ease_out((t - 0.3) / 1.6)
    tw = d.textlength(title, font=f_title)
    y = H * 0.36 + (1 - p) * 40
    bold_text(d, ((W - tw) / 2, y), title, f_title, (255, 255, 255, int(255 * p)), 2)
    sw = d.textlength(sub, font=f_sub)
    text_reveal(d, ((W - sw) / 2, y + 120), sub, f_sub, (190, 205, 255, 255), (t - 1.2) / 1.4)
    gw = d.textlength(tag, font=f_small)
    if t > 2.6:
        a = int(255 * ease_out((t - 2.6) / 0.6))
        d.rounded_rectangle(((W - gw) / 2 - 22, y + 190, (W + gw) / 2 + 22, y + 238), 24, outline=(120, 160, 255, a), width=2)
        d.text(((W - gw) / 2, y + 196), tag, font=f_small, fill=(220, 230, 255, a))
    # 扫光线
    sx = (t / INTRO) * (W + 600) - 300
    d.polygon([(sx, 0), (sx + 60, 0), (sx - 140, H), (sx - 200, H)], fill=(255, 255, 255, 10))
    # 4s 前最后 0.15 秒闪白（接冲击）
    if t > INTRO - 0.15:
        a = int(255 * (t - (INTRO - 0.15)) / 0.15)
        img = Image.blend(img, Image.new("RGB", (W, H), (255, 255, 255)), min(1.0, max(0.0, (a) / 255)))
    return img


def app_frame(bg, t, lt, k, app, phones, colors, pulse, fonts, n):
    key, name, cat, feats = app
    f_app, f_cat, f_feat, f_idx, f_small = fonts
    col = colors[key]
    img = bg.frame(t, col, (0.66, 0.5), 0.9 + 0.25 * pulse)
    # 刚进来的 0.2 秒：白闪退掉（接上一段的冲击 / 转场）
    if lt < 0.2:
        img = Image.blend(img, Image.new("RGB", (W, H), (255, 255, 255)), 0.6 * (1 - lt / 0.2))
    d = ImageDraw.Draw(img, "RGBA")
    # 大号序号做底纹
    idx = f"{k + 1:02d}"
    d.text((110, 150), idx, font=f_idx, fill=(255, 255, 255, 26))
    # 左侧文字
    p = ease_out(lt / 0.6)
    x0 = 130 - (1 - p) * 80
    d.rounded_rectangle((x0, 360, x0 + d.textlength(cat, font=f_cat) + 40, 408), 22, fill=col + (int(200 * p),))
    d.text((x0 + 20, 364), cat, font=f_cat, fill=(255, 255, 255, int(255 * p)))
    bold_text(d, (x0, 430), name, f_app, (255, 255, 255, int(255 * p)), 2)
    for i, ft in enumerate(feats):
        q = ease_out((lt - 0.5 - i * 0.35) / 0.5)
        if q <= 0:
            continue
        yy = 560 + i * 64
        d.ellipse((x0, yy + 16, x0 + 14, yy + 30), fill=col + (int(255 * q),))
        text_reveal(d, (x0 + 34 - (1 - q) * 30, yy), ft, f_feat, (225, 232, 255, int(255 * q)), q * 1.3)
    # 进度条
    d.rounded_rectangle((130, 930, 130 + 520, 936), 3, fill=(255, 255, 255, 40))
    d.rectangle((130, 930, 130 + max(2, 520 * ((k + lt / APP_LEN) / n)), 936), fill=col + (230,))
    d.text((130, 950), f"{k + 1} / {n}", font=f_small, fill=(200, 210, 240, 200))
    # 右侧三部手机：依次滑入，带倾斜与远近
    ph = phones[key]
    # 第一张（首页）在中间、最前；另两张左右斜着、靠后
    slots = [(1330, 110, 0, 1.0), (1060, 150, -6, 0.86), (1600, 150, 6, 0.86)][: len(ph)]
    order = [i for i in (1, 2, 0) if i < len(ph)]
    breathe = 1 + 0.012 * pulse
    for rank, i in enumerate(order):
        x, y, ang, sc = slots[i]
        q = ease_out((lt - 0.12 * rank) / 0.7)
        out_q = ease_in_out((lt - (APP_LEN - 0.35)) / 0.35)  # 段尾滑出
        im = ph[i]
        s = sc * breathe * (0.85 + 0.15 * q)
        im2 = im.resize((int(im.width * s), int(im.height * s)), Image.BILINEAR)
        if ang:
            im2 = im2.rotate(ang * (0.4 + 0.6 * q), resample=Image.BICUBIC, expand=True)
        float_y = 10 * math.sin(t * 1.6 + i)
        cx = x + (1 - q) * 700 - out_q * 1400
        cy = y + float_y + (1 - q) * 60
        a = q * (1 - out_q)
        if a <= 0.01:
            continue
        if a < 0.99:
            al = im2.split()[3].point(lambda v: int(v * a))
            im2.putalpha(al)
        img.paste(im2, (int(cx - im2.width / 2 + 150), int(cy - 40)), im2)
    return img


def wall_frame(bg, t, lt, thumbs, f_title):
    img = bg.frame(t, (120, 80, 255), (0.5, 0.5), 0.8)
    wall = Image.new("RGBA", (W + 900, H + 1400), (0, 0, 0, 0))
    cols = 9
    tw = 220 + 26
    for c in range(cols):
        direction = 1 if c % 2 else -1
        off = (lt * 160 * direction) % (tw * 2)
        y = -tw * 2 + off
        i = c * 3
        while y < wall.height:
            th = thumbs[i % len(thumbs)]
            wall.alpha_composite(th, (c * tw + 40, int(y)))
            y += th.height + 26
            i += 7
    wall = wall.rotate(-14, resample=Image.BICUBIC, expand=False)
    p = ease_out(lt / 0.5) * (1 - ease_in_out((lt - 3.5) / 0.5))
    if p < 1:
        wall.putalpha(wall.split()[3].point(lambda v: int(v * p)))
    img.paste(wall, (-450, -700), wall)
    d = ImageDraw.Draw(img, "RGBA")
    # 中间压一条深色带 + 标题
    d.rectangle((0, H / 2 - 90, W, H / 2 + 90), fill=(8, 10, 22, int(200 * p)))
    s = "每一页都在真机上跑过"
    sw = d.textlength(s, font=f_title)
    bold_text(d, ((W - sw) / 2, H / 2 - 60), s, f_title, (255, 255, 255, int(255 * p)), 2)
    return img


def outro_frame(bg, t, lt, f_title, f_sub, f_small, n_apps):
    img = bg.frame(t, (80, 120, 255), (0.5, 0.45), 1.0)
    if lt < 0.2:
        img = Image.blend(img, Image.new("RGB", (W, H), (255, 255, 255)), 0.8 * (1 - lt / 0.2))
    d = ImageDraw.Draw(img, "RGBA")
    stats = [(str(n_apps), "个新示例"), ("66", "个页面"), ("125", "个场景用例"), ("13", "个引擎问题已修")]
    for i, (num, label) in enumerate(stats):
        q = ease_out((lt - 0.2 - i * 0.25) / 0.6)
        if q <= 0:
            continue
        x = 260 + i * 370
        y = 330 + (1 - q) * 50
        shown = num
        if num.isdigit():
            shown = str(int(round(int(num) * min(1.0, q * 1.1))))
        bold_text(d, (x, y), shown, font(110), (255, 255, 255, int(255 * q)), 2)
        d.text((x + 4, y + 140), label, font=f_sub, fill=(190, 205, 255, int(255 * q)))
    q = ease_out((lt - 1.4) / 0.8)
    s = "Mini Render · 一份源码，原生窗体 / Android / Flutter 同一个画面"
    sw = d.textlength(s, font=f_sub)
    d.text(((W - sw) / 2, 700 + (1 - q) * 20), s, font=f_sub, fill=(230, 236, 255, int(255 * q)))
    fade = ease_in_out((lt - 3.0) / 1.0)
    if fade > 0:
        img = Image.blend(img, Image.new("RGB", (W, H), (0, 0, 0)), min(1.0, max(0.0, (int(255 * fade) / 255))))
    return img


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("shots")
    ap.add_argument("audio")
    ap.add_argument("out")
    ap.add_argument("--fps", type=int, default=30)
    ap.add_argument("--preview", type=float, default=0)
    build(ap.parse_args())


if __name__ == "__main__":
    main()
