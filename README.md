# Mini Render

**一个全新的、自绘的小程序渲染引擎 —— 不基于 WebView，也不依赖任何系统 UI 控件。**

用 Rust 从零实现：自己解析 WXML/WXSS、自己算布局、自己把每个像素画进帧缓冲，再配一个基于 QuickJS 的逻辑层（App / Page / Component / 模块系统 / Promise）。同时内置一个**编译器**，可把同一份小程序源码编译成其他端可直接运行的源码（当前已实现 HTML 目标）。

> 纯 Rust、无系统 UI 依赖；核心库可编译为 **Android / iOS / Windows / macOS / Linux** 原生库供各端集成。

---

## 🧠 渲染原理（这引擎和别的有什么不一样）

### 它不是什么

| 常见方案 | 做法 | 本引擎 |
|---------|------|--------|
| 微信小程序 WebView 渲染层 | WXML → DOM，交给浏览器内核排版绘制 | ❌ 不用 DOM、不用浏览器内核 |
| React Native / Weex | JS 描述 → 映射到系统原生控件（UIView / android.view） | ❌ 不映射系统控件 |
| Flutter | Dart + Skia 自绘 | ✅ 思路相近，但这里是 **Rust + 自研光栅器**，无 Skia 依赖 |
| Electron / Tauri WebView | 打包一个浏览器 | ❌ 不打包浏览器 |

整个渲染链路是自己的：**没有 WebView、没有 Skia、没有系统控件**，产物是一个纯 Rust 库（`cdylib` / `staticlib` / `rlib`），落地只需要一块可写的像素缓冲。

### 一帧是怎么画出来的

```text
WXML 源码 ──┐
            │  ① 解析：手写 WXML 解析器 → 节点树；WXSS 解析器 → 样式表
WXSS 源码 ──┤     （CSS 选择器引擎：标签/类/#id/*/属性/后代/子代 + 特异性排序）
            │
page.js ────┘  ② 逻辑层：QuickJS 执行 App/Page/Component，setData 产出数据快照
                  ↓
            ③ 模板求值：{{ }} 表达式引擎 + wx:if / wx:for 展开 → 渲染节点树
                  ↓
            ④ 样式计算：命中的 CSS + 内联 style 合成计算样式（含继承语义：
                         color / font-size / font-weight / line-height ...）
                  ↓
            ⑤ 布局：Taffy(Flexbox) 计算盒模型；文本节点挂 **自定义度量函数**，
                     由字体真实字形宽度决定 min-content / max-content 与换行行数
                  ↓
            ⑥ 绘制：自研 2D 光栅器逐层画进 Canvas（RGBA 像素缓冲）
                     · 路径填充：扫描线 + even-odd，4× 超采样抗锯齿
                     · 圆角：三次贝塞尔逼近四分之一圆（K=0.5523），按 min(w,h)/2 夹紧
                     · 文本：fontdue 光栅化字形 + 自建字形缓存 + 多字体回退
                     · 彩色 Emoji：自己读 Apple `sbix` 位图表取 PNG 字形（轮廓光栅
                       器画不出位图字体），按字号缓存后 blit，与文字同基线
                     · 图片：双线性采样、object-fit 等价的 5 种 mode、GIF 逐帧
                     · 其它：线性渐变、阴影、Alpha 混合、clip 裁剪栈
                  ↓
            ⑥' CSS 动画：`@keyframes` 在**绘制阶段**按全局时钟求值（transform /
                     opacity / 颜色只影响绘制，不触发重排 → 动画帧成本≈静态帧）；
                     `transform` 的缩放/旋转/倾斜把子树画到离屏画布再逆仿射采样贴回
                  ↓
            ⑦ 分层合成：正常流 → 宿主外壳(tabBar) → position:fixed 覆盖层
                         （保证全屏遮罩能压暗 tabBar，与浏览器层叠顺序一致）
                  ↓
            ⑧ 上屏：softbuffer 把像素缓冲贴到窗口；或 save_png 出图；或交给各端宿主
```

关键设计取舍：

- **文本度量驱动布局**：文本不是"估算宽度"，而是把 `TextMeasure` 上下文挂到 Taffy 叶子上，由真实字形度量参与布局。这样"定宽容器内按容器宽换行"和"收缩容器被内容撑开"两种 CSS 语义能同时成立。
- **行高对齐浏览器**：`line-height: normal` 按行内实际用字区分 —— 含 CJK 用 1.375、纯西文用 1.1777（对 headless Chrome 实测校准），避免逐行累积垂直漂移。
- **字体全进程共享**：系统字体解析一次约 0.9s，因此用 `OnceLock` 全局共享一份（只读 + 内部字形缓存自带锁），任何数量的渲染器实例创建成本都是常数级（实测 ~42ns）。
- **无 GC、无 DOM diff**：每帧从渲染节点树直接绘制，布局结果带缓存；重绘只做像素写入，不维护中间 DOM。
- **动画只重绘不重排**：CSS 动画求值放在绘制期而非布局期，代价是 `@keyframes` 里改 `width/height` 这类会引发重排的属性不生效（改 `transform`/`opacity`/颜色都生效）。
- **缺失的关键帧用元素自身的值补**：`@keyframes spin { to { transform: rotate(360deg) } }` 这种只写 `to` 的写法（最常见的转圈写法），缺的 `0%` 要取元素的计算值。少了这条，整个周期恒等于 360° —— 视觉上就是永远不转。
- **不给盒子留"保险余量"**：文本盒宽度就是字形度量之和（只向上取整到整像素），换行判定另留 0.5px 亚像素容差。曾经用"每个文本盒 +4px"防误换行，结果所有按内容定宽的元素（徽标/标签/胶囊）都比浏览器宽一圈。

### 帧成本是怎么压下来的

纯软件光栅意味着每一帧的每个像素都由 CPU 写出，所以性能全靠减少"无用像素写入"和"无用内存拷贝"。商城首页从 **49.6ms/帧** 压到 **6ms 级**（375×667 @2x，750×1334 物理像素），带全屏遮罩弹层的重状态下仍稳定 **~89 FPS**。

绘制侧：

| 优化 | 做法 | 为什么有效 |
|------|------|-----------|
| **矩形行填充** | `fill_rect` 先把裁剪矩形并入范围，不透明色直接 `pixels[start..end].fill(..)` 整行写；半透明色把 alpha 系数提到循环外只做整数乘加 | 原来逐像素 `set_pixel`，每个像素都重做越界检查 + 裁剪矩形的浮点截断。背景/卡片/全屏遮罩占满屏面积，这一步收益最大 |
| **视口裁剪** | `cull_outside_viewport` 横纵双向剔除屏幕外节点（余量 `VIEWPORT_CULL_MARGIN_PX`；带 transform/animation 的节点不剔） | 滚动页面里大部分节点在视口外，此前照样跑完整绘制流程 |
| **只清可见带** | 宿主用 `Canvas::clear_band` 只清「视口 ± 裁剪余量」 | 页面画布是整页高的（首页 750×6870），每帧全量 `clear` 相当于 20MB memset |
| **图片两级过滤** | 缩小时先按目标尺寸做一次面积（盒式）降采样并缓存（mip），逐帧只在这张小图上做双线性；再用 `draw_image_cached` 缓存最终缩放结果 | 纯双线性只看 4 个邻域，缩小超过 1 倍就漏采样，照片边缘明显锯齿。而面积滤波很贵：轮播平移时亚像素偏移每帧都变，不分两级的话按帧重算（实测 135 → 58 FPS） |
| **行级 blit** | 图片贴回走 `blend_row`，越界与裁剪按行判一次 | 大面积逐像素 blit 里，边界判断本身就是主要开销 |
| **绘制期浅拷贝** | `shallow_for_draw` 只复制节点自身样式/属性，不复制子树 | 原先绘制每个节点前 `node.clone()`，等于把整棵子树复制一遍 |

样式与数据侧（一次 `setData` 的重建成本 **32ms → 4ms**）：

| 优化 | 做法 | 为什么有效 |
|------|------|-----------|
| **选择器解析期编译** | `StyleRule` 持有编译好的复合选择器，匹配时直接用 | 匹配是「每个节点 × 每条规则」的二重循环，从前在里面现场分词解析选择器字符串 —— 一次全页重建十万次。建树+样式 **28.7ms → 2.9ms** |
| **setData 脏标记** | `__native_page_update` 是原生回调，只置一个 AtomicBool | 从前 JS 侧把整份 data `JSON.stringify` 塞进日志缓冲，没有任何消费方 —— 每次 setData 白付一次全量序列化 |
| **数据快照缓存** | 页面数据存 `Arc<Value>`，只在脏了才做一次 JS→Rust 的 JSON 往返 | 从前每帧都取一次整份页面数据 |
| **上屏零边界检查** | `present_to_buffer` 逐行用切片 `zip` 转换 | 整屏 100 万像素的索引访问，边界检查占比可观 |

配套改动：`load_image` 返回 `Arc<ImageData>` 且像素数据是 `Arc<Vec<u8>>`（GIF 每帧不再深拷贝整张 RGBA）；组件 id 与图片 cache key 去掉浮点格式化。

帧调度上，`RedrawRequested` 结束时若 `is_animating()` 就**睡到下一个刷新时点**再自续一次 `request_redraw()`：只靠事件循环的定时唤醒每周期要多绕一圈（144Hz 屏上只有 ~47FPS），完全不限速又会以 2~4 倍刷新率空转烧核。时点必须从**帧开始**算 —— 从 present 之后算会多出一整个渲染耗时（16.7ms 的节拍变成 21ms，只剩 46FPS）。空闲时回到 `Wait` 休眠。

逻辑层的 `setData` 必须能自己触发重绘：定时器/网络回调改数据时页面上可能一个 CSS 动画都没有，没有脏标记的话秒杀倒计时会一直显示旧值。

```bash
# 逐秒帧率与帧内分段：「N 帧/秒，最慢一帧 X ms（逻辑 / 渲染[页面 覆盖层 tabBar] / 上屏）」
# 并附一行「整帧归因」：这一秒里整屏重绘各是被什么触发的（需重绘 / swiper / scroll-view / 动画）
MINI_FPS=1 cargo run --release --bin mini-app-window -- sample-app
# 布局重建分段：「模板求值 / 建树+样式 / 布局 / 换行修正」；并打印每次 setData 的失效范围
MINI_LAYOUT_LOG=1 cargo run --release --bin mini-app-window -- sample-app
# 滚动诊断：内容高 / 视口 / 画布高 / 当前位置与上限；外加每次拖动的手势归属
# （🖐 手势归属：Undecided -> Area{id,axis} / Page / EdgeBack / Blocked，以及内层为何交棒给页面）
MINI_SCROLL_LOG=1 cargo run --release --bin mini-app-window -- sample-app
# 关掉 setData 的增量失效（一律整帧）：用于逐像素对照验证，见 tools/damage-check.sh
MINI_NO_DAMAGE=1 cargo run --release --bin mini-app-window -- sample-app
# 网络请求日志（方法 / URL / 状态码 / 耗时）与 storage 读写日志
MINI_NET_LOG=1 MINI_STORAGE_LOG=1 MINI_IMG_LOG=1 ./run.sh tea-app
# 绘制耗时按组件类型/绘制阶段归因（image / text / 背景:阴影 / 背景:边框环 …）
MINI_DRAW_LOG=1 cargo run --release --bin mini-app-window -- tea-app --drag 8x60 --snapshot target/drag
# font-family 解析到了哪个字体文件；远程图片下载/缓存/占位原因
MINI_FONT_LOG=1 MINI_IMG_LOG=1 ./run.sh tea-app
```

> **滑动手感要能无头测**：`--drag <每帧像素>x<帧数>` 用与交互窗体同一套鼠标事件模拟一次
> 手指拖动，逐帧计时后给出 p50/p95/最慢与超预算帧比例。快照永远只出一帧、也不走拖动路径，
> 「滑动发抖」这类问题此前只能靠人肉拖窗口，改动前后无法比对。

> 「整帧归因」这一行是为了让性能问题能收敛：**「为什么这一帧又整屏重画了」如果只能靠猜就永远查不完**。它直接指出了首页每秒一顿的元凶是倒计时的 `setData`（每秒 1 次整帧），而不是当时以为的滚动或动画。

各页单帧耗时（优化前 → 后）：

| 页面 | 前 | 后 |
|------|----|----|
| `sample-app` 首页（轮播 + 倒计时 + 骨架动画） | 49.6ms | **6.1ms** |
| `sample-app` 组件页 | 8.6ms | **1.0ms** |
| `sample-app` 能力展示页 | 10.3ms | **2.2ms** |
| `sample-app` 购物车 | 2.0ms | **0.5ms** |
| `news-app` 首页 | 24.3ms | **~11ms** |
| `news-app` 详情 | 13.0ms | **~5ms** |

帧节奏与局部重绘（这两条决定「高刷有没有被用上」）：

| 优化 | 做法 | 为什么有效 |
|------|------|-----------|
| **动画帧只重绘损伤区** | 只有 CSS/JS 动画要重绘时，取动画元素包围盒的并集做裁剪矩形，只清、只画这一小块 | 一个 `animation: pulse infinite` 的小徽标，从前逼着整屏每帧重新光栅化（首页 8~14ms/帧）。浏览器靠图层合成避免这件事，这里用裁剪矩形达到同样效果 |
| **滚动不重绘** | 页面画布用内容坐标，只要视口还落在「已绘制条带」内，滚动只是取不同切片上屏 | 滚动期间大部分帧只花上屏的 3~4ms |
| **裁剪余量自适应** | 滚动时余量留 400 物理 px（少触发重绘），静止时收到 48（偶发整帧重绘只画可见区，帧尖峰更低） | 两种场景要的是相反的东西 |
| **覆盖层按需重绘** | `position:fixed` 层钉在视口上，滚动不影响它；只在数据/按压态/换页时重画 | 一次全屏遮罩的重绘是 4ms |
| **swiper 只在换页时出帧** | 换页做 0.3s 滑动过渡（与微信一致），其余时间不要求出帧 | 从前「有多于一项就每帧重绘」，而 swiper 平时根本不动 |
| **固定节拍限速** | 下一帧时点是「上一时点 + 一个刷新周期」的累加，睡到差 0.9ms 再自旋对齐 | 从「本帧开始 + 周期」算的话，每帧都把 sleep 的过冲算进新起点，节拍越走越偏。平均帧率看着达标、实际帧间隔在抖，正是「高刷没体现出来」的手感 |

`MINI_FPS=1` 会打印帧间隔区间，这个数比平均帧率更能说明手感：优化后商城首页稳定在 **6.7~7.2ms**（144Hz 的节拍是 6.94ms），此前是 **6.9~25ms**。

> 已知还没做完的一条：`setData` 会让整棵渲染树重建 + 整条带重绘（首页秒杀倒计时每秒一次，约 18ms 的单帧尖峰 = 掉一帧）。彻底解决要做增量失效 —— 对比新旧渲染树，只重画真正变了的节点。目前只做到了「动画帧」的损伤区重绘，数据变化仍走整帧。

> 明确没做的事：不引入 GPU（项目定位就是"给我一块像素缓冲我就能画"），不做元素级位图缓存（与动画/裁剪栈交互复杂）。抗锯齿也**没有**换成子采样：圆和圆角的覆盖率用的是有符号距离的线性斜坡，对曲率半径远大于像素的边界，面积占比在法线方向本来就是线性的 —— 换成 4×4 子采样反而量化成 1/16 档，实测与 Chrome 的差异从 4.7% 涨到 5.3%。

### 滚动手感是按 iOS/微信那套做的

滚动不是"位置加减再夹到边界"：

- **越界走橡皮筋**：超出边界的位移按 `1 - 1/(x·0.55/d + 1)` 衰减（`d` 取真实视口尺寸，页面内的小 `scroll-view` 用自己的高度而不是整屏高）。
- **手势结束回弹**：触控板给 `TouchPhase::Ended`（winit 把 macOS 的动量阶段也映射进来），据此立刻回弹；鼠标滚轮没有抬手事件，由控制器的静默计时兜底。
- **拖拽与滚轮同一套语义**：滚轮/触控板维护一个"未夹紧"的累计位置，再经同一个橡皮筋映射成显示位置，所以两种输入的手感一致。
- **内容变短要收回位置**：换页或列表收起后如果位置还停在原处，会停在画布之外的空白上，而且因为"不越界"永远不会触发回弹。

窗体在拿到第一帧的真实内容高之前，滚动上限是 0（不是某个写死的常数）—— 否则内容不足一屏的页面在首帧前就能被拉出几百像素空白。

**按压反馈**支持两种写法，两者共用同一条代码路径：CSS 的 `:active`，和小程序的 `hover-class`。建树期就把「按压态整套样式」算好（把目标元素标记为 pressed 并补上 hover-class 的类名，重新取一次 CSS 声明），绘制期按需切换；声明了 `transition` 就在常态与按压态之间按缓动插值。代价是按压只影响绘制类属性（背景/颜色/透明度/transform）—— 按一下就重排整页在纯软件光栅上太贵。

> 这里原来有个隐蔽的坑：未知伪类一律「宽松放行」，于是 `.btn:active{background:orange}` **永远生效**，元素看起来一直是按下态。`:hover` / `:focus` 在触屏语义下不成立，现在一律不匹配。

**JS 操作样式**走微信的正式 API：`wx.createAnimation()` 链式累积一步的目标值，`step()` 定格，`export()` 交给 `animation="{{animData}}"`。渲染层按 actions 顺序插值（translate / rotate / scale / skew / opacity / backgroundColor），起始时刻按载荷指纹记忆 —— 同一份 export 继续播，换了新的就重新开始。编译出的 H5 有一份同语义实现（映射成 CSS transition），两端行为一致。`width`/`height` 会引发重排，与 CSS 动画同样的取舍：不支持。

**下拉刷新**按微信的语义分成两件事：回弹一直有，指示器与回调由页面 json 的 `enablePullDownRefresh` 决定（缺省继承 `app.json` 的 `window`）。下拉不到 64px 只回弹；到位松手才进入刷新态 —— 内容被按住在露出指示器的位置（等价 iOS 的 `contentInset.top`），三点指示器循环呼吸，直到逻辑层调用 `wx.stopPullDownRefresh()` 才收回归位。`wx.startPullDownRefresh()` 可以从逻辑层主动进入。原生端与编译出的 H5 用同一套阈值、同一条橡皮筋公式、同样的三点外观。

### 弹窗不穿透

`position: fixed` 层画在页面之上，坐标是视口坐标（不含滚动偏移），所以命中判定必须**先只在覆盖层里找，命中就到此为止**：

- 事件绑定带 `is_fixed` 标记，命中冒泡可以只在覆盖层或只在正常流里做。从前是「拿全局命中结果，再用包围盒是否落在视口内来猜它是不是 fixed」—— 页面顶部的普通元素同样满足这个条件，于是弹窗弹着也能点到底下的商品。
- 覆盖层各子树根的视口包围盒被记下来：**落在里面的点击一律由覆盖层消费**，即使那一点没有任何处理器（典型就是只写了半透明遮罩）。
- 按压态、拖动、滚轮同样被拦住：弹窗弹着的时候页面不该滚动，下层的输入框也不该获得焦点。
- 覆盖层在「内容没变」的帧里跳过重绘（性能优化），但事件绑定每帧清空重建 —— 跳过的帧要把上一次的绑定与遮挡区域补回来，否则那些帧里点击照样穿透。

> 这里还连带修掉一个更隐蔽的问题：带 `scale`/`rotate` 的容器走离屏合成路径，而那条路径**直接把子树的事件绑定丢掉了**（原注释写作「已知限制」）。弹窗普遍用 `animation: popIn`（含 scale）做入场，结果弹窗里的关闭按钮、领取按钮全是死的，点击还会穿到下层。现在改为在离屏那趟之后，按**未变换**的真实几何重新注册一遍绑定 —— 对「围绕中心缩放/旋转」这类实际用法足够接近。

`--click <x,y>` 可以脚本化验证这套语义：它走的是与交互窗体完全相同的命中/冒泡/导航链路。

```bash
# 弹窗弹着点弹窗外的遮罩 -> onCloseCoupon；点弹窗内部 -> onNoop（不导航）；
# 点弹窗里的按钮 -> onClaimCoupons；关掉弹窗再点页面 -> 正常导航
cargo run --release --bin mini-app-window -- sample-app --snapshot target/t \
    --route pages/index/index --settle 1 --click 180,300
```

> 有一条不变量必须守住：**上屏用的滚动偏移，必须等于画布上那条带被绘制时的偏移**。页面画布只画「视口 ± 裁剪余量」，偏移变了却没重画，上屏就会取到没画过的区域 —— 表现为滑动时一片空白、停下来内容才出现。所以帧循环里按位置比对来决定重绘，而不是依赖每条输入路径都记得置脏标记（漏一处就会露白）。

### 触控与事件按 Skyline 对齐

从「能点」到「像小程序」差的是四层东西，这一轮四层都补上了。

**1. 绑定语法是通用解析，不是属性名白名单。** 从前引擎按固定属性名认事件（只有 `bindtap`/`catchtap`/`bindchange`…），于是 `bindtouchstart`、`bind:tap`、`catchtouchmove`（遮罩锁滚动的标准写法）、`capture-bind:`、`mut-bind:` 全部被无声忽略 —— 自定义手势、事件捕获、互斥绑定统统没有。现在按**长前缀优先**解析 `capture-catch:` / `capture-bind:` / `mut-bind:` / `catch:` / `bind:`（冒号可省），事件名任意（自定义组件的 `bind:myEvent`、uni-app 的 `bind:__l` 都走同一条），`data-*` 共享给同一节点上的每条绑定。继续枚举属性名是没有尽头的，所以拒绝白名单。

**2. 传播顺序在一处收口。** `dispatch_chain(x, y, event_type, scope)` 一次给出按序要调用的绑定：捕获阶段由外向内 → 冒泡阶段由内向外，`catch*` 触发后终止（含自己），`capture-catch:` 在捕获阶段就掐断整条链，`mut-bind:` 同组只触发最内层那条而 `bind` 照旧。层级用**包围盒面积升序**近似（子节点面积必然不大于父节点）—— 给 `RenderNode` 与整条绘制递归加 depth 参数改动面太大，而面积法就是现网一直在用的判据。

**3. 触摸序列是真的序列。** 宿主的指针事件先进一个纯逻辑状态机（`TouchTracker`），再翻成微信语义：`touchstart` 在宿主决定怎么处理这次按下**之前**就派发（页面自己实现的手势才拿得到起点）；移动超过 10px 的 slop 取消 tap 与 longpress；按住 350ms 发一次 `longpress`；被滚动接管的那一刻补一次 `touchcancel`（手势竞争的失败方要能收尾）。事件对象由 Rust 侧构造后交给 `__dispatchEvent`：`type`/`timeStamp`（页面打开至今）/`target`/`currentTarget`/`detail`/`touches`/`changedTouches`，`touchend` 与 `touchcancel` 的 `touches` 为空。

- `target` 取命中点上**最内层的绑定**（不限事件类型），`currentTarget` 是挂处理函数的那个节点。列表项常见写法是 `bindtap` 只写在外层容器上、行号靠 `e.target.dataset.id` 取 —— 用链首当 target 的话页面永远拿到容器的 dataset，表现是「点哪一行都当第一行」。
- **tap 没有时长上限**。旧实现要求 `elapsed < 300ms` 才算点击，按住一会儿再松手就点不动了；微信里按住两秒松手同样是一次 tap。
- 惯性滚动中按下：那一下只是**停住**，不算点击（iOS/微信一致）。

**4. 手势仲裁：方向锁定 + 嵌套传递。** 归属**不在按下时决定**，而是等第一次明显位移（4px）按主方向锁定：

| 场景 | 旧行为 | 现在 |
|------|--------|------|
| 横向 `scroll-view`（首页那排卡片）里竖着划 | 被横向容器吃掉，什么都不动 | 页面滚动 |
| 纵向列表里横着划 | 页面跟着上下跳 | 谁也不动 |
| 内层列表滚到底继续上划 | 卡住 | 交棒给页面继续滚 |
| 内层内容不足一屏（`max_scroll == 0`） | 短列表挡住整页滚动 | 直接交给页面 |
| 遮罩上写 `catchtouchmove` | 照样滚 | 锁住 |

嵌套传递的判据是「到边界且仍朝该方向推，**或** `max_scroll <= 0.5`」。曾经用「位置没变」判断，但控制器有橡皮筋越界 —— 到边界后位置照样在动，短列表那个场景实测失效。手势候选取的是命中点上**最内层的 `scroll-view`**（`hit_test_scroll_area`），不是最上层的元素：卡片、按钮盖在 `scroll-view` 上面时普通命中测试返回的是卡片，于是真正装内容的容器永远得不到手势。

**5. 左边缘侧滑返回。** 起点落在最左 20px 且栈里还有上一页时，往右划升级成宿主级手势：位移 1:1 跟手，上一页在下面按视差（`(1-进度)·屏宽/3`）露出来并轻微压暗，交界处有投影；松手时位移过 35% 屏宽或速度过 320px/s 就 `navigateBack`，否则滑回去、页面状态一点不变。栈底（tab 首页）没有这个手势，弹窗/picker 弹着时也不开。判定与收尾动画是纯逻辑，合成是一段像素搬运（`copy_within` + 逐行填充），两者都能离开窗体单测。

- 下面那一页必须真的显示出来，而那时它已经不是当前页 —— 渲染器、交互表、滚动位置全换了新页，没法重新画。所以在**它被覆盖的那一刻**用 `present_to_buffer` 留一张视口图（与真正上屏的合成结果逐像素一致），返回时用完即弃。
- 合成放在所有覆盖层**之上**：被推走的是「整个页面」，页面自己的弹窗、Toast 要跟着一起走，否则弹窗会诡异地钉在原处。

> 这一轮踩到最贵的一个坑：**页面滚动分支不能置 `needs_redraw`**（侧滑跟手同理）。整页内容已经画在长画布上，滚动只是换一条切片上屏；置脏等于把「滚动不重绘」那条优化整个废掉（实测首页拖动 3.7ms → 6.5ms）。`scroll-view` 内滚动仍要置脏 —— 它的内容是按自身偏移画进整页画布的，没有独立切片。

> 另一条：无头输入与交互窗体必须**统一到同一组入口**（`on_pointer_press/move/release`）。此前两套逻辑并存，结果无头链路驱动的是 `scroll-view`、真机驱动的是页面滚动 —— 无头全绿而手上不对，测的根本不是同一套东西。

无头验证这些语义：

```bash
# 一次完整触摸序列（按住 600ms 会真的触发 longpress，因为按住期间照常出帧）
mini-app-window sample-app --route pages/index/index --touch 180,300,600 --snapshot target/t
# 滑动手势：按下 → 逐步移动 → 抬起（惯性/回弹会走完再截图，所以两次跑结果一致）
mini-app-window sample-app --route pages/index/index --swipe 200,500,200,60,20 --snapshot target/t
# 停在终点不抬手：用来截「手势进行中」的那一帧（侧滑跟手的位移、视差、投影）
mini-app-window sample-app --eval "wx.navigateTo({url:'/pages/detail/detail'})" \
    --swipe-hold 5,400,160,400,20 --snapshot target/t
```

侧滑返回与 `wx.navigateBack()` 的结果是逐字节一致的（同一条退栈路径），这也是它的回归判据。

> 顺带修掉两个既有 bug：① `navigate_back` 没有重建上一页的自定义组件实例（往前走时 `__resetPageComponents()` 把它一起作废了），于是从二级页返回首页，底部那条自定义 tabBar 整条消失 —— 现在返回时按页面 json 的 `usingComponents` 重新挂载，「返回首页」与「直接进首页」的快照逐字节一致。② 静态渲染路径（画廊/离屏/双端对比）画固定层时忘了置 `registering_fixed`，覆盖层里的事件绑定会以**内容坐标**混进正常流那张表，页面一滚就停在旧位置反过来挡住真正的元素。

### setData 只重绘变化的那一块

`setData` 曾经一律整帧重绘。首页那种长页面一帧要 13~18ms，而秒杀倒计时每秒 setData 一次 —— 于是稳定的 145FPS 里每秒插进一个长帧，丢掉两三帧。平均帧率看着很高，手上就是**每秒一顿**。

现在布局重建之后会把新旧两棵 `RenderNode` 树连同各自的 taffy 布局并行走一遍，只收集「文本/属性变了」或「几何变了」的节点包围盒（几何变化要同时算上旧位置和新位置，否则旧像素擦不掉），并据此裁剪这一帧的清屏与绘制。倒计时那一下的失效范围是 **45×33 像素**，而不是整屏。覆盖层同理：变化不在 `position:fixed` 子树里，就直接复用上一张覆盖层画布（首页那个弹窗从前要陪着每秒重画一次，白付 4ms）。

判断依据只看 `text` 与 `attrs`：样式是由标签 + class/style 属性 + 祖先链 + 兄弟序号推导出来的，而结构相等已经由这趟并行遍历本身保证。**一切拿不准的情况都退回整帧**：结构变了、跑到 `scroll-view`/`swiper`/带 transform 的子树里（那些子树的绘制坐标不等于布局坐标）、影响面超过视口三分之一。少画一块留下脏像素，比多画一次严重得多，所以偏置是刻意保守的。

这条优化必须配一道逐像素对照校验，否则「少画了一块」这种 bug 只会以偶发脏像素的形式出现：

```bash
# 同一次 setData 分别走增量路径与强制整帧路径（MINI_NO_DAMAGE=1），截图必须逐字节一致
bash tools/damage-check.sh
```

> 这道校验第一次跑起来是**空转**的：`--eval` 原本在首帧之前执行，于是两边都走整帧路径，全绿但什么也没测到。现在注入前会先出一帧 —— 真机上 `setData` 永远发生在已渲染的页面上。诊断开关 `MINI_LAYOUT_LOG=1` 会打印每次 setData 的失效范围，以及退回整帧时的具体原因，用来确认校验没有再次变成空转。

> 连带修掉一个隐蔽的耦合：局部重绘帧只访问了裁剪范围内的节点，收集到的「动画元素包围盒」是不完整的。拿它覆盖旧记录，下一个动画帧的损伤区就只剩这一小块，范围外的动画会**就此冻住**。现在只有「裁剪范围覆盖了全部动画元素」的帧（整帧、或损伤区本就由动画包围盒算出来的帧）才更新这份记录。

### 局部重绘要「少算」，不只是「少画」

设了损伤区之后，绘制期的视口裁剪也要把**离损伤区太远的子树整棵剪掉**。否则一个
45×33 的倒计时损伤区，照样要把视口里六百来个节点逐个走一遍、各自发起绘制，
最后才在像素级被裁掉 —— 白付约 2.5ms，而整帧预算只有 6.94ms。

> 这里踩过一个自己挖的坑：绘制入口原本写的是 `self.damage_clip.take()`。像素级裁剪
> 照样正确，所以看不出问题，但 `cull_outside_viewport` 拿到的永远是 `None`，
> 子树裁剪等于没生效。**改完没量到收益时，先确认自己的开关真的通到了目标位置。**

### 组件的时钟不能挂在绘制上

swiper 的自动播放原先只在 `draw_swiper_container` 里推进。于是它一旦没被画到
（滚出视口被裁掉、或者局部重绘帧里整棵子树被剪掉），`last_update` 就永远停在原地，
`elapsed >= interval` 恒成立 → 每帧都报「我要出帧」→ 宿主每帧整屏重绘。
带轮播的长页面因此被永久钉在 ~100FPS，帧间隔 14~17ms 忽大忽小。

现在自动播放由宿主每帧推进一次（`advance_due_swipers()`），与是否被绘制解耦：
翻页只在到点那一帧发生，其余帧 swiper 不再索要重绘。**凡是「状态推进」写在绘制路径里
的组件，都会在裁剪优化面前变成隐性的每帧重绘源。**

### 超时的那一帧不要再补睡一个周期

帧限速原本在「落后超过一整帧」时把下一帧对齐到 `now + 一个周期`。但一帧已经干了
11ms 活、本来就超预算了，再补睡 6.9ms 会把**帧间隔**顶到 17ms —— 手上感觉到的顿挫
是帧间隔，不是干活时间，白睡这一下等于把一次超时放大成两帧。现在直接对齐到 `now`。

首页（长页 + 轮播 + 每秒倒计时 setData）实测：

| | 帧间隔峰值 | 最慢一帧 |
|---|---|---|
| 本轮之前 | 24.9ms | 17.9ms |
| setData 增量失效 | 19.9ms | 12.4ms |
| + 损伤区裁剪 | 17.0ms | 10.0ms |
| + swiper 解耦 & 限速修正 | **11.4ms** | **11.4ms** |

稳态 144FPS / 6.9~7.0ms。剩下的一次超时是每秒那一帧 setData 仍要**整棵重建布局树**
（模板 1.0ms + 建树与样式 4.0ms + 布局 0.3ms），要压到预算内需要增量树打补丁 ——
基础设施（`FramePlan`、损伤区、`tools/damage-check.sh`）已经就位。

> 反过来，**页面滚动本身不该触发重绘**。页面画布是整页高的、用内容坐标，滚动只是让上屏按新偏移取不同的切片（上屏每帧都做）。所以「正在滚动」不进重绘闸门 —— 只有滚出已绘制条带那一刻才补画一次。曾经把「正在滚动/惯性中」也算进闸门，于是滚动的每一帧都整条带重画（约 6~7ms），刚好卡在 144Hz 的 6.9ms 预算边缘，偶尔超一点就丢帧 —— 这正是「滑动像抖动、高刷没体现出来」的根因。改掉之后滚动的绝大多数帧只花上屏的拷贝时间（约 2~3ms），只有跨条带的那一帧才有一次整页重绘的尖峰。`scroll-view` 内滚动仍要重绘（它的内容按自身偏移画进整页画布，没有独立切片）。

### picker 选择器

`<picker>` 不是页面里的一段 DOM，而是宿主弹出的底部浮层（和微信一致）：点一下弹出、选好按「确定」才回调 `bindchange`，点「取消」或遮罩直接关。渲染层只负责把「这里有个 picker、它有哪些选项、当前选到第几个、变更回调叫什么」登记下来（`bindchange` 本身不是可命中事件，靠事件绑定表找不到它），面板的状态机 / 绘制 / 命中在宿主侧。

五种 mode 都支持：`selector`（单列下标）、`multiSelector`（多列下标数组）、`time`（时/分）、`date`（年/月/日，`fields` 控制到哪一级、`start`/`end` 限定年份、换月自动重算天数含闰年）、`region`（省/市/区联动，内置一份行政区划表）。面板入场/退场是 ease-out 位移动画，滚轮离中心越远越淡。编译出的 H5 有一份同构实现（底部面板 + scroll-snap 滚轮），`detail.value` 的格式与原生端逐一对齐。

### 双端一致性是被量化验证的

引擎自带对比工具：同一份小程序源码，一边走原生渲染出 PNG，一边编译成 HTML 用 Chrome 截图，逐像素比对并产出报告。

```bash
cargo run --example compare -- --all --out target/render-compare
# 输出 rust.png / html.png / side-by-side.png / diff.png + report.json（变化像素比、MAE、RMSE）
```

更进一步，**被对比的可以是窗体宿主真实出的那一帧**：窗体带无头快照模式，走与交互运行完全相同的管线（页面加载、`app.wxss` 合并、自定义 tabBar、fixed 覆盖层、Toast/Modal、像素合成顺序）输出整帧 PNG，再交给对比工具。这样报告衡量的是「用户真正看到的画面 vs H5」，而不是示例里另写一份渲染。

```bash
# 整帧快照：可指定动画时刻 / 真实等待 / 滚动位置 / 注入交互后的状态
cargo run --release --bin mini-app-window -- sample-app --snapshot target/window-snap
cargo run --release --bin mini-app-window -- sample-app --snapshot target/s \
    --route pages/components/components --scroll 760 --time 0.45
cargo run --release --bin mini-app-window -- sample-app --snapshot target/s \
    --route pages/category/category --eval "__currentPage.onPlus({currentTarget:{dataset:{id:101}}})"

# 让窗体的帧参与逐像素对比
cargo run --release --example compare -- --all --rust-from target/window-snap --out target/window-compare
```

`--time` 与 `--settle` 是两件事，别混用：

- `--time <秒>`：把 CSS 动画时钟拨到某个时刻求值，**不消耗真实时间**。双端对比走这条 —— H5 侧为了截图确定性刻意禁用了页面脚本，那边的 JS 状态也不会往前跑。
- `--settle <秒>`：真实等待，让 `setInterval`/`setTimeout`、延时弹层、轮播自动播放跑起来。要「和真机一样」的画面时用它。`--eval` 在 `--settle` 之后执行，所以脚本里"关掉优惠券弹层"这种操作不会被随后的 setTimeout 又打开。
- `--frames <N>`：按刷新率跑 N 个**与交互窗体同一套闸门/损伤区逻辑**的帧再截图。局部重绘这类只在连续出帧时才暴露的问题（某个动画元素被漏出损伤区而静止），单帧快照永远走整帧重绘，测不出来。

```bash
# 真实运行 4 秒后截图：新人券已弹出、秒杀倒计时已走到 01:59:56
cargo run --release --bin mini-app-window -- sample-app --snapshot target/real \
    --route pages/index/index --settle 4 --time 4
```

含 `<canvas>` 的页面是个例外：canvas 的画面只能由 JS 画出来，静态 HTML 里是一块空白，所以对比工具对这类页面保留脚本 —— 否则等于拿"空画布"当参考基准，原生端画对了反而会让差异变大。

当前实测（375×667 @2x，窗体整帧 vs Chrome）：`sample-app` 15 页整体差异 **4.66%**（单页 1.5%~8.0%），`news-app` 6 页 **6.47%**，剩余差异集中在粗体字形与亚像素文本位置。

### 编译器：同一份源码，编译出别端源码

除了原生渲染，引擎还包含一个多目标编译器（`src/compiler/`）。它复用同一套解析器与数据快照，把小程序编译成**目标端的源码工程**，而不是套壳运行：

```bash
cargo run --bin mini-compiler                      # sample-app → dist-html/（裸名字即可）
cargo run --bin mini-compiler news-app dist-news html
```

HTML 目标产出结构化工程（`common/base.css` + 每页 html/css/js + 运行时），支持事件、`setData` 响应式重渲染、`model:` 双向绑定、`wx.*` 跳转、自定义 tabBar 切换。新增目标端只需实现 `CompileTarget` trait 并在 CLI 注册。

---

## 目录

- [渲染原理](#-渲染原理这引擎和别的有什么不一样)
- [特性总览](#-特性总览)
- [快速开始](#-快速开始)
- [浏览器调试预览](#-浏览器调试预览)
- [场景画廊](#-场景画廊64-个真实渲染)
- [视频与 Canvas](#-视频与-canvas)
- [支持的组件](#-支持的组件)
- [CSS 支持](#-css-支持)
- [小程序生命周期](#-小程序生命周期)
- [架构](#-架构)
- [素材图片生成](#-素材图片生成)
- [多平台 SDK 编译](#-多平台-sdk-编译)
- [使用示例](#-使用示例)
- [测试](#-测试)
- [项目结构](#-项目结构)

---

## ✨ 特性总览

| 能力 | 说明 |
|------|------|
| 🎨 **2D 渲染引擎** | 纯 Rust；抗锯齿描边/圆角/图片、Alpha 混合、阴影、渐变、双线性采样 |
| 📐 **Flexbox 布局** | 基于 [Taffy]；含文本按容器宽度自动换行的二次布局修正 |
| 🧩 **30+ 组件** | view/text/image/button/input/scroll-view/swiper/**canvas**/**video** 等 |
| ⚡ **QuickJS 运行时** | 完整 JS，App/Page/Component、Behavior、CommonJS 模块、Promise 微任务 |
| 🎯 **CSS 选择器引擎** | 标签/类/`#id`/`*`/属性/组合器/特异性/`@import`/`var()`/`calc()` |
| 📄 **WXML 模板** | `wx:if/elif/else`、`wx:for`、`<block>`、真正的 `{{ }}` 表达式 |
| ♻️ **完整生命周期** | App / Page / Component 三级钩子 + 组件 `pageLifetimes` 联动 |
| 🎬 **视频播放** | 手写 MP4 解复用 + openh264 解码 + rodio 音频（桌面端 macOS 音频接口） |
| 🖌️ **Canvas 2D** | `wx.createCanvasContext` 全套 API：路径/圆弧/文本/图片/渐变/变换 |
| 👆 **事件系统** | `bindtap` 冒泡、`catchtap` 阻止冒泡、`dataset`、输入事件 |
| 🖱️ **交互** | 滚动惯性/回弹、输入框、勾选/单选/开关/滑块 |
| 🌐 **浏览器调试** | 内置 HTTP 服务，浏览器打开链接即可实时操作 UI |
| ⏱️ **高刷适配** | 事件循环按显示器刷新率出帧（60/120/144Hz），空闲零占用 |
| 🔗 **多端集成** | C FFI（`mr_*`），可编译为 5 大平台原生库 |

---

## 🚀 快速开始

```bash
# 1) 安装 Rust（若未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2) 构建
cargo build --release

# 3) 跑一个小程序：交互式选择
./run.sh                            # 列出 sample/ 下的小程序，输编号或名字
./run.sh sample-app                 # 直接指定（裸名字 / sample/xxx / 任意路径都行）
./run.sh 4                          # 按菜单编号
./run.sh --list                     # 只看有哪些
./run.sh --fps sample-app           # 带帧率诊断跑

# 4) 其它示例
cargo run --example gallery         # 渲染全部 64 个场景到 doc/gallery/
cargo run --bin mini-devserver      # 浏览器调试预览（见下节）
cargo run --example video_player    # 独立视频播放窗口（自动循环 + 声音）
cargo run --bin mini-app-window     # 窗口应用（默认加载 sample-app）

# 5) 测试（281 个用例）
cargo test
```

### 示例小程序放在哪

全部收在 `sample/` 下，**一层子目录里有 `app.json` 就算一个小程序**，`run.sh` 与
`mini-launcher` 都按这个规则扫描（所以 `sample/_archive/` 这类归档不会被当成示例）：

| 目录 | 页数 | 说明 |
|------|------|------|
| `sample/sample-app` | 15 | 商城：轮播 / 秒杀倒计时 / 优惠券弹层 / 自定义 tabBar / Canvas |
| `sample/tea-app` | 35 | **uni-app 编译产物**（Vue 3 运行时 + 60 个 CommonJS 模块），验证真实工程链路 |
| `sample/news-app` | 6 | 资讯：长列表 / 视频 / 富文本 |
| `sample/real-sample` | 8 | 微信官方 demo |

命令行里写**裸名字**就行（`sample-app`），路径由 `src/app_dir.rs` 统一解析 ——
裸名字、`sample/xxx`、任意绝对/相对路径都接受。把这件事收在一处，是为了避免
「迁一次目录要改十几处硬编码字符串」。

---

## 🔄 编译为 HTML 工程（transpile）

因为 WXSS 基本是标准 CSS、WXML 与 HTML 结构一一对应，引擎内置**源码编译器**，把小程序
编译成浏览器可原生渲染的 HTML + CSS（`src/transpile.rs`）：

- WXML(+data) → HTML（`wx:for/wx:if/{{}}` 展开 + 标签映射；`bindtap`→`data-tap`、`data-*`→`data-ds-*`）
- WXSS → CSS（`rpx`→`px`，1rpx=0.5px @375；其余样式浏览器直接识别）

```bash
cargo run --bin mini-compiler                    # sample-app -> dist-html/（静态 HTML 工程）
cargo run --bin mini-compiler <小程序根> <输出目录>
# 打开 dist-html/index.html 查看每个页面编译出的独立 HTML
```

> 这既是「一套源码、两套 UI」（Rust canvas 渲染 + 浏览器 HTML 渲染）的基础，也是后续
> 「编译到 Android / iOS 原生源码」的中间层。

## 🌐 浏览器调试预览（HTML 直渲染，极速）

无需模拟器，在**浏览器里直接操作小程序 UI**：内置 HTTP 服务把页面编译成 HTML/CSS 交给
浏览器**原生渲染**（不再逐帧渲染图片），点击即时命中事件、`setData`/页面跳转后返回新的
HTML 片段做局部替换——单次交互 ~3ms，真实 DOM 可交互。

```bash
cargo run --bin mini-devserver                    # 默认加载 sample-app（首页）
cargo run --bin mini-devserver <小程序根> [端口]   # 端口被占用会自动顺延
```

启动后打开终端输出的 `http://127.0.0.1:9000`。仅使用 Rust 标准库网络，无额外依赖。

| 路由 | 作用 |
|------|------|
| `GET /` | 编译好的整页 HTML（base.css + 页面 CSS + body + 运行时） |
| `POST /event` | 命中事件 → 调用页面方法/`setData`/导航 → 返回新 HTML 片段 |
| `POST /back` | 返回上一页 |

---

## 📱 可用的示例小程序

`sample/sample-app`（商城）与 `sample/news-app`（头条新闻）不是静态样板，而是**功能闭环**的小程序，原生窗体与编译出的 H5 行为一致：

| | `sample-app` 商城（15 页） | `news-app` 新闻（6 页） |
|---|---|---|
| 首页 | 自动轮播 banner、金刚区、**进页即弹新人优惠券**（缩放入场动画）、限时秒杀真实倒计时、触底加载、下拉刷新 | 要闻轮播、频道横滑切换、下拉刷新、触底加载、卡片收藏 |
| 详情 | 图片轮播、规格半屏弹层、加购 / 立即购买、rich-text 图文、评价 | 正文字号设置**即时生效**、评论发布、点赞 / 收藏 / 关注持久化、相关阅读 |
| 交易 · 互动 | 购物车（storage 持久化）→ 确认订单（地址 / 配送 picker / 优惠券 / 支付方式 / 备注）→ 提交下单 → 订单列表 + 物流时间轴 + 确认收货；搜索页含历史与热搜 | 搜索（历史 + 热搜榜 + 排序）、热榜 + 签到日历 + Canvas 阅读统计 |
| 能力总览 | `pages/showcase`：CSS 动画、transform、彩色 emoji、竖向轮播、GIF、表单控件全家桶、进度 / 评分 / 徽标、rich-text、Canvas 图表、Toast / Loading / Modal / 操作面板 | 视频页可真实播放 `<video>`，自动播放开关持久化 |
| 返回上一页 | 二级页自绘返回栏（`wx.navigateBack`）；逻辑层页面栈复用实例，返回不会重跑 `onLoad` | 同上 |

跨页数据（购物车、订单、收藏、设置、搜索历史）统一走 `wx.storage`，所以编译成 H5 后即使每个页面是独立文档也不丢状态。

```bash
./run.sh sample-app     # 商城
./run.sh news-app       # 新闻
./run.sh tea-app        # uni-app 编译产物
./run.sh real-sample    # 微信官方 demo
```

### 还能跑真实工程的编译产物

`sample/tea-app` 是 **uni-app 编译到 mp-weixin 的产物**：36 个 wxml + 一个 259KB 的
Vue 3 运行时 + 60 个 CommonJS 模块。跑通它需要三件事，都是引擎侧的通用能力：

- **每个 `.js` 都按 CommonJS 模块作用域执行**（`module` / `exports` / 相对 `require`）。
  从前 app.js 是当裸脚本 eval 的，TS / uni-app 产物第一行 `Object.defineProperty(exports, …)`
  就 `ReferenceError`。页面 js 同样按模块执行但**不缓存** —— 每次进入要重跑，
  `Page({...})` 才会重新注册。
- 启动时把小程序目录下所有 `.js` 预注册成模块，`require('./common/vendor.js')` 才解析得到。
- `global` / `self` 指向 `globalThis`：打包器与框架运行时普遍靠这些别名做环境探测。
- **支持用 `Component()` 构造器定义页面**（微信允许，uni-app / Taro 就走这条）：它们的
  `wx.createPage` 把 Vue 组件选项转成组件定义后调 `Component()`，而不是 `Page()`。
  只认 `Page()` 的话页面永远注册不上，`__currentPage` 为 null、data 全空 ——
  模板里 `{{a}}` 渲染成空串、`bindtap="{{c}}"` 拿不到处理函数名，屏幕上只剩一张静态骨架。
  组件式页面还要真的跑 `created` / `attached`（框架在 attached 里挂载组件并触发首次
  `setData`），并补上组件实例该有的 `properties`（**求值后的属性值**，不是类型表）与
  `triggerEvent` 等方法。
- `wx.getLaunchOptionsSync` / `wx.loadFontFace`：框架在 `onLaunch` 里必调。
  远程字体本引擎不支持（字体来自系统），但**必须存在且回调 `success`** ——
  抛异常会打断整个 `onLaunch`，框架的响应式层就此起不来。

> 能定位到这些，前提是**报错得有内容**。这轮为此加了三件诊断，缺一个就得回去通读
> 几十万行 vendor 包：
>
> 1. `Error` 的 `message`/`stack` 是不可枚举属性，原来对异常对象做 `JSON.stringify`
>    永远得到 `{}` —— 屏幕上只有 `JS Exception: {}`。现在打印 `name: message` + 栈。
> 2. `console.error(err)` 也要带栈。框架的错误处理器就是这么调的，而 `String(err)`
>    只剩一句 `TypeError: not a function`，没有名字也没有位置。
> 3. **未实现 API 探针**：`wx` 外面套一层 `Proxy`，读到尚未实现的键就警告一次
>    （`[未实现的 API] wx.getLaunchOptionsSync`）。关键是**仍返回 `undefined`** ——
>    框架普遍用 `typeof wx.xxx === 'function'` 做能力探测，返回假函数会让它们走进
>    不存在的分支。
>
> 顺带把 `real-sample`（微信官方 demo）也一起修活了：它挂在模块作用域这同一个原因上。

### 远程图片必须异步加载

`<image src="https://…">` 曾经是在**绘制期同步** `ureq::get` 的 —— 渲染线程直接卡在 HTTP
往返上。实测 tea-app 首屏背景图 276KB / 3.2s，也就是那一帧卡 3.2 秒；长列表里每滚出
一张新图就再卡一次。**这是「很多页面滑动很卡」的直接原因**，跟布局、光栅化都无关。

而且失败结果被永久缓存成 `None`：网络抖一下，那张图这辈子都不会再出现（「图片没展示」）。

`src/renderer/components/image_net.rs` 改成和浏览器一致的三段式：

- 绘制期**只查表**，没有就登记「下载中」并立刻返回（本帧画占位），永不阻塞；
- 后台线程下载 + **解码**（解码大图同样贵，不能放回渲染线程），完成后置脏，宿主重绘一帧；
- 失败带**退避重试**（2s 起翻倍，上限 30s，最多 4 次），不再一次失败终身失败；
- **磁盘缓存**（`target/mini-imgcache/`，URL 哈希命名 + 先写临时文件再重命名）：
  同一张图第二次打开 6132ms → **11ms**。没有它，tea-app 那张 6 秒才下完的闪屏背景
  永远来不及在 5 秒的闪屏里出现。

有个配套细节容易漏：宿主必须在「有图在下载」时保持出帧、在「图下载完」时置脏重绘，
否则图片下完了也要等下一次别的原因触发重绘才显示。

### `page { … }` 与绝对定位的包含块

两个让 tea-app「差距有点大」的渲染语义缺口：

- **`page` 选择器之前完全被忽略**。页面底色写死 `#F5F5F5`，文字继承从内置默认值起步。
  于是在 `page` 上定义暖米色底（`#f7f3ec`）+ 宋体 + 字号的应用，整体色调与字形全对不上。
  现在 `page` 的 `background-color` 作为清屏色与画布外填充色，`color / font-size /
  font-weight / line-height / letter-spacing / text-align` 作为继承链的起点。
- **`position:absolute` 且没有定位祖先时，百分比高度按视口折算**。CSS 规则是「包含块为
  最近的定位祖先，没有则是初始包含块（视口）」，而布局引擎只按父节点解析百分比 ——
  父节点是 auto 高度时 `height:100%` 直接塌成 0，整屏铺底的背景图就此消失。

> 这两条里第二条我先做过头了：一开始把**宽度**也按视口折算，结果 canvas 页与组件页
> 与 H5 的差异从 5.4%/3.8% 恶化到 12.5%/10.2%。原因是元素的 `left/top` 偏移仍然相对父
> 节点，只改尺寸不改偏移，窄父节点里的绝对定位元素就会「变宽但不移位」。
> 还试过补一趟「根高度换成确定值」的布局来给百分比提供参照物，那会改变 flex 的分配规则，
> 同样两页整页错位。两次都靠**逐字节快照基线**当场发现并回退了 —— 最终只保留
> 「无定位祖先时按视口折算高度」这一条最小改动，15 页快照与基线逐字节一致、双端 4.66% 不变。

### 网络与缓存：让「拉数据 → 存起来 → 下次走缓存」真的成立

`wx.request` 之前是**完全没有实现**的（`ureq` 只用在图片加载上）。现在补齐：

- `src/net.rs`：请求切成「提交 → 后台线程跑 → 宿主按帧取回」三段，**不阻塞 JS 线程**。
  响应回来后由 `MiniApp::update()` 喂给 `__resolveRequest`，所以 `Promise` 链、
  `async/await` 都能正常推进（它们靠微任务队列，宿主每帧 pump 一次）。
- 微信语义逐条对齐：**4xx/5xx 仍走 `success`**（只是 `statusCode` 不是 2xx）、
  GET 的 `data` 拼进 query、`dataType` 缺省 `json` 且解析失败就原样给字符串、
  `header` 原样透出、`RequestTask.abort()` 之后不再回 `success` 但仍调 `complete`。
- `wx.setStorageSync` **跨启动持久化**（`src/storage_file.rs`，写到
  `target/mini-storage/<小程序名>.json`，按小程序隔离且**不写进 `sample/`**）。
  之前只有进程内的 HashMap，于是每次启动都是空的 ——
  任何「首次拉取存起来、之后走缓存」的逻辑永远只走首次分支，缓存代码等于从没执行过。
- `getSystemInfoSync` 补全 `safeArea` / `safeAreaInsets` / `statusBarHeight` 等字段
  （以及拆分出来的 `getWindowInfo` / `getDeviceInfo` / `getAppBaseInfo`）：
  这些是布局算式的输入，少一个就是一个 `cannot read property 'bottom' of undefined`。

网络语义有 6 个**离线**回归测试（`src/tests/network_tests.rs`）—— 直接调
`__resolveRequest` 模拟原生回调，不依赖外网，所以能进 CI。诊断开关
`MINI_NET_LOG=1` 打印每条请求的方法/URL/状态码/耗时，`MINI_STORAGE_LOG=1` 打印缓存读写。

补上这两块之后 tea-app 的表现变化很直接：首屏倒计时会真的走到 0 并
`switchTab` 进首页，首页从 3384 色（占位方块）变成 85860 色（真实照片）。

> 顺带修掉一个「排查时会把人带偏」的问题：`--frames` 跑帧时不冲 JS 输出缓冲，
> 也不处理导航请求。于是定时器/网络回调里的 `console.log` 全攒在缓冲里看不到，
> 靠时间驱动的跳转也只在日志里出现、页面其实没换 —— 看起来就像「回调没跑」。

**当前边界**：tea-app 是 uni-app x(UTS) 的产物，它的资源缓存工具在**首次运行**
（storage 为空）时会对 `''` 调 `.toMap()` 而抛异常 —— 那是 UTS 的 API，
微信的 `getStorageSync` 读不到时返回 `''`。按「对齐 Skyline」的原则这里不迁就：
**引擎不为偏离微信语义的写法让路**。所以它的首次运行仍会命中这个应用侧的问题，
第二次运行起（缓存已落盘，`getStorageSync` 返回 uni 包装过的 `UTSJSONObject`）一切正常。

### 页面内的自定义组件（`usingComponents`）

`<tab-bar/>` 这种页面 json 里声明的自定义组件，以前落到「未知标签」的兜底分支被当成空
`view` —— **整条底部导航（图标 + 文字）在原生端凭空消失**。而 uni-app / Taro 编译出来的
产物普遍把 tabBar 做成页面内组件（Skyline 下 tabBar 也推荐自绘），所以这不是个别写法。

打通它需要四段：

- **加载**（`src/using_components.rs`）：按页面 json 递归读组件三件套（组件自己也可以再
  声明组件）。样式做**单向隔离** —— 组件 WXSS 的每条选择器前缀成 `<标签名> …`，
  `:host` 改写成标签本身。`.icon` / `.label` 这类通名不做隔离会直接污染页面。
- **展开**（`TemplateEngine::render_with_components`）：保留一个以组件标签命名的宿主节点
  （承载使用方写在标签上的 class/style，也让 `:host` 与作用域前缀有落点），
  子树用**组件实例自己的 data** 求值 —— 组件的 `{{a}}` 与页面的 `{{a}}` 是两套东西。
- **实例**（`component_mount.rs` + `__mountPageComponent`）：以「组件路径」为键执行组件 js
  （`Component()` 借此注册到该路径，而不是把当前页面顶掉），再建实例。实例必须真的建出来：
  框架型产物是在 `attached` 里挂载自己的组件树并触发首次 `setData` 的，
  没有实例的话组件模板里全是空值。使用处的属性按微信规则转成组件属性
  （`u-p` → `uP`、`u-i` → `uI`，dashed → camelCase），uni-app 的 props 就是这么传的。
- **事件**：处理函数可能挂在页面上，也可能挂在组件实例上，`__callPageMethod` 两边都找。
  只找页面的话点了没反应 —— 底部导航点不动就是这个原因。

> 一个连带的大坑：`class="{{['tabbar', d]}}"` 是 uni-app **每个组件/页面根节点**的固定写法
> （`d` 是虚拟宿主类名）。通用插值会把它渲染成字面量 `['tabbar','']`，于是
> `.tabbar` / `.root` / `.page` 这些根节点样式**一条都命中不了** —— 底部导航既没有
> `position:fixed` 也没有背景，整页配色也对不上。现在 `class` / `style` 绑定到数组或对象时
> 按 CSS 语义拼接（空格 / 分号）。

### `font-family` 曾经被整条忽略

全局只有一个系统主字体（macOS 上是黑体系的 PingFang / Hiragino Sans GB），
于是所有声明宋体/衬线的文字都画成黑体。对以「书卷感」为设计语言的应用来说，
这是第一眼就能看出来的差别（tea-app 的品牌名、导航文字、正文全都指定
`"Songti SC","STSong",serif`）。

`src/text_family.rs` 按浏览器的规则解析：**字体栈从左到右取第一个「本机存在」的字族**，
通用族兜底（`serif` → 宋体、`monospace` → Menlo、`sans-serif` / `system-ui` → 默认字体）。
`"Playfair Display","Times New Roman",serif` 在 macOS 上就落到 Times New Roman ——
与浏览器一致，因为 Playfair 确实没装。

- 每个字族一份 `TextRenderer`（自带字形缓存），按规范化后的字体栈字符串缓存，
  同一条 `font-family` 只解析、只加载一次；
- 缺字形时**逐字回退**到系统默认字体：`"Times New Roman"` 没有汉字，
  不回退的话中文全是豆腐块；
- `font-family` 是继承属性，`page { font-family }` 作为整页基线；
- **度量与绘制必须用同一族**：字形宽度、自然行高都随字体变，
  用默认字体量、用宋体画的话盒子和文字对不上，还会误换行。
  所以 `TextMeasure` 带上字体栈，taffy 的度量闭包、二次换行修正、裸文本节点全部按它取字体。

诊断：`MINI_FONT_LOG=1` 打印每条 `font-family` 解析到了哪个字体文件。

### 绝对定位的包含块高度塌成 0

```text
.page { flex: 1; position: relative }   /* 子节点全是绝对定位 → 没有在流内容 → 高 0 */
.background { position: absolute; inset: 0; width: 100%; height: 100% }
```

微信/Skyline 里页面根节点就是视口高，`.page` 因此是满屏的；而我们的布局根是
`height: auto`（内容高驱动滚动），于是这类「整屏铺底」的写法整屏塌掉：
闪屏页的背景照片不见了、文案全挤在顶部。

建树期已经处理了「一个定位祖先都没有」的情况，现在补上「有定位祖先、但它的使用高度是 0」：
`correct_absolute_heights` 在**布局之后**按实际使用高度判断，塌成 0 就退回视口高度，
并顺带处理 `top`/`bottom` 都给了而高度 auto 的情况。放在布局之后是必须的 ——
「包含块的使用高度」只有布局算完才知道。

### 异步图片到位时必须整帧重绘

远程图片下载完成属于「内容变了但数据没变」，`setData` 的失效范围算不出它。
于是在**每秒都有 setData** 的页面上（闪屏倒计时、首页秒杀），增量范围只覆盖那一小块文字，
刚到位的图片永远不会被重画 —— 表现就是图片一直停在占位图。现在
`image_net::take_dirty()` 会把这一帧标记成强制整帧。

> 同一个问题在快照工具里还有一层：`--settle` 从前只 `update` 不出帧，
> 而**异步资源是绘制期才发起的**（绘制到 `<image src="http…">` 才知道要下载它），
> 等待的那几秒里根本没人发起下载。现在 `--settle` 跑的是完整的一帧（与交互窗体同一套闸门），
> 截图前再补一帧。`MINI_IMG_LOG=1` 下画占位会打印 src，
> 用来区分「还在下载 / 下载失败 / src 为空」这三种完全不同的原因。

### 颜色的 alpha 曾经被整条丢掉

`rgba(…, .55)` 一律画成不透明色 —— 半透明遮罩、淡出、发丝描边全糊成实色块。
tea-app 首页 banner 上那道横线就是这么来的：`.hero-wash-fade` 是
「顶部 55% → 底部 0%」的淡出，解析成两个不透明色标之后变成一块实色，
在它结束的地方留下一条硬边。

根因是**同一份颜色语法有四个入口各写一份实现**：WXSS 声明、内联 `style`、
渐变色标、canvas `fillStyle`。其中两份把第四个分量直接扔了。现在统一到
`components::color_parse` 一处（`#rgb`/`#rgba`/`#rrggbb`/`#rrggbbaa`、
`rgb()/rgba()` 的逗号式与 `r g b / a` 新语法、百分比通道、常用命名色），
WXSS 解析器改成薄封装调它。

同一批修的还有两个「值被空白切碎」的问题：`border: 2rpx solid rgba(0, 0, 0, .04)`
和 `box-shadow: 0 8rpx 24rpx rgba(76, 52, 29, .06)` 以前直接 `split_whitespace()`，
`rgba(0,` `0,` `0,` `.04)` 四段残片全都解析失败 —— 边框退回默认灰、阴影退回
50% 纯黑。现在按**括号感知**的空白切分（`split_top_ws`）。tea-app 里这类声明有 83 条。

### `letter-spacing` 会让文字凭空少一行

盒子宽度按内容宽定，断行算法却用另一把尺子量：`measure_text_weighted` 按
`(n-1)` 份字间距算，断行按 `n` 份算。于是「刚好装下」被判成「装不下」，
最后一个字掉到第二行，**盒子高一倍**。

Chrome 实测 `letter-spacing:10px` 的 4 字符 inline-block 正好宽 40px（不是 30px）——
CSS 把字间距加进每个字形的前进宽度，末字符也算。按这个口径统一之后：
tea-app 首页 `.brand-cn` 不再变成两行，被它挤出定高导航栏的 `PHOENIX YUNXIU`
也回来了。（sample/news 两个示例不用 letter-spacing，15 页快照逐字节不变。）

### 滑动帧成本：先量出来，再改

滑动手感只有**连续拖动**才测得出来，而快照永远只出一帧。所以先加了两件工具：

```bash
# 模拟手指拖动并逐帧计时（每帧上移 8px，共 60 帧）
./target/release/mini-app-window tea-app --route pages/category/category \
    --settle 2 --drag 8x60 --snapshot target/drag
# 🖐  pages/category/category：60 帧，单帧耗时 平均 5.32ms  p50 5.49ms  p95 8.89ms  最慢 9.94ms
#    ↳ 最慢一帧 9.94ms 的渲染构成：页面 9.71ms  覆盖层 0.32ms  tabBar 0.00ms

MINI_DRAW_LOG=1 ...   # 再按组件类型/绘制阶段摊开耗时
#    ↳ 绘制归因（合计 523.9ms）：view 188.2ms/2346次  image 121.0ms/832次
#      背景:圆角填充 102.8ms/460次  背景:边框环 79.3ms/646次  text 19.5ms/1620次
```

有了归因，三个热点一目了然（也说明**先量再改**为什么值得）：

- **`box-shadow` 用 12 层半透明圆角矩形叠出模糊**：`0 8rpx 24rpx` 在 2 倍屏上
  blur=24px → 12 层，每层都要把整张卡片面积做一遍带抗锯齿的路径填充，
  一张卡一帧 140 万次像素混合。改成 CSS 的做法：把阴影形状画成 8 位覆盖率掩膜、
  可分离盒式模糊三遍逼近高斯（σ = blur/2），**掩膜按几何尺寸缓存**，
  每帧只剩一趟乘加合成（`components::shadow`）。与 Chrome 的剖面差在 3~10/255 之间。
- **抗锯齿路径填充横扫整个包围盒**：`border` 的圆角描边是「外圈套内圈」的环形路径，
  中间是空的，而填充器逐像素扫过 662px 宽的卡片，其中 650+ 个像素算出覆盖率 0。
  改成**按扫描线区间遍历**（合并各子扫描线的区间，同一像素只访问一次），
  输出逐像素不变 —— 分类页这一项 354ms → 79ms。
- **渐变逐像素投影 + 查色标**：实际用到的渐变几乎全是轴对齐的，垂直方向同一行同色
  （整行一次填充）、水平方向所有行同色序列（只算一行，其余行整行复用），
  只有圆角波及的那几行退回逐像素（`components::gradient`）。
- scroll-view 从离屏缓存上屏是逐像素 `set_pixel`（一屏一百万次带边界+裁剪判断），
  改成按行成片拷贝、整行不透明时退化成 memcpy。

tea-app 十个页面的拖动帧（60 帧一次，取平均/最慢）：

| 页面 | 改前 | 改后 |
| --- | --- | --- |
| 首页 | 7.17ms / 最慢 11.6ms | **3.74ms / 10.0ms** |
| 产区选茶 | 9.76ms / 最慢 14.0ms | **5.32ms / 9.9ms** |
| 我的 | 8.95ms / 最慢 11.4ms | **2.49ms / 6.9ms** |

其余页面 0.09~5.74ms。全部落进 60Hz 预算，多数落进 120Hz 预算；
最慢帧仍会碰到 144Hz 的 6.94ms 上限 —— 因为拖动时整屏内容仍在逐帧重新光栅化，
而 Skyline 那边滑动是纯合成。下一步是把 scroll-view 的内容也纳入离屏缓存复用。

多步交互也能脚本化验证（每段 `--eval` 之后宿主会把导航跑完）：

```bash
cargo run --release --bin mini-app-window -- sample-app --snapshot target/flow \
    --route pages/detail/detail --eval "__currentPage.onBuyNow()"          # 进入确认订单页
cargo run --release --bin mini-app-window -- news-app --snapshot target/nav \
    --route pages/home/home \
    --eval "__currentPage.onOpenArticle({currentTarget:{dataset:{id:301}}})" \
    --eval "wx.navigateBack()"                                            # 进详情再返回
```

---

## 🖼️ 场景画廊（64 个真实渲染）

以下页面均由本引擎真实渲染输出（纯 WXML + WXSS + 数据），一键生成：

```bash
python3 scripts/gen_assets.py   # 首次：生成真实商品图/头像/图标素材
cargo run --example gallery     # 渲染 64 个场景到 doc/gallery/
```

### 电商 · 社交 · 导航

| | | |
|:---:|:---:|:---:|
| <img src="doc/gallery/01_login.png" width="230"/><br/>**登录页** | <img src="doc/gallery/02_product_list.png" width="230"/><br/>**商品列表** | <img src="doc/gallery/03_product_detail.png" width="230"/><br/>**商品详情** |
| <img src="doc/gallery/04_cart.png" width="230"/><br/>**购物车** | <img src="doc/gallery/05_profile.png" width="230"/><br/>**个人中心** | <img src="doc/gallery/06_settings.png" width="230"/><br/>**设置页** |
| <img src="doc/gallery/07_chat.png" width="230"/><br/>**聊天对话** | <img src="doc/gallery/08_feed.png" width="230"/><br/>**动态流** | <img src="doc/gallery/09_grid_menu.png" width="230"/><br/>**宫格导航** |

### 表单 · 数据 · 生活

| | | |
|:---:|:---:|:---:|
| <img src="doc/gallery/10_form.png" width="230"/><br/>**表单填写** | <img src="doc/gallery/11_dashboard.png" width="230"/><br/>**数据看板** | <img src="doc/gallery/12_gallery.png" width="230"/><br/>**图片画廊** |
| <img src="doc/gallery/13_weather.png" width="230"/><br/>**天气** | <img src="doc/gallery/14_orders.png" width="230"/><br/>**订单列表** | <img src="doc/gallery/15_home.png" width="230"/><br/>**商城首页** |
| <img src="doc/gallery/16_contacts.png" width="230"/><br/>**通讯录** | <img src="doc/gallery/17_music.png" width="230"/><br/>**音乐播放器** | <img src="doc/gallery/18_tags.png" width="230"/><br/>**标签与徽章** |

### 弹窗 · 滑动 · 交互

| | | |
|:---:|:---:|:---:|
| <img src="doc/gallery/19_modal.png" width="230"/><br/>**确认弹窗** | <img src="doc/gallery/20_action_sheet.png" width="230"/><br/>**操作面板** | <img src="doc/gallery/21_toast.png" width="230"/><br/>**Toast 提示** |
| <img src="doc/gallery/22_swiper.png" width="230"/><br/>**Swiper 轮播** | <img src="doc/gallery/23_h_scroll.png" width="230"/><br/>**左右滑动** | <img src="doc/gallery/24_v_scroll.png" width="230"/><br/>**上下滑动** |
| <img src="doc/gallery/25_picker.png" width="230"/><br/>**底部选择器** | <img src="doc/gallery/26_calendar.png" width="230"/><br/>**日历** | <img src="doc/gallery/27_rating_steps.png" width="230"/><br/>**评分与物流** |

### 电商大促 · 组件补充

| | | |
|:---:|:---:|:---:|
| <img src="doc/gallery/28_ecommerce_home.png" width="230"/><br/>**电商首页**（轮播/秒杀/瀑布流） | <img src="doc/gallery/29_coupon_popup.png" width="230"/><br/>**优惠券弹窗** | <img src="doc/gallery/30_tabbar.png" width="230"/><br/>**底部 TabBar** |
| <img src="doc/gallery/31_search_nav.png" width="230"/><br/>**自定义搜索栏** | <img src="doc/gallery/32_input_events.png" width="230"/><br/>**输入与事件** | <img src="doc/gallery/33_video.png" width="230"/><br/>**视频播放器** |
| <img src="doc/gallery/34_canvas.png" width="230"/><br/>**Canvas 2D 绘图** | <img src="doc/gallery/35_gif_frame1.png" width="230"/><br/>**GIF 动图**（逐帧播放） | |

### 资讯类（与 `news-app` 示例同款设计）

| | | |
|:---:|:---:|:---:|
| <img src="doc/gallery/36_news_feed.png" width="230"/><br/>**新闻信息流**（频道横滑/热榜） | <img src="doc/gallery/37_news_article.png" width="230"/><br/>**文章详情**（长文/评论） | <img src="doc/gallery/38_news_video.png" width="230"/><br/>**视频频道** |
| <img src="doc/gallery/39_news_mine.png" width="230"/><br/>**我的**（开关/滑块设置） | | |

### 更多商业场景

| | | |
|:---:|:---:|:---:|
| <img src="doc/gallery/40_logistics.png" width="230"/><br/>**物流跟踪**（时间轴） | <img src="doc/gallery/41_live_shopping.png" width="230"/><br/>**直播带货** | <img src="doc/gallery/42_food_order.png" width="230"/><br/>**外卖点餐**（左右分栏） |
| <img src="doc/gallery/43_member_center.png" width="230"/><br/>**会员中心** | <img src="doc/gallery/44_payment.png" width="230"/><br/>**支付收银台** | <img src="doc/gallery/45_reviews.png" width="230"/><br/>**评价晒单** |
| <img src="doc/gallery/46_search_result.png" width="230"/><br/>**搜索结果** | <img src="doc/gallery/47_message_center.png" width="230"/><br/>**消息中心** | <img src="doc/gallery/48_checkin.png" width="230"/><br/>**签到打卡** |
| <img src="doc/gallery/49_market_board.png" width="230"/><br/>**行情看板** | <img src="doc/gallery/50_hotel_booking.png" width="230"/><br/>**酒店预订** | <img src="doc/gallery/51_health_dashboard.png" width="230"/><br/>**运动健康** |

### CSS 动画 · 变换 · 控件度量

`58_css_anim_frame1~4` 是同一份 WXML/WXSS 在 `t = 0 / 0.3 / 0.6 / 0.9s` 的四张快照 —— 固定动画时钟就能把 `@keyframes` 的任意一帧稳定截出来，因此动画也能进回归对比。

| | | |
|:---:|:---:|:---:|
| <img src="doc/gallery/58_css_anim_frame1.png" width="230"/><br/>**CSS 动画 t=0s** | <img src="doc/gallery/58_css_anim_frame3.png" width="230"/><br/>**CSS 动画 t=0.6s**（旋转/脉冲/弹跳） | <img src="doc/gallery/59_transform.png" width="230"/><br/>**transform**（平移/缩放/旋转/倾斜） |
| <img src="doc/gallery/60_form_controls.png" width="230"/><br/>**表单控件**（switch 各态/勾选/滑块） | <img src="doc/gallery/61_emoji_text.png" width="230"/><br/>**彩色 Emoji 混排** | |


---

## 🎬 视频与 Canvas

### 视频

`<video>` 组件自带 MP4 解复用（区分音/视频轨道）、H.264 解码（openh264）、音频解码与播放（symphonia + rodio，桌面端默认 macOS 音频接口）。支持 `autoplay` / `loop` / `muted` / `controls`。

```bash
cargo run --example video_player    # 独立播放窗口（自动循环，按刷新率出帧）
cargo run --example video_decode    # 离屏抓帧验证解码（输出 doc/videos/frame_*.png）
```

```html
<video src="doc/videos/video.mp4" autoplay="true" loop="true" controls="true" object-fit="cover" />
```

### Canvas 2D

`wx.createCanvasContext(id)` 返回完整的 2D 上下文，命令经桥接在 native 侧光栅化：

```javascript
const ctx = wx.createCanvasContext('demo');
ctx.setFillStyle('#07c160');
ctx.fillRect(20, 20, 120, 80);
ctx.beginPath();
ctx.arc(100, 200, 50, 0, 2 * Math.PI);
ctx.setStrokeStyle('#ff3b30'); ctx.setLineWidth(6); ctx.stroke();
ctx.setFontSize(24); ctx.setTextAlign('center'); ctx.fillText('Hello', 100, 300);
ctx.save(); ctx.translate(200, 200); ctx.rotate(0.6); ctx.fillRect(-40, -40, 80, 80); ctx.restore();
ctx.draw();
```

支持：`fillRect/strokeRect/clearRect`、`beginPath/moveTo/lineTo/arc/quadraticCurveTo/bezierCurveTo/rect/closePath`、`fill/stroke`、`fillText/strokeText`、`drawImage`、`setFillStyle/StrokeStyle/LineWidth/LineCap/LineJoin/FontSize/TextAlign/TextBaseline/GlobalAlpha`、`save/restore`、以及 `translate/rotate/scale` 完整仿射变换、线性/径向渐变。

后备缓冲按**设备分辨率**分配：元素逻辑尺寸 × 设备像素比，基础变换矩阵同步预乘这个比例，所以上面这些指令仍然用逻辑坐标下发，画出来是原生分辨率而不是放大的马赛克。元素真实尺寸只有布局后才知道，而 `ctx.draw()` 通常发生在 `onLoad` —— 因此指令流会被记下来，后备缓冲按实际尺寸重建后重放一遍。少了这一步，上下文只能用一个写死的默认尺寸，再被 1:1 拷进 2 倍分辨率的页面画布，内容就只落在元素左上角的四分之一里。

---

## 🧩 支持的组件

| 分类 | 组件 |
|------|------|
| 基础 | `view` `text` `image` `icon` `rich-text` |
| 表单 | `button` `input` `textarea` `checkbox` `checkbox-group` `radio` `radio-group` `switch` `slider` `progress` `picker` `picker-view` `picker-view-column` |
| 容器 | `scroll-view` `swiper` `swiper-item` |
| 媒体 | `video` `canvas` |

---

## 🎯 CSS 支持

- **选择器**：`view`、`.class`、`#id`、`*`、`.a.b`（复合）、`.a .b`（后代）、`.a > .b`（子）、`[type="primary"]` 属性选择器、伪类；按 (id, class, tag) 计算特异性并叠加书写顺序。
- **取值**：`rpx`/`px`/`%`/`vw`/`vh`/`em`/`rem`、`#rgb`/`#rrggbb`/`#rrggbbaa`/`rgb()`/`rgba()`/命名颜色/渐变、`var(--x, fallback)`、`calc(a + b)`（同单位）、`@import`。
- **布局**：`display` `flex-*` `justify-content` `align-*` `width/height/min/max` `padding/margin`（1–4 值简写、`auto` 居中）`position` `top/right/bottom/left` `gap`。
- **外观/文本/变换**：`background` `color` `border`（宽度精确、抗锯齿）`border-radius`（四角）`box-shadow` `opacity` `overflow`；`font-size` `font-weight` `text-align` `text-decoration` `line-height` `letter-spacing` `white-space` `text-overflow`；`transform`（translate/scale/rotate）`z-index`。
- **文本换行**：`display:block` / `width:100%` 的文本按容器宽度自动换行，盒子高度随行数增长（二次布局修正）。
- **简写 vs 细项**：级联把所有中选规则合并成一张表，简写与细项的先后关系在这一步就没了。落地时因此按固定档位排序（`border` 这类全能简写 → `border-color` 这类边/方向简写 → 细项），保证细项永远覆盖简写。

> 这里曾经藏着一个**渲染结果随机**的 bug：`.dot{border:2rpx solid #ccc}` 加 `.dot.on{border-color:#FF6B35}`，合并后表里同时有 `border` 和 `border-color`，而落地是按 HashMap 遍历顺序做的 —— Rust 每个进程一个随机 hash 种子，于是同一份源码**每次运行结果都可能不同**（地址页那个选中圆点时橙时灰）。修掉之后，固定动画时钟下 15 个页面的整帧快照跨进程逐字节稳定，这条也成了双端对比可信的前提。

---

## ♻️ 小程序生命周期

三级生命周期钩子均已接入（JS 注册 + Native 分发）：

| 层级 | 钩子 |
|------|------|
| **App** | `onLaunch` · `onShow` · `onHide` · `onError` · `onPageNotFound` · `onUnhandledRejection` · `onThemeChange` |
| **Page** | `onLoad` · `onShow` · `onReady` · `onHide` · `onUnload` · `onPullDownRefresh` · `onReachBottom` · `onPageScroll` · `onResize` · `onTabItemTap` · `onShareAppMessage` · `onShareTimeline` · `onAddToFavorites` |
| **Component** | `created` · `attached` · `ready` · `moved` · `detached`；`pageLifetimes`：`show` · `hide` · `resize` |

- 页面 `onShow/onHide/onResize` 会**联动**页面内组件的 `pageLifetimes`；
- 页面 `onUnload` 自动回收其组件实例并触发各组件 `detached`。

---

## 🏗️ 架构

```
┌─────────────────────────────────────────────────┐
│                    Mini App                       │
│  ┌──────────────────────────────────────────┐    │
│  │           JavaScript (QuickJS)            │    │
│  │  App · Page · Component · Behavior        │    │
│  │  require/module · Promise · setData 路径   │    │
│  └──────────────────────────────────────────┘    │
│                      ↕ Bridge                     │
│  ┌──────────────────────────────────────────┐    │
│  │              Native (Rust)                │    │
│  │  Canvas 渲染 · Taffy 布局 · 事件系统       │    │
│  │  WXML 解析 · WXSS 选择器引擎 · 模板引擎     │    │
│  └──────────────────────────────────────────┘    │
│                      ↕ FFI (mr_*)                 │
│  ┌──────────────────────────────────────────┐    │
│  │  Host: Android / iOS / Windows /          │    │
│  │        macOS / Linux / C / C++            │    │
│  └──────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

模块按职责细分，便于扩展维护：`parser/`（wxml/wxss/expr/template）、`renderer/`（`wxml_renderer/` + `components/*` 每组件独立文件 + `style_parse` CSS 取值解析）、`layout/`、`js/`（runtime/api/bridge）、`ui/`（交互/滚动）、`runtime/`、`bin/`（各可执行程序）。

渲染器本身按单一职责拆成一组模块（原先是一个 3000 行的单文件，改动一处要在无关代码里翻半天）：

| 模块 | 职责 |
|------|------|
| `mod.rs` | 类型定义（事件绑定 / picker 登记 / 布局缓存）与渲染器构造 |
| `entry.rs` | 对外渲染入口：各入口只差「给不给交互上下文、给不给滚动偏移」 |
| `layout.rs` | 建树 + 样式解析 + flexbox 求解 + 换行高度修正 + 按数据指纹缓存 |
| `invalidate.rs` | `setData` 的增量失效：新旧树比对，算出只需重绘哪一块 |
| `draw.rs` | 视口裁剪、绘制分派，以及不带交互的绘制路径 |
| `draw_interactive.rs` | 页面主路径：边画边登记交互（顶层与子节点两个入口） |
| `animation.rs` | `@keyframes` / `transition` / `wx.createAnimation` 求值与 transform 合成、损伤区 |
| `pressed.rs` | 按压态（`:active` / `hover-class`）与 switch 拨动的进度与烘焙 |
| `fixed_layer.rs` | `position: fixed` 覆盖层（独立画布 + 视口坐标） |
| `hit_test.rs` | 事件绑定表、命中与冒泡（覆盖层与正常流分开）、picker 可点区域 |
| `interactions.rs` | 有内部状态的元素登记（勾选/开关/滑块/输入/滚动区/按压 view） |

拆分是**纯代码搬迁**，并且被验证过：拆分前后 15 个页面的整帧快照（固定动画时钟）逐字节相同。

### 项目规则（`.kiro/steering/`）

协作约定写成了三条常驻规则，Kiro 每次会话都会读到：

| 文件 | 约定 |
|------|------|
| `00-scan-policy.md` | **默认不扫 `sample/` 等已固化路径**。排查顺序固定为「先看诊断产物 → 再看引擎代码 → 最后才定向读一眼小程序源码」，只有确认问题落在某个具体页面后才读那一个片段 |
| `10-skyline-target.md` | 唯一对标对象是微信 [Skyline 渲染引擎](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/introduction.html)；测试目标必须可判定；**流畅度看最慢一帧与帧间隔峰值，不看平均帧率** |
| `20-code-structure.md` | 单个 `.rs` 超过 500 行就要拆（数据表 / 不可分割算法 / 平台胶水可例外但需注明理由）；重构与改行为分两次做，纯搬迁必须用固定动画时钟的逐字节快照证明等价 |

---

## 🖌️ 素材图片生成

画廊中的商品图/Banner/头像等**真实照片**由火山引擎「豆包·文生图」生成，播放器/TabBar 图标由 PIL 以 4x 超采样绘制为**抗锯齿透明 PNG**：

```bash
python3 scripts/gen_assets.py
# -> doc/gallery/assets/*.jpg          （AI 生成的商品/封面/头像/Banner）
# -> doc/gallery/assets/icons/*.png    （PIL 绘制的透明矢量图标）
```

`<image>` 支持本地文件与网络 URL，`mode` 支持 `scaleToFill/aspectFit/aspectFill/widthFix` 等；透明 PNG 不会被套上不透明底框，仅在加载失败时回退占位符。

---

## 📦 多平台 SDK 编译

`crate-type = ["cdylib", "staticlib", "rlib"]`，可产出动态库(.so/.dylib/.dll)、静态库(.a)与 Rust 库。仓库 `scripts/` 提供各平台构建脚本。

| 平台 | 脚本 | 产物 |
|------|------|------|
| macOS | `scripts/build-macos.sh` | `libmini_render.dylib` / `.a`（arm64+x86_64 通用） |
| Linux | `scripts/build-linux.sh [target]` | `libmini_render.so` / `.a` |
| Windows | `scripts/build-windows.ps1` | `mini_render.dll` + 导入库 `.lib` |
| Android | `scripts/build-android.sh` | 4 个 ABI 的 `libmini_render.so`（jniLibs 结构） |
| iOS | `scripts/build-ios.sh` | `MiniRender.xcframework` |

> 移动端需先启用 rquickjs 的 `bindgen`（`*-apple-ios`/`*-linux-android` 无预置绑定，需 libclang），并将桌面端依赖（winit/softbuffer/rodio/openh264/arboard/ureq）改为可选特性、以 `--no-default-features` 构建精简核心库。详见脚本内注释。

重新生成 C 头文件：

```bash
cargo install cbindgen
bash scripts/gen-header.sh   # -> include/mini_render.h
```

---

## 💡 使用示例

### Rust — 渲染 WXML/WXSS 到图片

```rust
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::{Canvas, Color};
use serde_json::json;

fn main() {
    let wxml = WxmlParser::new(r#"<view class="card"><text class="t">{{title}}</text></view>"#).parse().unwrap();
    let ss = WxssParser::new(r#".card{padding:24rpx;background-color:#fff;border-radius:16rpx;} .t{font-size:34rpx;color:#333;}"#).parse().unwrap();
    let mut r = WxmlRenderer::new(ss, 375.0, 667.0);
    let mut canvas = Canvas::new(375, 667);
    canvas.clear(Color::from_hex(0xF5F5F5));
    r.render(&mut canvas, &wxml, &json!({ "title": "Hello Mini" }));
    canvas.save_png("out.png").unwrap();
}
```

### Rust — 驱动小程序逻辑层

```rust
use mini_render::runtime::MiniApp;

fn main() -> Result<(), String> {
    let mut app = MiniApp::new(375, 667)?;
    app.init()?;
    app.define_module("utils/util", "exports.double = x => x * 2;")?;
    app.load_script(r#"
        const util = require('utils/util');
        Page({ data: { count: 0 }, inc() { this.setData({ count: util.double(this.data.count + 1) }); } });
    "#)?;
    app.eval("__currentPage.inc()")?;
    println!("data = {}", app.eval("__getPageData()")?); // {"count":2}
    Ok(())
}
```

### C / C++ — 通过 FFI 使用 2D 渲染

```c
#include "mini_render.h"
int main(void) {
    Canvas* c = mr_canvas_new(375, 667);
    mr_canvas_clear(c, 245, 245, 245, 255);
    mr_canvas_draw_circle(c, 70, 80, 30, 74, 144, 217, 255, 0, 0);
    mr_canvas_save_png(c, "card.png");
    mr_canvas_free(c);
    return 0;
}
```

```bash
cargo build --release
clang examples/demo.c -Iinclude -Ltarget/release -lmini_render -o demo_c
DYLD_LIBRARY_PATH=target/release ./demo_c
```

### Android / iOS

`libmini_render.so` / `MiniRender.xcframework` 暴露 C 接口，在 NDK / Swift 侧渲染后把 RGBA 拷入 `Bitmap` / `CGContext`：

```c
Canvas* c = mr_canvas_new(w, h);
mr_canvas_clear(c, 255, 255, 255, 255);
size_t need = (size_t)w * h * 4; uint8_t* buf = malloc(need);
mr_canvas_get_pixels(c, buf, need);   // RGBA8888 -> AndroidBitmap / UIImage
free(buf); mr_canvas_free(c);
```

---

## 🧪 测试

覆盖表达式引擎、WXSS 选择器（含 `var()`/`calc()`）、模板控制流、布局与文本换行、全组件渲染、Canvas 2D、交互、滚动/惯性、页面栈路由、组件模型、CommonJS 模块、Promise、生命周期、事件冒泡等：

```bash
cargo test          # 281 个用例
cargo test route    # 路由/页面栈/组件/模块/异步/生命周期
cargo test canvas   # Canvas 2D 上下文与命令
cargo test scroll   # 滚动与惯性
```

---

## 📁 项目结构

```
mini-render/
├── src/
│   ├── canvas.rs / color.rs / geometry.rs / paint.rs / path.rs / text.rs
│   ├── ffi.rs                  # C FFI (mr_*)
│   ├── event.rs                # 事件系统
│   ├── bin/                    # mini-app / mini-app-window / mini-launcher / mini-devserver
│   │   └── app_window/         # 窗体宿主：导航 / tabBar / picker / 覆盖层 / 输入层
│   │       ├── touch.rs        #   触摸状态机与微信语义的事件对象（纯逻辑）
│   │       ├── gesture.rs      #   拖动归属仲裁：方向锁定 / 嵌套传递（纯逻辑）
│   │       ├── edge_back.rs    #   左边缘侧滑返回：判定 + 上一页合成（纯逻辑）
│   │       └── pointer_input.rs#   把上面三者接到宿主状态（窗体与无头共用同一条链路）
│   ├── js/                     # QuickJS runtime / api（App/Page/Component）/ bridge
│   ├── parser/                 # wxml / wxss（选择器引擎）/ expr / template
│   ├── renderer/               # wxml_renderer/*（布局/绘制/动画/命中/覆盖层）+ components/*
│   ├── ui/                     # interaction / scroll_controller / scroll_cache
│   ├── runtime/                # MiniApp 应用运行时
│   └── tests/                  # 单元/集成测试
├── examples/                   # gallery / video_player / video_decode / demo
├── scripts/                    # 各平台 SDK 构建脚本 + gen_assets.py 素材生成
├── assets/                     # 字体资源
├── include/mini_render.h       # C 头文件
├── doc/                        # 文档、画廊图片、视频素材
└── sample-app/                 # 示例小程序
```

---

## 📄 License

MIT

[Taffy]: https://github.com/DioxusLabs/taffy
