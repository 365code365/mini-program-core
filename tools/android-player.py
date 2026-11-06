#!/usr/bin/env python3
"""真机量云听音乐（商店里的 music-online）播放页三段：进播放页、拖进度条、返回歌单页，按轮重复。

    python3 tools/android-player.py [install] [--rounds 3] [--native]

每段给出：屏上（SurfaceFlinger 引擎那一层）重复几帧 / 最长空档、引擎 >8ms 的帧（`debug.mini.damage_log`）与最慢的几帧。
等待一律轮询日志（等「page pages/player/player」），不写死 sleep；日志开一条 logcat 流写进本地文件再找
（一加 PLY110 的 logcat 环 256KB，几十秒就冲掉）。坐标按 1272 宽的屏写（页面按 dp 排，别的宽度等比换算）。
APK：Flutter 示例 `flutter build apk --release`（--native：`sdk/android/example` 的 `./gradlew assembleRelease`）。
"""
import json
import os
import re
import statistics
import subprocess
import sys
import threading
import time
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ADB = os.path.expanduser("~/Library/Android/sdk/platform-tools/adb")
SERIAL = os.environ.get("ANDROID_SERIAL", "3B669L00NA000000")
STORE = os.environ.get("STORE_URL", "https://mini-program.51aihelp.com")
OUT = os.path.join(ROOT, "target", "_player")
NATIVE = "--native" in sys.argv
ROUNDS = int(sys.argv[sys.argv.index("--rounds") + 1]) if "--rounds" in sys.argv else 3
PKG = "dev.minirender.example" if NATIVE else "dev.minirender.mini_render_example"
APK = (
    "sdk/android/example/app/build/outputs/apk/release/app-release.apk"
    if NATIVE
    else "sdk/flutter/example/build/app/outputs/flutter-apk/app-release.apk"
)
HOSTTAG = "mini-example" if NATIVE else "flutter"


def adb(*a, check=False, text=True):
    r = subprocess.run([ADB, "-s", SERIAL, *a], capture_output=True, text=text)
    if check and r.returncode:
        sys.exit(f"adb {' '.join(a)}：{r.stderr}")
    return r.stdout


def screen_k():
    s = adb("shell", "wm", "size")
    m = re.search(r"Override size: (\d+)x", s) or re.search(r"Physical size: (\d+)x", s)
    return int(m.group(1)) / 1272 if m else 1.0


K = 1.0


def tap(x, y):
    adb("shell", "input", "tap", str(round(x * K)), str(round(y * K)))


def swipe(x0, y0, x1, y1, ms):
    adb("shell", "input", "swipe", *[str(round(v * K)) for v in (x0, y0, x1, y1)], str(ms))


def mark(name):
    adb("shell", "log", "-t", "mini-mark", name)


class Log:
    """一条 logcat 流写进本地文件（monotonic 时间戳，与 SurfaceFlinger 同一个钟）"""

    def __init__(self, path):
        adb("logcat", "-c")
        self.path = path
        self.f = open(path, "w", encoding="utf-8", errors="replace")
        self.p = subprocess.Popen(
            [ADB, "-s", SERIAL, "logcat", "-v", "monotonic", "-s", "mini-render:I", f"{HOSTTAG}:I", "mini-mark:I"],
            stdout=self.f, stderr=subprocess.DEVNULL,
        )

    def text(self):
        self.f.flush()
        return open(self.path, encoding="utf-8", errors="replace").read()

    def wait(self, pat, since, timeout):
        """等日志里在 `since`（字符偏移）之后出现 pat；返回新的偏移"""
        end = time.time() + timeout
        while time.time() < end:
            t = self.text()
            m = re.search(pat, t[since:])
            if m:
                return since + m.end()
            time.sleep(0.15)
        return None

    def close(self):
        self.p.terminate()
        self.f.close()


def engine_layers():
    out = adb("shell", "dumpsys", "SurfaceFlinger", "--list")
    ls = [l.strip() for l in out.splitlines() if f"SurfaceView[{PKG}" in l and "BLAST" in l]
    ls = [re.sub(r"^RequestedLayerState\{", "", l) for l in ls]
    return [re.sub(r" parentId=.*$", "", l) for l in ls]


class Presents:
    """每 0.6s 取一次 SurfaceFlinger --latency（只留最近 127 帧），按上屏时刻去重拼起来"""

    def __init__(self):
        self.layers = engine_layers()
        for l in self.layers:
            adb("shell", f"dumpsys SurfaceFlinger --latency-clear '{l}'")
        self.ts = {l: set() for l in self.layers}
        self.stop = False
        self.th = threading.Thread(target=self.run, daemon=True)
        self.th.start()

    def poll(self):
        for l in self.layers:
            for line in adb("shell", f"dumpsys SurfaceFlinger --latency '{l}'").splitlines()[1:]:
                x = line.split()
                if len(x) == 3 and 0 < int(x[1]) < 2**62:
                    self.ts[l].add(int(x[1]))

    def run(self):
        while not self.stop:
            self.poll()
            time.sleep(0.6)

    def finish(self):
        self.stop = True
        self.th.join()
        self.poll()
        # 帧最多的那一层是引擎画面（另一层可能是视频 / 地图）
        return sorted(max(self.ts.values(), key=len)) if self.ts else []


def parse_log(text):
    marks, frames, pumps = [], [], []
    for l in text.splitlines():
        m = re.match(r"\s*(\d+)\.(\d+)\s+\d+\s+\d+\s+\w\s+([\w-]+)\s*:\s*(.*)", l)
        if not m:
            continue
        t = int(m.group(1)) * 10**9 + int(m.group(2).ljust(9, "0")[:9])
        tag, msg = m.group(3), m.group(4)
        if tag == "mini-mark":
            marks.append((t, msg.strip()))
        elif "🩹 [SDK]" in msg:
            mm = re.search(r"🩹 \[SDK\] (\S+).*?([\d.]+)ms\s*(〔慢.*)?$", msg)
            if mm:
                frames.append((t, mm.group(1), float(mm.group(2)), msg))
        elif "⏱ [SDK] pump" in msg:
            mm = re.search(r"pump ([\d.]+)ms", msg)
            if mm:
                pumps.append((t, float(mm.group(1)), msg))
    return marks, frames, pumps


def report(presents, marks, frames, pumps):
    gaps_all = [(b - a) / 1e6 for a, b in zip(presents, presents[1:])]
    period = statistics.median([g for g in gaps_all if g < 40]) if gaps_all else 8.33
    marks = marks + [(presents[-1] + 1 if presents else 0, "end")]
    rows = []
    for (t0, name), (t1, _) in zip(marks, marks[1:]):
        p = [x for x in presents if t0 <= x < t1]
        g = [(b - a) / 1e6 for a, b in zip(p, p[1:])]
        g = [x for x in g if x < 400]
        # 这一段的显示周期按这一段自己的帧间隔取：ColorOS 没有触摸时把屏降到 60Hz（16.7ms），
        # 按全程的 90Hz（11.0ms）算，60Hz 下每一帧都会被算成「重复一帧」—— 静置、歌单页那几段的 49.5% 就是这么来的
        # 哪个刷新率的 vsync 网格最贴这些间隔（真掉帧在 90Hz 下是 22ms，贴 90 的网格、不贴 60 的；
        # 都贴（全是 33ms）时取高的那个，宁可多算重复）
        def fit(c):
            return statistics.mean(abs(x / c - round(x / c)) for x in g) if g else 1
        ph = next((c for c in sorted({period, 1000 / 60}) if fit(c) < 0.12), period)
        rep = sum(round(x / ph) - 1 for x in g if x > ph * 1.5)
        fr = [f for f in frames if t0 <= f[0] < t1]
        slow = [f for f in fr if f[2] > 8.0]
        pu = [x for x in pumps if t0 <= x[0] < t1]
        kinds = {}
        for f in fr:
            kinds[f[1]] = kinds.get(f[1], 0) + 1
        ms = sorted(f[2] for f in fr)
        p50 = ms[len(ms) // 2] if ms else 0
        rows.append((name, len(p), rep, max(g) if g else 0, len(fr), len(slow), p50, ms[-1] if ms else 0, kinds, slow, pu, 1000 / ph))
    print(f"显示周期 {period:.2f}ms")
    for name, n, rep, mx, nf, ns, p50, worst, kinds, slow, pu, hz in rows:
        print(f"【{name:8}】{hz:3.0f}Hz 屏上 {n:3} 帧 重复 {rep:3}（{100 * rep / max(1, n + rep):4.1f}%） 最长 {mx:4.0f}ms ｜ 引擎 {nf:3} 帧 p50 {p50:5.1f} 最慢 {worst:5.1f}ms >8ms {ns:3}  {kinds}")
        for f in sorted(slow, key=lambda f: -f[2])[:3]:
            print("     " + re.sub(r"滚动 [\d.-]+ 条带 [\d.]+\.\.\d+\s+", "", f[3].split("🩹 [SDK] ")[-1])[:220])
        for x in sorted(pu, key=lambda x: -x[1])[:2]:
            if x[1] > 12:
                print("     ⏱ " + x[2].split("⏱ [SDK] ")[-1][:220])
    return rows


def main():
    global K
    os.makedirs(OUT, exist_ok=True)
    if "install" in sys.argv:
        print(adb("install", "-r", os.path.join(ROOT, APK)).strip().splitlines()[-1])
    K = screen_k()
    app = json.load(urllib.request.urlopen(f"{STORE}/api/apps/music-online", timeout=10))
    adb("shell", "setprop", "debug.mini.damage_log", "1")
    adb("shell", "am", "force-stop", PKG)
    log = Log(os.path.join(OUT, "log.txt"))
    path = "pages%2Fplaylist%2Fplaylist%3Fid%3D101"
    if NATIVE:
        link = json.dumps({"key": app["appId"], "path": "pages/playlist/playlist", "query": {"id": "101"}, "scene": 1065}, separators=(",", ":"))
        adb("shell", f"am start -n {PKG}/.MainActivity --es storeLink '{link}' --es storeUrl '{STORE}'")
    else:
        adb("shell", f"am start -a android.intent.action.VIEW -d 'minirender://open?appid={app['appId']}&path={path}' {PKG}")
    pos = log.wait(r"page pages/playlist/playlist", 0, 60)
    if pos is None:
        log.close()
        sys.exit("⚠ 歌单页没打开（看 target/_player/log.txt）")
    # 歌单的歌拉到（列表画出来）再动：等引擎安静下来
    time.sleep(2.5)
    pr = Presents()
    for k in range(1, ROUNDS + 1):
        mark(f"进入#{k}")
        tap(600, 1320)
        p2 = log.wait(r"page pages/player/player", pos, 8)
        if p2 is None:
            print(f"⚠ 第 {k} 轮没进到播放页")
            break
        pos = p2
        time.sleep(1.2)
        mark(f"静止#{k}")
        time.sleep(1.5)
        mark(f"拖动#{k}")
        swipe(290, 2109, 1000, 2109, 2500)
        time.sleep(0.4)
        mark(f"拖后#{k}")
        time.sleep(1.0)
        mark(f"返回#{k}")
        tap(66, 210)
        p3 = log.wait(r"page pages/playlist/playlist", pos, 6)
        if p3 is None:
            print(f"⚠ 第 {k} 轮没回到歌单页")
            break
        pos = p3
        time.sleep(1.2)
        mark(f"歌单#{k}")
        time.sleep(0.8)
    mark("结束")
    time.sleep(0.5)
    presents = pr.finish()
    text = log.text()
    log.close()
    adb("shell", "setprop", "debug.mini.damage_log", '""')
    marks, frames, pumps = parse_log(text)
    if not presents:
        sys.exit("⚠ 没量到上屏（引擎那一层没找到）")
    report(presents, marks, frames, pumps)


if __name__ == "__main__":
    main()
