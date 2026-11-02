#!/usr/bin/env python3
"""示例小程序 / 小游戏的上架前冒烟检查：静态检查 + 无头逐页跑一遍，有一条错就算不过。

    python3 tools/sample-smoke.py pet bike-share          # sample/<名> 或 sample-game/<名>
    python3 tools/sample-smoke.py --all                   # sample/ 下全部小程序 + sample-game/ 下带 store.meta.json 的游戏
    python3 tools/sample-smoke.py --keep pet              # 留下截图（target/_smoke/<名>/），默认跑完就删

静态检查（不跑引擎）：
  - app.json 的 pages / subpackages 每页都有 .wxml 与 .js；tabBar 的页在 pages 里、图标文件存在
  - usingComponents（app.json、页面、组件的 .json）都解析得到 .js + .wxml
  - wxml / wxss / js / json 里写死的本地资源路径（图片、音频、视频、字体）文件都在
  - 上架信息：store.meta.json（name / description / category / tags / developer）、
    project.config.json 的 appid（wx + 16 位十六进制，全仓唯一）、包根 icon.png（144×144）
  - 包大小（不含 store.meta.json）超过 2MB 记警告（微信主包上限 2MB）
运行检查（target/release/mini-app-window，CPU 后端、固定日期）：
  - 小程序：逐页快照，日志里出现 JS 异常、未实现的 API、页面 / 文件不存在、跳过的页面一律算错；
    截出来几乎是一种颜色的页面记警告（多半是没渲染出来）
  - 小游戏：--query auto=1 跑 180 帧，同样按日志判；最后一帧几乎纯色记警告
只删自己的 storage（target/mini-storage/<名>.json），不动别的会话的数据。只用标准库（有 Pillow 时才查纯色）。
"""
import json
import os
import re
import shutil
import struct
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.path.join(ROOT, 'target', 'release', 'mini-app-window')
OUT = os.path.join(ROOT, 'target', '_smoke')
RES_EXT = r'png|jpe?g|gif|svg|webp|mp3|wav|m4a|aac|ogg|mp4|ttf|otf|woff2?'
RES_RE = re.compile(r'''["'(]\s*((?:\.{1,2}/|/)?[\w\-./]+\.(?:%s))\s*["')]''' % RES_EXT, re.I)
# 日志里这些算错（行首符号是引擎的约定：❌ 逻辑层异常、⚠ 页面 / 文件问题）
BAD = re.compile(r'❌|JS Exception|未实现的 API|⚠.*不存在|跳过 pages|\[ERROR\]|panicked|Uncaught')
# 这些虽然带 ⚠ / 关键字，但不是包的问题
BENIGN = re.compile(r'创建选项没给 appId|--eval 执行失败')
APPID_RE = re.compile(r'^wx[0-9a-f]{16}$')


def find_dir(name):
    for base in ('sample', 'sample-game'):
        d = os.path.join(ROOT, base, name)
        if os.path.isfile(os.path.join(d, 'app.json')) or os.path.isfile(os.path.join(d, 'game.json')):
            return d
    if os.path.isdir(name):
        return os.path.abspath(name)
    sys.exit(f'找不到 {name}：sample/ 与 sample-game/ 下都没有')


def all_names():
    names = []
    for e in sorted(os.listdir(os.path.join(ROOT, 'sample'))):
        if e.startswith(('_', 'zz')):
            continue
        if os.path.isfile(os.path.join(ROOT, 'sample', e, 'app.json')):
            names.append(e)
    for e in sorted(os.listdir(os.path.join(ROOT, 'sample-game'))):
        if os.path.isfile(os.path.join(ROOT, 'sample-game', e, 'store.meta.json')):
            names.append(e)
    return names


def load_json(path, errs):
    try:
        with open(path, encoding='utf-8') as f:
            return json.load(f)
    except FileNotFoundError:
        errs.append(f'缺文件 {os.path.relpath(path, ROOT)}')
    except json.JSONDecodeError as e:
        errs.append(f'JSON 解析失败 {os.path.relpath(path, ROOT)}：{e}')
    return None


def png_size(path):
    with open(path, 'rb') as f:
        head = f.read(24)
    if head[:8] != b'\x89PNG\r\n\x1a\n':
        return None
    return struct.unpack('>II', head[16:24])


def all_appids():
    seen = {}
    for base in ('sample', 'sample-game'):
        for e in os.listdir(os.path.join(ROOT, base)):
            if e.startswith(('_', 'zz')):   # 归档 / 原版备份（zz-coffee-orig）本来就和正式版同 appid
                continue
            p = os.path.join(ROOT, base, e, 'project.config.json')
            if os.path.isfile(p):
                try:
                    a = json.load(open(p, encoding='utf-8')).get('appid')
                except Exception:
                    continue
                seen.setdefault(a, []).append(f'{base}/{e}')
    return seen


def check_meta(d, errs, warns, appids):
    meta = load_json(os.path.join(d, 'store.meta.json'), errs)
    if meta:
        for k in ('name', 'description', 'category', 'tags', 'developer'):
            if not meta.get(k):
                errs.append(f'store.meta.json 缺 {k}')
    pc = load_json(os.path.join(d, 'project.config.json'), errs)
    if pc:
        a = pc.get('appid', '')
        if not APPID_RE.match(a or ''):
            errs.append(f'project.config.json 的 appid「{a}」不是 wx + 16 位十六进制')
        elif len(appids.get(a, [])) > 1:
            errs.append(f'appid {a} 重复：{", ".join(appids[a])}')
    icon = os.path.join(d, 'icon.png')
    if not os.path.isfile(icon):
        errs.append('包根缺 icon.png（商店图标）')
    elif png_size(icon) != (144, 144):
        warns.append(f'icon.png 尺寸 {png_size(icon)}，商店图标约定 144×144')
    size = 0
    for base, dirs, files in os.walk(d):
        dirs[:] = [x for x in dirs if not x.startswith('.') and x != 'node_modules']
        for f in files:
            if f not in ('store.meta.json', '.DS_Store'):
                size += os.path.getsize(os.path.join(base, f))
    if size > 2 * 1024 * 1024:
        warns.append(f'包 {size / 1048576:.2f}MB，超过微信主包 2MB')
    return size


def resolve(d, from_dir, ref):
    ref = ref.split('?')[0].split('#')[0]
    if ref.startswith('/'):
        return os.path.join(d, ref.lstrip('/'))
    cand = os.path.normpath(os.path.join(from_dir, ref))
    if os.path.exists(cand):
        return cand
    return os.path.join(d, ref)   # 小程序里不带 / 的路径有时按包根解析（image 的 src 就是相对页面，这里两种都试）


def check_resources(d, errs):
    for base, dirs, files in os.walk(d):
        dirs[:] = [x for x in dirs if not x.startswith('.') and x != 'node_modules']
        for f in files:
            if not f.endswith(('.wxml', '.wxss', '.js', '.json')) or f in ('project.config.json', 'store.meta.json'):
                continue
            path = os.path.join(base, f)
            try:
                text = open(path, encoding='utf-8').read()
            except UnicodeDecodeError:
                continue
            if len(text) > 400_000:   # 打包好的引擎库（three.js 之类）里的字符串不是本包的资源
                continue
            for m in RES_RE.finditer(text):
                ref = m.group(1)
                # 不带 / 的裸文件名多半是运行时再拼目录（`IMG + 'latte.png'`），静态查不准，跳过
                if '/' not in ref or '//' in ref or ref.startswith(('http', 'data:', 'wxfile', 'cloud')):
                    continue
                if not os.path.exists(resolve(d, base, ref)):
                    errs.append(f'{os.path.relpath(path, d)} 引用的 {ref} 不存在')


def comp_path(d, json_dir, p):
    if p.startswith('plugin://'):
        return None
    full = os.path.join(d, p.lstrip('/')) if p.startswith('/') else os.path.normpath(os.path.join(json_dir, p))
    if os.path.isdir(full):
        full = os.path.join(full, 'index')
    return full


def check_components(d, json_path, errs, seen):
    if json_path in seen:
        return
    seen.add(json_path)
    j = load_json(json_path, errs) if os.path.isfile(json_path) else None
    if not j:
        return
    for tag, p in (j.get('usingComponents') or {}).items():
        full = comp_path(d, os.path.dirname(json_path), p)
        if full is None:
            continue
        for ext in ('.js', '.wxml'):
            if not os.path.isfile(full + ext):
                errs.append(f'{os.path.relpath(json_path, d)} 的组件 <{tag}> → {p}{ext} 不存在')
        check_components(d, full + '.json', errs, seen)


def static_miniprogram(d, errs):
    app = load_json(os.path.join(d, 'app.json'), errs)
    if not app:
        return []
    pages = list(app.get('pages') or [])
    for sp in (app.get('subpackages') or app.get('subPackages') or []):
        pages += [sp['root'].rstrip('/') + '/' + p for p in sp.get('pages', [])]
    if not pages:
        errs.append('app.json 没有 pages')
    for p in pages:
        for ext in ('.wxml', '.js'):
            if not os.path.isfile(os.path.join(d, p + ext)):
                errs.append(f'页面 {p}{ext} 不存在')
    tab = app.get('tabBar') or {}
    for it in tab.get('list', []):
        if it.get('pagePath') not in pages:
            errs.append(f'tabBar 的 {it.get("pagePath")} 不在 pages 里')
        for k in ('iconPath', 'selectedIconPath'):
            if it.get(k) and not os.path.isfile(os.path.join(d, it[k].lstrip('/'))):
                errs.append(f'tabBar 图标 {it[k]} 不存在')
    if tab.get('custom'):
        for ext in ('.js', '.wxml'):
            if not os.path.isfile(os.path.join(d, 'custom-tab-bar', 'index' + ext)):
                errs.append(f'tabBar.custom 但缺 custom-tab-bar/index{ext}')
    seen = set()
    check_components(d, os.path.join(d, 'app.json'), errs, seen)
    for p in pages:
        check_components(d, os.path.join(d, p + '.json'), errs, seen)
    return pages


def near_blank(png):
    try:
        from PIL import Image
    except ImportError:
        return False
    im = Image.open(png).convert('RGB').resize((64, 64))
    colors = im.getcolors(64 * 64) or []
    top = max(c for c, _ in colors) if colors else 0
    return top > 64 * 64 * 0.97


def run(cmd, env):
    p = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True, timeout=300)
    return p.returncode, (p.stdout or '') + (p.stderr or '')


def runtime(name, d, kind, errs, warns, keep):
    if not os.path.isfile(BIN):
        errs.append('没有 target/release/mini-app-window（先 cargo build --release）')
        return
    out = os.path.join(OUT, name)
    shutil.rmtree(out, ignore_errors=True)
    store = os.path.join(ROOT, 'target', 'mini-storage', os.path.basename(d) + '.json')
    if os.path.isfile(store):
        os.remove(store)
    env = dict(os.environ, MINI_FAKE_NOW=os.environ.get('MINI_FAKE_NOW', '2024-03-15'), MINI_BACKEND='cpu')
    if kind == 'game':
        code, log = run([BIN, d, '--query', 'auto=1', '--frames', '180', '--snapshot', out], env)
    else:
        code, log = run([BIN, d, '--snapshot', out, '--settle', '0.5', '--time', '2'], env)
    if code != 0:
        errs.append(f'引擎退出码 {code}')
    for line in log.splitlines():
        if BAD.search(line) and not BENIGN.search(line):
            errs.append('运行：' + line.strip()[:300])
    pngs = []
    for base, _, files in os.walk(out):
        pngs += [os.path.join(base, f) for f in files if f.endswith('.png') and not f.endswith('.native.png')]
    if not pngs:
        errs.append('一张截图都没出来')
    for png in sorted(pngs):
        if near_blank(png):
            warns.append(f'截图几乎是纯色：{os.path.relpath(png, ROOT)}')
    if os.path.isfile(store):
        os.remove(store)
    if not keep:
        shutil.rmtree(out, ignore_errors=True)
    return len(pngs)


def check(name, keep, appids):
    d = find_dir(name)
    kind = 'game' if os.path.isfile(os.path.join(d, 'game.json')) else 'app'
    errs, warns = [], []
    size = check_meta(d, errs, warns, appids)
    if kind == 'app':
        pages = static_miniprogram(d, errs)
    else:
        pages = []
        if not os.path.isfile(os.path.join(d, 'game.js')):
            errs.append('缺 game.js')
        load_json(os.path.join(d, 'game.json'), errs)
    check_resources(d, errs)
    shots = runtime(name, d, kind, errs, warns, keep)
    errs = list(dict.fromkeys(errs))
    tag = '✅' if not errs else '❌'
    shape = f'{len(pages)} 页' if kind == 'app' else '小游戏'
    print(f'{tag} {name:16} {shape:6} {size / 1024:7.0f}KB  截图 {shots or 0}  错 {len(errs)}  警告 {len(warns)}')
    for e in errs[:40]:
        print('    ✗ ' + e)
    for w in warns[:20]:
        print('    ! ' + w)
    return not errs


def main(argv):
    keep = '--keep' in argv
    names = [a for a in argv if not a.startswith('--')]
    if '--all' in argv:
        names = all_names()
    if not names:
        sys.exit(__doc__)
    appids = all_appids()
    bad = [n for n in names if not check(n, keep, appids)]
    print(f'\n{len(names) - len(bad)}/{len(names)} 通过' + (f'；不过：{" ".join(bad)}' if bad else ''))
    sys.exit(1 if bad else 0)


if __name__ == '__main__':
    main(sys.argv[1:])
