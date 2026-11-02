#!/usr/bin/env python3
"""示例小程序「功能缺口」静态盘点（临时工具，本轮完善示例用，用完删）。

  S  占位文案：即将开放 / 敬请期待 / 暂未开放 / 演示版暂不支持 …
  D  WXML 里绑定了、JS 里没有的处理函数
  N  跳转到 app.json 里没有的页面
  E  看起来能点、但自身与祖先都没有任何绑定的元素（按 class / 文案判断，有误报）
  T  处理函数只弹一个 toast / modal 就结束（可能是假功能）
  A  showActionSheet / showModal 没有 success 回调（选了等于没选）
  B  自定义 tabBar 的页面 onShow 里没有 getTabBar().setData({ selected })（微信里选中态会错）

用法：python3 target/tmp/zz_samples/audit.py [app ...]   不给就扫全部在范围内的示例
"""
import glob
import json
import os
import re
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', 'sample')
SKIP = {'_archive', 'target', 'tea-小程序页面', '一些问题', 'touch-probe', 'sample-app', 'news-app', 'tea-app', 'real-sample'}

STUB = re.compile(r"即将开放|即将上线|开发中|敬请期待|暂未开放|尚未开放|演示版暂|暂不支持|待开放|后续开放|正在建设|coming soon", re.I)
BIND = re.compile(r'\b(?:bind|catch|capture-bind|capture-catch|mut-bind):?([a-z]+)\s*=\s*"([^"]*)"')
TAG = re.compile(r'<(/?)([a-zA-Z][\w-]*)((?:[^>"\']|"[^"]*"|\'[^\']*\')*?)(/?)>', re.S)
CLS = re.compile(r'\bclass\s*=\s*"([^"]*)"')
LOOKS = re.compile(r'(?:^|[\s_-])(btn|button|more|link|entry|go|ops?|action|add|plus|minus|del|delete|edit|share|fav|like|collect|call|cta|chip|refresh|icon-btn|kefu|service)(?:[\s_-]|$)')
VERB = re.compile(r'^\s*(查看更多|更多|全部|查看全部|查看|去[^\s{}<]{0,4}|立即[^\s{}<]{0,4}|马上[^\s{}<]{0,4}|领取|领券|兑换|充值|提现|编辑|删除|修改|添加|新增|管理|设置|分享|收藏|关注|联系[^\s{}<]{0,3}|拨打[^\s{}<]{0,3}|导航|复制|取消订单|申请退款|退款|评价|再来一单|申请|预约|购买|支付|下单|提交|确认|切换|登录|退出登录|签到|续费|开通|升级|查看详情|详情\s*[>›]?|[>›»])\s*$')
INHERENT = {'button', 'navigator', 'picker', 'switch', 'checkbox', 'radio', 'slider', 'input', 'textarea', 'label', 'swiper', 'scroll-view', 'movable-view'}
VOID = {'image', 'input', 'icon', 'switch', 'checkbox', 'radio', 'slider', 'progress', 'import', 'include', 'wxs', 'textarea', 'video', 'canvas', 'map'}


def js_methods(src):
    names = set(re.findall(r'^\s{1,8}(?:async\s+)?([A-Za-z_$][\w$]*)\s*\([^)]*\)\s*\{', src, re.M))
    names |= set(re.findall(r'^\s{1,8}([A-Za-z_$][\w$]*)\s*:\s*(?:async\s*)?(?:function|\(|[A-Za-z_$][\w$]*\s*=>)', src, re.M))
    names |= set(re.findall(r'\bthis\.([A-Za-z_$][\w$]*)\s*=\s*(?:function|\()', src))
    return names


def app_cfg(app_dir):
    return json.load(open(os.path.join(app_dir, 'app.json'), encoding='utf-8'))


def pages_of(cfg):
    pages = set(cfg.get('pages', []))
    for sp in cfg.get('subpackages', cfg.get('subPackages', [])) or []:
        for q in sp.get('pages', []):
            pages.add(sp['root'].rstrip('/') + '/' + q)
    return pages


def dead_looking(wsrc):
    out = []
    stack = []  # (tag, bound, info, text)
    pos = 0
    for m in TAG.finditer(wsrc):
        close, tag, attrs, selfclose = m.group(1), m.group(2).lower(), m.group(3), m.group(4)
        text = wsrc[pos:m.start()]
        pos = m.end()
        if stack and text.strip() and not stack[-1][1] and stack[-1][3] is None:
            stack[-1] = stack[-1][:3] + (text.strip(),)
        if close:
            while stack:
                t, b, info, txt = stack.pop()
                if not b and info and (info[1] or (txt and VERB.match(txt))):
                    out.append(f"line {info[0]} <{t} class=\"{info[2]}\"> {txt or ''}".strip())
                if t == tag:
                    break
            continue
        bound = bool(BIND.search(attrs)) or tag in INHERENT or bool(stack and stack[-1][1])
        if tag == 'navigator' and 'url' not in attrs and 'target' not in attrs:
            bound = bool(stack and stack[-1][1])
        cm = CLS.search(attrs)
        cls_plain = re.sub(r'\{\{.*?\}\}', ' ', cm.group(1) if cm else '')
        looks = bool(LOOKS.search(cls_plain))
        line = wsrc.count('\n', 0, m.start()) + 1
        info = (line, looks, cls_plain.strip()[:40])
        if selfclose or tag in VOID:
            if not bound and looks and tag not in ('image', 'input', 'textarea'):
                out.append(f"line {line} <{tag} class=\"{info[2]}\">")
            continue
        stack.append((tag, bound, info, None))
    return out


def audit(app):
    p = os.path.join(ROOT, app)
    cfg = app_cfg(p)
    pages = pages_of(cfg)
    res = {k: [] for k in 'SDNETAB'}
    tab = cfg.get('tabBar') or {}
    if tab.get('custom') and os.path.isdir(os.path.join(p, 'custom-tab-bar')):
        for item in tab.get('list', []):
            js = os.path.join(p, item['pagePath'] + '.js')
            if os.path.exists(js) and 'getTabBar' not in open(js, encoding='utf-8').read():
                res['B'].append(item['pagePath'])
    for wx in sorted(glob.glob(p + '/**/*.wxml', recursive=True)):
        base = wx[:-5]
        rel = os.path.relpath(base, p)
        wsrc = open(wx, encoding='utf-8').read()
        js = base + '.js'
        jsrc = open(js, encoding='utf-8').read() if os.path.exists(js) else ''
        ms = js_methods(jsrc)
        for ev, h in BIND.findall(wsrc):
            if h and '{{' not in h and h not in ms:
                res['D'].append(f"{rel}: {h}")
        for i, l in enumerate(wsrc.split('\n')):
            if STUB.search(l):
                res['S'].append(f"{rel}.wxml:{i + 1}: {l.strip()[:100]}")
        for e in dead_looking(wsrc):
            res['E'].append(f"{rel}.wxml {e}")
    for js in sorted(glob.glob(p + '/**/*.js', recursive=True)):
        rel = os.path.relpath(js, p)
        jsrc = open(js, encoding='utf-8').read()
        for i, l in enumerate(jsrc.split('\n')):
            if STUB.search(l) and not l.strip().startswith('//'):
                res['S'].append(f"{rel}:{i + 1}: {l.strip()[:100]}")
        for m in re.finditer(r"url:\s*[`'\"](/?[^`'\"?$]+)", jsrc):
            u = m.group(1).lstrip('/')
            if u and u not in pages and not u.startswith('http') and not u.endswith('/'):
                res['N'].append(f"{rel}: {u}")
        for m in re.finditer(r'^(\s{2,4})([A-Za-z_]\w*)\s*\([^)]*\)\s*\{\s*\n((?:.*\n)*?)\1\}', jsrc, re.M):
            lines = [x.strip() for x in m.group(3).split('\n') if x.strip()]
            if lines and len(lines) <= 2 and all(re.match(r'(wx\.show(Toast|Modal)|return\b|const |let )', x) for x in lines):
                res['T'].append(f"{rel}: {m.group(2)}")
        for m in re.finditer(r'^(\s{2,4})([A-Za-z_]\w*)\s*\([^)]*\)\s*\{\s*(wx\.show(?:Toast|Modal)\(.*\));?\s*\}', jsrc, re.M):
            res['T'].append(f"{rel}: {m.group(2)}")
        for m in re.finditer(r'wx\.(showActionSheet|showModal)\(\{', jsrc):
            depth, j = 0, m.end() - 1
            while j < len(jsrc):
                if jsrc[j] == '{':
                    depth += 1
                elif jsrc[j] == '}':
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            body = jsrc[m.end() - 1:j + 1]
            if 'success' not in body and 'then' not in jsrc[j:j + 12] and 'complete' not in body:
                if m.group(1) == 'showActionSheet' or not re.search(r'showCancel\s*:\s*false', body):
                    line = jsrc.count('\n', 0, m.start()) + 1
                    flat = re.sub(r'\s+', ' ', body)[:80]
                    res['A'].append(f"{rel}:{line}: {m.group(1)} {flat}")
    return res


def main():
    apps = sys.argv[1:] or sorted(a for a in os.listdir(ROOT)
                                  if a not in SKIP and not a.startswith('zz') and os.path.exists(os.path.join(ROOT, a, 'app.json')))
    total = {k: 0 for k in 'SDNETAB'}
    for app in apps:
        r = audit(app)
        print(f"## {app}  " + ' '.join(f"{k}={len(v)}" for k, v in r.items()))
        for k in 'SDNABTE':
            for x in r[k]:
                print(f"   {k} {x}")
            total[k] += len(r[k])
    print('TOTAL', ' '.join(f"{k}={v}" for k, v in total.items()))


if __name__ == '__main__':
    main()
