#!/usr/bin/env python3
"""Khronos WebGL 一致性测试子集（game-doc/04 A8.1）在小游戏引擎里跑（无头，ANGLE + SwiftShader）。

  python3 tools/webgl-cts/run.py [--cts <WebGL 仓库 sdk/tests>] [--only 子串] [--frames N]

测试页列表在 tools/webgl-cts/list.txt（相对 sdk/tests/conformance）。每页：抽出 <script>（外链的读进来、内联的原样）、
<canvas id>、着色器脚本，拼成一个 game.js（shim.js 在最前），用 mini-app-window 无头跑 N 帧，收 T:CTS done 那一行。
一页的结论：done 且 fail = 0 → PASS；done 但有 fail → FAIL（列出失败子项）；没跑完 → TIMEOUT / ERROR。
结果写 target/_cts/report.md 与 report.json。CTS 仓库默认在 /tmp/cts/WebGL（tools/webgl-cts/fetch.sh 拉）。
"""
import html.parser, json, os, re, shutil, subprocess, sys, argparse

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


class Page(html.parser.HTMLParser):
    def __init__(self):
        super().__init__()
        self.scripts, self.dom, self.cur, self.onload = [], {}, None, None

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == 'script':
            # 老页面用 language="x-shader/x-vertex" 而不是 type
            self.cur = {'src': a.get('src'), 'type': a.get('type') or a.get('language') or '', 'id': a.get('id'), 'text': ''}
        elif tag == 'body' and a.get('onload'):
            self.onload = a['onload']
        elif tag == 'canvas' and a.get('id'):
            self.dom[a['id']] = {'tag': 'canvas', 'width': int(a.get('width', 300) or 300), 'height': int(a.get('height', 150) or 150)}

    def handle_data(self, data):
        if self.cur is not None:
            self.cur['text'] += data

    def handle_endtag(self, tag):
        if tag == 'script' and self.cur is not None:
            s, self.cur = self.cur, None
            if s['type'].startswith('x-shader') or s['type'].startswith('text/something') or ('shader' in s['type']):
                if s['id']:
                    self.dom[s['id']] = {'tag': 'script', 'type': s['type'], 'text': s['text']}
            elif s['id'] and s['type'] and 'javascript' not in s['type']:
                self.dom[s['id']] = {'tag': 'script', 'type': s['type'], 'text': s['text']}
            else:
                self.scripts.append(s)


def build(cts, rel, out):
    path = os.path.join(cts, 'conformance', rel)
    p = Page()
    p.feed(open(path, encoding='utf-8').read())
    base = os.path.dirname(path)
    parts, srcs = [], []
    for s in p.scripts:
        if s['src']:
            f = os.path.normpath(os.path.join(base, s['src']))
            srcs.append(s['src'])
            parts.append((s['src'], open(f, encoding='utf-8').read()))
            if s['src'].endswith('js-test-pre.js'):
                parts.append(('hook', ''))
        else:
            parts.append(('inline', s['text']))
    shim = open(os.path.join(ROOT, 'tools/webgl-cts/shim.js'), encoding='utf-8').read()
    post = open(os.path.join(cts, 'js/js-test-post.js'), encoding='utf-8').read()
    js = ['var __CTS_DOM = %s;' % json.dumps(p.dom), 'var __CTS_POST = %s;' % json.dumps(post),
          'var __CTS_SCRIPTS = %s;' % json.dumps(srcs), 'var __CTS_PATH = %s;' % json.dumps('/conformance/' + rel), shim]
    for name, src in parts:
        # 页面里的 <script> 是经典脚本：顶层 var / function 是全局的。间接 eval 一段带 "use strict" 的源码时它们反而只在 eval 里，
        # 后面的脚本（js-test-post.js 读 successfullyParsed）就看不到 —— 去掉开头的指令
        src = re.sub(r'^[ \t]*(["\'])use strict\1;?[ \t]*$', '', src, flags=re.M)
        # 同理：经典脚本之间共享顶层的 let / const（js-test-pre.js 的 const RESULTS），间接 eval 里它们只在那一段 —— 第 0 列的改成 var
        src = re.sub(r'^(const|let) ', 'var ', src, flags=re.M)
        if name == 'hook':
            js.append('__ctsHook();')
            continue
        js.append('try { (0, eval)(%s); } catch (e) { console.log("T:CTS error " + %s + " " + e + " " + (e && e.stack || "")); }' % (json.dumps(src), json.dumps(name)))
    # <body onload> 与 window.onload：所有脚本跑完之后调（浏览器在 load 事件里调）
    js.append('setTimeout(function () { try { %s; if (typeof window.onload === "function") window.onload(); __ctsLoad.forEach(function (f) { f(); }); } catch (e) { console.log("T:CTS error onload " + e + " " + (e && e.stack || "")); } }, 0);' % (p.onload or ''))
    if os.path.isdir(out):
        shutil.rmtree(out)
    os.makedirs(out)
    open(os.path.join(out, 'game.json'), 'w').write('{ "deviceOrientation": "portrait" }')
    open(os.path.join(out, 'game.js'), 'w', encoding='utf-8').write('\n'.join(js))
    # 着色器文件、资源按测试页的相对路径找：sdk/tests 里的非 .js 文件硬链接进包里的 cts/（shim.js 的同步 XHR 读它）。
    # .js 不放：引擎会把包里的 .js 全部注册成模块（严格模式下 pnglib.js 这类直接语法错误）
    for d, _, fs in os.walk(cts):
        for f in fs:
            if f.endswith('.js') or f.endswith('.html'):
                continue
            src = os.path.join(d, f)
            dst = os.path.join(out, 'cts', os.path.relpath(src, cts))
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            try:
                os.link(src, dst)
            except OSError:
                shutil.copy(src, dst)
    return out


def run(cts, rel, frames):
    out = os.path.join(ROOT, 'target/_cts', re.sub(r'[^A-Za-z0-9_.-]', '_', rel))
    build(cts, rel, out)
    try:
        r = subprocess.run([os.path.join(ROOT, 'target/release/mini-app-window'), out, '--frames', str(frames)], capture_output=True, text=True, timeout=180,
                           env=dict(os.environ, MINI_WEBGL='swiftshader', MINI_GAME_LOG_ALL='1'))
        log = r.stdout + r.stderr
    except subprocess.TimeoutExpired as e:
        log = (e.stdout or b'').decode() if isinstance(e.stdout, bytes) else (e.stdout or '')
        return {'test': rel, 'status': 'TIMEOUT', 'fails': [], 'log': log[-2000:]}
    m = re.search(r'T:CTS done (\{.*?\})', log)
    fails = re.findall(r'FAIL[^\n]*', log)
    errs = re.findall(r'T:CTS error [^\n]*', log)
    if not m:
        return {'test': rel, 'status': 'ERROR' if errs else 'TIMEOUT', 'fails': (errs + fails)[:8], 'log': log[-1500:]}
    res = json.loads(m.group(1))
    st = 'PASS' if res['fail'] == 0 and res['pass'] > 0 and not errs else 'FAIL'
    return {'test': rel, 'status': st, 'pass': res['pass'], 'fail': res['fail'], 'fails': (errs + fails)[:8]}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--cts', default='/tmp/cts/WebGL/sdk/tests')
    ap.add_argument('--only', default='')
    ap.add_argument('--frames', type=int, default=900)
    a = ap.parse_args()
    # 以 ! 开头的是「不适用于小游戏」的页（理由写在行尾），不跑、不进分母，报告里列出来
    raw = [l.strip() for l in open(os.path.join(ROOT, 'tools/webgl-cts/list.txt')) if l.strip() and not l.startswith('#')]
    na = [l[1:].split('#')[0].strip() + '：' + (l.split('#', 1)[1].strip() if '#' in l else '') for l in raw if l.startswith('!')]
    tests = [l for l in raw if not l.startswith('!')]
    tests = [t for t in tests if a.only in t]
    results = []
    for t in tests:
        r = run(a.cts, t, a.frames)
        results.append(r)
        print('%-8s %s %s' % (r['status'], t, '' if r['status'] == 'PASS' else (r.get('fails') or [''])[0][:160]), flush=True)
    n = len(results)
    ok = sum(1 for r in results if r['status'] == 'PASS')
    sub_p = sum(r.get('pass', 0) for r in results)
    sub_f = sum(r.get('fail', 0) for r in results)
    os.makedirs(os.path.join(ROOT, 'target/_cts'), exist_ok=True)
    json.dump(results, open(os.path.join(ROOT, 'target/_cts/report.json'), 'w'), ensure_ascii=False, indent=1)
    with open(os.path.join(ROOT, 'target/_cts/report.md'), 'w') as f:
        f.write('# WebGL CTS 子集：%d/%d 页通过（%.1f%%），子项 %d 过 / %d 失败\n\n' % (ok, n, 100.0 * ok / max(n, 1), sub_p, sub_f))
        if na:
            f.write('不适用、不计入（%d 页）：\n\n' % len(na) + ''.join('- %s\n' % x for x in na) + '\n未通过：\n\n')
        for r in results:
            if r['status'] != 'PASS':
                f.write('- %s %s：%s\n' % (r['status'], r['test'], '；'.join(x[:200] for x in r.get('fails', [])[:3])))
    print('页通过 %d/%d（%.1f%%），子项 %d 过 / %d 失败' % (ok, n, 100.0 * ok / max(n, 1), sub_p, sub_f))


if __name__ == '__main__':
    main()
