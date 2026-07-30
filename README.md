# mini-render

**自绘的小程序渲染引擎。用 Rust 从零实现 —— 不基于 WebView、不映射系统原生控件、不依赖 Skia。**

给它一份微信小程序源码（WXML / WXSS / JS / app.json）和一块可写的像素缓冲，它把界面画出来、
把交互跑起来。对齐目标是微信的 **Skyline**（同样是原生渲染，同样没有 DOM）。

下面四张是引擎自己渲染的输出（纯 WXML + WXSS + 数据，375×667 @2x，无浏览器参与）：

| | | | |
|:---:|:---:|:---:|:---:|
| <img src="doc/gallery/28_ecommerce_home.png" width="180"/><br/>电商首页 | <img src="doc/gallery/36_news_feed.png" width="180"/><br/>资讯信息流 | <img src="doc/gallery/42_food_order.png" width="180"/><br/>外卖点餐 | <img src="doc/gallery/60_form_controls.png" width="180"/><br/>表单控件 |

全部 65 张见 [`doc/场景画廊.md`](doc/场景画廊.md) —— 它们同时是逐字节可复现的回归基线。

---

## 目录

- [核心介绍](#核心介绍)
  - [它是什么、不是什么](#它是什么不是什么)
  - [一帧是怎么画出来的](#一帧是怎么画出来的)
  - [八个关键设计取舍](#八个关键设计取舍)
  - [现在到什么程度了（实测）](#现在到什么程度了实测)
  - [能力边界：能做什么、明确不做什么](#能力边界能做什么明确不做什么)
  - [适合 / 不适合的场景](#适合--不适合的场景)
  - [技术栈与依赖](#技术栈与依赖)
- [30 秒跑起来（桌面）](#30-秒跑起来桌面)
- [集成进 App（SDK）](#集成进-appsdk)
- [支持范围](#支持范围)
- [测试与回归](#测试与回归)
- [仓库结构](#仓库结构)
- [文档](#文档)

---

## 核心介绍

### 它是什么、不是什么

一句话：**小程序源码进，像素出**。中间没有浏览器内核、没有系统控件、没有第三方图形库。

```
输入                        引擎（纯 Rust 库）                     输出
────────────────────────────────────────────────────────────────────────────
my-mini-app/                                                    ┌──────────┐
├── app.json      ─┐                                            │  RGBA    │
├── app.wxss       │   解析 → 逻辑层 → 模板求值 → 样式计算        │  像素    │
├── pages/*.wxml   ├─▶  → Flexbox 布局 → 自研 2D 光栅器  ──────▶ │  缓冲    │
├── pages/*.wxss   │      → 分层合成（页面 / tabBar / fixed）     └──────────┘
├── pages/*.js    ─┘                                                  │
└── 图片等资源                                                         ▼
                                                       窗口 / UIView / SurfaceView / PNG
```

产物是一个普通的 Rust 库（`cdylib` / `staticlib` / `rlib`），落地条件只有一条：
**给我一块可写的像素缓冲**。所以同一份代码能跑在 Android、iOS、Windows、macOS、Linux 上，
也能在没有屏幕的 CI 里直接出 PNG。

它和常见方案的区别：

| 方案 | 做法 | 本引擎 |
|---|---|---|
| 微信小程序的 WebView 渲染层 | WXML → DOM，交给浏览器内核排版绘制 | ❌ 不用 DOM、不用浏览器内核 |
| React Native / Weex | JS 描述 → 映射成系统原生控件（`UIView` / `android.view`） | ❌ 不映射系统控件（不受各端控件差异摆布） |
| Flutter | Dart + Skia 自绘 | ✅ 思路最接近，但这里是 **Rust + 自研光栅器**，无 Skia 依赖 |
| Electron / Tauri | 打包/复用一个浏览器 | ❌ 不打包浏览器（产物 34MB 级，不是 100MB 级） |
| 微信 **Skyline** | 原生渲染 + 自绘组件 | ✅ 对齐目标：语义、手感、事件模型都按它对 |

**为什么值得自绘。** WebView 方案的痛点不在「能不能渲染」，而在你**控制不了**：内核版本随
系统走、同一份 CSS 在不同 Android 上排版不同、首屏要等内核起来、滚动与手势的手感由内核决定、
想插一层原生能力就要架桥。自绘把这些变成自己的代码：**布局与绘制逻辑各端只有一份**，帧调度、手势仲裁、内存上限
全都在手里 —— 各端的剩余差异只来自系统字体（要彻底消掉就把字体一起打包，
`assets/` 里放了 Noto Sans SC，`TextRenderer::from_file` 直接加载）。代价也很实：CSS 覆盖面要自己一条一条补
（见[能力边界](#能力边界能做什么明确不做什么)），性能靠 CPU 省着花。

### 一帧是怎么画出来的

八步，每步都能单独打开诊断日志（`MINI_*` 开关见[架构文档](doc/架构与实现原理.md#帧成本是怎么压下来的)）：

| # | 阶段 | 做什么 | 关键实现 |
|---|---|---|---|
| ① | **解析** | WXML → 节点树；WXSS → 样式表 | 手写解析器；CSS 选择器引擎（标签/类/#id/`*`/属性/后代/子代 + 特异性排序），选择器在**解析期就编译**好 |
| ② | **逻辑层** | 跑 `App` / `Page` / `Component`，`setData` 产出数据快照 | QuickJS（rquickjs）；CommonJS 模块、`Promise`/`async`、定时器、微任务每帧 pump |
| ③ | **模板求值** | `{{ }}` 表达式 + `wx:if` / `wx:for` 展开 → 渲染节点树 | 自研表达式引擎；`class`/`style` 绑定到数组/对象时按 CSS 语义拼接 |
| ④ | **样式计算** | 命中的 CSS + 内联 `style` 合成计算样式 | 继承语义（`color` / `font-size` / `font-weight` / `line-height` / `letter-spacing` …）；简写与细项按固定档位排序落地 |
| ⑤ | **布局** | Flexbox 求解盒模型 | taffy 0.12；**文本节点挂自定义度量函数**，换行与 min/max-content 由真实字形宽度决定；布局后有第二遍修正（`reflow`）收拾只有算完才知道的溢出 |
| ⑥ | **绘制** | 逐层画进 RGBA 画布 | 自研 2D 光栅器：扫描线 + even-odd 填充、4× 超采样抗锯齿、圆角三次贝塞尔逼近（K=0.5523）、fontdue 字形 + 多字体回退、Apple `sbix` 彩色 emoji、图片双线性 + 面积两级过滤、线性/径向渐变、掩膜化 `box-shadow`、裁剪栈 |
| ⑥' | **动画** | `@keyframes` / `transition` / `wx.createAnimation` | 在**绘制期**按全局时钟求值：`transform`/`opacity`/颜色只影响绘制，不触发重排 → 动画帧成本≈静态帧 |
| ⑦ | **分层合成** | 正常流 → tabBar → `position:fixed` 覆盖层 | 与浏览器层叠顺序一致（全屏遮罩能压暗 tabBar）；覆盖层用视口坐标、独立画布、按需重绘 |
| ⑧ | **上屏** | 贴到窗口 / 交给宿主 / 存 PNG | 桌面走 softbuffer；移动端把 RGBA 交给宿主 View；无头模式直接 `save_png` |

逻辑层与渲染层之间只有**数据快照**这一条通道（`setData` → `Arc<Value>`），没有 DOM、没有
虚拟节点 diff、没有 GC 压力。

### 八个关键设计取舍

这些是「为什么这么写」的核心，踩过的坑都写在 [`doc/踩坑记录.md`](doc/踩坑记录.md)。

1. **文本度量驱动布局，不估算宽度。** `TextMeasure` 上下文挂到 taffy 叶子上，由真实字形参与
   布局。这样「定宽容器内按容器宽换行」和「收缩容器被内容撑开」两种 CSS 语义能同时成立。
   反面教材：曾给每个文本盒 +4px「保险余量」，结果所有按内容定宽的徽标/标签/胶囊都比浏览器宽一圈。
2. **动画只重绘不重排。** `@keyframes` 里改 `width`/`height` 这类会引发重排的属性不生效，
   改 `transform`/`opacity`/颜色都生效。换来的是「动画帧和静态帧一样便宜」。
3. **`setData` 只重绘变化的那一块。** 新旧渲染树并行走一遍，收集「文本/属性变了」或「几何变了」
   的节点包围盒当裁剪矩形。首页秒杀倒计时的失效范围是 **45×33 像素**，不是整屏。
   拿不准就退回整帧 —— 少画一块留下脏像素比多画一次严重得多。
4. **页面滚动不重绘。** 页面画布是整页高的、用内容坐标，滚动只是取不同切片上屏（每帧 2~3ms）。
   只有滚出「已绘制条带」那一刻才补画一次。
5. **滚动手感按 iOS/微信那套做。** 橡皮筋衰减 `1 - 1/(x·0.55/d + 1)`、回弹是带初速度的
   **临界阻尼弹簧**（不是固定时长缓动）、惯性撞边界不当场停死而是把动量交给弹簧。
6. **手势归属不在按下时决定。** 等第一次明显位移（4px）按主方向锁定，再按「到边界仍在推」
   交棒给外层。横向卡片列表里竖着划该滚页面，纵向列表里横着划谁也不动。
7. **宿主层沉进 lib，桌面与移动端共用一份。** 页面栈、覆盖层、触摸状态机、手势仲裁、
   picker 面板、像素合成都在 `src/host/`，只有帧调度各自实现。两条链路由
   `tools/sdk-parity.sh`（逐像素）+ `src/tests/engine_input_tests.rs`（输入行为）双重守着。
8. **不引入 GPU、不做元素级位图缓存。** 定位就是「给我一块像素缓冲我就能画」；位图缓存与
   动画/裁剪栈的交互复杂度不值当。抗锯齿也没换成子采样 —— 实测与 Chrome 的差异反而从
   4.7% 涨到 5.3%。

### 现在到什么程度了（实测）

**性能**（375×667 @2x，即 750×1334 物理像素，CPU 逐像素写出）：

| 场景 | 优化前 | 现在 |
|---|---|---|
| 商城首页整帧（轮播 + 每秒倒计时 + 骨架动画） | 49.6 ms | **6.1 ms** |
| 商城首页稳态帧间隔（144Hz 屏，节拍 6.94ms） | 6.9~25 ms 抖动 | **6.7~7.2 ms** |
| 首页最慢一帧 / 帧间隔峰值 | 17.9 / 24.9 ms | **11.4 / 11.4 ms** |
| 一次 `setData` 的重建成本 | 32 ms | **4 ms** |
| 拖动单帧（tea-app 首页，60 帧平均） | 7.17 ms | **3.74 ms** |
| 远程图片二次打开（磁盘缓存） | 6132 ms | **11 ms** |
| 首帧「建树+样式」（三条中文字体栈的应用） | 3093 ms | **955 ms** |

**双端一致性**（同一份源码：一边原生渲染，一边编译成 HTML 用 Chrome 截图，逐像素比）：

| 示例 | 页数 | 与 Chrome 的变化像素比 |
|---|---|---|
| `sample-app`（商城） | 15 | **4.55%** |
| `news-app`（资讯） | 6 | **6.33%** |

剩余差异集中在粗体字形与亚像素文本位置。这个数字是**回归判据**，不是宣传语 ——
任何渲染改动都要先看它有没有变坏。

**跑得起来的真实工程**（不是 demo 级的自造样板）：

| 小程序 | 规模 | 说明 |
|---|---|---|
| `sample/tea-app` | 36 页 | **uni-app 编译到 mp-weixin 的产物**：259KB Vue 3 运行时 + 60 个 CommonJS 模块，用 `Component()` 构造器定义页面 |
| `sample/real-sample` | 8 页 | 微信官方 demo |
| `sample/sample-app` | 15 页 | 商城闭环：购物车 → 确认订单 → 下单 → 订单/物流，跨页状态走 `wx.storage` |
| `sample/news-app` | 6 页 | 资讯闭环：频道横滑、正文字号即时生效、评论、收藏持久化 |

**回归规模**（每次改动都要全绿，命令见[测试与回归](#测试与回归)）：

| 判据 | 规模 |
|---|---|
| 单元测试 | lib **471** + bin **9**；其中宿主纯逻辑 41 条（手势仲裁 13 / 侧滑返回 12 / picker 面板 10 / 触摸状态机 6）、SDK 指针链路 7 条 |
| 场景画廊 | **65** 张，固定动画时钟下逐字节可复现 |
| 局部重绘校验 | 增量重绘 vs 强制整帧，5 个场景**逐字节相同** |
| SDK 与桌面一致性 | sample 15/15、news 6/6 **逐像素相同** |
| 指针层与覆盖层 | 6 张确定性交互快照 + 2 条手势断言 + **空转守卫** |
| 编译警告 | **0**（`cargo check --all-targets`） |

### 能力边界：能做什么、明确不做什么

**已实现**（详表见[支持范围](#支持范围)）：

- **24 个组件标签**（容器 / 文本媒体 / 表单三类，含 `swiper`、`scroll-view`、`picker-view`）；
- **事件**：六种绑定前缀（`bind` / `catch` / `capture-bind` / `capture-catch` / `mut-bind` / `bind:`）
  与完整的捕获-冒泡链，触摸序列按微信语义产出（slop 取消 tap、350ms longpress、被滚动接管补 `touchcancel`）；
- **模板**：`wx:if` / `wx:elif` / `wx:else` / `wx:for`、`block`、表达式引擎、`class`/`style` 绑定、
  页面 json 的 `usingComponents`（自定义组件含样式隔离与独立数据作用域）；
- **样式**：选择器与特异性层叠、继承、`@import`、CSS 变量、`rpx`、`calc()`、渐变、阴影、
  `transform` / `transition` / `@keyframes`；
- **API**：39 个 `wx.*`（`request` 含 `abort`、storage 全套且跨启动持久化、Toast/Loading/Modal、
  五个路由 API、设备信息全套、`createAnimation`、`createCanvasContext`）；
- **生命周期**：三级共 28 个钩子（App 7 / Page 13 / Component 5 + `pageLifetimes` 3），
  页面 `onShow/onHide/onResize` 会联动页面内组件；
- **交互**：惯性滚动与临界阻尼回弹、手势仲裁、侧滑返回、下拉刷新、picker 底部面板、局部重绘；
- **媒体**：Canvas 2D（完整 2D 上下文 + 设备分辨率后备缓冲）、`<video>`（MP4 解复用 + H.264 软解 + 音频）。

**明确不做**（不是没排期，是取舍）：

| 不做 | 原因 |
|---|---|
| GPU 渲染 | 定位是「有像素缓冲就能画」；纯 CPU 才能在无 GPU 环境（CI、嵌入式）里出图 |
| 解析 `.wxapkg` / 签名校验 | 分包、灰度、完整性属于宿主的事；引擎按**目录**读 |
| 把 `<web-view>` / `<map>` 编进引擎 | 微信里它们是原生组件层。引擎算位置与参数，控件由宿主用 `WKWebView` / `android.webkit.WebView` / ArkUI `Web()` 放上去（[理由](doc/原生App集成指南.md)） |
| 远程字体 `wx.loadFontFace` | 字体来自系统。API 存在且回调 `success`（否则会打断框架的 `onLaunch`），但不真的下载 |
| 迁就偏离微信语义的写法 | 对齐目标只有 Skyline 一个。写法与微信不一致时，按微信来 |

**还缺的**（13 条，每条都写了影响，见 [`doc/引擎测试说明.md`](doc/引擎测试说明.md) 第六节）：
`wx:key` 的复用语义、`template` / `slot` / `wxs`、`scroll-view` 的滚动事件、
`<video>` 的播放事件、`wx.login` 等云能力 API、移动端 SDK 的左边缘侧滑返回。

**最大的已知短板是内存**：实测峰值 600MB~1GB，其中 **98% 是字体** —— fontdue 加载时把字体里
全部 29352 个字形几何预展开（一个 CJK 字面约 300MB）。缓存已按字节封顶（图片 64MB / 字形 24MB /
2 个字体文件，LRU），也有 `trim_memory()` 给宿主在内存告警时调用，但真正压峰值要换成惰性字体
后端（`swash` / `cosmic-text`），预计每字面 300MB → 约 23MB。这会改变每个字形的抗锯齿，
需要单独一轮重定基线。

### 适合 / 不适合的场景

**适合**

- 自家 App 想内嵌小程序容器，又不想背 WebView 的版本差异与首屏成本；
- 要求**渲染结果可控且可复现**：布局与光栅化各端同一份代码，同一套字体下输出逐像素相同
  （跨系统的差异只来自系统字体，内置字体即可消除）；
- 需要在无屏环境批量出图：营销长图、服务端渲染卡片、UI 回归基线；
- 想把一份小程序源码同时输出成 H5（内置编译器，本仓库正是用它当双端对比的参照）；
- 需要把渲染细节握在自己手里：帧调度、手势手感、内存上限、诊断日志。

**不适合**

- 需要完整 Web 生态（任意 CSS 特性、第三方 H5 SDK、`<iframe>`）—— 那就该用 WebView；
- 重度 3D / 大量滤镜 —— 纯 CPU 光栅撑不住，该上 GPU 方案；
- 期望「零适配跑通任意线上小程序」—— 还有 13 条已知差距，复杂应用需要逐项验证；
- 内存极紧的设备 —— 字体后端换完之前峰值偏高。

### 技术栈与依赖

依赖都跟到当前稳定版（`winit` 例外：最新的 0.31 只有 beta，稳定线仍是 0.30）。

| 依赖 | 版本 | 用途 |
|---|---|---|
| [taffy](https://github.com/DioxusLabs/taffy) | 0.12 | Flexbox / block 布局求解（`box-sizing` 真正生效靠它） |
| [rquickjs](https://github.com/DelSkayn/rquickjs) | 0.12 | QuickJS 绑定：逻辑层的 JS 运行时 |
| [fontdue](https://github.com/mooman219/fontdue) | 0.9 | 字形光栅化 |
| [image](https://github.com/image-rs/image) | 0.25 | 图片解码（PNG / JPEG / GIF 逐帧 / WebP 等，走 image 的默认解码器集） |
| [ureq](https://github.com/algesten/ureq) | 3.3 | `wx.request` 与远程图片（必须 `http_status_as_error(false)`，微信里 4xx/5xx 也走 `success`） |
| [symphonia](https://github.com/pdeljanov/Symphonia) | 0.6 | 音频解复用与解码（AAC / MP3 / MP4） |
| [openh264](https://github.com/ralfbiedert/openh264-rs) | 0.9 | H.264 软解（可选特性 `h264`） |
| [winit](https://github.com/rust-windowing/winit) + [softbuffer](https://github.com/rust-windowing/softbuffer) | 0.30 / 0.4 | 桌面窗口与软件帧缓冲（可选特性 `desktop`） |
| [rodio](https://github.com/RustAudio/rodio) | 0.22 | 桌面音频播放（可选特性 `audio`） |
| [jni](https://github.com/jni-rs/jni-rs) | 0.21 | Android JNI 入口（只在 android 目标编译） |

工程规模：**148 个 `.rs` 文件 / 约 4.9 万行**（含 28 个测试文件），单文件超 500 行就拆
（例外要在文件头写理由）。除 FFI 边界（`extern "C"`）外全库**没有 `unsafe`**，也没有 `static mut`：
全局状态一律走 `RwLock` / `OnceLock` / `thread_local`。

---

## 30 秒跑起来（桌面）

```bash
# 需要 Rust（https://rustup.rs）
cargo build --release

# 交互式选一个示例小程序
./target/release/mini-launcher

# 或直接指定
./target/release/mini-app-window sample-app
./target/release/mini-app-window sample/tea-app --route pages/index/index
```

无窗口出图（CI / 回归用）：

```bash
./target/release/mini-app-window sample-app --route pages/index/index \
    --settle 0 --time 2 --snapshot target/shot
```

常用动作参数（按书写顺序执行，可重复）：`--touch x,y` / `--swipe x1,y1,x2,y2`
/ `--drag px×帧数` / `--type 文本` / `--key enter` / `--wheel` / `--wait 秒` / `--frames N`。
完整列表 `--help`；诊断开关（`MINI_SCROLL_LOG` / `MINI_FPS` / `MINI_FONT_LOG` …）见
[`doc/引擎测试说明.md`](doc/引擎测试说明.md) 与
[`doc/架构与实现原理.md`](doc/架构与实现原理.md#帧成本是怎么压下来的)。

示例小程序清单、浏览器实时预览（`mini-devserver`）、编译成 HTML 工程、以及把引擎当库用的
Rust / C 代码示例，见 [`doc/示例与调试.md`](doc/示例与调试.md)。

---

## 集成进 App（SDK）

集成方只用**一个 View**。详见 [`sdk/README.md`](sdk/README.md) 与
[`doc/原生App集成指南.md`](doc/原生App集成指南.md)。

### 1. 编库

```bash
bash tools/build-mobile.sh android   # → sdk/android/src/main/jniLibs/<abi>/libmini_render.so
bash tools/build-mobile.sh ios       # → sdk/ios/MiniRender.xcframework（设备 + 模拟器）
bash tools/build-mobile.sh check     # 只做交叉编译检查
```

脚本已经带好了三个**必须**的参数（缺一个都编不过，原因写在脚本注释里）：

| 参数 | 为什么 |
|---|---|
| `--no-default-features` | 关掉桌面依赖：`winit` 在 android 目标必须选 activity 后端否则编译失败；`arboard`（剪贴板）只支持桌面；`rodio` 要开系统声卡 |
| `--features bindgen` | `rquickjs-sys` 没有为 iOS/Android 预生成 C 绑定，要现场生成 |
| `IPHONEOS_DEPLOYMENT_TARGET` | rustc 默认按 iOS 10 链接，而 C 依赖按 SDK 新版本编，不一致会缺 `___chkstk_darwin` |

特性开关：

| 特性 | 默认 | 说明 |
|---|---|---|
| `desktop` | ✅ | 桌面窗体（winit + softbuffer + 剪贴板） |
| `audio` | ✅ | 内置音频播放（`<video>` 的声音） |
| `h264` | ✅ | 内置 H.264 软解。**移动端建议关掉**：交给系统播放器硬解更省电，且 openh264 在 iOS 模拟器目标上编不过 |
| `bindgen` | ❌ | 交叉编译时打开 |

### 2. 打成依赖（推荐）

**Android（AAR → Maven）**

```bash
bash tools/build-mobile.sh android
cd sdk/android && ./gradlew publishToMavenLocal
```

业务工程：

```kotlin
repositories { mavenLocal() }                       // 或你的内网 Maven
implementation("dev.minirender:mini-render:0.1.0")
```

**iOS（Swift Package / CocoaPods）**

```swift
// Package.swift
.package(path: "../mini-program-core/sdk/ios")      // 或 .package(url: ..., from: "0.1.0")
```

```ruby
# Podfile
pod 'MiniRender', :path => '../mini-program-core/sdk/ios'
```

### 3. 接入代码

Android：

```kotlin
val mini = MiniProgramView(this)
setContentView(mini)
mini.open(File(filesDir, "my-mini-app"))          // 解包后的小程序目录

override fun onBackPressed() {
    if (!mini.goBack()) super.onBackPressed()      // 页面栈空了才退出
}
override fun onResume() { super.onResume(); mini.onHostResume() }
override fun onPause()  { super.onPause();  mini.onHostPause() }
```

iOS：

```swift
let mini = MiniProgramView(frame: view.bounds)
view.addSubview(mini)
mini.open(appDir: unpackedDir)

if !mini.goBack() { navigationController?.popViewController(animated: true) }
```

两个 View 内部包好了：专用渲染线程（引擎不是线程安全的）、尺寸与 dpr 换算、
触摸三段（含历史点 / `coalescedTouches`，不抽稀）、按需出帧（静止不耗电）、
沙盒目录、生命周期与内存告警。

### 4. 只有两件事必须你决定

1. **小程序包从哪来**：下载 / 校验 / 解包到沙盒。引擎按**目录**读，不解析 `.wxapkg`、
   不做签名校验 —— 分包、灰度、完整性属于宿主的事。
2. **原生层组件谁承载**：`<web-view>` / `<video>` / `<map>` 在微信里是原生组件层
   （层级最高，只有 `cover-view` 能盖住）。引擎算位置与参数，控件由你用
   `WKWebView` / `android.webkit.WebView` / ArkUI `Web()` 放上去。
   **不建议把 wry 编进引擎**（鸿蒙无后端、Android 要求改宿主 Activity），
   理由详见集成指南。

### 产物体积（arm64 实测）

| | |
|---|---|
| Android `.so`（strip 后） | 34.4 MB |
| iOS 静态库 `.a` | 184 MB（链进 App 后会被死代码消除大幅缩减） |

偏大，两个已知原因还没优化：`page_loader.rs` 用 `include_str!` 把示例小程序当兜底
内置了（正式 SDK 该去掉）；symphonia / openh264 解码器整个链进来了（移动端可关 `h264`）。

---

## 支持范围

| 类别 | 已支持 |
|---|---|
| 组件（24 个标签） | 容器：view / block / scroll-view / swiper / swiper-item<br/>文本与媒体：text / rich-text / icon / image / video / canvas<br/>表单：button / input / textarea / switch / slider / progress / checkbox / checkbox-group / radio / radio-group / picker / picker-view / picker-view-column |
| 事件 | touchstart/move/end/cancel、longpress/longtap、tap、change、focus/input/blur/confirm、image 的 load/error、页面 onPageScroll / onReachBottom / onPullDownRefresh；六种绑定前缀（bind / catch / capture-bind / capture-catch / mut-bind / bind:）与捕获-冒泡完整链路 |
| WXML | wx:for / wx:if / elif / else / for-item / for-index、block、表达式引擎、class 与 style 绑定、自定义组件（usingComponents） |
| WXSS | 标签/类/id/属性/伪类选择器、后代与子代组合器、特异性层叠、`:first/last/only-child`、`:nth-child(An+B)`、`:active`、`@import`、`@keyframes`、CSS 变量、rpx、渐变、阴影、transform、transition/animation |
| wx.\* | request（含 abort）、storage 全套、showToast/showLoading/showModal、下拉刷新、createAnimation、createCanvasContext、设备信息全套、五个路由 API、App 级监听（onError 等） |
| 交互 | 惯性滚动与临界阻尼回弹、手势仲裁（主轴锁定 / 内外层交接）、侧滑返回、下拉刷新、picker 面板、局部重绘（损伤区） |

**图标**用的是 WeUI 官方矢量数据（`success` 是实心圆挖出对勾形的洞，不是白色描边对勾）。

已知差距（`wx:key` / `template` / `slot` / `wxs`、`scroll-view` 的滚动事件、
`video` 的播放事件、`wx.login` 等未实现 API…）列在
[`doc/引擎测试说明.md` 第六节](doc/引擎测试说明.md)，共 13 条，每条都写了影响。

---

## 测试与回归

```bash
rm -rf target/mini-storage                 # ① 必做，见下
cargo test --release                       # ② lib 473 + bin 9
bash tools/damage-check.sh                 # ③ 增量重绘 == 整帧重绘（逐字节）
cargo run --release --example gallery      # ④ 65 张场景图
bash tools/tab-click-check.sh              # ⑤ 三个 app 的 tabBar 点击
bash tools/sdk-parity.sh                   # ⑥ 移动端 SDK 与桌面窗体逐像素一致
bash tools/interaction-check.sh target/_ia # ⑦ 指针层与覆盖层（picker/Modal/按压/手势）
bash tools/clean-target.sh                 # ⑧ 清理（必做，见下）
```

涉及渲染的改动再加逐页快照 + 与 HTML 参照实现对比：

```bash
bash tools/snapshot-all.sh sample-app target/fin2_sample --settle 0 --time 2
./target/release/examples/compare --all --rust-from target/fin2_sample --out target/fin2_cmp
python3 tools/pixdiff.py a.png b.png       # 差异占比 + 包围盒
```

当前基线：sample-app **4.55%**、news-app **6.33%**（与编译出的 HTML 对比的变化像素比）。

### 两条硬规则

**跑完必须清 `target/`。** 每个截图/对比工具都往 `target/<自己起的名字>` 里写，
一轮回归多出几十个目录，实测涨到过 **26GB**。

```bash
bash tools/clean-target.sh            # 清测试产物 + target/debug（保留 release，不重编）
bash tools/clean-target.sh --targets  # 再清交叉编译目标目录（iOS/Android，约 6GB）
bash tools/clean-target.sh --all      # 连 cargo 缓存一起清
bash tools/clean-target.sh --dry      # 先看会删什么
```

基线目录（`target/fin2_*`）也一并清 —— 一条 `snapshot-all.sh` 就能重新生成。

**跑快照前必须 `rm -rf target/mini-storage`。** 无头点击会把数据落到
`target/mini-storage/<app>.json`，下次启动读回来页面内容就变了，曾让购物车页从
1.51% 假崩到 10.05%。另外 `sample-app` 的基线必须用 `--settle 0`（首页有延时浮层，
`--settle ≥ 0.5` 会造成 93.9% 的假差异）。

细则见 [`.kiro/steering/30-test-and-cleanup.md`](.kiro/steering/30-test-and-cleanup.md)。

---

## 仓库结构

```
src/
├── parser/          WXML / WXSS / 模板与表达式引擎
├── renderer/        渲染器
│   ├── components/  24 个标签的建树与绘制（含 WeUI 图标字形表、SVG 路径解析）
│   │   └── base/      公共底座：类型 / 文本度量 / 换行 / 事件属性 / 样式落地 / 盒子绘制
│   └── wxml_renderer/  布局缓存 / 绘制调度 / 命中测试 / 动画 / 局部重绘失效
├── host/            **宿主层（平台无关）**：页面栈、覆盖层、触摸状态机、像素合成
│   └── engine.rs      MiniEngine —— 移动端 SDK 的核心
├── js/              QuickJS 绑定；`prelude/*.js` 是按域拆分的逻辑层前置代码
│                    （module / console / storage / ui / route / device / network …）
├── runtime/         MiniApp（逻辑层驱动、定时器、桥事件）
├── ui/              交互管理、滚动控制器
├── compiler/html/   编译成 HTML 工程（双端对比的参照实现）
├── ffi_app.rs       App 级 C ABI（mr_app_*）
├── ffi_jni.rs       Android JNI 入口
└── bin/             桌面宿主
    ├── window.rs      只剩 struct + new() + main()（从前 1877 行装了四件事）
    └── app_window/    按职责分片：winit_app（winit 适配）/ frame（出帧闸门）/
                       frame_render（合成上屏）/ page_host（页面栈路由）/
                       scroll_host / tabbar_host / overlay_host / headless_run

sdk/                 Android（AAR）/ iOS（XCFramework + SPM + CocoaPods）
tools/               构建与回归脚本
examples/            画廊、双端对比、内存/字体探针、SDK 一致性
sample/              示例小程序（sample-app / news-app / tea-app …）
doc/                 文档与场景图
```

**桌面宿主与移动端 SDK 共用 `src/host/`**，只有帧调度各自实现；
`tools/sdk-parity.sh` 逐像素守着这条边界（当前 sample 15/15、news 6/6 一致）。

工程约定：单个 `.rs` 超过 500 行就拆（例外要在文件头写理由），见
[`.kiro/steering/`](.kiro/steering/)。各模块的职责与渲染器内部的分工见
[`doc/架构与实现原理.md`](doc/架构与实现原理.md#分层结构)。

---

## 文档

| 文档 | 内容 |
|---|---|
| [`doc/架构与实现原理.md`](doc/架构与实现原理.md) | 一帧是怎么画出来的、分层与模块划分、帧成本怎么压下来、局部重绘、滚动手感与触控对齐、依赖版本与 taffy 适配 |
| [`doc/示例与调试.md`](doc/示例与调试.md) | 示例小程序清单、`examples/` 各示例、浏览器预览、编译成 HTML、视频/Canvas、Rust/C/移动端代码示例 |
| [`doc/场景画廊.md`](doc/场景画廊.md) | 65 张真实渲染（同时是回归基线），以及素材怎么生成 |
| [`doc/踩坑记录.md`](doc/踩坑记录.md) | 12 类「曾经错在哪」：现象 → 根因 → 现在的做法 → 回归判据 |
| [`doc/引擎测试说明.md`](doc/引擎测试说明.md) | 能力矩阵、T1~T9 逐项测试清单与判据、内存分项实测、已知差距 13 条 |
| [`doc/原生App集成指南.md`](doc/原生App集成指南.md) | 极简集成、宿主九件事、C ABI、iOS/Android/鸿蒙步骤、验收清单 |
| [`doc/触控对齐测试报告.md`](doc/触控对齐测试报告.md) | 滑动/点击/长按/手势与微信的逐项对照实测 |
| [`doc/实现组件说明.md`](doc/实现组件说明.md) | 组件实现状态明细 |
| [`sdk/README.md`](sdk/README.md) | SDK 三行接入与三条必须知道的约定 |

## 已知的最大待办

三件，按优先级：

1. **内存峰值**（600MB~1GB，98% 是字体）—— 要换惰性字体后端，细节见
   [能力边界](#能力边界能做什么明确不做什么)那一节；
2. **`setData` 仍会整棵重建布局树**（首页每秒那一帧约 11ms，卡在 144Hz 预算边缘）——
   要做增量树打补丁，基础设施（`FramePlan` / 损伤区 / `damage-check.sh`）已经就位；
3. **移动端 SDK 的左边缘侧滑返回**还没接 —— 它要在页面被覆盖那一刻留一张视口图，
   属于宿主的帧管理，跟 SDK 的局部重绘一起做更合适。

## 许可

MIT。内置图标矢量数据来自腾讯官方开源的 WeUI（MIT）。
