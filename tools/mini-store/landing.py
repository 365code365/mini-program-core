"""H5 跳小程序的落地页：`/open?appid=wx…&path=pages/x/x&query=a%3D1`（或 `/a/<appid>`）。

对应微信的「URL Link / URL Scheme」：网页里放这个链接，手机浏览器打开后页面自动拉起
示例 App（URL Scheme `minirender://open?...`），App 再按 AppID 下载 / 更新并打开小程序。
自动拉起失败（没装 App、浏览器拦了）时页面上留一个按钮手动点。

Scheme 的形状仿 `weixin://dl/business/?appid=&path=&query=`：
    minirender://open?appid=<AppID>&path=<页面路径>&query=<urlencode 过的 a=1&b=2>
"""
import html
from urllib.parse import urlencode

from store_pkg import APPID_RE, find

SCHEME = 'minirender'
_COLORS = ['#07c160', '#10aeff', '#fa9d3b', '#e64340', '#6467f0', '#1485ee', '#91d300', '#576b95']


def scheme_url(appid, path='', query=''):
    q = {'appid': appid}
    if path:
        q['path'] = path
    if query:
        q['query'] = query
    return f'{SCHEME}://open?{urlencode(q)}'


def _color(app_id):
    # 与 admin.js、两个示例 App 同一个哈希：图标颜色三处一致
    h = 0
    for ch in app_id:
        h = (h * 31 + ord(ch)) & 0xFFFFFFFF
    return _COLORS[h % len(_COLORS)]


def render(appid, path='', query=''):
    """返回 (状态码, HTML)"""
    meta = find(appid) if APPID_RE.match(appid or '') else None
    if not meta:
        return 404, _page('小程序不存在', '<p class="err">没有找到这个小程序（AppID 不对，或已下架）。</p>')
    path = path.lstrip('/')[:200]
    query = query[:500]
    link = scheme_url(meta['appId'], path, query)
    e = html.escape
    body = f'''
<div class="icon" style="background:{_color(meta['id'])}">{e(meta['name'][:1] or '?')}</div>
<h1>{e(meta['name'])}</h1>
<p class="desc">{e(meta.get('description') or '')}</p>
<a id="open" class="btn" href="{e(link)}" data-href="{e(link)}">打开小程序</a>
<p class="hint">正在打开 mini-render 示例 App…<br>没反应的话点上面的按钮；还没装 App 先装示例 App。</p>
<p class="meta">AppID {e(meta['appId'])}{(' · 页面 ' + e(path)) if path else ''}</p>'''
    return 200, _page(f"打开「{meta['name']}」", body)


def _page(title, body):
    t = html.escape(title)
    return f'''<!doctype html>
<html lang="zh-CN"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{t}</title>
<style>
body{{margin:0;font:15px/1.6 -apple-system,BlinkMacSystemFont,"PingFang SC","Microsoft YaHei",sans-serif;background:#f7f7f7;color:#222;text-align:center}}
main{{max-width:420px;margin:0 auto;padding:72px 24px}}
.icon{{width:80px;height:80px;border-radius:50%;color:#fff;font-size:34px;font-weight:600;line-height:80px;margin:0 auto 16px}}
h1{{font-size:20px;margin:0 0 6px}}
.desc{{color:#888;margin:0 0 36px;min-height:1em}}
.btn{{display:block;background:#07c160;color:#fff;text-decoration:none;border-radius:8px;padding:12px;font-size:17px}}
.hint{{color:#999;font-size:13px;margin-top:20px}}
.meta{{color:#bbb;font-size:12px;margin-top:40px}}
.err{{color:#e64340}}
</style></head>
<body><main>{body}</main><script src="/open.js"></script></body></html>'''
