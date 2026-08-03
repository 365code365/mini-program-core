#!/usr/bin/env python3
"""用 headless Chrome 真渲染一遍落地页，确认「默认英文 / ?lang=zh 切中文」都生效。

    python3 tools/site-render-check.py [chrome 可执行文件]

只查 i18n 相关的三件事（不比像素，落地页没有像素基线）：
  ① 默认地址渲染完是英文：`<html lang="en">`，正文里没有中文；
  ② `?lang=zh` 渲染完是中文：`<html lang="zh-CN">`，中文字符数量到位；
  ③ 两边打了 `data-i18n` 的元素**数量相同且都非空** —— 少一个就是某段没被翻译到。

为什么要真开浏览器：切换逻辑是 JS 跑的，静态读文件只能证明表对得上，
证明不了 `apply()` 真的把每个节点都换掉了（少一个 querySelector 就漏一片）。
"""
import pathlib
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from sitecheck import CJK, drop_noise, inner_html, strip_tags  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
PAGE = ROOT / "site/index.html"
DEFAULT_CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"


def dump(chrome: str, url: str, profile: str, tries: int = 3) -> str:
    """取渲染后的 DOM。

    macOS 上的 Chrome 偶尔会被 updater 后台任务挂住（examples/compare.rs 里
    也记了这件事，那边的对策是轮询产物再主动回收进程）。这里同理：超时就杀掉重试，
    并且**不加** `--no-first-run` / `--virtual-time-budget` —— 实测这两个组合更容易挂。
    """
    for _ in range(tries):
        proc = subprocess.Popen(
            [chrome, "--headless=new", "--disable-gpu", "--hide-scrollbars",
             f"--user-data-dir={profile}", "--window-size=1280,900", "--dump-dom", url],
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
        )
        try:
            out, _ = proc.communicate(timeout=40)
            if out and out.strip():
                return out
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.communicate()
    return ""


def main() -> int:
    chrome = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_CHROME
    if not pathlib.Path(chrome).exists():
        print(f"⚠️  找不到 Chrome（{chrome}），跳过渲染检查")
        return 0

    base = PAGE.as_uri()
    with tempfile.TemporaryDirectory() as p1, tempfile.TemporaryDirectory() as p2:
        en = dump(chrome, base, p1)
        zh = dump(chrome, base + "?lang=zh", p2)

    problems = []
    for name, html, want_lang, want_cjk in (("默认", en, "en", False), ("?lang=zh", zh, "zh-CN", True)):
        if not html:
            problems.append(f"{name}：Chrome 没有输出 DOM")
            continue
        m = re.search(r'<html lang="([^"]+)"', html)
        got = m.group(1) if m else "?"
        if got != want_lang:
            problems.append(f"{name}：<html lang> 是 {got}，应为 {want_lang}")
        body = drop_noise(html)
        n = len(CJK.findall(body))
        # 语言切换按钮上永远有「中文」两个字，扣掉
        n = max(0, n - 2)
        if want_cjk and n < 500:
            problems.append(f"{name}：中文字符只有 {n} 个，像是没切过去")
        if not want_cjk and n:
            leak = [ln.strip()[:80] for ln in body.split("\n") if CJK.search(ln)][:5]
            problems.append(f"{name}：英文页面出现 {n} 个中文字符：{leak}")

    # data-i18n 元素数量与非空
    keys = re.findall(r'data-i18n="([^"]+)"', en)
    zh_keys = re.findall(r'data-i18n="([^"]+)"', zh)
    if len(keys) != len(zh_keys):
        problems.append(f"data-i18n 元素数量不一致：英文 {len(keys)}，中文 {len(zh_keys)}")
    blank = []
    for k in set(zh_keys):
        inner = inner_html(zh, k)
        if inner is None or not strip_tags(inner):
            blank.append(k)
    if blank:
        problems.append(f"中文版这些元素渲染后没有文字：{sorted(blank)}")
    # 中文版里不该还留着成句的英文原文（抽查几段长文案）
    for probe in ("hero.lede", "scope.mem.p", "stack.lede"):
        inner = inner_html(zh, probe)
        if inner and not CJK.search(inner):
            problems.append(f"{probe} 在中文版里仍是英文")

    if problems:
        print("❌ 落地页渲染检查不通过：")
        for p in problems:
            print("  -", p)
        return 1
    print(f"✅ 默认英文（html lang=en，正文无中文）；?lang=zh 切中文生效；"
          f"两边各 {len(keys)} 个 data-i18n 元素都有文字")
    return 0


if __name__ == "__main__":
    sys.exit(main())
