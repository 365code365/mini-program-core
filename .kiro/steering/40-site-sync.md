# 落地页跟着代码走：优化完要改 site 介绍

用户要求（2026-11-02）：**做完、优化完代码，都要更新 `site/` 的网站介绍**，包括里面写的「限制」。
落地页上的能力、数字、限制是对外的承诺，代码变了而页面没变，就是在页面上写了假话。

## 什么改动要改 site

凡是下面这些变了，同一轮里把 `site/index.html` 对应那段改掉（不另开一轮）：

- **能力**：新支持 / 新补齐的组件、API、宿主能力、平台（例：map / web-view / live-player 由 SDK 自带之后，
  `sky.g6` 的标签样式、`scope.yes.*`、`start.you.*`、`dev.feat.*` 都要跟）；
- **限制**：做掉了的限制从 `stack.todo.*` / `scope.no.*` 里改掉或换成新的真实缺口；新发现的限制（真机上播不了的格式、
  只在某个宿主上有的能力）写进去，别让页面比代码乐观；
- **数字**：性能优化的前后对比（`dev.m*`、`perf.*`、`mem.*` 的 `metric-num` 与说明）、测试总数（`stat.5`、`stack.reg.r1k`）、
  演示页数（`perf.real.r5k`）、组件 / API 计数（`sky.c*`，HTML 里那段注释写了口径）。数字要有出处（测试场景总表那一轮的记录、
  避坑清单），写清机型 / 场景，不写没测过的；
- 纯内部重构、修了一个用户看不见的 bug：不用改 site，但收尾时说一句「site 不受影响」。

## 怎么改（三处都要动，一处都不能漏）

1. **英文**写在 `site/index.html`（`data-i18n` 元素的内容就是英文原文）；
2. **简体中文**在 `site/assets/i18n.js` 的 `ZH` 表；
3. **其余 14 种语言**在 `site/assets/i18n/<代码>.js`（zh-tw ja ko de es fr it pt ru tr vi th id hi）：改了英文的键，
   每种语言都要重新翻，不许留旧译文（旧译文说的是旧事实）。术语沿用那种语言已有的译法（先看同一个文件里
   `sky.leg.same`、`sky.leg.ph` 这类老键怎么译的，例：日语「同一レイヤー描画」、德语「Same-Layer-Rendering」）。
4. 能改已有的键就别加新键：加一个键就是 16 份文案。版式、样式只改一份 HTML。

改完跑这两条，都过了才算完：

```bash
python3 tools/site-i18n-check.py     # 键对齐、标签与 <code> 原样、没漏翻、英文页面无中文
python3 tools/site-render-check.py   # headless Chrome：默认英文、?lang=zh、?lang=ja / de 懒加载
```

并在《测试场景总表》那一轮的记录里加一行「site」：改了哪几段、两条检查的结论。

## 别做的

- 别把页面写得比代码乐观：「Android 与 iOS 都有」之前，确认两边真机都验过；只在原生视图里有、Flutter 插件里还没有的，照实写。
- 别改成两个 HTML 文件、别把英文放进 JS 表（理由见 `site/assets/i18n.js` 文件头与 steering 30 的「改 site/」一段）。
