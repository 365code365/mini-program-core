#!/usr/bin/env python3
"""decodeAudioData 的 Chrome 对拍（game-doc/05 GT-AU5）。
    python3 tools/game-audio-decode-compare.py [--out 目录] [--chrome 路径] [--thresholds 文件.json]
- 素材：ffmpeg 现生成 48k 立体声 1 秒（左 440Hz、右 1320Hz，幅度 0.5）的 wav / mp3（lame 192k）/ m4a（aac 192k）/ ogg（vorbis q5），
  放进夹具 g-webaudio-decode 的 assets/（不进仓库）；
- 引擎：mini-app-window 跑夹具（MINI_GAME_LOG_ALL=1：样本分好几百行打出来，不能被每帧 500 行的限流丢掉），
  从 stdout 的 `T:EV dec / pcm` 取解码结果；
- Chrome：同一份 game.js，wx.createWebAudioContext → OfflineAudioContext(2, 48000, 48000)（decodeAudioData 重采样到上下文采样率），
  文件内容以 base64 放进 HTML，console.log 收进 DOM，`--dump-dom` 读回。
报告：每个格式的采样数 / 声道 / 采样率（必须相同）与样本的最大绝对误差、误差 RMS；
另给「最佳对齐」：在 ±2048 帧里找使误差 RMS 最小的偏移（编码器延迟 / 填充处理不同会表现为偏移）。
`--thresholds`：`{"<文件名>": {"max": …}}`，给了就判定（元数据不同一律失败）。
"""
import argparse
import base64
import json
import re
import shutil
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
FIXTURE = "g-webaudio-decode"
SRC = "aevalsrc='0.5*sin(2*PI*440*t)|0.5*sin(2*PI*1320*t)':s=48000:d=1"
FORMATS = {"t.wav": ["-c:a", "pcm_s16le"], "t.mp3": ["-c:a", "libmp3lame", "-b:a", "192k"],
           "t.m4a": ["-c:a", "aac", "-b:a", "192k"], "t.ogg": ["-c:a", "libvorbis", "-q:a", "5"]}

SHIM = r"""
(function () {
  var cfg = window.__CFG, lines = [];
  console.log = function (s) { lines.push(String(s)); if (/^T:DONE/.test(s)) document.getElementById('out').textContent = 'LOG:' + JSON.stringify(lines); };
  wx.createWebAudioContext = function () { return new OfflineAudioContext(2, 48000, 48000); };
  wx.arrayBufferToBase64 = function (buf) {
    var u = new Uint8Array(buf), s = '', CH = 0x8000;
    for (var k = 0; k < u.length; k += CH) s += String.fromCharCode.apply(null, u.subarray(k, k + CH));
    return btoa(s);
  };
  wx.getFileSystemManager = function () {
    return { readFileSync: function (p) {
      var b = cfg.files[p.replace(/^assets\//, '')]; if (!b) throw new Error('no file ' + p);
      var s = atob(b), u = new Uint8Array(s.length); for (var i = 0; i < s.length; i++) u[i] = s.charCodeAt(i); return u.buffer;
    } };
  };
  window.__start = function () { window.__MODS_REQ('game.js'); };
})();
"""


def make_assets(pkg: Path):
    a = pkg / "assets"
    a.mkdir(parents=True, exist_ok=True)
    for name, args in FORMATS.items():
        subprocess.run(["ffmpeg", "-y", "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", SRC, *args, str(a / name)], check=True)


def parse(lines):
    out, parts = {}, {}
    for l in lines:
        m = re.match(r"T:EV (dec|pcm) (.*)$", l.strip())
        if not m:
            continue
        o = json.loads(m.group(2))
        if m.group(1) == "pcm":
            parts.setdefault(o["name"], {})[o["i"]] = o["s"]
        else:
            out[o["name"]] = o
    for n, o in out.items():
        if "parts" in o:
            p = parts.get(n, {})
            o["pcm"] = "".join(p[i] for i in range(o["parts"])) if len(p) == o["parts"] else ""
    return out


def run_engine(pkg: Path, out: Path):
    r = subprocess.run([str(ROOT / "target/release/mini-app-window"), str(pkg), "--frames", "30", "--hz", "60", "--snapshot", str(out / "snap")],
                       capture_output=True, text=True, timeout=180, env=dict(__import__("os").environ, MINI_GAME_LOG_ALL="1"))
    res = parse(r.stdout.splitlines())
    if len(res) != len(FORMATS):
        sys.exit(f"引擎只交回 {sorted(res)}：\n{r.stdout[-1500:]}\n{r.stderr[-1500:]}")
    return res


def run_chrome(chrome: str, pkg: Path, out: Path):
    mods = []
    for p in sorted(pkg.rglob("*.js")):
        rel = p.relative_to(pkg).as_posix()
        mods.append(f'{json.dumps(rel)}: {{ fn: function (module, exports, require) {{\n{p.read_text(encoding="utf-8")}\n}} }}')
    files = {n: base64.b64encode((pkg / "assets" / n).read_bytes()).decode() for n in FORMATS}
    cfg = {"frames": 0, "hz": 60, "fonts": {}, "query": {}, "files": files}
    shim = (ROOT / "tools/game-ref/shim.js").read_text(encoding="utf-8")
    shim = shim.replace("window.__start = function () { req('')('game.js');", "window.__MODS_REQ = req(''); window.__start = function () { req('')('game.js');")
    html = out / "ref.html"
    html.write_text(f"""<!doctype html><html><head><meta charset="utf-8"></head><body><pre id="out">PENDING</pre>
<script>window.__CFG = {json.dumps(cfg)}; window.__MODS = {{ {','.join(mods)} }};</script>
<script>{shim}</script><script>{SHIM}</script>
<script>try {{ window.__start(); }} catch (e) {{ document.getElementById('out').textContent = 'ERROR ' + e; }}</script></body></html>""", encoding="utf-8")
    for k in range(3):
        # 每次一个新的配置目录：上一个 Chrome 进程还没退干净时，同一个目录会被它占着，新进程直接把请求转给它就退出
        prof = out / f"profile{k}"
        shutil.rmtree(prof, ignore_errors=True)
        r = subprocess.run([chrome, "--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check", "--disable-extensions",
                            "--disable-component-update", "--disable-background-networking",
                            "--virtual-time-budget=20000", f"--user-data-dir={prof}", "--dump-dom", html.as_uri()],
                           capture_output=True, text=True, timeout=120)
        m = re.search(r'<pre id="out">(.*?)</pre>', r.stdout, re.S)
        if m and m.group(1).startswith("LOG:"):
            return parse(json.loads(m.group(1)[4:].replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", '"')))
        if m and m.group(1).startswith("ERROR"):
            sys.exit(f"Chrome 里夹具抛错：{m.group(1)[:1500]}")
    sys.exit("Chrome 没交回结果（3 次）")


def pcm(o):
    raw = base64.b64decode(o["pcm"])
    v = struct.unpack(f"<{len(raw) // 4}f", raw)
    n = o["len"]
    return [v[c * n:(c + 1) * n] for c in range(o["ch"])]


def errs(a, b, off):
    worst, se, n = 0.0, 0.0, 0
    for ca, cb in zip(a, b):
        for i in range(max(0, -off), min(len(cb), len(ca) - off)):
            d = abs(ca[i + off] - cb[i])
            worst = max(worst, d)
            se += d * d
            n += 1
    return worst, (se / max(n, 1)) ** 0.5


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=None)
    ap.add_argument("--chrome", default=DEFAULT_CHROME)
    ap.add_argument("--thresholds", default=None)
    a = ap.parse_args()
    out = Path(a.out or ROOT / "target/_game_audio_compare" / FIXTURE).resolve()
    if out.exists():
        shutil.rmtree(out)
    pkg = out / "pkg" / FIXTURE
    shutil.copytree(ROOT / "sample-game" / FIXTURE, pkg)
    shutil.copytree(ROOT / "sample-game" / "_lib", pkg / "_lib")
    make_assets(pkg)
    eng, ref = run_engine(pkg, out), run_chrome(a.chrome, pkg, out)
    th = json.loads(Path(a.thresholds).read_text()) if a.thresholds else None
    bad, report = [], {}
    for name in FORMATS:
        e, r = eng[name], ref[name]
        if "err" in e or "err" in r:
            print(f"{name}: 引擎 {e.get('err', 'ok')} / Chrome {r.get('err', 'ok')}")
            bad.append(name)
            continue
        meta = (e["ch"], e["len"], e["rate"]) == (r["ch"], r["len"], r["rate"])
        pa, pb = pcm(e), pcm(r)
        mx, rms = errs(pa, pb, 0)
        # 粗找最佳偏移：先 16 帧一步，再在附近逐帧
        coarse = min(range(-2048, 2049, 16), key=lambda o: errs([c[:12000] for c in pa], [c[:12000] for c in pb], o)[1])
        best = min(range(coarse - 16, coarse + 17), key=lambda o: errs(pa, pb, o)[1])
        bmx, brms = errs(pa, pb, best)
        report[name] = {"engine": [e["ch"], e["len"], e["rate"]], "chrome": [r["ch"], r["len"], r["rate"]], "max": mx, "rms": rms,
                        "best_offset": best, "best_max": bmx, "best_rms": brms}
        lim = th.get(name, {}).get("max") if th else None
        ok = meta and (lim is None or mx <= lim)
        if not ok:
            bad.append(name)
        print(f"{name:6} 引擎 ch={e['ch']} len={e['len']} rate={e['rate']:.0f} | Chrome ch={r['ch']} len={r['len']} rate={r['rate']:.0f} | "
              f"最大误差 {mx:.3e} RMS {rms:.3e} | 最佳偏移 {best:+d} 帧：最大 {bmx:.3e} RMS {brms:.3e}" + ("" if ok else "  ✗"))
    (out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))
    if th and bad:
        sys.exit(1)


if __name__ == "__main__":
    main()
