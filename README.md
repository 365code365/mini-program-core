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
- **不给盒子留"保险余量"**：文本盒宽度就是字形度量之和（只向上取整到整像素），换行判定另留 0.5px 亚像素容差。曾经用"每个文本盒 +4px"防误换行，结果所有按内容定宽的元素（徽标/标签/胶囊）都比浏览器宽一圈。

### 双端一致性是被量化验证的

引擎自带对比工具：同一份小程序源码，一边走原生渲染出 PNG，一边编译成 HTML 用 Chrome 截图，逐像素比对并产出报告。

```bash
cargo run --example compare -- --all --out target/render-compare
# 输出 rust.png / html.png / side-by-side.png / diff.png + report.json（变化像素比、MAE、RMSE）
```

更进一步，**被对比的可以是窗体宿主真实出的那一帧**：窗体带无头快照模式，走与交互运行完全相同的管线（页面加载、`app.wxss` 合并、自定义 tabBar、fixed 覆盖层、Toast/Modal、像素合成顺序）输出整帧 PNG，再交给对比工具。这样报告衡量的是「用户真正看到的画面 vs H5」，而不是示例里另写一份渲染。

```bash
# 整帧快照：可指定动画时刻 / 滚动位置 / 注入交互后的状态
cargo run --release --bin mini-app-window -- sample-app --snapshot target/window-snap
cargo run --release --bin mini-app-window -- sample-app --snapshot target/s \
    --route pages/components/components --scroll 760 --time 0.45
cargo run --release --bin mini-app-window -- sample-app --snapshot target/s \
    --route pages/category/category --eval "__currentPage.onPlus({currentTarget:{dataset:{id:101}}})"

# 让窗体的帧参与逐像素对比
cargo run --release --example compare -- --all --rust-from target/window-snap --out target/window-compare
```

当前实测（375×667 @2x，窗体整帧 vs Chrome）：`sample-app` 15 页整体差异 **4.8%**（单页 1.5%~7.8%），`news-app` 6 页 **6.8%**，剩余差异集中在大图缩放插值与粗体字形/亚像素文本位置。

### 编译器：同一份源码，编译出别端源码

除了原生渲染，引擎还包含一个多目标编译器（`src/compiler/`）。它复用同一套解析器与数据快照，把小程序编译成**目标端的源码工程**，而不是套壳运行：

```bash
cargo run --bin mini-compiler                      # sample-app → dist-html/
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

# 3) 运行示例
cargo run --example gallery         # 渲染全部 64 个场景到 doc/gallery/
cargo run --bin mini-devserver      # 浏览器调试预览（见下节）
cargo run --example video_player    # 独立视频播放窗口（自动循环 + 声音）
cargo run --bin mini-app-window     # 窗口应用（加载 sample-app）
cargo run --bin mini-launcher       # 小程序启动器（扫描 sample 目录）

# 4) 测试（214 个用例）
cargo test
```

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

## 📱 两个可用的示例小程序

仓库里的 `sample-app`（商城）与 `news-app`（头条新闻）不是静态样板，而是**功能闭环**的小程序，原生窗体与编译出的 H5 行为一致：

| | `sample-app` 商城（15 页） | `news-app` 新闻（6 页） |
|---|---|---|
| 首页 | 自动轮播 banner、金刚区、**进页即弹新人优惠券**（缩放入场动画）、限时秒杀真实倒计时、触底加载、下拉刷新 | 要闻轮播、频道横滑切换、下拉刷新、触底加载、卡片收藏 |
| 详情 | 图片轮播、规格半屏弹层、加购 / 立即购买、rich-text 图文、评价 | 正文字号设置**即时生效**、评论发布、点赞 / 收藏 / 关注持久化、相关阅读 |
| 交易 · 互动 | 购物车（storage 持久化）→ 确认订单（地址 / 配送 picker / 优惠券 / 支付方式 / 备注）→ 提交下单 → 订单列表 + 物流时间轴 + 确认收货；搜索页含历史与热搜 | 搜索（历史 + 热搜榜 + 排序）、热榜 + 签到日历 + Canvas 阅读统计 |
| 能力总览 | `pages/showcase`：CSS 动画、transform、彩色 emoji、竖向轮播、GIF、表单控件全家桶、进度 / 评分 / 徽标、rich-text、Canvas 图表、Toast / Loading / Modal / 操作面板 | 视频页可真实播放 `<video>`，自动播放开关持久化 |
| 返回上一页 | 二级页自绘返回栏（`wx.navigateBack`）；逻辑层页面栈复用实例，返回不会重跑 `onLoad` | 同上 |

跨页数据（购物车、订单、收藏、设置、搜索历史）统一走 `wx.storage`，所以编译成 H5 后即使每个页面是独立文档也不丢状态。

```bash
cargo run --release --bin mini-app-window -- sample-app   # 商城
cargo run --release --bin mini-app-window -- news-app     # 新闻
```

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

模块按职责细分，便于扩展维护：`parser/`（wxml/wxss/expr/template）、`renderer/`（wxml_renderer + `components/*` 每组件独立文件 + `style_parse` CSS 取值解析）、`layout/`、`js/`（runtime/api/bridge）、`ui/`（交互/滚动）、`runtime/`、`bin/`（各可执行程序）。

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
cargo test          # 214 个用例
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
│   ├── js/                     # QuickJS runtime / api（App/Page/Component）/ bridge
│   ├── parser/                 # wxml / wxss（选择器引擎）/ expr / template
│   ├── renderer/               # wxml_renderer / components/*（含 style_parse、canvas、video）
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
