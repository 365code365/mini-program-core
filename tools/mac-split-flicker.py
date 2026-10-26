#!/usr/bin/env python3
"""macOS 分栏闪烁量具：真录屏 + 真鼠标点击，逐帧量屏上两栏变了几次。

用法（要「屏幕录制」与「辅助功能」权限给运行它的终端 / Kiro）：

    python3 tools/mac-split-flicker.py [--app chat] [--clicks 200,300;200,400;200,500] [--keep]

做的事：
1. 起「灵犀 for Mac」的小程序子进程（`-miniHost 1`），窗口拉到 900pt（分栏）；
2. Quartz 找到窗口在屏上的位置，ffmpeg 按 60fps 录那一块（avfoundation 屏幕捕获，屏上真正显示的像素）；
3. cliclick 发真鼠标点击（窗口内 pt 坐标），每下隔 1.5 秒；
4. 逐帧比：每一帧左栏 / 右栏与前一帧差多少像素，并找 **A→B→A 回跳**（这一帧与前前帧几乎一样、与前一帧差很多
   —— 屏上闪了一下）。结论写 target/mac-flicker/<时刻>/report.txt，帧留在同目录（--keep 时不删）。

判据（2026-10-27 定）：每次点击里左栏不许回跳（A→B→A 次数 = 0），右栏换页最多一次大变化之外不许回跳。
"""
import argparse, base64, json, os, shutil, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
APP = ROOT / "sdk/macos/example/build/dd/Build/Products/Debug/灵犀.app/Contents/MacOS/灵犀"


def find_window(pid, min_w=200):
    import Quartz
    for _ in range(150):
        for w in Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionOnScreenOnly, Quartz.kCGNullWindowID):
            if w.get("kCGWindowOwnerPID") == pid and w.get("kCGWindowLayer") == 0:
                b = w["kCGWindowBounds"]
                if b["Width"] >= min_w:
                    return b["X"], b["Y"], b["Width"], b["Height"]
        time.sleep(0.1)
    sys.exit("找不到小程序窗口")


def display_for(x, y):
    """(avfoundation 设备号, 屏原点, 像素比)。avfoundation 的 Capture screen N 按 CGGetActiveDisplayList 的顺序"""
    import Quartz
    _, ids, n = Quartz.CGGetActiveDisplayList(8, None, None)
    out = subprocess.run(["ffmpeg", "-hide_banner", "-f", "avfoundation", "-list_devices", "true", "-i", ""],
                         capture_output=True, text=True).stderr
    first = min(int(l.split("[")[2].split("]")[0]) for l in out.splitlines() if "Capture screen" in l)
    for i, d in enumerate(ids[:n]):
        b = Quartz.CGDisplayBounds(d)
        if b.origin.x <= x < b.origin.x + b.size.width and b.origin.y <= y < b.origin.y + b.size.height:
            scale = Quartz.CGDisplayPixelsWide(d) / b.size.width
            mode = Quartz.CGDisplayCopyDisplayMode(d)
            scale = Quartz.CGDisplayModeGetPixelWidth(mode) / b.size.width
            return first + i, (b.origin.x, b.origin.y), scale
    sys.exit("窗口不在任何屏上")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--app", default="chat")
    ap.add_argument("--width", type=int, default=900)
    ap.add_argument("--clicks", default="200,300;200,400;200,500;200,600")
    ap.add_argument("--fps", type=int, default=60)
    ap.add_argument("--keep", action="store_true")
    a = ap.parse_args()
    out = ROOT / "target/mac-flicker" / time.strftime("%Y%m%d-%H%M%S")
    out.mkdir(parents=True)
    subprocess.run(["pkill", "-x", "灵犀"])
    link = base64.b64encode(json.dumps({"key": a.app, "path": "", "query": {}, "scene": 1001, "referrer": None}).encode()).decode()
    log = open(out / "app.log", "w")
    app = subprocess.Popen([str(APP), "-miniHost", "1", "-link", link, "-miniWidth", str(a.width)], stdout=log, stderr=log)
    time.sleep(4)  # 下载 / 解包 / 首页，4 秒时拉宽
    wx, wy, ww, wh = find_window(app.pid, a.width - 2)
    time.sleep(1)
    wx, wy, ww, wh = find_window(app.pid, a.width - 2)
    dev, (ox, oy), scale = display_for(wx, wy)
    crop = (int((wx - ox) * scale), int((wy - oy) * scale), int(ww * scale), int(wh * scale))
    clicks = [tuple(float(v) for v in c.split(",")) for c in a.clicks.split(";") if c]
    dur = 2 + 1.5 * len(clicks)
    vid = out / "screen.mkv"
    rec = subprocess.Popen(["ffmpeg", "-hide_banner", "-loglevel", "error", "-f", "avfoundation", "-capture_cursor", "0",
                            "-pixel_format", "bgr0", "-framerate", str(a.fps), "-i", f"{dev}:none", "-t", str(dur),
                            "-vf", "crop=%d:%d:%d:%d,scale=iw/2:ih/2" % (crop[2], crop[3], crop[0], crop[1]),
                            "-c:v", "ffv1", "-y", str(vid)])
    time.sleep(1.0)
    stamps = []
    for (x, y) in clicks:
        gx, gy = int(wx + x), int(wy + y)
        stamps.append(time.time())
        r = subprocess.run(["cliclick", f"c:{gx},{gy}"], capture_output=True, text=True)
        if r.returncode:
            print("cliclick 失败（辅助功能权限？）", r.stderr)
        time.sleep(1.5)
    rec.wait()
    app.terminate()
    frames = out / "f"
    frames.mkdir()
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-i", str(vid), str(frames / "%05d.png")])
    analyze(out, frames, ww, a.width)
    if not a.keep:
        shutil.rmtree(frames)
        vid.unlink()


def analyze(out, frames, ww, total):
    import numpy as np
    from PIL import Image
    fs = sorted(frames.glob("*.png"))
    arr = [np.asarray(Image.open(f).convert("L")).astype(np.int16) for f in fs]
    h, w = arr[0].shape
    half = w // 2
    top = int(h * 0.09)  # 去掉标题栏 / 导航栏那一条（标题会换，不算闪）
    lines = []
    events = []
    for side, sl in (("左栏", slice(0, half)), ("右栏", slice(half, w))):
        diffs = []
        for i in range(1, len(arr)):
            d = np.abs(arr[i][top:, sl] - arr[i - 1][top:, sl]) > 12
            diffs.append(int(d.sum()))
        bounces = []
        for i in range(2, len(arr)):
            prev = int((np.abs(arr[i][top:, sl] - arr[i - 1][top:, sl]) > 12).sum())
            back = int((np.abs(arr[i][top:, sl] - arr[i - 2][top:, sl]) > 12).sum())
            if prev > 2000 and back < prev * 0.2:
                bounces.append((i, prev, back))
        changed = [(i + 1, d) for i, d in enumerate(diffs) if d > 300]
        lines.append(f"{side}: 变化帧 {len(changed)}，A→B→A 回跳 {len(bounces)}")
        lines.append("  变化帧（帧号:像素）: " + " ".join(f"{i}:{d}" for i, d in changed))
        lines.append("  回跳: " + " ".join(f"{i}(差前一帧{p} 差前前帧{b})" for i, p, b in bounces))
        events.append(len(bounces))
    rep = "\n".join([f"帧数 {len(arr)}，尺寸 {w}x{h}（半分辨率）"] + lines)
    (out / "report.txt").write_text(rep)
    print(rep)
    print("结果目录", out)


if __name__ == "__main__":
    main()
