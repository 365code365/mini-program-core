# 做示例时发现的引擎问题（未改引擎，示例里已绕开）

1. **`display: grid` 不排版**：子项按块级元素纵向堆叠，`grid-template-columns/rows`、`grid-column`、`grid-row: a / span n` 都不生效
   （样式表里与内联 style 都一样）。复现：`target/_gp` 一类最小页（`.g{display:grid;grid-template-columns:100px 100px}` 下 4 个子块）。
   `doc/实现组件说明.md` 写的是支持 grid。campus 课表改用绝对定位。
2. **带自定义 tabBar 的小程序里 WXS 调用返回空**：同一个 `<wxs module="fmt" src="../../utils/fmt.wxs"/>` + `{{fmt.level(95)}}`，
   app.json 去掉 `tabBar`（custom: true）就正常显示「优秀」，加回来就是空串；页面是否是 tab 页都一样，`src` 外链与页内 `<wxs>` 都一样。
   campus 的模板里写 `{{fmt.level(x) || item.lv}}`，JS 侧给同一规则的兜底字段。
3. **`wx.showModal({ editable: true })` 没有输入框**（`src/js/prelude/ui.js` 注释里写明未实现）：`res.content` 拿不到。
   insure 的推荐码在拿不到 `content` 时改用页内输入弹层。
4. `--route` 不接受 query（`pages/chat/chat?id=c2` 会被当成页面名跳过）——只是无头工具的限制，走查改用 `--eval 'wx.navigateTo(...)'`。
5. **同一块级 view 里相邻的多个 `<text>` 不做行内排列**，各占一行（`前<text>中</text>后` 排成三行）。绕开：容器 `display:flex; flex-direction:row`。
6. **app.json 的全局 `usingComponents` 不生效**（`src/host/page_loader.rs` 只读页面自己的 json）。绕开：每个页面 json 里各自声明。
7. 桌面端 `wx.chooseImage` 回 `fail cancel`，页面分不清「用户取消」与「没有相册」：示例里常驻「使用示例图」入口。
8. **`view` 里直接写的长文字不换行**（溢出屏幕），包一层 `<text>` 才换行。绕开：长文字一律包 `<text>`。
9. **模板表达式里对字符串取下标 / 调方法返回空**：`{{s[0]}}`、`{{s.charAt(0)}}`、`{{s.slice(0,1)}}`。绕开：JS 里算好字段。
10. **`rich-text` 不分段、不加粗**：`<p>a</p><p>b</p>`、`<br/>`、nodes 数组都排成一行，`<b>/<strong>` 不加粗；自身带 `display:block` 时被压成一字一行。
11. `editor` 的 `format('bold')` 只回 statuschange 不改正文（`src/builtin_components/editor.js` 注明的取舍）；`undo/redo` 不触发 `bindinput`。
12. **`<text>` 的 `decode` / `space` 没实现**：`&nbsp;`、`&gt;` 原样显示。
13. **`wx.navigateTo` 不给被盖住的页面派 `onHide`**（返回时又派 `onShow`），tab 页的定时器被盖住后继续跑。
14. **flex 子项里的百分比 margin 按祖先宽度算**（宽度百分比是对的）：`flex:3` 子项 225px 里 `margin-right:2%` 算成 6px。
15. 单指触摸：`movable-view` 的 `bindscale`、`scale-gesture-handler` 拿不到双指缩放（引擎注释写明）。
16. 无头工具：`--scroll` 不按动作顺序执行（启动即生效）；给了 `--time` 后 `wx.createAnimation` 过渡停在起点。
17. 桌面端 `<camera>` 只画占位且不触发 `binderror`；`map` / `live-player` 只画「宿主未接入」占位（设计如此）。
18. **`wx.getBackgroundAudioManager()` 每次调用都抛 `TypeError: property is not configurable`**：`src/js/prelude/audio.js` 里 `__makeAudio` 用 defineProperty 定义的 `src` 没写 `configurable: true`，背景音频管理器再 defineProperty('src') 时就抛。scenic / meditate 用 try/catch 降级到 InnerAudioContext。
