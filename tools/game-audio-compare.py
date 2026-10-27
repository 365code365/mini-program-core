#!/usr/bin/env python3
"""WebAudio 的 Chrome 参考对拍（game-doc/05 GT-AU4）。
    python3 tools/game-audio-compare.py [夹具=g-webaudio-offline] [--seconds 2] [--segments 8] [--runs 2]
                                        [--thresholds 文件.json] [--out 目录] [--chrome 路径]
同一份 game.js：
- 引擎：`mini-app-window <包> --frames N --hz 60 --snapshot <out>/engine`，环境变量 MINI_AUDIO_DUMP 把离线渲染的输出
  （48k 立体声交错 f32）写下来。上下文在 game.js 加载时建，它的 0 时刻 = dump 的第 0 帧；
- Chrome：headless 打开拼好的 HTML（tools/game-ref/shim.js + 下面的音频垫片），wx.createWebAudioContext 换成
  `new OfflineAudioContext(2, 秒数 × 48000, 48000)`，game.js 跑完 startRendering，交错 f32 的 base64 写进 DOM，`--dump-dom` 读回。
按 `--segments` 等分时间轴，逐段逐声道给「最大绝对误差、误差 RMS、参考 RMS」。
`--runs N`：两边各跑 N 次先确认自稳定（逐字节相同），不稳定直接报错 —— 自己都不稳定，阈值没有意义。
`--thresholds`：`{"segments": {"0": 1e-4, …}, "default": 0.01}`（最大绝对误差），给了就判定，超出的段退出码 1。
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

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
RATE = 48000

AUDIO_SHIM = r"""
(function () {
  var cfg = window.__CFG;
  wx.createWebAudioContext = function () {
    if (window.__actx) throw new Error('夹具只能建一个上下文');
    return (window.__actx = new OfflineAudioContext(2, Math.round(cfg.seconds * 48000), 48000));
  };
  window.__start = function () {
    window.__MODS_REQ('game.js');
    window.__actx.startRendering().then(function (buf) {
      var n = buf.length, l = buf.getChannelData(0), r = buf.getChannelData(1), f = new Float32Array(n * 2);
      for (var i = 0; i < n; i++) { f[2 * i] = l[i]; f[2 * i + 1] = r[i]; }
      var u = new Uint8Array(f.buffer), s = '', CH = 0x8000;
      for (var k = 0; k < u.length; k += CH) s += String.fromCharCode.apply(null, u.subarray(k, k + CH));
      document.getElementById('out').textContent = 'PCM:' + btoa(s);
    }, function (e) { document.getElementById('out').textContent = 'ERROR ' + e; });
  };
})();
"""


def assemble(fixture: str, work: Path) -> Path:
    pkg = work / fixture
    if pkg.exists():
        shutil.rmtree(pkg)
    shutil.copytree(ROOT / "sample-game" / fixture, pkg)
    shutil.copytree(ROOT / "sample-game" / "_lib", pkg / "_lib")
    return pkg


def run_engine(pkg: Path, out: Path, seconds: float) -> bytes:
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    dump = out / "engine.f32"
    frames = int(seconds * 60) + 12
    env = dict(os.environ, MINI_AUDIO_DUMP=str(dump))
    r = subprocess.run([str(ROOT / "target/release/mini-app-window"), str(pkg), "--frames", str(frames), "--hz", "60",
                        "--snapshot", str(out / "snap")], capture_output=True, text=True, timeout=180, env=env)
    need = int(seconds * RATE) * 8
    data = dump.read_bytes() if dump.exists() else b""
    if len(data) < need:
        sys.exit(f"引擎输出不够 {seconds}s（{len(data)} / {need} 字节，退出码 {r.returncode}）：\n{r.stdout[-1500:]}\n{r.stderr[-1500:]}")
    if "T:FAIL" in r.stdout or "Error" in r.stderr[-2000:] and "T:READY" not in r.stdout:
        sys.exit(f"夹具在引擎里出错：\n{r.stdout[-1500:]}\n{r.stderr[-1500:]}")
    return data[:need]


def build_html(pkg: Path, seconds: float) -> str:
    mods = []
    for p in sorted(pkg.rglob("*.js")):
        rel = p.relative_to(pkg).as_posix()
        mods.append(f'{json.dumps(rel)}: {{ fn: function (module, exports, require) {{\n{p.read_text(encoding="utf-8")}\n}} }}')
    cfg = {"frames": 0, "hz": 60, "fonts": {}, "query": {}, "seconds": seconds}
    shim = (ROOT / "tools/game-ref/shim.js").read_text(encoding="utf-8")
    # shim.js 的 req 是闭包里的；给音频垫片留一个入口（不改 shim.js 本身：2D 对拍还在用它）
    shim = shim.replace("window.__start = function () { req('')('game.js');", "window.__MODS_REQ = req(''); window.__start = function () { req('')('game.js');")
    return f"""<!doctype html><html><head><meta charset="utf-8"></head><body>
<pre id="out">PENDING</pre>
<script>window.__CFG = {json.dumps(cfg)}; window.__MODS = {{ {','.join(mods)} }};</script>
<script>{shim}</script>
<script>{AUDIO_SHIM}</script>
<script>try {{ window.__start(); }} catch (e) {{ document.getElementById('out').textContent = 'ERROR ' + e + '\\n' + e.stack; }}</script>
</body></html>"""


def run_chrome(chrome: str, html: Path, profile: Path, seconds: float) -> bytes:
    for k in range(3):
        # 每次一个新的配置目录：上一个 Chrome 进程还没退干净时，同一个目录会被它占着，新进程把请求转给它就退出
        prof = profile.with_name(f"{profile.name}-{k}")
        shutil.rmtree(prof, ignore_errors=True)
        try:
            r = subprocess.run([chrome, "--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check",
                                "--disable-extensions", "--disable-component-update", "--disable-background-networking",
                                "--virtual-time-budget=20000", f"--user-data-dir={prof}", "--dump-dom", html.as_uri()],
                               capture_output=True, text=True, timeout=120)
        except subprocess.TimeoutExpired:
            continue
        m = re.search(r'<pre id="out">(.*?)</pre>', r.stdout, re.S)
        if m and m.group(1).startswith("PCM:"):
            data = base64.b64decode(m.group(1)[4:])
            if len(data) == int(seconds * RATE) * 8:
                return data
        if m and m.group(1).startswith("ERROR"):
            sys.exit(f"Chrome 里夹具抛错：{m.group(1)[:2000]}")
    sys.exit("Chrome 没交回 PCM（3 次）")


def floats(b: bytes):
    return struct.unpack(f"<{len(b) // 4}f", b)


def compare(eng: bytes, ref: bytes, segments: int):
    a, b = floats(eng), floats(ref)
    frames = len(b) // 2
    per = frames // segments
    rows = []
    for s in range(segments):
        for ch in (0, 1):
            worst, se, sr, at = 0.0, 0.0, 0.0, None
            for i in range(s * per, (s + 1) * per):
                x, y = a[2 * i + ch], b[2 * i + ch]
                d = abs(x - y)
                if d > worst:
                    worst, at = d, i
                se += d * d
                sr += y * y
            rows.append({"seg": s, "ch": "LR"[ch], "max": worst, "at": at, "rms_err": (se / per) ** 0.5, "rms_ref": (sr / per) ** 0.5})
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("fixture", nargs="?", default="g-webaudio-offline")
    ap.add_argument("--seconds", type=float, default=2.0)
    ap.add_argument("--segments", type=int, default=8)
    ap.add_argument("--runs", type=int, default=1)
    ap.add_argument("--out", default=None)
    ap.add_argument("--chrome", default=DEFAULT_CHROME)
    ap.add_argument("--thresholds", default=None)
    a = ap.parse_args()
    out = Path(a.out or ROOT / "target/_game_audio_compare" / a.fixture).resolve()
    out.mkdir(parents=True, exist_ok=True)
    pkg = assemble(a.fixture, out / "pkg")
    html = out / "ref.html"
    html.write_text(build_html(pkg, a.seconds), encoding="utf-8")
    eng_runs, ref_runs = [], []
    for k in range(a.runs):
        eng_runs.append(run_engine(pkg, out / f"engine{k}", a.seconds))
        ref_runs.append(run_chrome(a.chrome, html, out / f"profile{k}", a.seconds))
    for name, runs in (("引擎", eng_runs), ("Chrome", ref_runs)):
        md5s = {hashlib.md5(r).hexdigest() for r in runs}
        if len(md5s) != 1:
            sys.exit(f"{name} 自己 {a.runs} 次不一致：{md5s}")
    (out / "engine.f32").write_bytes(eng_runs[0])
    (out / "chrome.f32").write_bytes(ref_runs[0])
    rows = compare(eng_runs[0], ref_runs[0], a.segments)
    th = json.loads(Path(a.thresholds).read_text()) if a.thresholds else None
    bad = []
    print(f"{'段':>2} {'声道':>2} {'最大误差':>11} {'位置(帧)':>9} {'误差RMS':>11} {'参考RMS':>9}")
    for r in rows:
        lim = None
        if th:
            lim = th.get("segments", {}).get(str(r["seg"]), th.get("default"))
            if lim is not None and r["max"] > lim:
                bad.append(r)
        flag = "  ✗" if r in bad else ""
        print(f"{r['seg']:>2} {r['ch']:>4} {r['max']:>11.3e} {r['at'] if r['at'] is not None else '-':>9} {r['rms_err']:>11.3e} {r['rms_ref']:>9.4f}"
              + (f"  (阈值 {lim:.3e})" if lim is not None else "") + flag)
    worst = max(r["max"] for r in rows)
    print(f"整体最大误差 {worst:.3e}；两边各 {a.runs} 次自稳定。结果：{out}")
    (out / "report.json").write_text(json.dumps(rows, ensure_ascii=False, indent=1))
    if bad:
        sys.exit(1)


if __name__ == "__main__":
    main()
