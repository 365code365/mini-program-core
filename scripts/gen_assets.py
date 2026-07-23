#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""生成画廊所需的真实图片素材：
- 用豆包文生图 API 生成商品图/专辑封面/banner/头像
- 用 PIL 绘制播放器控制图标等矢量图标
产物输出到 doc/gallery/assets/。已存在的文件跳过（避免重复消耗 API）。
"""
import json, os, sys, urllib.request, ssl

API = "https://ark.cn-beijing.volces.com/api/v3/images/generations"
KEY = "269caa99-215f-43fc-bcee-73f14a764069"
MODEL = "doubao-seedream-5-0-pro-260628"
OUT = "doc/gallery/assets"
os.makedirs(OUT, exist_ok=True)
os.makedirs(OUT + "/icons", exist_ok=True)

_ctx = ssl.create_default_context()
_ctx.check_hostname = False
_ctx.verify_mode = ssl.CERT_NONE


def gen(name, prompt, size="1K"):
    path = f"{OUT}/{name}.jpg"
    if os.path.exists(path):
        print("  skip", name)
        return
    body = json.dumps({
        "model": MODEL, "prompt": prompt, "response_format": "url",
        "size": size, "stream": False, "watermark": False,
    }).encode()
    req = urllib.request.Request(API, data=body, headers={
        "Content-Type": "application/json", "Authorization": "Bearer " + KEY,
    })
    try:
        r = urllib.request.urlopen(req, timeout=180, context=_ctx)
        data = json.load(r)
        url = data["data"][0]["url"]
        img = urllib.request.urlopen(url, timeout=180, context=_ctx).read()
        with open(path, "wb") as f:
            f.write(img)
        print("  ok  ", name, len(img), "bytes")
    except Exception as e:
        print("  FAIL", name, repr(e))


# ============ AI 生成图片 ============
IMAGES = [
    ("album", "album cover art, dreamy starry night sky with a bright shining star, purple and blue gradient, minimal, artistic, square"),
    ("banner", "e-commerce promotion banner background, vibrant orange to pink gradient, abstract geometric shapes, no text, clean modern"),
    ("p_earbuds", "white wireless bluetooth earbuds in charging case, product photo, pure white background, studio lighting, e-commerce"),
    ("p_watch", "modern smart watch with black sport band, product photo, pure white background, studio lighting, e-commerce"),
    ("p_sneakers", "pair of white and blue running sneakers, product photo, pure white background, studio lighting, e-commerce"),
    ("p_backpack", "stylish grey casual backpack, product photo, pure white background, studio lighting, e-commerce"),
    ("p_thermos", "stainless steel insulated water bottle thermos, product photo, pure white background, studio lighting"),
    ("p_lipstick", "luxury red lipstick cosmetic, product photo, pure white background, studio lighting, e-commerce"),
    ("p_coffee", "cup of latte coffee with latte art, top view, product photo, clean background"),
    ("p_cake", "delicious strawberry cream cake slice on a plate, product photo, clean background"),
    ("avatar1", "portrait of a smiling young asian woman, headshot, soft studio light, clean background, professional"),
    ("avatar2", "portrait of a young asian man, headshot, soft studio light, clean background, professional"),
    ("p_phone", "modern flagship smartphone with full screen display, black color, front and back view, product photo, pure white background, studio lighting, e-commerce"),
    ("p_phone2", "modern flagship smartphone back view showing triple camera, black glass, product photo, pure white background, studio lighting"),
    ("logo", "minimal modern app logo icon, green rounded square with a white leaf or spark symbol, flat design, clean, centered, app store icon style"),
    # 天气城市背景（宽幅横图，顶部留白便于叠加文字）
    ("w_sunny", "sunny clear blue sky over a modern city skyline, bright daylight, a few soft white clouds, vibrant, cinematic wide shot"),
    ("w_cloudy", "overcast grey cloudy sky over a city skyline, soft diffused light, moody atmosphere, cinematic wide shot"),
    ("w_rain", "rainy day over a city skyline, dark storm clouds, wet reflective streets, cinematic wide shot, moody blue tones"),
    ("w_night", "city skyline at night with glowing warm lights and skyscrapers, deep blue starry sky, cinematic wide shot"),
]

print("生成 AI 图片 ...")
for name, prompt in IMAGES:
    gen(name, prompt)


# ============ PIL 绘制图标 ============
try:
    from PIL import Image, ImageDraw
except Exception as e:
    print("PIL 不可用，跳过图标绘制:", e)
    sys.exit(0)

S = 96  # 图标画布尺寸


def new_icon():
    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    return im, ImageDraw.Draw(im)


def save(im, name):
    im.save(f"{OUT}/icons/{name}.png")
    print("  icon", name)


def i_play(color):
    im, d = new_icon()
    d.polygon([(30, 22), (30, 74), (76, 48)], fill=color)
    save(im, "play")


def i_pause(color):
    im, d = new_icon()
    d.rounded_rectangle([28, 24, 42, 72], radius=4, fill=color)
    d.rounded_rectangle([54, 24, 68, 72], radius=4, fill=color)
    save(im, "pause")


def i_prev(color):
    im, d = new_icon()
    d.rounded_rectangle([24, 26, 32, 70], radius=3, fill=color)
    d.polygon([(72, 26), (72, 70), (38, 48)], fill=color)
    save(im, "prev")


def i_next(color):
    im, d = new_icon()
    d.polygon([(24, 26), (24, 70), (58, 48)], fill=color)
    d.rounded_rectangle([64, 26, 72, 70], radius=3, fill=color)
    save(im, "next")


def i_shuffle(color):
    im, d = new_icon()
    w = 6
    d.line([(22, 30), (74, 66)], fill=color, width=w)
    d.line([(22, 66), (74, 30)], fill=color, width=w)
    d.polygon([(74, 22), (74, 40), (60, 30)], fill=color)
    d.polygon([(74, 74), (74, 56), (60, 66)], fill=color)
    save(im, "shuffle")


def i_repeat(color):
    im, d = new_icon()
    w = 6
    d.arc([24, 24, 72, 72], start=300, end=210, fill=color, width=w)
    d.polygon([(60, 18), (74, 30), (58, 34)], fill=color)
    save(im, "repeat")


def i_heart(color, fill=True):
    im, d = new_icon()
    if fill:
        d.pieslice([24, 26, 50, 52], 180, 360, fill=color)
        d.pieslice([46, 26, 72, 52], 180, 360, fill=color)
        d.polygon([(26, 42), (70, 42), (48, 74)], fill=color)
    save(im, "heart" if fill else "heart_o")


red = (255, 76, 76, 255)
white = (255, 255, 255, 255)
i_play(white)
i_pause(white)
i_prev(white)
i_next(white)
i_shuffle((200, 200, 210, 255))
i_repeat((200, 200, 210, 255))
i_heart(red, True)


# ---- 底部 TabBar 图标（线稿，普通灰 + 选中绿两态）----
# 关键：以 4x 超采样绘制再 LANCZOS 缩小，得到抗锯齿、边缘平滑的高清图标。
GRAY = (153, 153, 153, 255)
GREEN = (7, 193, 96, 255)
SS = 4  # 超采样倍率


def _sp(pts, k):
    """按倍率缩放点集"""
    return [(x * k, y * k) for (x, y) in pts]


def _sb(box, k):
    """按倍率缩放包围盒"""
    return [v * k for v in box]


def _tab_home(d, c, k):
    w = 6 * k
    # 屋顶
    d.line(_sp([(16, 48), (48, 20), (80, 48)], k), fill=c, width=w, joint="curve")
    # 房身（左墙 -> 底 -> 右墙）
    d.line(_sp([(27, 45), (27, 78), (69, 78), (69, 45)], k), fill=c, width=w, joint="curve")


def _tab_cat(d, c, k):
    w = 6 * k
    r = 6 * k
    for ox in (22, 52):
        for oy in (22, 52):
            d.rounded_rectangle(_sb([ox, oy, ox + 22, oy + 22], k), radius=r, outline=c, width=w)


def _tab_cart(d, c, k):
    w = 6 * k
    # 提手 + 车斗左侧斜边 + 底边
    d.line(_sp([(15, 24), (26, 24), (35, 62), (69, 62)], k), fill=c, width=w, joint="curve")
    # 车斗上沿 + 右侧斜边到底边右端
    d.line(_sp([(30, 36), (78, 36), (69, 62)], k), fill=c, width=w, joint="curve")
    # 两个轮子（实心）
    d.ellipse(_sb([36, 68, 48, 80], k), fill=c)
    d.ellipse(_sb([58, 68, 70, 80], k), fill=c)


def _tab_user(d, c, k):
    w = 6 * k
    # 头
    d.ellipse(_sb([35, 18, 61, 44], k), outline=c, width=w)
    # 肩（半圆弧）
    d.arc(_sb([26, 50, 70, 90], k), start=180, end=360, fill=c, width=w)


def tab_icon(name, drawer):
    for suffix, col in (("", GRAY), ("_on", GREEN)):
        big = Image.new("RGBA", (S * SS, S * SS), (0, 0, 0, 0))
        d = ImageDraw.Draw(big)
        drawer(d, col, SS)
        small = big.resize((S, S), Image.LANCZOS)
        small.save(f"{OUT}/icons/tab_{name}{suffix}.png")
        print("  icon tab_" + name + suffix)


tab_icon("home", _tab_home)
tab_icon("cat", _tab_cat)
tab_icon("cart", _tab_cart)
tab_icon("user", _tab_user)


# ---- 搜索图标（放大镜，4x 超采样抗锯齿）----
def i_search(color=GRAY):
    big = Image.new("RGBA", (S * SS, S * SS), (0, 0, 0, 0))
    d = ImageDraw.Draw(big)
    d.ellipse(_sb([24, 24, 58, 58], SS), outline=color, width=6 * SS)
    d.line(_sp([(55, 55), (76, 76)], SS), fill=color, width=7 * SS)
    big.resize((S, S), Image.LANCZOS).save(f"{OUT}/icons/search.png")
    print("  icon search")


i_search()

print("完成。")
