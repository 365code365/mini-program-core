#!/usr/bin/env python3
"""落地页中英双语的一致性检查。

    python3 tools/site-i18n-check.py

页面本身是英文（`site/index.html`），中文在 `site/assets/i18n.js` 的表里，切换时替换
innerHTML。这么做的前提是**键必须两边对齐** —— 少一个键，那一段切到中文时会**静默留在
英文**，页面照样能打开，没人会发现。这个脚本就是来堵这个的：

  ① HTML 里每个 data-i18n / data-i18n-alt / data-i18n-aria 的键，表里都要有；
  ② 表里每个键都要有人用（除了几个给 JS 用的 ui.* / meta.*）；
  ③ 英文页面里不该残留中文（除了语言切换按钮上那个「中文」字样）；
  ④ 键不许重复定义（JS 对象字面量里重复键会静默覆盖）。
"""
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from sitecheck import CJK, drop_noise, inner_html  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
HTML = ROOT / "site/index.html"
I18N = ROOT / "site/assets/i18n.js"

# 只给 JS 用、不出现在 HTML 属性里的键
JS_ONLY = {
    "ui.copy", "ui.copied", "ui.copyFail", "ui.copyAria",
    "meta.title", "meta.desc",
}
def main() -> int:
    html = HTML.read_text(encoding="utf-8")
    js = I18N.read_text(encoding="utf-8")

    used = set()
    for attr in ("data-i18n", "data-i18n-alt", "data-i18n-aria"):
        used |= set(re.findall(attr + r'="([^"]+)"', html))

    # 表里的键：只取 ZH 对象里 '键': 开头的行
    zh_block = js[js.index("var ZH = {"): js.index("var DICT = {")]
    keys = re.findall(r"^\s*'([^']+)':", zh_block, re.M)
    defined = set(keys)

    problems = []

    dupes = sorted({k for k in keys if keys.count(k) > 1})
    if dupes:
        problems.append(f"表里有重复键（后一个会静默覆盖前一个）：{dupes}")

    missing = sorted(used - defined)
    if missing:
        problems.append(f"HTML 用到但表里没有的键（切中文会留在英文）：{missing}")

    orphan = sorted(defined - used - JS_ONLY)
    if orphan:
        problems.append(f"表里定义了却没人用的键：{orphan}")

    # 英文页面里的中文残留：按行报，方便定位。
    # 注释与内联 <script> 不算 —— 仓库里的注释本来就是中文，那些不会渲染给读者看。
    visible = drop_noise(html)
    leaks = []
    for i, line in enumerate(visible.split("\n"), 1):
        if CJK.search(line) and 'data-lang="zh"' not in line:
            leaks.append(f"{i}: {line.strip()[:90]}")
    if leaks:
        problems.append("英文页面里残留中文：\n    " + "\n    ".join(leaks))

    # 两种语言的链接与换行数量要一致：少一个 <a> 就是某个语言下链接消失了，
    # 少一个 <br> 就是标题断行位置不同（视觉上很明显）。
    zh_map = dict(re.findall(r"^\s*'([^']+)': '(.*)',?$", zh_block, re.M))
    shape = []
    for key in sorted(used):
        if key not in zh_map:
            continue
        inner = inner_html(html, key)
        if inner is None:
            continue
        for tag in ("<a ", "<br"):
            if inner.count(tag) != zh_map[key].count(tag):
                shape.append(f"{key}: 英文 {inner.count(tag)} 个 {tag.strip()}，"
                             f"中文 {zh_map[key].count(tag)} 个")
    if shape:
        problems.append("两种语言的结构对不上：\n    " + "\n    ".join(shape))

    if problems:
        print("❌ 落地页 i18n 检查不通过：")
        for p in problems:
            print("  -", p)
        return 1

    print(f"✅ 落地页 i18n 一致：{len(used)} 个键在 HTML 里用到，"
          f"表里 {len(defined)} 个（含 {len(JS_ONLY)} 个给 JS 用），无残留中文")
    return 0


if __name__ == "__main__":
    sys.exit(main())
