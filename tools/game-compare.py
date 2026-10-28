#!/usr/bin/env python3
"""小游戏的 Chrome 参考对比（game-doc/05 §2.4、§4.4；GT-F3 / C4）。

    python3 tools/game-compare.py <夹具> [--frames N] [--hz 60] [--cells 2x8] [--runs 3] [--out 目录] [--chrome 路径]
                                  [--thresholds 文件.json]

同一份 game.js：
- 引擎：`mini-app-window <包> --frames N --hz H --snapshot <out>/engine`，取 `frame_NNNN.native.png`（主画布原分辨率）；
- Chrome：headless 打开拼好的 HTML（tools/game-ref/shim.js 垫片 + 夹具全部 .js + fonts/ 下的字体作为 data URL 的 @font-face），
  跑 N 帧后把主画布的 toDataURL 写进 DOM，`--dump-dom` 读回。不截屏：没有窗口尺寸 / 缩放 / 滚动条的问题。
两边都压到黑底（引擎上屏就是压黑底），按 `--cells` 切格（列 x 行），逐格给「差异像素占比、包围盒、最大通道差」。
「差异」= 任一通道差 > 容差（`--tol`，默认 0：逐像素精确比）。

`--runs N`：两边各跑 N 次，先确认各自自稳定（N 次 md5 相同），否则报错退出 —— 自己都不稳定，阈值没有意义。
`--thresholds`：`{"cells": {"0": 3.5, …}, "default": 5.0}`（百分比），给了就判定，超出的格退出码 1。

夹具的 fonts/ 不进仓库：脚本从 assets/ 补 Arial.ttf / Monaco.ttf（夹具 game.js 里 loadFont 的就是这两个）。
"""
import argparse
import base64
import hashlib
import json
import os
import re
import shutil
import struct
import subprocess
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"


def family_name(data: bytes):
    """name 表 nameID 1（与 src/game/fonts.rs 同一个取法：Windows 英文优先，其次 Mac 英文）"""
    u16 = lambda o: struct.unpack(">H", data[o:o + 2])[0]
    u32 = lambda o: struct.unpack(">I", data[o:o + 4])[0]
    base = u32(12) if data[:4] == b"ttcf" else 0
    n = u16(base + 4)
    name = None
    for i in range(n):
        r = base + 12 + i * 16
        if data[r:r + 4] == b"name":
            name = u32(r + 8)
    if name is None:
        return None
    count, strings = u16(name + 2), name + u16(name + 4)
    best = None
    for i in range(count):
        r = name + 6 + i * 12
        plat, lang, nid, ln, off = u16(r), u16(r + 4), u16(r + 6), u16(r + 8), u16(r + 10)
        if nid != 1:
            continue
        raw = data[strings + off:strings + off + ln]
        text = raw.decode("latin-1") if plat == 1 else raw.decode("utf-16-be", "replace")
        rank = 0 if (plat, lang) == (3, 0x0409) else 1 if (plat, lang) == (1, 0) else 2
        if text.strip() and (best is None or rank < best[0]):
            best = (rank, text.strip())
    return best[1] if best else None


def assemble(fixture: str, work: Path) -> Path:
    pkg = work / fixture
    if pkg.exists():
        shutil.rmtree(pkg)
    shutil.copytree(ROOT / "sample-game" / fixture, pkg)
    shutil.copytree(ROOT / "sample-game" / "_lib", pkg / "_lib")
    fonts = pkg / "fonts"
    fonts.mkdir(exist_ok=True)
    for f in ("Arial.ttf", "Monaco.ttf"):
        if not (fonts / f).exists():
            shutil.copy(ROOT / "assets" / f, fonts / f)
    return pkg


def run_engine(pkg: Path, out: Path, frames: int, hz: int) -> Image.Image:
    if out.exists():
        shutil.rmtree(out)
    exe = ROOT / "target/release/mini-app-window"
    r = subprocess.run([str(exe), str(pkg), "--frames", str(frames), "--hz", str(hz), "--snapshot", str(out)],
                       capture_output=True, text=True, timeout=120)
    p = out / f"frame_{frames:04d}.native.png"
    if not p.exists():
        sys.exit(f"引擎没出图（退出码 {r.returncode}）：\n{r.stdout[-2000:]}\n{r.stderr[-2000:]}")
    return Image.open(p).convert("RGBA")


def build_html(pkg: Path, frames: int, hz: int) -> str:
    mods = []
    for p in sorted(pkg.rglob("*.js")):
        rel = p.relative_to(pkg).as_posix()
        src = p.read_text(encoding="utf-8")
        mods.append(f'{json.dumps(rel)}: {{ fn: function (module, exports, require) {{\n{src}\n}} }}')
    fonts, faces = {}, []
    for f in sorted((pkg / "fonts").glob("*.[to]tf")):
        data = f.read_bytes()
        fam = family_name(data) or f.stem
        fonts[f"fonts/{f.name}"] = fam
        b64 = base64.b64encode(data).decode()
        faces.append(f"@font-face {{ font-family: '{fam}'; src: url(data:font/ttf;base64,{b64}); }}")
    cfg = {"frames": frames, "hz": hz, "fonts": fonts, "query": {}}
    shim = (ROOT / "tools/game-ref/shim.js").read_text(encoding="utf-8")
    fams = json.dumps(sorted(set(fonts.values())))
    return f"""<!doctype html><html><head><meta charset="utf-8"><style>{''.join(faces)}</style></head><body>
<pre id="out">PENDING</pre>
<script>window.__CFG = {json.dumps(cfg)}; window.__MODS = {{ {','.join(mods)} }};</script>
<script>{shim}</script>
<script>
// 字体全部加载完再跑 game.js（canvas 的 fillText 不会等字体）
Promise.all({fams}.map(function (f) {{ return document.fonts.load('16px "' + f + '"'); }}))
  .then(function () {{ try {{ window.__start(); }} catch (e) {{ document.getElementById('out').textContent = 'ERROR ' + e + '\\n' + e.stack; }} }});
</script></body></html>"""


def run_chrome(chrome: str, html: Path, profile: Path) -> Image.Image:
    for attempt in range(3):
        try:
            r = subprocess.run([chrome, "--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check",
                                "--disable-extensions", "--disable-component-update", "--disable-background-networking",
                                "--virtual-time-budget=5000", f"--user-data-dir={profile}", "--dump-dom", html.as_uri()],
                               capture_output=True, text=True, timeout=60)
        except subprocess.TimeoutExpired:
            continue
        m = re.search(r'<pre id="out">(.*?)</pre>', r.stdout, re.S)
        if m and m.group(1).startswith("data:image/png;base64,"):
            raw = base64.b64decode(m.group(1).split(",", 1)[1])
            tmp = profile.parent / "chrome.png"
            tmp.write_bytes(raw)
            return Image.open(tmp).convert("RGBA")
        if m and m.group(1).startswith("ERROR"):
            sys.exit(f"Chrome 里夹具抛错：{m.group(1)[:2000]}")
    sys.exit("Chrome 没交回画布（3 次）")


def on_black(img: Image.Image) -> Image.Image:
    bg = Image.new("RGBA", img.size, (0, 0, 0, 255))
    return Image.alpha_composite(bg, img).convert("RGB")


def md5(img: Image.Image) -> str:
    return hashlib.md5(img.tobytes()).hexdigest()


def compare(a: Image.Image, b: Image.Image, cols: int, rows: int, tol: int):
    w, h = a.size
    la, lb = a.load(), b.load()
    cw, ch = w // cols, h // rows
    cells = []
    for r in range(rows):
        for c in range(cols):
            x0, y0 = c * cw, r * ch
            n, worst, box = 0, 0, None
            for y in range(y0, y0 + ch):
                for x in range(x0, x0 + cw):
                    pa, pb = la[x, y], lb[x, y]
                    d = max(abs(pa[0] - pb[0]), abs(pa[1] - pb[1]), abs(pa[2] - pb[2]))
                    if d > tol:
                        n += 1
                        worst = max(worst, d)
                        box = (x, y, x, y) if box is None else (min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y))
            cells.append({"cell": r * cols + c, "rect": [x0, y0, x0 + cw - 1, y0 + ch - 1], "pct": round(100.0 * n / (cw * ch), 3),
                          "px": n, "max": worst, "bbox": box})
    return cells


def diff_image(a: Image.Image, b: Image.Image, tol: int) -> Image.Image:
    w, h = a.size
    out = Image.new("RGB", (w * 3, h), (255, 255, 255))
    out.paste(a, (0, 0))
    out.paste(b, (w, 0))
    la, lb = a.load(), b.load()
    d = Image.new("RGB", (w, h), (255, 255, 255))
    ld = d.load()
    for y in range(h):
        for x in range(w):
            pa, pb = la[x, y], lb[x, y]
            if max(abs(pa[i] - pb[i]) for i in range(3)) > tol:
                ld[x, y] = (255, 0, 0)
    out.paste(d, (w * 2, 0))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("fixture")
    ap.add_argument("--frames", type=int, default=2)
    ap.add_argument("--hz", type=int, default=60)
    ap.add_argument("--cells", default="2x8")
    ap.add_argument("--runs", type=int, default=1)
    ap.add_argument("--tol", type=int, default=0)
    ap.add_argument("--out", default=None)
    ap.add_argument("--chrome", default=DEFAULT_CHROME)
    ap.add_argument("--thresholds", default=None)
    a = ap.parse_args()
    out = Path(a.out or ROOT / "target/_game_compare" / a.fixture).resolve()
    out.mkdir(parents=True, exist_ok=True)
    pkg = assemble(a.fixture, out / "pkg")
    html = out / "ref.html"
    html.write_text(build_html(pkg, a.frames, a.hz), encoding="utf-8")
    eng, chr_ = [], []
    for i in range(a.runs):
        eng.append(on_black(run_engine(pkg, out / f"engine{i}", a.frames, a.hz)))
        chr_.append(on_black(run_chrome(a.chrome, html, out / f"profile{i}")))
    for name, imgs in (("引擎", eng), ("Chrome", chr_)):
        hs = {md5(x) for x in imgs}
        if len(hs) != 1:
            sys.exit(f"{name} {a.runs} 次结果不同（{len(hs)} 种 md5）：自己都不稳定，不能比")
    if eng[0].size != chr_[0].size:
        sys.exit(f"尺寸不同：引擎 {eng[0].size}，Chrome {chr_[0].size}")
    eng[0].save(out / "engine.png")
    chr_[0].save(out / "chrome.png")
    diff_image(eng[0], chr_[0], a.tol).save(out / "side-by-side.png")
    cols, rows = (int(v) for v in a.cells.split("x"))
    cells = compare(eng[0], chr_[0], cols, rows, a.tol)
    total = sum(c["px"] for c in cells) * 100.0 / (eng[0].size[0] * eng[0].size[1])
    th = json.loads(Path(a.thresholds).read_text()) if a.thresholds else None
    bad = []
    ver = subprocess.run([a.chrome, "--version"], capture_output=True, text=True).stdout.strip()
    print(f"【{a.fixture}】{ver}，{a.frames} 帧 @{a.hz}Hz，自稳定 {a.runs} 次，整图差异 {total:.3f}%（容差 {a.tol}）")
    for c in cells:
        limit = None if th is None else th.get("cells", {}).get(str(c["cell"]), th.get("default"))
        inside = c["bbox"] is None or (c["rect"][0] <= c["bbox"][0] and c["rect"][1] <= c["bbox"][1]
                                       and c["bbox"][2] <= c["rect"][2] and c["bbox"][3] <= c["rect"][3])
        flag = ""
        if limit is not None and (c["pct"] > limit or not inside):
            flag = f"  ✗ 超过阈值 {limit}%"
            bad.append(c["cell"])
        print(f"  格 {c['cell']:2d}：{c['pct']:7.3f}%  {c['px']:5d} 像素  最大通道差 {c['max']:3d}  包围盒 {c['bbox']}{flag}")
    (out / "report.json").write_text(json.dumps({"chrome": ver, "total_pct": round(total, 3), "cells": cells}, ensure_ascii=False, indent=1))
    print(f"  → {out}/side-by-side.png（左引擎、中 Chrome、右差异）")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
