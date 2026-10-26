"""mini-store 的搜索：关键词检索、分类、联想词、热搜（照微信「搜索小程序」那一套给示例 App 用）。

- 检索：关键词按空格切成几段，**每一段都要命中**（名称 / 标签 / 分类 / 开发者 / 描述，名称最重），
  再按命中分数 + 热度（被下载的次数）排序。名称还认「按顺序出现的字」（搜「饱外」能找到「饱了么外卖」），
  拼音首字母不做：要一张汉字拼音表，示例服务不值得。
- 热搜：有结果的搜索词按次数排；搜得少的时候拿热门小程序的标签补齐，不会空着。
- 搜索次数存在 `$DATA/searches.json`，一个词一个计数（只记有结果的、长度 ≤ 20 的，防止被灌垃圾）。
"""
import json
import math
import os
import threading
import time

from store_pkg import CATEGORIES, DATA, all_metas, public_meta, write_json

SEARCH_LOCK = threading.Lock()
MAX_TERMS = 2000


def _searches_path():
    return os.path.join(DATA, 'searches.json')


def _load_searches():
    try:
        with open(_searches_path(), encoding='utf-8') as f:
            return json.load(f)
    except (FileNotFoundError, ValueError):
        return {}


def _record(q):
    q = q.strip()
    if not q or len(q) > 20:
        return
    with SEARCH_LOCK:
        s = _load_searches()
        e = s.get(q) or {'n': 0}
        e['n'] += 1
        e['t'] = int(time.time())
        s[q] = e
        if len(s) > MAX_TERMS:
            # 留最常搜的那一半
            s = dict(sorted(s.items(), key=lambda kv: -kv[1]['n'])[:MAX_TERMS // 2])
        write_json(_searches_path(), s)


def _subseq(term, text):
    it = iter(text)
    return all(ch in it for ch in term)


def _term_score(term, m):
    t = term.lower()
    name = m.get('name', '').lower()
    tags = [x.lower() for x in m.get('tags') or []]
    if name == t:
        return 100
    if name.startswith(t):
        return 80
    if t in name:
        return 60
    if t in tags:
        return 50
    if any(t in x for x in tags):
        return 40
    if t == (m.get('category') or '').lower():
        return 35
    if t in (m.get('developer') or '').lower():
        return 25
    if t in (m.get('description') or '').lower():
        return 15
    if len(t) >= 2 and _subseq(t, name):
        return 10
    if t == (m.get('appId') or '').lower() or t == m.get('id', ''):
        return 90
    return 0


def search(q='', category='', limit=50, record=True, offset=0):
    """返回 (结果列表, 总数)。q 为空时按分类列出（热度排序）；offset 给管理页翻页用"""
    terms = [t for t in (q or '').split() if t][:6]
    out = []
    for m in all_metas():
        pm = public_meta(m)
        if category and pm.get('category') != category:
            continue
        score = 0
        for t in terms:
            s = _term_score(t, pm)
            if s == 0:
                score = -1
                break
            score += s
        if score < 0:
            continue
        hot = math.log1p(pm.get('downloads', 0)) * 2
        out.append((score + hot, pm))
    out.sort(key=lambda x: (-x[0], x[1].get('order', 1000), x[1]['id']))
    # 记进热搜的只是「像个词」的搜索：整段出现在头一个结果的名称 / 标签 / 分类里（「饱外」这种跳字匹配不记）
    # 翻到第二页以后不再记：同一次搜索翻几页不该把热搜次数刷几遍
    if record and offset == 0 and terms and out:
        top = out[0][1]
        hay = [top.get('name', '')] + list(top.get('tags') or []) + [top.get('category', '')]
        if all(any(t.lower() in h.lower() for h in hay) for t in terms):
            _record(' '.join(terms))
    offset = max(0, offset)
    return [pm for _, pm in out[offset:offset + limit]], len(out)


def categories():
    """每个分类有几个小程序（没有的不列），按固定顺序"""
    counts = {}
    for m in all_metas():
        c = m.get('category') or '其他'
        counts[c] = counts.get(c, 0) + 1
    return [{'name': c, 'count': counts[c]} for c in CATEGORIES if counts.get(c)]


def hot_keywords(limit=10):
    words = [w for w, _ in sorted(_load_searches().items(), key=lambda kv: (-kv[1]['n'], -kv[1].get('t', 0)))]
    seen = set(words[:limit])
    out = words[:limit]
    if len(out) < limit:
        # 冷启动：热门小程序的名字与第一个标签
        for pm in sorted((public_meta(m) for m in all_metas()), key=lambda p: -p.get('downloads', 0)):
            for w in [pm['name']] + (pm.get('tags') or [])[:1]:
                if w not in seen and len(out) < limit:
                    seen.add(w)
                    out.append(w)
    return out


def suggest(q, limit=8):
    """输入框联想：名称与标签里含这段字的词，名称优先、前缀优先"""
    t = (q or '').strip().lower()
    if not t:
        return []
    cands = []
    for m in all_metas():
        for w, base in [(m.get('name', ''), 0)] + [(x, 10) for x in m.get('tags') or []]:
            lw = w.lower()
            if t in lw:
                cands.append((base + (0 if lw.startswith(t) else 5) + len(w) / 100, w))
    seen, out = set(), []
    for _, w in sorted(cands):
        if w not in seen:
            seen.add(w)
            out.append(w)
    return out[:limit]
