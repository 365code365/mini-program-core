# 组件对齐：缺口补齐计划与测试计划（2026-11-03）

来源：同日的组件审计（官方 82 个标签逐页对照，见会话里的「微信组件缺口审计」）。
对标：微信 Skyline（steering 10）。官方文档 `https://developers.weixin.qq.com/miniprogram/dev/component/<组件>.html`
（属性表抽不全时用同名 `.md`）。

## 总规矩（每个工作包都适用）

1. **性能第一**：任何改动不许让滚动 / 动画帧退回整帧重画；绘制热路径（每个节点都走的函数）先判再分配；
   新增的每帧逻辑要说清每帧代价。动了上屏、帧调度、失效分析、滚动、绘制热路径的，最后统一上真机按顿挫验（steering 30）。
2. **先对齐语义再写代码**：每一项先读官方文档那一条（默认值、取值、事件 detail 的字段与**类型**），写进单测断言。
   官方没写清的行为写「待验」，不猜。
3. **不改冻结名单里的小程序**（steering 00）；逐字节基线（gallery / damage / sdk-parity）以 CPU 后端为准，
   有像素变化的必须是「修正」并能说出是哪一项。
4. **单文件 ≤500 行**（steering 20），新文件按职责放；改了超 500 行的文件优先顺手拆，但**拆分与改行为分两次**。
5. **测试只跑定向的**：`cargo test --release --lib <过滤词>` / `--test <文件>`。全量回归、真机、site 检查最后统一跑（见末尾测试计划）。
   新用例优先放进 `src/tests/` 或已有的 e2e 文件，**别新建 `tests/*.rs`**（每个 75MB 二进制）。
6. **并行开发**：几个工作包同时在同一工作区改。只用 `str_replace` 改已有文件（不整文件覆盖）；不 `git checkout/stash/reset`；
   编译报的错如果在别人的文件里，等一两分钟重试，不去改别人的文件；**不跑** `regress.sh`、`clean-target.sh`、真机脚本、`cargo clean`；
   不 `export CARGO_*`。共享文件（`registry.rs`、`app-info.js` 的 canIUse 表、`native_placeholder.rs`、`builtin_components/mod.rs`）只做最小插入。
7. 诊断开关、取舍写进代码注释与 README；新组件登记进 `wx.canIUse` 表（并且只在真的实现了才登记）。

---

## 工作包

### WP-A 文本、富文本、列表构造器（渲染器）
- A1 text：`-webkit-line-clamp`（配 `display:-webkit-box; -webkit-box-orient:vertical; overflow:hidden`）、Skyline `max-lines` + `overflow`（clip / fade / ellipsis / visible）。
- A2 text：`decode`（`&nbsp; &lt; &gt; &amp; &apos; &ensp; &emsp;`）、`space`（ensp / emsp / nbsp）。
- A3 rich-text：`nodes` 的 HTML 字符串与节点数组都转成真正的子树（块级 p/div/h1-h6/ul/ol/li/blockquote/hr/pre/table 系列，行内 span/b/strong/i/em/u/s/del/ins/sub/sup/a/code/small/big，`<br>`，`<img>`），
  属性 `style` 全量走样式系统、`class` 匹配页面 WXSS（与微信一致）、HTML 实体解码、官方白名单之外的标签连子树移除、`space` 属性；内部节点不派事件。
  UA 默认样式（h1~h6 字号与粗细、p 的上下 margin、ul/ol 缩进与项目符号）按 WebKit / 微信默认值。
- A4 list-builder / grid-builder：按 `list` + `slot:item` / `slot:index` 展开子模板；list-builder 竖排（type=static 用 child-height），grid-builder 复用 grid-view 布局（type / cross-axis-count / max-cross-axis-extent / gap / padding）；
  `bind:itembuild` / `bind:itemdispose` 按「进出视口裁剪窗」派发（detail `{index}`）。list-view 的 `padding` 属性。
- A5 cover-view 默认 `white-space:nowrap; line-height:1.2`；span 的行内流（文本可以接在图片后面折行）如果做不到要写明近似。

### WP-B 表单与输入
- B1 input / textarea：`focus` / `auto-focus`（挂载与属性变 true 时聚焦并拉起键盘，变 false 失焦）、`cursor`、`selection-start/end`、`cursor-color`、`hold-keyboard`、
  按「完成」失焦也发 `blur`、input 的 `bindchange`（失焦时内容变了才发）、focus 的 `detail.height`（键盘高，宿主报来前为 0）、bindinput 的 `cursor` / `keyCode` 真值、
  textarea 的 bindinput 返回值**不**替换内容、blur 带 `cursor`、`placeholder-style` 的 font-size / font-weight、textarea 默认 placeholder-class 为 `textarea-placeholder`。
- B2 textarea：`auto-height`（随行数长高，`min-height` / `max-height` 生效）、`bindlinechange {height, heightRpx, lineCount}`、`disable-default-padding`。
- B3 picker：time 的 `start` / `end`；date / time 默认值按**本地时区**；region 换成完整的省市区数据（带区划码），change 带 `code`（与 `postcode`，拿不到写明），`custom-item`、`level`（province / city / region，sub-district 写明不支持或给数据）。
- B4 picker-view：`indicator-style` 的 border / background-color、`indicator-class`、`mask-class`、`mask-style`、列里的项高度自动设成选中框高度。
- B5 radio：互斥只在同一个 radio-group 内；点已选中的项不再发 change。checkbox-group / radio-group / label `for` 不依赖「本帧视口里的交互元素表」（长列表滚出去也算）。label 包着的 submit 按钮能提交。
- B6 button：`loading`（转圈，动画只重画按钮那一块）、`disabled` 的按钮不派发 bindtap（与微信一致），form-type=reset 也复位 picker 与 checkbox/switch 的初始值。
- B7 icon `size` 与 progress `stroke-width` / `font-size` / `border-radius` 认 px / rpx；progress 默认直角、`color` 属性、`active` / `active-mode` / `duration` / `bindactiveend`。

### WP-C 逻辑层、内置组件、事件链
- C1 `worklet:` 前缀：组件属性（`worklet:ongesture`、`worklet:should-response-on-move`、`worklet:should-accept-gesture`、`worklet:onsizeupdate`）与事件
  （scroll-view / swiper 的 `worklet:onscrollstart` / `onscrollupdate` / `onscrollend`）都接上；worklet 回调的参数是事件对象本身（字段在顶层），在逻辑线程同步调。
- C2 手势组件：POSSIBLE(0) / CANCELLED(4) 状态、抬手速度（最近 100ms 的速度）、translation 起点 bug、`should-accept-gesture` 返回 false 发 CANCELLED、
  `simultaneous-handlers` / `tag` 的最小仲裁（内层识别后外层不再识别，除非互相列在 simultaneous-handlers 里）、`native-view` 代理 scroll-view 时内部列表照常滚、
  去掉多出来的包装层（虚组件不参与布局）。
- C3 draggable-sheet：吸附与 `scrollTo` 的动画（animated / duration / easingFunction）、面板里的 scroll-view 能滚（列表到顶再下拉才拖面板，面板没到最大时上推先拖面板）、
  `select('.x').node()` 不带 id 也能拿到 DraggableSheetContext。
- C4 button `open-type` 全部取值与回调：share（调 onShareAppMessage + 宿主事件）、getPhoneNumber / getRealtimePhoneNumber / chooseAvatar / getUserInfo / openSetting /
  contact / feedback / launchApp / agreePrivacyAuthorization / liveActivity，走已有的 hostApi / 宿主事件；宿主没接时按官方 errMsg 回 fail。配套属性（session-from、app-parameter、lang …）随宿主请求带上。
- C5 navigator：`target="miniProgram"` 时 open-type 正确分流（navigate → navigateToMiniProgram、navigateBack → navigateBackMiniProgram、exit → exitMiniProgram），`bind:success / fail / complete`。
- C6 image 的 `bindload` / `binderror` 在 SDK 宿主（MiniEngine）上派发（现在只有桌面窗体派发）。
- C7 page-container：侧滑返回先关容器；`close-on-slide-down` 跟手；一页一个。
- C8 开放能力：ad / ad-custom / official-account / official-account-publish / store-* / functional-page-navigator 挂上后派 `binderror`（或 fail）；open-data 按官方现状显示「微信用户」与灰头像（default-avatar / default-text）；reward / store-home 空白。
- C9 snapshot.takeSnapshot：把子树离屏画成 PNG（file / arraybuffer，rgba / png）。open-container 的 closed / open 外观（颜色、阴影、圆角）。share-element 的跨页飞跃（navigateTo / navigateBack 时同 key 元素从旧位置插值到新位置）。
- C10 navigation-bar 颜色动画参数下传；page-meta 的 `root-font-size` / `page-font-size` / `page-style` / `bind:resize` / `bind:scroll`；audio binderror 的数字码；movable-view 的 friction ≤0 回默认值。

### WP-D 滚动、吸顶、轮播
- D1 **`position: sticky`**（页面滚动与 scroll-view 内都要）+ sticky-section（push-pinned-header、padding）+ sticky-header（offset-top、allow-overlapping、padding、`bindstickontopchange {isStickOnTop}`）。
  **硬要求**：吸顶元素随滚动只重画自己那一块（像 fixed 覆盖层或动画包围盒那样），不能每帧整条带重画；damage-check 逐字节一致。
- D2 scroll-view：`bounces`、`show-scrollbar`（画滚动条，只重画滚动条那一条）、`paging-enabled`、`fast-deceleration`、`min-drag-distance`、`clip=false`、`scroll-anchoring`、`reverse`（内容增长时贴底）、
  scroll-x + scroll-y 同时（双向）、`dragend` 的 velocity、`enable-back-to-top` 的自动开启、`refresherstatuschange` 与下拉二级（two-level 七个属性）、`type="nested"` + nested-scroll-header / body（里外层衔接、offset-top）。
- D3 ScrollViewContext：scrollEnabled / bounces / showScrollbar / pagingEnabled / fastDeceleration / decelerationDisabled 可写且生效，scrollTo 双轴，scrollIntoView 的 alignment，triggerRefresh / closeRefresh / triggerTwoLevel / closeTwoLevel。
- D4 swiper：swiper-item `item-id` → change / animationfinish 的 `currentItemId`；`clip=false`；`snap-to-edge`。

### WP-E 媒体与画布
- E1 canvas：drawImage 9 参数源裁剪、3 / 5 参数按原图尺寸；`createImage` 真加载（width / height、onerror）；没写尺寸时 300×150；roundRect、isPointInPath / isPointInStroke、fill('evenodd')、
  getTransform 跟踪全部变换、fillText maxWidth、strokeText 描边、缺的合成模式；旧接口 `draw(reserve, cb)` 语义（false 清屏、true 不重放、回调在画完之后）与属性式写法；bind:error；touch 事件的画布内 x/y；disable-scroll。
- E2 image：SVG（解码成位图）、`fade-in`、`show-menu-by-longpress`（长按弹宿主菜单：保存 / 识别 / 转发，宿主没接就弹 actionSheet）、`forceHttps`、cloud:// 走 fail。
- E3 video：子节点（盖在视频上的 cover-view 控件）渲染、不再强制 4px 圆角、同层播放时弹幕由引擎画在视频上、`enable-play-gesture`（双击）、`enable-progress-gesture`（横滑）、`auto-pause-if-navigate`、`bindprogress`。
- E4 live-player：`orientation` / `mode` 透传、statechange 码表按现行文档。

### WP-F 宿主侧原生组件（Kotlin / Swift）
- F1 map：controls（+ controltap）、customCallout（slot="callout"）、show-location、enable-zoom / enable-scroll 分开、marker title、callout textAlign / anchorX/Y、wxfile:// 图标、polyline colorList / level、polygon zIndex、
  rgba() 颜色、poitap / polylinetap、setting 整组覆盖、MapContext.openMapApp。Android 原生与 iOS 原生同步（Flutter Android 复用 Android 那份）。
- F2（下一轮）camera、live-pusher、voip-room、Flutter iOS 的 map / web-view / 同层视频：宿主侧整块开发，本轮不做，写进计划。

### WP-Z 收尾（最后做）
- Z1 新示例小程序 `sample/wx-components`（组件大全）：每个补齐的组件 / 属性一页或一节，可交互、可肉眼核对；过 `tools/sample-smoke.py`。
- Z2 site：能力 / 限制 / 数字（16 种语言，steering 40）；README 的组件清单与诊断开关；`doc/测试场景总表.md` 第五节加一轮。

---

## 测试计划（开发期只跑定向；全部改完统一跑一次）

### 开发期（每个工作包自己）
- 每一项至少一个可判定的断言：属性 → 像素或布局数值；事件 → `e.detail` 的字段与**类型**；时序 → 事件顺序。
- 跑法：`cargo test --release --lib <过滤词>`，或现有 e2e 文件 `--test wx_compat` / `--test <已有文件> <过滤词>`。

### 统一测试（全部合入之后，按顺序）
1. 构建：`cargo build --release --all-targets`（一次）。
2. 全量回归后台跑：`bash tools/regress-async.sh 组件对齐`（test / damage / gallery / tabclick / snapshots / sdk-parity / interaction / H5）。
   gallery 有变化的逐张列出原因（只允许是本轮的修正）；damage 必须全一致；H5 不回退（sample 4.61% / news 6.60%）。
3. 回归跑着时：新示例 `python3 tools/sample-smoke.py wx-components`，桌面无头逐页快照肉眼核对。
4. 真机（一加 PLY110，串行：引擎库 → flutter build apk → 装包；原生示例 gradle）：
   - 顿挫基线（改前先量一份）：`index-slow`、`wxss-slow`、`perf-idle`、静心播放页 `store-idle`，改后同场景对比，**不许回退**；
   - 新功能：新示例的吸顶列表页、长 rich-text 页、auto-height 输入页各量一次 `*-slow`（屏上重复率、>8ms 帧）；
   - 原生组件：动了 map 的跑 `bash tools/android-native-components.sh`（原生与 Flutter 各一次）；
   - 输入：真机上 focus / auto-focus 拉起键盘、hold-keyboard、auto-height（截图看）。
5. site：`python3 tools/site-i18n-check.py`、`python3 tools/site-render-check.py`。
6. 收尾：`bash tools/clean-target.sh`；`doc/测试场景总表.md` 记一轮，每项 ✅ / ❌（附数字）/ 没跑（附原因）。

## 进度（各工作包收尾时在这里写一行：做了什么、没做什么及原因、加了哪些测试）

### WP-A（文本、富文本、列表构造器）· 2026-11-03

- **A1 ✅** `display:-webkit-box; -webkit-line-clamp:N`（写在 text 自己或它的父级上；不是 -webkit-box 时不生效，同浏览器）、Skyline `max-lines` + `overflow`（clip / fade / ellipsis / visible，默认 visible）。
  盒高 = min(行数, N) 行（度量叶子 `TextMeasure::max_lines`、定宽路径、reflow 补高三处都截），绘制第 N 行按 overflow 收尾（ellipsis 同 Chrome：本行行尾让出「...」，不拉下一行的字）。
  父级 clamp 下的**裸文本**换成按容器宽折行的度量叶子（`text_attrs::clamped_bare_leaf`）。🟡 近似：fade 按字设透明度（Flutter 是横向渐变）；visible 的语义（盒子 N 行、字照画到外面）待验；混排文字（嵌套 text / span）只裁到 N 行高、没有省略号。
- **A2 ✅** text `decode`（官方七个实体；`&quot;` / 数字实体不在表里，原样显示，待验）、`space`（ensp U+2002 / emsp U+2003 / nbsp U+00A0，不合并）。
- **A3 ✅（🟡 几处近似）** rich-text 的 `nodes`（数组 / 单个对象 / HTML 字符串）在模板展开时转成真子树（`parser/template/builder.rs` → `components/rich_text/{html,tree,ua}.rs`）：块级 → `rt-<标签>`（按 view 建，WebKit 默认样式，em 按字号换算）；只有文字 / `<br>` 的段落是**一个** `<text>`，带行内标签 / 图片的段落是一个 `<span>`（拆 run）；`<img>` → `<image>`（宽高属性折成样式，没高 `widthFix`、没宽 `max-width:100%`）；`<li>` 挂项目符号（ul • ◦ ▪、ol 认 start / type）；`<pre>` 保留空白；白名单外连子树丢；class 匹配页面 / 组件 WXSS（打组件归属）、style 全量；实体解码、HTML 空白合并、`space`；内部节点无事件（点击归 rich-text 自己）。
  HTML 分词器自写：未闭合、无引号 / 单引号属性、注释、DOCTYPE、script / style 连内容丢、`<p>` / `<li>` / `<td>` 隐式结束、深度上限 128。`nodes="{{expr}}"` 直接求 JSON（不再经单引号插值，文字里带撇号也不坏）；`nodes` 原串仍留在节点上给 H5 编译用。
  🟡 外边距不折叠（第二个起的块不落 margin-top，p→h2 这种会窄一点）；i / em 不斜（引擎无 font-style）；sub / sup 只缩小不偏移；表格按弹性盒（tr 横排、格子等分，无 colspan，Skyline 本身也不支持表格布局）；Skyline 的 `mode` 属性没接（一律按 default）。
- **A4 🟡** list-builder / grid-builder：按 `list`（+ `child-count`）展开 `slot:item` / `slot:index` 模板（不写值默认 item / index，多根模板记 `__per_item`）；list-builder 竖排、`padding="[上,右,下,左]"`、type=static 时没写高的项按 `child-height` 定高；grid-builder 复用 grid-view（aligned / masonry、列数、两个 gap、padding）；list-view 的 `padding` 属性。
  `bind:itembuild` / `bind:itemdispose`（detail `{index}`，number）按「页面视口 ∩ 一路 scroll-view 盒子（含 `cache-extent`）」的可见窗派发，离开的先 dispose、进来的再 build（`host/builder_events.rs`，挂在 `observers::tick`，SDK 与桌面都走）。
  每帧代价：构造器路径每次布局找一遍，之后每帧沿路径走 + 在项里二分（瀑布流逐项，O(项数)）。**没做**：按需建树 / 回收（项全部建树布局，与 wx:for 同代价）、`initial-child-count`；页面销毁时剩下的项发不发 itemdispose 待验（现在不发）。canIUse 表加了 list-builder、grid-builder。
- **A5 ✅ / 🟡** cover-view 默认 `white-space:nowrap; line-height:1.2`（nowrap 经 `InheritedText::white_space` 只交给 cover-view 的直接子文本；裸文本本来就不按容器宽折行）。
  `<span>` 登记成行内混排（`components/inline_flow.rs`）：文字接在图片后面、到行尾折行，行内标签可多层，`<br>` / `\n` 真换行；🟡 行尾空格不吃、run 不带事件（图片 run 保留）、带底色的一段不能中间折行；里面有 navigator / view 这类组件时退回普通容器（横排换行，每个子节点一个盒子）。

**测试**（`src/tests/text_richtext_tests.rs` 16 个，已登记进《测试场景总表》第二节）：
`a1_webkit_line_clamp_夹住父级里的文字为两行`、`a1_text_自己的_line_clamp_与_max_lines`、`a1_max_lines_overflow_clip_不画第三行_visible_照画`、`a2_decode_解码官方实体_不写不解`、`a2_space_emsp_比普通空格宽`、
`a3_html_字符串_块级与_ua_样式`、`a3_class_匹配页面样式_style_全量生效`、`a3_白名单外连子树移除_图片变成_image`、`a3_内部节点不派事件_点击归_rich_text_自己`、`a3_列表项目符号与_space`、`a3_长文一万字_建树布局耗时`、
`a4_list_builder_按_list_展开_定高与_padding`、`a4_grid_builder_复用_grid_view_布局`、`a4_itembuild_itemdispose_detail_index_是_number`、`a5_cover_view_默认不换行_行高_1_2`、`a5_span_文字接在图片后面折行`；
另有单测 `rich_text::html::{容错_未闭合_无引号_注释_脚本, 隐式结束_p_与_li}`、`rich_text::tree::{白名单外连子树丢_段落收拢, 序号}`、`text_attrs::解码官方七个实体_其余原样`；`registry` 的标签数 32 → 36（+span / list-builder / list-view / grid-builder）。

**编译环境说明**：本轮工作区一直编不过（WP-E 的 `canvas/commands.rs` 引用还没写出来的 `image_src` / `round_rect` / `fill_text_max`，另一个工作包改到一半的 `host/engine_input.rs` 借用错误），
所以定向测试与 gallery 是在 `target/wpa_copy`（同步工作区源码，只把这几个编不过的文件换回 HEAD、清空写到一半的 `src/tests/wpc/*`）里跑的：
定向 16/16 + 相关 60/60；整个 lib 1079/1079 通过（同一副本）。合入后请在工作区重跑 `cargo test --release --lib text_richtext`。

**gallery**（同一副本跑 `cargo run --release --example gallery`，对 HEAD）：变了 2 张，都不是本包：`51_health_dashboard.png`（0.311%，包围盒 x[255,704] y[1082,1334]，进度条那块）、
`60_form_controls.png`（高 1682 → 1700）—— 两个场景都没用 line-clamp / rich-text / span / cover-view / 构造器（grep 过 `examples/gallery*`），用的是 progress / icon / button（WP-B 在改）。其余 63 张逐字节不变（35_gif 这次也没变）。

**性能**（桌面 release，375×667@2x，`a3_长文一万字_建树布局耗时` 连跑 3 次）：rich-text 长文 13190 字符（300 段，每 3 段一段带 `<b>`）首次渲染（展开 + 建树 + 布局 + 画一屏）21.5~27.0ms，
渲染节点 3701（纯文字段落一段一个 text 节点，混排段落逐字一个 run）；setData 换一份再排 6.5~6.9ms（一次 17.3ms，机器换页中）；其中 HTML 解析 + 转子树 0.56~0.61ms。
热路径：text 建树多 3 次属性查找、不分配；样式落地每节点多一次 `rt-` 前缀比较；没有构造器的页面每帧只多一次线程局部比较。

**没动 / 留给别人**：H5 编译那条路的 rich-text 仍按原 `nodes` 出 HTML（浏览器自己排）；site 与 README（按计划归 WP-Z：要写的能力是 line-clamp / max-lines、decode / space、rich-text 子树、span 混排、list-builder / grid-builder，限制见上面各条 🟡）；
`text.rs`（646 行）、`property.rs`（527 行）原本就超 500 行，本轮只往里加了几行、新逻辑放进 `text_attrs.rs` / `inline_flow.rs`，拆分留给单独一次纯搬迁（别的工作包同时在改 property.rs）。

### WP-C（2026-11-05，接续上一轮断网的半成品）
- **C1 worklet: 前缀 ✅**：组件属性 `worklet:ongesture`→`onGestureEvent`、`worklet:should-*`、`worklet:onsizeupdate` 走 `parser::template::worklet_prop`；scroll-view / swiper 的 `worklet:onscrollstart|update|end` 登记成 `worklet:*` 绑定，由 `host::own_events` 在组件事件之后同步调，参数是事件对象本身（detail 字段放顶层，`evt.detail` 也给）。swiper 没有开始 / 结束事件：第一次 transition 补 scrollstart，animationfinish 当 scrollend（detail 用最后一次 `{dx, dy}`）。没写 worklet:on* 的组件每个事件只多一次绑定表扫描、不分配。
- **C2 手势组件 ✅（scale 单指近似）**：虚组件（模板根不进树，绑定并进唯一子节点，`gesture_merge.rs`；处理函数 `__gh:<uid>:<方法>` 由 `page-components.js` 路由）；POSSIBLE(0) / CANCELLED(4)；tap 恒为 state 1（官方原话），double-tap 沿用 3（官方没写，待验）；速度按最近 100ms；平移起点 bug 修了；should-accept-gesture 在认出那一刻问、false → CANCELLED；竞技场（`globalThis.__ghArena`）从内到外推进、drag 先于 pan（官方 vertical-drag 包 pan 的例子），`simultaneous-handlers` 任一方列了对方 `tag` 就并行；`native-view` 不锁滚动、这串触摸不补 touchcancel，逻辑层可拦（`host::gesture_native`，只在这种触摸里每下 move 多一次 eval）。拖动锁滚动改成 `catch:__lockscroll`（只锁滚动、不截使用方的 touchmove 冒泡）。没做：多指（scale / rotation 恒 1 / 0）、force-press；横向 drag 不锁纵向页面滚动。
- **C3 draggable-sheet ✅（一次拖动不换手）**：吸附与 `scrollTo({animated=true, duration=300, easingFunction='ease'})` 走 `animation` 通道（渲染层插值 translateY，只重画面板）；面板里的 scroll-view：宿主仲裁（`host::gesture` `SheetHit`）——面板没到最大时上推、列表在顶时下拉归面板，其余列表滚（接管时面板退回）；`select('.x').node()` 不带 id 也拿到 Context（宿主带组件实例键）；`worklet:onsizeupdate {pixels, size}`。没做：面板拖到最大后同一次拖动里接着滚列表。
- **C4 button open-type ✅**：点击（bindtap 之后）按 `__opentype` 标记绑定找按钮（disabled 不做），`open-type.js`：share（onShareAppMessage({from:'button', target}) → 宿主事件 `{type:'share'}`，支持 promise 3s）、getPhoneNumber / getRealtimePhoneNumber / chooseAvatar / contact / launchApp（失败走 binderror）/ liveActivity 走 hostCall `button.<open-type>`（配套属性随请求带上）、getUserInfo / openSetting 复用 wx 的宿主能力、feedback 发 `openFeedback`、agreePrivacyAuthorization 直接同意。宿主没接按既有的 `<名>:fail 当前宿主未实现…` 回（官方真机上的失败原文随环境，待验）。
- **C5 navigator ✅**：`target="miniProgram"` 时 navigate → navigateToMiniProgram、navigateBack → navigateBackMiniProgram（带 extra-data）、exit → 退出；`bind:success / fail / complete` 派给 navigator 自己。非 miniProgram 的 `open-type="exit"` 不再退出（官方：只在 target="miniProgram" 时生效）。
- **C6 image 事件在 SDK ✅**：`engine_frame.rs` 每次重画之后取 `take_image_events()` 派发（`own_events::dispatch_image_events`），load 的 detail 是原始像素尺寸（数字）。
- **C7 page-container ✅**：左边缘右滑（≥60px，官方没给数，待验）先关容器、不滑页面（`engine_edge_back.rs`，只在 SDK；桌面窗体的侧滑没接）；close-on-slide-down 跟手（只往下）、不够 1/5 屏高弹回、够了从当前位置离场；一页一个（第二个 show 不出来并告警）。
- **C8 开放能力 ✅**：ad / ad-custom → `binderror {errCode:1004}`、不占高；official-account(-publish) → `{status:5, errMsg}`（errMsg 原文待验）；store-product / coupon / gift 点了 `entererror {code:-1, message}`（code 待验），store-home 空白；functional-page-navigator 点了 `bind:fail`；open-data：昵称「微信用户」、头像灰块（#e6e6e6，没画白色人形）、其余为空显示 default-text 并发 error。都登记进 canIUse。没做：「reward」——官方组件表里没找到这个标签。
- **C9 🟡**：snapshot.takeSnapshot ✅（`host::snapshot` + `wxml_renderer/snapshot.rs` 静态路径离屏画子树：file / arraybuffer × png / rgba，另带 width/height；不支持 Promise 与官方一致；边长上限 8192）。open-container 🟡：只做 closed 外观（closed-color / -border-radius / -elevation→box-shadow 近似），从节点放大的路由转场没做（route_anim 只有整页推入）。share-element 🟡：做了与 page-container 配合的那条（容器弹出时从页面上同 key 元素飞过去、关时飞回，animation 通道），跨页面路由飞跃（Skyline navigateTo）没做——要宿主转场层画飞跃物；飞跃没扣掉容器自己的入场位移（用 position="center" 才准）。
- **C10 🟡**：navigation-bar / setNavigationBarColor 的 `animation {duration, timingFunc}` 下传到宿主事件 ✅；page-meta `bind:resize`、`bind:scroll`（绑了才给页面包 onPageScroll）、`bind:scrolldone` ✅；`root-font-size` / `page-font-size` / `page-style` ❌（要渲染层改页面根样式与 rem 基准，引擎 rem 固定 16px，留给渲染器）；audio binderror 的 errMsg 是数字码 ✅（InnerAudioContext errCode 10002→2、10004→3、其余→4，映射待验）；movable-view friction / damping ≤0 回默认 2 / 20 ✅。
- **测试**（`src/tests/wpc/`，22 个）：gesture.rs（虚组件 flex、pan 状态序列与速度、嵌套 vertical-drag 胜 pan、should-accept false / tap state 1、simultaneous、native-view、worklet scroll-view、worklet_prop、面板仲裁单测）、events.rs（SDK image 事件、sheet scrollTo/node/onsizeupdate、sheet 里的列表、navbar 动画参数 + movable 兜底、audio 数字码）、open_type.rs（fail / hostCall、share、getUserInfo + 隐私、navigator 分流）、pages.rs（侧滑关容器 + 一页一个、下滑跟手、开放能力 + open-data、snapshot + open-container、share-element）；`own_events` 里 swiper worklet 映射单测；`gesture_merge` 单测；改了 `tests/wx_compat.rs`（pan 先 POSSIBLE）、`tests/demo_wx_gap_e2e.rs`（scrollTo / 吸附带动画后等它走完）、`game_profile_tests`（prelude 表多了 open-type）。
- **跑法与结论**：工作区里 WP-E 的 canvas 半成品（commands.rs 引用还没写的 text2d / image_src）编不过，所以在隔离副本（/tmp，WP-E 那 6 个文件换回 HEAD，其余与工作区一致）里跑：`--lib` 全部 1102 过（两遍）、`--test wx_compat` 10 过、demo_wx_gap_e2e / scroll_view_e2e / edge_back_e2e / builtin_component_e2e / engine_gaps_e2e / frameset_e2e 等过；`form_e2e disabled_button_does_not_fire_tap` 失败，去掉 WP-C 的点击改动照样失败，是 WP-B 进行中的用例。
- **gallery**：同一副本跑 65 张，与 HEAD 比只有 `51_health_dashboard`（0.311%，包围盒 x[255,704] y[1082,1334]，进度条）与 `60_form_controls`（高 1682→1700）变了，都是 WP-B 的 progress / button；gallery 场景里没有 WP-C 的任何组件 / 属性，WP-C 0 张。工作区本身等 WP-E 编得过之后再跑一次确认。
- **真机要验**：native-view 代理 scroll-view 时手势回调与滚动的跟手、draggable-sheet 里列表 / 面板的换手手感、page-container 侧滑关闭的距离阈值、page-container 下滑跟手与 share-element 飞跃的帧（都是每 move 一次 setData animation，只重画面板 / 元素那块，要按顿挫量）。site 与 README 组件清单留给 WP-Z。

### WP-E（媒体与画布）· 2026-11-05（接续两轮断网的半成品；第一步先补齐 `image_src` / `round_rect` / `fill_text_max` / `stroke_text_max`，`cargo build --release --lib` 恢复通过）

- **E1 canvas ✅（🟡 三处）**：
  - drawImage（`canvas/image_src.rs`）：3 参数按图的固有尺寸、5 参数缩放、9 参数源矩形 → 目标矩形（SVG 按固有尺寸换算栅格坐标）；图源解码一次进缓存（本地文件带「长度 + 修改时间」戳，总量 64MB，先逐出最早的；从前每条指令读盘 + 解码一次），认包内路径 / wxfile:// / data: URI / SVG。
    没有源矩形、矩阵只有平移 + 等比缩放、globalAlpha=1、无合成 / 阴影时仍走旧的 `Canvas::draw_image`（像素不变）；其余走 `draw_image_rgba`（仿射 + 双线性，旋转 / 半透明 / 合成模式从此对了）。
  - `createImage` 真加载：`__native_canvas_image` 解码（进同一份缓存），`width / height` 是固有尺寸、读不到 / cloud:// 发 `onerror({errMsg})`，远程图先 `wx.downloadFile`，drawImage 用下载好的本地文件。
  - roundRect（上下文与 Path2D，radii 1~4 个按标准展开、超 4 个 / 负数抛 RangeError、相邻和超边长等比缩、贝塞尔控制点过矩阵）；`fill('evenodd')` / `clip('evenodd')`（默认改成 HTML 的 nonzero，`canvas/fill_rule.rs`：不自交的图形两种规则逐字节相同）；
    isPointInPath / isPointInStroke（`canvas/hit2d.rs`，同步 `__native_canvas_hit`，先执行排着的指令；描边按「到折线距离 ≤ 线宽/2」，线帽 / 斜接近似 🟡）；getTransform 跟踪 translate / rotate / scale / transform / setTransform 并随 save / restore；
    fillText maxWidth 横向压缩、strokeText 真描轮廓（都走 text_raster 离屏光栅；描边色是渐变 / 有阴影时退回旧的填充近似）；补齐 overlay / color-dodge / color-burn / hard-light / soft-light / exclusion / hue / saturation / color / luminosity（W3C 公式，`fx_composite.rs`）。
  - 旧接口 `draw(reserve, cb)`：每次 draw 交出整队并清空（从前 reserve=true 会把旧指令再画一遍），原生按 `__draw` 头复位状态（官方示例：两种 reserve 都回到默认样式）、false 先清屏；重建后备缓冲按批重放；回调前读一次 0×0 像素把排着的指令先执行掉（回调在画完之后）。属性式写法（fillStyle / lineWidth / font / globalAlpha / shadow* …，draw 后回到默认）。
  - 没写尺寸时 300×150（各边独立补；只写一边时浏览器按 2:1 推另一边，微信没写，待验 🟡）；canvas 自己的触摸事件触点带画布内 `x / y`（`host/touch.rs`，pageX 照留）；`disable-scroll` + 绑了 touch / longtap 时挂 `catch:__lockscroll` 锁页面滚动。
  - ❌ `bind:error`（同页 canvas-id 重复）：要在建树时按页面统计 canvas-id，增量建树（`layout_incremental`）不会重跑没变的节点，得在渲染器里另加一趟，留给下一轮。没做：Canvas 2D 1365×1365 上限、`drawImage(canvas)`（画布当图源）。
- **E2 image ✅（🟡 SVG 两处）**：SVG 第一次解码时 resvg 栅格化成固有尺寸 ×2（长边 ≤2048）进现有图片缓存，scaleToFill 撑满（官方：Skyline 撑满）；本地 .svg 的 widthFix / heightFix 只解析拿固有尺寸。
  🟡 bindload 报的是栅格尺寸（固有 ×2）、裁剪类 mode 按两倍尺寸画；SVG 里的 `<text>` 与内嵌位图不画（没开 resvg 的 text / raster-images）。另补 `<image src="data:…">`（base64 / 百分号编码）。
  `fade-in`（`image/fade.rs`）：第一次画出来时 300ms 线性渐显（官方没给时长，待验），渐显期间按「会换帧的动图」续帧，只重画那张图；没有渐显中的图时每个 image 节点每帧只多读一个原子数。
  `show-menu-by-longpress`（`image/attrs.rs`）：挂 longpress 绑定 `__imgmenu:<src>` → `__imageMenu`（media.js）：宿主在 hostApis 接了 `image.showMenu` 就交宿主，否则弹 actionSheet（发送给朋友 / 收藏发宿主事件 `{type:'imageMenu', action, src}`，保存图片走 saveImageToPhotosAlbum），派发成功不再发 tap。`forceHttps`：http:// → https://（写回节点 src）。cloud:// 走 binderror（errMsg 仍是 `GET_IMAGE_FAILED`）。
- **E3 video ✅（🟡 两处）**：非叶子（registry 一行 + `wxml_renderer/layout.rs` 一行：带子节点时沿用 build 里的默认 300×225 与黑底，不重算样式），子节点画在视频之上，点到带绑定的子控件不再切控制条 / 不触发进度手势；去掉强制 4px 圆角（只认 CSS）；
  同层渲染时弹幕也由引擎画（时间用宿主报的位置），`sendDanmu` 不再告警丢掉；`enable-play-gesture` 双击（间隔 <300ms，待验）切换播放；`enable-progress-gesture`（默认 true）起播过的视频横划画面拖进度（相对拖动，划过整宽 = 整段时长，比例待验）；
  `auto-pause-if-navigate`（默认 true）：navigateTo 盖住当前页时暂停，内置播放器的 pause 当场派给这一页（`host/engine_keep.rs` 一行）。🟡 只接了 SDK（MiniEngine）那条，桌面窗体的跳页没接；
  `bindprogress`：内置播放器原本就报 `{buffered:100}`，同层渲染本地文件在 loadedmetadata 后补发 100；🟡 网络地址要宿主按缓冲进度回传 `progress`（Android / iOS 的 VideoHost 都还没报）。
- **E4 live-player ✅**：`orientation`（vertical / horizontal）与 `mode`（live / RTC）规范成合法值（非法值回官方默认）放进 `videos` 的 attrs 透传宿主；statechange 码表与 message 按现行文档（2003 改为「视频首帧渲染事件」，现行表里没有的 2006 不再报，ended 不报码）；netstatus 的 info 按官方表给全键；`auto-pause-if-navigate` 同样生效（暂停命令交宿主）。
- **测试**（全部在已有文件里，没新建 tests/*.rs）：`src/tests/engine_render_tests.rs` 的 `e1_drawimage_三参数按原图尺寸_五参数缩放_九参数裁源`、`e1_create_image_真加载_宽高_onerror_svg`、`e1_roundrect_evenodd_ispointinpath_gettransform`、`e1_filltext_maxwidth_压缩_stroketext_描轮廓`、`e1_旧接口_draw_reserve_语义_属性式写法_回调在画完之后`、`e1_canvas_默认_300x150_disable_scroll_锁滚动`；
  `src/tests/image_event_tests.rs` 的 `e2_svg_解码成位图_data_uri_也认_scale_to_fill_撑满`、`e2_fade_in_从透明渐显_不写不渐显`、`e2_cloud_文件_id_走_binderror`、`e2_长按菜单_宿主没接弹_action_sheet_返回已处理`；
  `tests/video_e2e.rs` 的 `e3_子节点画在视频上_点子控件不切控制条_圆角只认css`、`e3_双击切换播放_跳页自动暂停`、`e3_起播后横划画面拖进度_关掉不拖`；`tests/same_layer_video_e2e.rs` 的 `e3_同层渲染_弹幕由引擎画_send_danmu_与_bindprogress`，直播用例加了 orientation / mode / ended 断言；
  单测 `canvas::fill_rule::简单图形交出全部交点_自交才合并`、`canvas::hit2d::{点在路径里_nonzero_与_evenodd, 点在描边上_按线宽}`、`canvas::fx_composite::补齐的混合模式_红叠灰`、`canvas::image_src::data_uri_两种编码`、`host::touch::canvas_触点带画布内坐标`、`image::svg::{认得出_svg, 按固有尺寸两倍栅格化_左半红右半透明}`、`image::fade::渐显_从透明到不透明_结束后不再报帧`、`image::attrs::{force_https_只换_http, 长按菜单只在开了属性时挂}`、`host::engine_live_player::tests::{透传的_orientation_mode_规范成合法值, 状态码按现行码表}`。
  跑过（工作区本身）：上面这些全过；`--lib` 的 canvas / image / gif / animated / video / registry / style_reuse / component_render / skyline_component / renderer_tests / layout_tests / touch / wx_semantics / native_bridge_contract / wx_api_tests / game_profile 过；`--test video_e2e` 9/9、`--test same_layer_video_e2e` 6/6、`--test io_api_e2e canvas`、`--test query_e2e` 5/5 过。`interaction_tests::test_radio_exclusive` 失败，是 WP-B 正在改的 radio 互斥（B5），与本包无关。
- **gallery**（工作区跑 65 张，对 HEAD）：本包只变 `33_video.png`（0.042%，244 像素，包围盒 x[24,725] y[91,485] = 视频框四个角：CSS 的 `border-radius:16rpx` 生效，从前被强制 4px 盖掉，属修正）；`51_health_dashboard` / `60_form_controls` 是 WP-B；`34_canvas` 不变（简单图形 nonzero 与 even-odd 逐字节相同，gallery 直接调 `execute_canvas_draw`、没有 `__draw` 头）；35_gif 这次也没变。
  drawImage 3/5 参数改按原图尺寸、旧接口 draw(true) 不重放：gallery 里没有用到的场景，0 张。
- **SVG 依赖体积**：resvg 0.45.1（`default-features = false`）多 17 个纯 Rust 小 crate（usvg / tiny-skia / roxmltree / svgtypes / kurbo / simplecss …）；桌面 release、不开 LTO、两边都关增量量的 cdylib：56.05MB → 57.07MB，**+995KB**；移动端是 thin LTO，应更小，没单独量。feature `svg`（默认开）关掉照常编过（量体积那次就是关着编的）；`build-mobile.sh` / `build-android-abi.sh` 的 feature 列表已带 `svg`。
- **共享文件的最小改动**：registry.rs（video 一行）、wxml_renderer/layout.rs（一行）、wxml_renderer/animated_image.rs（两行，渐显当动图）、components/mod.rs（导出）、js/bridge_io.rs（`__native_canvas_hit` / `__native_canvas_image`）、page-components.js（`__imgmenu:` 一行）、host/engine_keep.rs（一行）。
- **真机要验**：canvas drawImage 每条多一次 stat（缓存命中）在一加上的代价、strokeText / maxWidth 离屏光栅的耗时；fade-in 渐显只重画那张图（按顿挫量）；同层视频上画弹幕每交一帧多画几行字的代价、视频子节点（cover-view）在同层视频 / Flutter 上的叠放；横划拖进度与外层 swiper 的冲突（与微信一样会抢，`enable-progress-gesture=false` 关掉）；双击间隔手感；Android / iOS 宿主收到暂停命令真的暂停（含 live-player）；宿主播放器是否按 `orientation` / `mode` 处理（现在只是透传）；网络视频的 bindprogress 要宿主补；SVG 首次栅格化在真机上的耗时。
- **没动 / 留给别人**：site 与 README 的能力 / 限制（WP-Z：canvas 新能力、SVG / data: 图片、fade-in、长按菜单、video 子节点与手势、上面各条 🟡 / ❌）；`doc/测试场景总表.md` 没有新测试文件要登记。

### WP-B（表单与输入）· 2026-11-05（接续前两轮断网的半成品：button / progress / progress_anim / icon / self_anim / form 复位 / disabled 按钮审过后保留并补完）

- **B1 ✅（🟡 两处）** input / textarea：`focus` / `auto-focus` 受控（挂载为真、假→真聚焦并发 `inputFocus` 让宿主拉键盘；真→假且正聚焦时失焦；登记时比对 `ui/interaction/focus_attr.rs`，画完由 `host/input_focus.rs` 执行，SDK 每帧一次表查找）；按属性聚焦时光标放 `cursor`、`selection-start/end` 选中那段；`cursor-color`（default / green 用微信绿，其余按颜色）；`hold-keyboard`（点页面照常派 tap、不失焦）；按「完成」confirm 之后补 `blur`（confirm-hold 不补，SDK 与桌面同）；input `bindchange`（失焦时与聚焦时的值不同才发，顺序按 DOM：change 先于 blur，真机顺序待验）；focus `detail.height` = 宿主最近报的键盘高（没报为 0）；bindinput 的 `cursor` / `keyCode` 真值（keyCode 按 DOM：退格 8、回车 13、Delete 46、字符给字符码，iOS 待验）；textarea 的 bindinput 返回值不替换内容；textarea 的 blur 带 `cursor`（input 的 blur 照官方不带）；`placeholder-style` / `placeholder-class` 的 font-size / font-weight（Skyline 的对象写法也认）；textarea 默认 placeholder-class 为 `textarea-placeholder`。顺带：从一个输入框点到另一个时旧的先发 blur（从前不发）。
  附带信息（光标、keyCode、是否改过）经 `ui/interaction/edit_note.rs` 线程局部传给派发，不改十来处调用签名。
  🟡 没写 id 的输入框在 scroll-view 里滚动后身份变（含坐标），focus 仍为真会再聚焦一次；桌面窗体没接按属性聚焦（只 SDK）；selection 只在引擎里，`inputFocus` 只带 cursor，宿主输入法的镜像选区要真机验。
- **B2 ✅** textarea `auto-height`（style.height 不生效，高 = 行数 × 行高 + 内边距，min/max-height 照常夹；行数按画出来的，变了才整树重排一次，`components/input/auto_height.rs`；打字不经 setData 也长高）、`bindlinechange {height, heightRpx, lineCount}`（number，height 不含内边距，待验；首帧不发）、`disable-default-padding`。🟡 两个都没写 id / name、占位符又相同的 auto-height 共用一个行数。
- **B3 ✅（🟡 数据缺口）** time 的 `start` / `end`（裁时列与起止小时的分钟列、初值夹进范围）；date / time 默认值按本地时区（`localtime_r` 的 tm_gmtoff，非 unix 平台按 UTC）；region 换成国家统计局 2023 统计用区划代码（modood/Administrative-divisions-of-China `pca-code.json`，31 省 342 市 3056 县，`host/region_pca.txt` 60KB，第一次弹 region 时解析），change 带 `code`（6 位）；`custom-item`（每列顶部，选了它后面的列只剩它，code 给空串待验）；`level` province / city / region。🟡 没有港澳台；东莞 / 中山 / 儋州 / 嘉峪关第三列是乡镇（9 位码）；`postcode` 没有可信数据不给（undefined）；`sub-district` 没数据按三列。
- **B4 ✅** picker-view `indicator-style` / `indicator-class` 的 height（px / rpx）、background-color、border；`mask-style` / `mask-class` 纯色与 none；写了选中框高度时列里每项高度设成它（`components/picker_view_style.rs`，建树解析一次写进属性）。🟡 渐变蒙层的自定义颜色按默认画；Skyline 写在 picker-view-column 上的 indicator-style 没接；没写选中框高时项高仍按布局（沿用从前，WebView 默认 34px 时会强制项高）。
- **B5 ✅** radio 互斥只在同一个 radio-group（从前全页互斥）；点已选中的 radio 不变、不发 change、照常派 tap；radio-group / checkbox-group 的 detail.value 与互斥按布局树找成员（`WxmlRenderer::choice_group`，滚出视口的也算）；label `for` 指向视口外的控件时直接改状态（🟡 那一下不派 change：它的绑定没登记）；label 包着 / 指向 form-type 按钮时提交 / 重置表单。radio-group 的 detail.value 仍是字符串（官方文档写数组，实际是字符串，待验）。
- **B6 ✅** `loading` 转圈（WeUI 12 辐条，1 秒一圈，只把按钮盒子记进动画包围盒，用例断言每帧都是「局部」）；disabled 按钮不登记绑定（WebView 版 `wx-button[disabled]` 是 pointer-events:none），点击落到外层；form reset 让 switch / checkbox / radio 回到属性里的 checked、picker 回到属性值（picker 本来就不留状态）。🟡 画进 scroll-view 离屏缓存的按钮 / 进度条不动（转圈停第 0 格、进度直接终态）。
- **B7 ✅** icon `size`、progress `stroke-width` / `font-size` / `border-radius` 认 px / rpx；progress 默认直角（官方默认 0）、`color`（activeColor 旧名）、右侧百分比默认 16px（官方）、`active` / `active-mode`（backwards / forwards）/ `duration`（每 1% 毫秒，默认 30）/ `bindactiveend`（detail `{curPercent}` 照 WebView 版，官方没写，待验），动画只重画进度条盒子；无头 `--time` 定住时钟时直接画终态。
- **测试**（`src/tests/wpb/`，14 个；`tools` 没加脚本，已登记进《测试场景总表》第二节）：
  input.rs `b1_focus_属性_挂载聚焦_变假失焦_change_与_detail_类型`、`b1_hold_keyboard_点页面不失焦_完成键发_confirm_再_blur_没改不发_change`、`b1_textarea_返回值不替换_换输入框先_blur_旧的_focus_height_是键盘高`、`b1_textarea_失焦_detail_cursor_是_number`、`b1_selection_start_end_按属性聚焦时选中那一段`、`b1_占位符字号_光标颜色_textarea_默认占位符类`、`b2_auto_height_随行数长高_linechange_detail_与_max_height`、`b2_disable_default_padding_去掉默认内边距`；
  controls.rs `b5_radio_互斥只在同组_点已选中的不发_change`、`b5_长列表_组的值与_label_for_不看视口`、`b6_disabled_按钮不派_tap_loading_只重画按钮那一块`、`b7_progress_默认直角_rpx_color_active_与_activeend_icon_size_rpx`；
  picker.rs `b3_region_确定之后_detail_带_code`、`b4_picker_view_选中框样式与类_蒙层_项高跟选中框`；
  单测 `picker_sheet::build::wpb_tests::{b3_time_start_end_裁列与夹初值, b3_date_默认值是本地的今天}`、`picker_sheet::region::tests::b3_region_完整数据_code_custom_item_level`、`region_data::tests::全表_直辖市改名_六位码`、`picker_view_style::tests::行内样式_rpx_border_mask`、`input::placeholder::tests::声明串与对象两种写法`、`edit_note::tests::按_id_与值对上_前面没取的一并丢掉`、`progress_anim` 3 个；
  改了 `src/tests/interaction_tests.rs`（`test_radio_exclusive` → `test_radio_select_and_reclick_is_noop`：互斥挪到宿主按组做）、`tests/form_e2e.rs`（reset 回到属性值、`disabled_button_does_not_fire_tap`）。
  跑过：`--lib wpb` 14/14、picker_sheet / region_data / input / textarea / radio / picker / interaction / form / label / checkbox / caret / button / progress / icon / switch / focus / keyboard / engine_input_tests / wpc 全过；`--test` input_semantics_e2e、chat_keyboard_e2e、input_focus_visual_e2e、form_e2e、keyboard_shift_e2e、component_e2e、builtin_component_e2e、io_api_e2e、sdk_damage_caret、wx_compat 全过（工作区直接跑，WP-E 已能编过）。
- **gallery**（工作区跑 65 张，对 HEAD）：`60_form_controls` 高 1682→1700：从第 1329 行（进度条那一行）起不同，下面整体下移 18px 逐像素相同 —— 带 show-info 的 progress 行高从「stroke+4=10px」改成按右侧百分比的行盒（官方 font-size 默认 16 → 18.84px，×2 ≈ 18 物理像素），同时直角、百分比 16px、左间距 15px；`51_health_dashboard` 0.311%（包围盒 x[255,704] y[1082,1334]，三条带 show-info 的进度条：直角 + 百分比 16px）；`33_video` 0.042%（x[24,725] y[91,485]，视频那块，场景里没有 WP-B 的组件，是 WP-E 的视频改动）。其余 62 张逐字节不变（35_gif 这次也没变）。
- **没做 / 留给别人**：site 与 README 组件清单（归 WP-Z：能力见上，限制见各条 🟡）；`hit_test.rs`（507 行，几个包都往里加了几行）没拆；宿主侧没改（Android / Flutter / iOS 收到 `inputFocus` 就拉键盘，不区分是不是点出来的，所以没编宿主）。
- **真机要验**：`focus` / `auto-focus` 拉起键盘（Android 原生 / Flutter / iOS，尤其页面刚打开就 auto-focus 时视图还没拿到焦点）；`hold-keyboard` 点页面键盘不收；selection-start/end 之后打字（宿主输入法镜像的选区）；bindchange 与 blur 的先后、keyCode 在 iOS 上的值；auto-height 输入页 `*-slow` 顿挫（换行那一下整树重排一次）；loading / progress active 按顿挫量（只重画自己那块）。

### WP-F（宿主侧原生组件）· F1 map · 2026-11-05（前两轮断网，开工时 `git status sdk/` 没有半成品，从头做；F2 按计划本轮不做）

逐项（Android 原生 `sdk/android/.../MiniMapView.kt` 一份，Flutter Android 直接编进同一份源码；iOS 原生 `sdk/ios/Sources/MiniRender/MiniMapView.swift`）：

| 项 | Android | iOS | 说明 |
|---|---|---|---|
| controls + `controltap {controlId}` | ✅ | ✅ | 按 position 钉在框里、不随地图动，画在最上面、点击最先判；`clickable` 默认 false（不可点的不挡地图 tap）；没写宽高按图片像素 / 2（与 marker 同一换算） |
| customCallout（slot="callout"） | ✅ | ✅ | 引擎侧真画：`wxml_renderer/map_callout.rs` 布局时把 slot 挪出画面（不占流、不画、点不到），带 marker-id 的每个子节点各自绝对定位按内容定宽高；`host/engine_map_callouts.rs` 按 `<snapshot>` 那条静态路径离屏出图、存 PNG（文件名 = 像素哈希，内容不变路径不变）进 `sameLayerViews` 的 `callouts`；宿主按 marker 的 `customCallout {display, anchorX, anchorY}` 贴在标记上方，有它时忽略 callout / title。🟡 只在布局树整棵重建时重新出图：只改气泡里文字的 setData 宿主手里还是旧图；气泡内部不分子节点，整块算 callouttap |
| show-location 定位蓝点 | ✅ | ✅ | 有权限才定位（Android `ACCESS_FINE/COARSE_LOCATION`，iOS whenInUse / always），**SDK 不弹授权**（微信 3.13.2 起会自动弹）；没权限不显示、不崩，日志一行 `🗺 show-location：没有定位权限…`；WGS-84 → GCJ-02 换算；精度圈 + 方向三角（Android `bearing` / iOS `course`，只有移动中有；没接罗盘） |
| enable-zoom / enable-scroll 分开 | ✅ | ✅ | 只关拖动：单指拖不动（页面也不滚）、捏合绕框中心缩放；只关缩放：第二根手指与双击不缩放；都关：引擎不把手势交给地图（拖动滚页面，原逻辑） |
| marker `title` | ✅ | ✅ | 没有 callout / customCallout 时当一个 BYCLICK 的白底气泡（点标记开、再点关），点它发 callouttap（官方没写，待验） |
| callout `textAlign` / `anchorX` / `anchorY` | ✅ | ✅ | textAlign 多行对齐（默认按 left，官方没写默认值，待验）；anchorX / Y 是气泡整体偏移（向右 / 向下为正） |
| wxfile:// 与临时文件图标 | ✅ | ✅ | `sameLayerViews` 事件新带 `wxfileRoot`（引擎 wxfile 根目录），`SameLayerViews.resolve` 认 `wxfile://tmp/…` / `usr/…`（不许 `..`）；marker / control / arrowIconPath 都走它。**要新编的引擎库**（旧库不报根目录 → 找不到、不画） |
| polyline `colorList` / `level` / `arrowIconPath` | ✅ / 🟡 / ✅ | ✅ / 🟡 / ✅ | colorList 一段一色、不够用最后一个；arrowIconPath 图标朝走向每 60px 一个、高跟线宽（≥8）；level 照解析但三个取值画出来一样（栅格瓦片，路名 / POI 烧在图里，线只能在整张瓦片之上） |
| polygon `zIndex` / `dashArray` | ✅ | ✅ | zIndex 解析时稳定排序；dashArray `[实, 空]`，[0,0] 实线 |
| 颜色 rgba() / #RGBA | ✅ | ✅ | 另认 rgb()、百分比通道；#RGBA 与 #RRGGBBAA 一样透明度在后 |
| poitap / polylinetap | ❌ / ✅ | ❌ / ✅ | poitap：瓦片是栅格图，没有 POI 数据可点（要换矢量 SDK 才有）；polylinetap：点到线的屏上距离 ≤ max(8, 线宽/2+边框)，detail `{longitude, latitude}`，这一下不再发 tap（官方没写，待验） |
| setting 整组覆盖 | ✅ | ✅ | 写了的键盖过同名属性、没写的看属性（官方「仅设置的属性会生效」）：认 showLocation / showScale / enableZoom / enableScroll / enableSatellite；rotate / skew / 3D / 路况 / 指南针 / 个性化没有对应能力，忽略。引擎 `gesture_view_at` 也按 setting 判「两个都关就不接手势」 |
| MapContext.openMapApp | ✅ | ✅ | Android：`geo:lat,lng?q=lat,lng(名称)` Intent，系统选 App；`preferApplication` 装了就 setPackage 直达（amap / baidu / tencent / google / petal），没有 App 回 `openMapApp:fail 没有可用的地图 App`。iOS：preferApplication 走各家 scheme（高德 dev=0、百度 coord_type=gcj02、腾讯、Google），否则 / 打不开退回 `maps://?daddr=`（苹果地图 URL 交不过去目的地名称）。🟡 坐标原样按 GCJ-02 交，苹果地图 / geo: 在国内按哪个坐标系认待验 |
| moveToLocation 不给坐标用定位 | ✅ | ✅ | 有权限：蓝点位置 → 系统缓存的最后位置；没有：退回属性里的中心点（原行为） |

**性能**：气泡 / 标签 / title 按 dpr 画成一张位图挂在气泡对象上（属性变了随对象换），拖动 / 缩放每帧只 drawBitmap —— 从前每帧每个气泡量字 + 画阴影 + 画字；控件 / 自定义气泡 / 箭头图标走 `MapTiles.icon` 的缓存；虚线 / dashArray 的 `DashPathEffect` 解析时建一次（从前每帧每条虚线 new 一个）；定位三角复用一个 Path。新增的每帧代价：每个控件一次贴图、蓝点几个圆；没有每帧分配位图的路径。polyline 命中只在点击时算。

**引擎侧改动（最小）**：新文件 `src/renderer/wxml_renderer/map_callout.rs`、`src/host/engine_map_callouts.rs`；`wxml_renderer/mod.rs` / `host/mod.rs` 各一行 mod；`layout.rs` 一处（tag == "map" 时 place_slot）；`snapshot.rs` 的 `draw_snapshot` 改成 `pub(super)`；`engine_host_views.rs`：sync 里给地图带 callouts、事件带 `wxfileRoot`，`gesture_view_at` 认 setting。
**测试**：`tests/same_layer_view_e2e.rs` 加 `f1_自定义气泡出图交给宿主_slot_不画在页面上_setting_盖过属性`（callouts 两项、markerId 是 number、宽高 80/120×30 逻辑、PNG 160×60 且是红底、slot 位置是地图底色、setting 关掉后不发 nativeViewTouch 且页面滚 >50）；整个文件 4/4 过。没新建 tests/*.rs。
**演示页**（`sdk/flutter/example/assets/demo/pages/map`，SDK 示例、不在冻结名单）：加了 control（左上角头像，controltap）、王府井的 customCallout（slot cover-view）、第一条路线 colorList（含 rgba）、多边形 dashArray / zIndex、callout textAlign / anchorY、按钮「禁止拖动」（setting.enableScroll）与「openMapApp」、controltap / polylinetap / poitap 日志；`runAuto` 追加检查项 `moveNoArgs`（moveToLocation 不给坐标也 success）并进 `NC-CHECK map` 的 ok（脚本判据没动）。

**编译结论**：`cargo check --release --lib` 过；Android `cd sdk/android/example && ./gradlew assembleRelease -q` 过（exit 0）；Flutter `flutter analyze` No issues；iOS `xcodebuild -project sdk/ios/example/MiniRenderExample.xcodeproj -scheme MiniRenderExample -sdk iphonesimulator ARCHS=arm64` BUILD SUCCEEDED（generic 模拟器目标默认带 x86_64，而 xcframework 只有 arm64 模拟器片，要 `ARCHS=arm64`）。**APK / xcframework 里的引擎库是旧的**：customCallout 出图与 wxfileRoot 要先 `tools/build-mobile.sh` 编新库才生效（宿主代码对旧库是安全的：没有 callouts 就不画自定义气泡，wxfile 图标找不到就不画）。

**真机要验**：① 拖 / 捏合有常显气泡、控件的地图跟手（`🗺 同层组件` 那行的中位 / 最慢不回退，原生与 Flutter 各一次 `android-native-components.sh`）；② 自定义气泡（王府井）的位置与清晰度、点它发 callouttap；③ 控件点击 controltap；④ 「禁止拖动」后单指拖不动、页面也不滚、捏合仍缩放；⑤ 给示例 App 授定位权限后蓝点出现、坐标与瓦片对得上（GCJ 换算）、不授权不崩；⑥ openMapApp 弹出地图 App（Android 系统选择 / iOS 苹果地图），目的地位置在国内是否偏移；⑦ 脚本的「点地图」那一下若恰好落在路线上会变成 polylinetap（日志 `[map] polylinetap`），脚本只认 tap / markertap / callouttap —— 拖动后通常已离开路线，遇到再看；⑧ iOS 同一套（`tools/ios-native-components.sh`）。
**没动 / 留给别人**：site 与 README（WP-Z：能力见上表，限制 = poitap ❌、level 不分层、customCallout 只在整棵重建时刷新、SDK 不弹定位授权、Flutter iOS 没有地图（F2））；F2（camera、live-pusher、voip-room、Flutter iOS 的 map / web-view / 同层视频）按计划下一轮。

### WP-D（滚动、吸顶、轮播）· 2026-11-05（子任务被网络中断打断三次，前几轮的代码审过保留；收尾与 D2 余项由主会话补完）

- **D1 吸顶 ✅**：CSS `position: sticky`（top / bottom，按最近的纵向滚动容器：页面或 scroll-view；被父容器内容盒推走）、`<sticky-header>`（offset-top、allow-overlapping、padding、`bindstickontopchange {isStickOnTop}` 只在变化时发、boolean）、`<sticky-section>`（push-pinned-header 默认 true、padding）。
  重画方案（`wxml_renderer/sticky.rs` 文件头）：页面滚动时吸住的元素按「上次画在哪 ∪ 这次画在哪」报损伤区（`sticky_damage`，SDK `engine_damage*.rs` / 桌面 `frame.rs`），只清、只画这两块；没吸住的帧不加东西（只合成）；scroll-view 里的在内容画完 / 贴完分块之后按吸住的位置画，块里与离屏缓存不画它。
  不吸的情形写明：祖先有 transform / 动画 / opacity<1 / overflow 非 visible / fixed / swiper / 横向 scroll-view；吸附里再套吸附。🟡 吸住时 SelectorQuery 报的是原位；横向 left / right 吸附不支持。
- **D2 scroll-view ✅（两项 ❌）**：bounces、paging-enabled、fast-deceleration（`scroll_behavior.rs` → `ui/scroll_controller/behavior.rs`）；show-scrollbar（enhanced 时画、停下渐隐、只重画那一条，`scrollbar.rs`）；dragend 的 velocity；scroll-anchoring（带 id / wx:key 的项）；reverse 贴底；enable-back-to-top 占屏 90% 自动开；type=nested + nested-scroll-header / body（`host/nested_scroll.rs`）；refresherstatuschange；
  主会话补：`min-drag-distance`（只对写了的 scroll-view，`host/gesture.rs` 方向锁定距离取二者大者）、bind:scroll 的 `boundaryVelocity`（碰边那一次给数，其余不带字段 = undefined，`isNaN` 同样为真）、下拉二级 `refresher-two-level-*`（`host/scroll_two_level.rs`：拉过阈值松手 / triggerTwoLevel / 属性开；closeTwoLevel / 属性关 / scroll-enabled 时推过 close-threshold 松手关，推得不够弹回；状态码 5 / 6 / 7 / 8；三个点在露出高度过二级阈值时不画，按位置判、不按状态判，保证局部重画一致）。
  ❌ scroll-view `clip=false`：内容滚到盒子外也要画，而 scroll-view 一动只重画自己的盒子 —— 盒外会留残影，要连带改损伤区，留下一轮；❌ scroll-x + scroll-y 同时（单轴控制器，照旧按纵向）；🟡 `refresher-two-level-pinned`、`refresher-ballistic-refresh-enabled` 只读不用（官方没写清）。
- **D3 ScrollViewContext ✅**：scrollEnabled / bounces / showScrollbar / pagingEnabled / fastDeceleration / decelerationDisabled 可写且生效、scrollTo、scrollIntoView（alignment / offset / animated）、triggerRefresh / closeRefresh、triggerTwoLevel / closeTwoLevel。
- **D4 swiper ✅**：swiper-item `item-id` → change / animationfinish 的 `currentItemId`、`current-item-id` 按项选、`clip=false`（盒子外的相邻项照画）、`snap-to-edge`。
- **测试**（`src/tests/wpd/`，每个用例都是「局部重画 vs 整帧」两台引擎逐帧逐字节比）：sticky.rs 9 个（页面吸顶只重画那块且点得到、吸住时 setData、被推走 / 吸底、scroll-view 里分块缓存与离屏缓存两种、叠成一摞 / 盖住、offset-top、没写 top 不吸 + section padding、`d1_bench_吸顶滚动每帧代价`（ignored，手动跑））；
  scroll_ctx.rs 13 个（d3 可写属性 / alignment / 刷新、d2 velocity / 滚动条 / anchoring / reverse / back-to-top / nested、**下拉二级、min-drag-distance、boundaryVelocity**）；swiper.rs 3 个；单测 `host::scroll_two_level::tests::属性与请求开关_推过关闭阈值才关`。`--lib wpd` 23 过 1 忽略，`--lib gesture` 29 过。
- 顺带修了自己写的测试死锁：一个用例里先后开两对 `Pair`，前一对拿着包根写锁 —— 先 drop 再开。
- **真机要量**：吸顶列表页上下滑（屏上重复率、>8ms 帧，页面与 scroll-view 两种）；show-scrollbar 渐隐；nested 里外层交接的跟手；下拉二级打开 / 关闭的弹簧。

### WP-Z（收尾：示例、统一测试、修出来的问题）· 2026-11-05
- **新示例 `sample/wx-components`（组件实验室，appid wxf81daf531db6c631）**：首页 + sticky / text / list / form / picker / scroll / gesture / media / open 共 10 页，每页对照官方文档写了能观察的结论（`detail` 的类型打到页内日志）。`tools/sample-smoke.py wx-components` 10 页 0 错。
- **新示例逼出来的引擎老问题（不是这一轮改坏的）**：
  - 裸文本（`<view>一段字</view>`）按继承的字重量宽：从前一律按常规体量，粗体字盒子偏窄、末字被裁（`layout.rs`）；
  - 裸文本超出父级内容区时收到父级宽、按行数补高（`reflow.rs`）：从前只收拾 `<text>`，长裸文本一行冲出屏幕。放得下的在量行数之前返回；
  - SVG 的 `bindload` 报固有尺寸（96×96），从前报的是 2× 栅格图的尺寸（`hit_test.rs`）；
  - 输入框 `change` 的兜底派发只找 input / textarea 的绑定（`host/tap.rs`）：从前会落到页面上唯一的 `change`（一个 picker），火车票示例的手机号被写进 typeIdx（`sample_train_e2e` 当场抓到）。
- **性能**：真机 list-builder 页（200 行）一次 setData「模板」31.9ms → 3.1~4.6ms：`wx:for` 与 list-builder 展开时每项 `data.clone()`，而 data 里装着列表本身（N 项 = N² 次深拷贝）。改成作用域只拷一次、每项改写 item / index 两个键（多模板变量名不同时渲染完还原）。这条对所有 `wx:for` 页面都生效。
  对齐网格（`grid-view` / `grid-builder` 非 masonry）进增量布局（`layout_incremental.rs` 的 `pure_tag`）：从前整个 grid 每次 setData 都重建，只有瀑布流真的在布局后改孩子。
  示例 list 页的 itembuild 计数改成 300ms 内最多 setData 一次（每个事件都 setData 会让整页每帧重建一次布局）。真机 list 页：最长一帧 33 → 22ms、pump 最慢 43.5 → 16.3ms。
- **测试**：`wxml_template_tests` 加 `wx_for_复用作用域_同名外层变量与嵌套循环照旧`、`list_builder_多个模板变量名不同_互不串值`；`text_richtext_tests::裸文本_粗体按粗体量宽_长的到容器边折行`；`src/tests/wpd/mod.rs::make` 冻结时钟（`freeze_clock_for_test`）：事件按墙上时钟打时间戳时两台引擎会错开一帧，`d2_bind_scroll` 偶发失败。
