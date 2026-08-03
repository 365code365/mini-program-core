"""落地页两个检查脚本共用的小工具（site-i18n-check.py / site-render-check.py）。

放这里是为了别写两遍 `inner_html` —— 它是唯一有点技巧的地方，写两遍就会漂。
"""
import re
from typing import Optional

CJK = re.compile(r"[\u4e00-\u9fff]")


def inner_html(html: str, key: str) -> Optional[str]:
    """取 `data-i18n="key"` 那个元素的 innerHTML（含嵌套标签）。

    不能用正则一把梭：`(.*?)</\\w+>` 会在**第一个**闭合标签处截断，于是
    含两个 `<a>` 的段落只数到一个、内容以 `<code>` 开头的段落被当成空的。
    这里按标签名数层级。
    """
    at = html.find('data-i18n="%s"' % key)
    if at < 0:
        return None
    start = html.rfind("<", 0, at)
    m = re.match(r"<([a-z0-9]+)", html[start:])
    if not m:
        return None
    tag = m.group(1)
    body = html.index(">", at) + 1
    depth, pos = 1, body
    open_re = re.compile(r"<%s\b" % tag, re.I)
    close_re = re.compile(r"</%s\s*>" % tag, re.I)
    while depth:
        nxt_open = open_re.search(html, pos)
        nxt_close = close_re.search(html, pos)
        if not nxt_close:
            return None
        if nxt_open and nxt_open.start() < nxt_close.start():
            depth += 1
            pos = nxt_open.end()
        else:
            depth -= 1
            if depth == 0:
                return html[body:nxt_close.start()]
            pos = nxt_close.end()
    return None


def strip_tags(html: str) -> str:
    return re.sub(r"\s+", " ", re.sub(r"<[^>]+>", " ", html)).strip()


def drop_noise(html: str) -> str:
    """去掉注释与内联脚本/样式：那些不会渲染给读者看，不该参与「有没有残留中文」的判断。"""
    for pat in (r"<!--.*?-->", r"<script\b[^>]*>.*?</script>", r"<style\b[^>]*>.*?</style>"):
        html = re.sub(pat, lambda m: "\n" * m.group(0).count("\n"), html, flags=re.S)
    return html
