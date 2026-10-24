#!/usr/bin/env python3
"""宿主承载的原生组件（示例里的 <map>）滑动时跟不跟页面：真机截屏量「地图上沿 − 卡片标题上沿」的变化 d（物理像素）。

    python3 tools/android-native-align.py flutter <apk…>   # 装 /tmp/flutter_<名>.apk 再量（A/B：盖上去 vs 同层渲染）
    python3 tools/android-native-align.py native           # 原生示例（已装好的那一份）

量法（T30-27）：进宿主 tab，把地图挪到屏幕中上部；手指按在地图**上方**的卡片上慢拖 700px（2.5s，盖上去的地图会吃掉
从它上面开始的拖动），期间连拍 8 张 × 3 轮。地图画进页面（同层渲染）时 d 恒为 0；盖在画面上时随上屏延迟忽大忽小。
SurfaceFlinger 的「屏上重复」量不出这个：两层各自都按时上屏了，错开的是两层之间。设备序列号用 ANDROID_SERIAL。
"""
import subprocess, sys, time, threading, os
from PIL import Image


def find_map(path):
    """(地图上沿, 卡片标题上沿)；找不到返回 None。地图背景 #E8EEF5（示例的假地图），标题是黑字"""
    im = Image.open(path).convert('RGB')

    def near(p, q, tol=6):
        return all(abs(a - b) <= tol for a, b in zip(p, q))

    w, h = im.size
    x = w // 5
    bg = (0xE8, 0xEE, 0xF5)
    rows = [y for y in range(h) if near(im.getpixel((x, y)), bg)]
    if not rows:
        return None
    # 最长的一段连续（网格线会打断：允许 6px 的空隙）
    segs, start, prev = [], rows[0], rows[0]
    for y in rows[1:]:
        if y - prev > 6:
            segs.append((start, prev))
            start = y
        prev = y
    segs.append((start, prev))
    top, bot = max(segs, key=lambda s: s[1] - s[0])
    if bot - top < 60:
        return None
    # 卡片标题（黑字）：从地图上沿往上找第一段深色行（中间的说明文字是灰的，不算），取这一段的顶。
    # 不越过导航栏（y < 300 是状态栏 + 导航栏，那里的黑色图标会被误认成标题）
    def dark(y):
        return any(sum(im.getpixel((xx, y))) < 120 for xx in range(40, w // 2, 3))
    title, y = -1, top - 1
    while y > 300 and not dark(y):
        y -= 1
    if y > 300:
        gap = 0
        while y > 300 and gap <= 8:
            if dark(y):
                title, gap = y, 0
            else:
                gap += 1
            y -= 1
        # 标题顶贴着导航栏：可能被裁了一半，不算
        if title <= 310:
            title = -1
    return (top, title) if title != -1 else None


DEV = os.environ.get('ANDROID_SERIAL', '3B669L00NA000000')
A = os.path.expanduser('~/Library/Android/sdk/platform-tools/adb')
OUT = 'target/_align'
os.makedirs(OUT, exist_ok=True)
kind = sys.argv[1]
pkg = 'dev.minirender.mini_render_example' if kind == 'flutter' else 'dev.minirender.example'
tag = 'flutter:I' if kind == 'flutter' else 'mini-example:I'

def D(*a, out=None):
    return subprocess.run([A, '-s', DEV, *a], stdin=subprocess.DEVNULL, stdout=out or subprocess.PIPE, stderr=subprocess.DEVNULL).stdout

def shot(path):
    with open(path, 'wb') as f:
        D('exec-out', 'screencap', '-p', out=f)

def find(path):
    return find_map(path)

def wait_log(s):
    for _ in range(40):
        if s in (D('logcat', '-d', '-s', tag) or b'').decode('utf-8', 'replace'):
            return True
        time.sleep(0.5)
    return False

for v in (sys.argv[2:] or ['native']):
    if kind == 'flutter':
        D('install', '-r', f'/tmp/flutter_{v}.apk')
    D('shell', 'am', 'force-stop', pkg); D('logcat', '-c')
    if kind == 'flutter':
        D('shell', 'monkey', '-p', pkg, '-c', 'android.intent.category.LAUNCHER', '1')
    else:
        # 原生示例的桌面图标进的是入口页（StoreActivity），内置示例直接起 MainActivity
        D('shell', 'am', 'start', '-n', f'{pkg}/.MainActivity')
    time.sleep(3)
    if kind == 'flutter':
        D('shell', 'input', 'tap', '500', '400')
    if not wait_log('page pages/index/index'):
        print(v, '首页没起来'); continue
    time.sleep(1.5); D('shell', 'input', 'tap', '945', '2270')
    if not wait_log('page pages/host/host'):
        print(v, '没进到宿主页'); continue
    time.sleep(1.5)
    # 把地图挪到屏幕中间偏上（上沿 700~1100）。每一下都**不从地图上开始拖**：盖上去的地图会吃掉触摸，
    # 从它上面开始的拖动页面根本不动（这本身就是那条路的毛病之一）
    found = None
    for _ in range(16):
        shot(f'{OUT}/{v}_seek.png')
        found = find(f'{OUT}/{v}_seek.png')
        if found and 700 < found[0] < 1100:
            break
        if not found:
            # 地图不在屏上：往上翻（拖动不带惯性，慢）
            D('shell', 'input', 'swipe', '540', '1150', '540', '350', '1000')
        elif found[0] >= 1100:
            s = max(found[0] - 100, 400)
            D('shell', 'input', 'swipe', '540', str(s), '540', str(s - (found[0] - 900)), '1000')
        else:
            s = min(found[0] + 560, 1700)
            D('shell', 'input', 'swipe', '540', str(s), '540', str(s + (900 - found[0])), '1000')
        time.sleep(0.8)
    if not found:
        print(v, '找不到地图'); continue
    time.sleep(1)
    shot(f'{OUT}/{v}_rest.png')
    rest = find(f'{OUT}/{v}_rest.png')
    d0 = rest[0] - rest[1]
    y0 = rest[1] - 60  # 手指按在标题上方一点（卡片上，不在地图上）
    ds = []
    for rnd in range(3):
        t = threading.Thread(target=D, args=('shell', 'input', 'swipe', '540', str(y0), '540', str(y0 + 700), '2500'))
        t.start(); time.sleep(0.25)
        for i in range(8):
            p = f'{OUT}/{v}_{rnd}_{i}.png'
            shot(p)
            m = find(p)
            if m:
                ds.append(m[0] - m[1] - d0)
        t.join(); time.sleep(0.8)
        # 拖回原位
        D('shell', 'input', 'swipe', '540', str(y0 + 700), '540', str(y0), '1500'); time.sleep(1.2)
    a = [abs(x) for x in ds]
    print(v, f'静止 d0={d0}', f'n={len(ds)} |d| 均值 {sum(a)/max(1,len(a)):.1f} 最大 {max(a or [0])} ≤2px {sum(1 for x in a if x <= 2)}/{len(a)}', 'd =', ds)
print('DRAGDONE')
