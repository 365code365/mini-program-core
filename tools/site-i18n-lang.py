#!/usr/bin/env python3
"""落地页的一种语言：校验译文 JSON，生成 `site/assets/i18n/<代码>.js`（切到这种语言时才加载）。

    python3 tools/site-i18n-lang.py <代码> <译文.json>     # 校验通过才写 js
    python3 tools/site-i18n-lang.py --check                 # 只校验 site/assets/i18n/ 下已有的全部语言
    python3 tools/site-i18n-lang.py --dump-en <输出.json>   # 导出英文原文（翻译的底稿）

英文写在 HTML 里、中文在 `assets/i18n.js` 的表里（见那个文件头）；其余语言一种一个文件，免得
首屏把十几种语言全下下来。校验的是「换进去之后版式不会坏」这几条：
  ① 键与英文一一对应（少一个就是那一段切过去还是英文）；
  ② 每个值里的标签（`<a href=…>`、`<code>`、`<br>`、`<span class="grad">`…）与英文是同一组（顺序可以变：
     日语、韩语的语序不同）；
  ③ 行内 `<code>` 里的内容原样（API 名、命令不能翻；多行代码块只许翻注释）；
  ④ 不能大段照抄英文（超过 30 个字符、与英文一字不差的值多于一成，多半是漏翻）。
"""
import json
import pathlib
import re
import sys
from collections import Counter

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from sitecheck import inner_html  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
HTML = ROOT / "site/index.html"
OUT = ROOT / "site/assets/i18n"
TAG = re.compile(r"<[^>]+>")
# 行内 <code>（不含换行）：API 名、类名、命令，不能翻。多行的代码块里注释可以翻
CODE = re.compile(r"<code>([^<\n]*?)</code>")


def english() -> dict:
    import html as H
    src = HTML.read_text(encoding="utf-8")
    en = {}
    for k in dict.fromkeys(re.findall(r'data-i18n="([^"]+)"', src)):
        en[k] = inner_html(src, k).strip()
    for attr, name in (("alt", "data-i18n-alt"), ("aria-label", "data-i18n-aria")):
        for m in re.finditer(r"<[^>]*\b" + name + r'="([^"]+)"[^>]*>', src):
            v = re.search(r"\b" + attr + r'="([^"]*)"', m.group(0))
            en[m.group(1)] = H.unescape(v.group(1)) if v else ""
    en["meta.title"] = re.search(r"<title>(.*?)</title>", src).group(1)
    en["meta.desc"] = H.unescape(re.search(r'<meta name="description" content="([^"]*)"', src).group(1))
    en.update({"ui.copy": "Copy", "ui.copied": "Copied", "ui.copyFail": "Copy failed", "ui.copyAria": "Copy this snippet"})
    return en


def validate(code: str, tr: dict, en: dict) -> list:
    problems = []
    missing = sorted(set(en) - set(tr))
    extra = sorted(set(tr) - set(en))
    if missing:
        problems.append(f"缺这些键：{missing[:20]}{' …' if len(missing) > 20 else ''}（共 {len(missing)} 个）")
    if extra:
        problems.append(f"多出这些键：{extra[:20]}")
    same = 0
    for k, src in en.items():
        v = tr.get(k)
        if v is None:
            continue
        if not isinstance(v, str) or not v.strip():
            problems.append(f"{k}：值为空")
            continue
        if Counter(TAG.findall(src)) != Counter(TAG.findall(v)):
            a, b = Counter(TAG.findall(src)), Counter(TAG.findall(v))
            problems.append(f"{k}：标签与英文不同 —— 少了 {list((a - b).elements())[:4]}，多了 {list((b - a).elements())[:4]}")
        if Counter(CODE.findall(src)) != Counter(CODE.findall(v)):
            problems.append(f"{k}：<code> 里的内容要原样保留（英文 {CODE.findall(src)[:4]}）")
        if len(src) > 30 and v.strip() == src.strip():
            same += 1
    long_keys = sum(1 for s in en.values() if len(s) > 30)
    if long_keys and same > long_keys * 0.1:
        problems.append(f"{same}/{long_keys} 段长文案与英文一字不差，像是没翻")
    return problems


def write_js(code: str, tr: dict, en: dict) -> pathlib.Path:
    OUT.mkdir(parents=True, exist_ok=True)
    ordered = {k: tr[k] for k in en}
    body = json.dumps(ordered, ensure_ascii=False, indent=1)
    p = OUT / f"{code}.js"
    p.write_text(
        "/* 落地页译文：" + code + "（tools/site-i18n-lang.py 生成，改译文请改这里再跑 --check） */\n"
        "(window.MR_I18N = window.MR_I18N || {})[" + json.dumps(code) + "] = " + body + ";\n",
        encoding="utf-8",
    )
    return p


def load_js(p: pathlib.Path) -> dict:
    s = p.read_text(encoding="utf-8")
    return json.loads(s[s.index("= {") + 2: s.rindex("}") + 1])


def main() -> int:
    a = sys.argv[1:]
    en = english()
    if a[:1] == ["--dump-en"]:
        pathlib.Path(a[1]).write_text(json.dumps(en, ensure_ascii=False, indent=1), encoding="utf-8")
        print(f"英文 {len(en)} 个键 → {a[1]}")
        return 0
    if a[:1] == ["--check"]:
        bad = 0
        files = sorted(OUT.glob("*.js"))
        for p in files:
            probs = validate(p.stem, load_js(p), en)
            if probs:
                bad += 1
                print(f"❌ {p.stem}：\n  - " + "\n  - ".join(probs[:15]))
        if not bad:
            print(f"✅ {len(files)} 种语言的译文与英文 {len(en)} 个键对齐，标签与 <code> 原样")
        return 1 if bad else 0
    if len(a) != 2:
        print(__doc__)
        return 2
    code, path = a
    tr = json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
    probs = validate(code, tr, en)
    if probs:
        print(f"❌ {code} 没写出去：\n  - " + "\n  - ".join(probs[:30]))
        return 1
    print(f"✅ {code}：{len(en)} 个键 → {write_js(code, tr, en).relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
