# mini-render

**自绘的小程序渲染引擎。用 Rust 从零实现，不基于 WebView、不映射系统控件、不依赖 Skia。**

输入是一份微信小程序源码（WXML / WXSS / JS / app.json），输出是一块 RGBA 像素缓冲。
对齐目标是微信的 **Skyline**（同样是原生渲染）。

```
WXML/WXSS ─┐
           ├─ 解析 → 模板求值 → 样式计算 → Flexbox 布局(taffy) → 自研光栅器 → RGBA
page.js ───┘        （逻辑层是 QuickJS：App / Page / Component / CommonJS / Promise）
```

| | |
|---|---|
| 产物 | `cdylib` / `staticlib` / `rlib` —— Android / iOS / Windows / macOS / Linux |
| 逻辑层 | QuickJS（rquickjs） |
| 布局 | taffy（Flexbox），文本节点挂自定义度量函数（按真实字形宽度决定换行） |
| 绘制 | 自研 2D 光栅器：扫描线 even-odd 填充 + 4× 超采样、圆角贝塞尔逼近、fontdue 字形、Apple `sbix` 彩色 emoji、双线性图片采样、渐变/阴影/裁剪栈 |
| 另有 | 内置编译器，可把同一份小程序源码编译成 HTML 工程（用作双端一致性的参照） |

下面四张是引擎自己渲染的输出（纯 WXML + WXSS + 数据，375×667 @2x）：

| | | | |
|:---:|:---:|:---:|:---:|
| <img src="doc/gallery/28_ecommerce_home.png" width="180"/><br/>电商首页 | <img src="doc/gallery/36_news_feed.png" width="180"/><br/>资讯信息流 | <img src="doc/gallery/42_food_order.png" width="180"/><br/>外卖点餐 | <img src="doc/gallery/60_form_controls.png" width="180"/><br/>表单控件 |

全部 65 张见 [`doc/场景画廊.md`](doc/场景画廊.md) —— 它们同时是逐字节可复现的回归基线。

---

## 目录

- [30 秒跑起来（桌面）](#30-秒跑起来桌面)
- [集成进 App（SDK）](#集成进-appsdk)
- [支持范围](#支持范围)
- [测试与回归](#测试与回归)
- [仓库结构](#仓库结构)
- [文档](#文档)

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
| 组件（22 个标签） | view / block / scroll-view / text / button / icon / image / progress / slider / switch / checkbox(-group) / radio(-group) / input / textarea / swiper(-item) / picker / picker-view(-column) / rich-text / video / canvas |
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
cargo test --release                       # ② lib 474 + bin 9
bash tools/damage-check.sh                 # ③ 增量重绘 == 整帧重绘（逐字节）
cargo run --release --example gallery      # ④ 65 张场景图
bash tools/tab-click-check.sh              # ⑤ 三个 app 的 tabBar 点击
bash tools/sdk-parity.sh                   # ⑥ 移动端 SDK 与桌面窗体逐像素一致
bash tools/clean-target.sh                 # ⑦ 清理（必做，见下）
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
│   ├── components/  22 个组件的建树与绘制（含 WeUI 图标字形表、SVG 路径解析）
│   └── wxml_renderer/  布局缓存 / 绘制调度 / 命中测试 / 动画 / 局部重绘失效
├── host/            **宿主层（平台无关）**：页面栈、覆盖层、触摸状态机、像素合成
│   └── engine.rs      MiniEngine —— 移动端 SDK 的核心
├── js/              QuickJS 绑定与 wx.* 实现
├── runtime/         MiniApp（逻辑层驱动、定时器、桥事件）
├── ui/              交互管理、滚动控制器
├── compiler/html/   编译成 HTML 工程（双端对比的参照实现）
├── ffi_app.rs       App 级 C ABI（mr_app_*）
├── ffi_jni.rs       Android JNI 入口
└── bin/             桌面宿主：window.rs（winit 适配）+ app_window/（事件适配）

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

**内存**：实测峰值 600MB~1GB，其中 **98% 是字体** —— fontdue 在加载时把字体里全部
29352 个字形几何预展开（一个 CJK 字面约 300MB）。图片/字形/字体缓存都已按字节封顶
（64/24MB + 2 个字体文件，LRU），也有 `trim_memory()` 给宿主在内存告警时调用，
但真正压峰值要换成惰性字体后端（`ab_glyph` / `swash` / `cosmic-text`），
预计每字面 300MB → 约 23MB。这会改变每个字形的抗锯齿，需要单独一轮重定基线。

## 许可

MIT。内置图标矢量数据来自腾讯官方开源的 WeUI（MIT）。
