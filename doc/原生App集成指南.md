# 把 mini-render 集成进原生 App

**先看这一节就够了**：SDK 已经封好，Android / iOS 各三行代码。
后面几节是原理与自定义（想自己控制帧循环、或者接鸿蒙/桌面时再看）。

---

## 极简集成（推荐）

### 1. 编库

```bash
bash tools/build-mobile.sh android   # → sdk/android/jniLibs/<abi>/libmini_render.so
bash tools/build-mobile.sh ios       # → sdk/ios/MiniRender.xcframework
bash tools/build-mobile.sh check     # 只做交叉编译检查
```

> 两个构建参数缺一个都编不过，脚本里已经带好：`--no-default-features`
> （关掉 winit/剪贴板/声卡这些移动端不该编进去的桌面依赖）、`--features bindgen`
> （`rquickjs-sys` 没有为 iOS/Android 预生成 C 绑定，要现场生成）。

### 2. Android（Kotlin）

把 `sdk/android/*.kt` 拷进工程，`jniLibs` 放到 `src/main/jniLibs/`：

```kotlin
class MiniActivity : AppCompatActivity() {
    private lateinit var mini: MiniProgramView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        mini = MiniProgramView(this)
        setContentView(mini)
        mini.open(File(filesDir, "my-mini-app"))     // 解包后的小程序目录
    }

    override fun onBackPressed() {
        if (!mini.goBack()) super.onBackPressed()     // 页面栈空了才退出
    }

    override fun onResume()  { super.onResume();  mini.onHostResume() }
    override fun onPause()   { super.onPause();   mini.onHostPause() }
    override fun onTrimMemory(level: Int) { super.onTrimMemory(level); mini.onHostTrimMemory() }
    override fun onDestroy() { super.onDestroy(); mini.close() }
}
```

### 3. iOS（Swift）

把 `MiniRender.xcframework` 拖进工程，`sdk/ios/MiniProgramView.swift` 拷进来：

```swift
let mini = MiniProgramView(frame: view.bounds)
view.addSubview(mini)
mini.open(appDir: unpackedDir)                  // 解包后的小程序目录

// 返回按钮 / 侧滑：
if !mini.goBack() { navigationController?.popViewController(animated: true) }
```

前后台与内存告警 `MiniProgramView` 自己监听了系统通知，不用管。

### 这两个 View 替你做了什么

| 事情 | 说明 |
|---|---|
| 专用渲染线程 | 引擎不是线程安全的（逻辑层是 QuickJS），所有调用都排到同一条线程 |
| 尺寸与 dpr | 按 View 的 `bounds` × `density`/`scale` 算，`rpx` 由引擎自己换算 |
| 触摸 | `onTouchEvent` / `touchesXxx` → 引擎的**指针三段**，含历史点/`coalescedTouches`（不抽稀） |
| 出帧 | 跟着 vsync，引擎回报还有活（`WANTS_MORE`：动画 / 惯性 / 定时器 / 请求在飞）才续帧；静止时不出帧（不耗电） |
| 上屏（Android） | 画在视图里的一块 SurfaceView 上，引擎线程直接交给 SurfaceFlinger（不经 UI 线程 `onDraw`）；后端默认 auto：GPU 合成（滚动帧只合成）→ CPU 直写 Surface → Bitmap，失败逐级回退。`backend` 可强制、`activeBackend` / `backendReason` 可查。为什么见 [滑动顿挫避坑清单](滑动顿挫避坑清单.md) |
| 导航栏（Android） | 默认引擎画（`navigationBar = true`，与 Flutter 插件一致）、页面转场默认开（`routeAnimation`）；`open` 之前可关 |
| 诊断开关（Android） | `adb shell setprop debug.mini.fps 1`（`damage_log` / `backend` / `cpu_pin` …同 Flutter 插件），重开小程序生效 |
| 上屏（iOS） | 引擎线程 + `CADisplayLink`（要 120Hz），画面画进自己的 `CAMetalLayer`，后端默认 auto（GPU 合成），失败回退 CPU（`CGImage`）；`backend` / `activeBackend` / `backendReason` |
| 宿主能力 | `hostApis["login"] = …`（两端都有）：点了名的 `wx.*` 交给宿主、回包走 `MiniHostReply`；Android 另**自带**选图 / 拍照 / 选视频 / 选文件 / 存相册 / 打开文档（`builtinHostApis`，透明的 `MiniMediaActivity` 代办，宿主不用转发 `onActivityResult`；清单要有它与 `MiniFileProvider`，AAR 自带） |
| 原生组件（Android） | `sameLayerViews["map"] = { MyMap() }`：宿主画、引擎画进页面（同层渲染），滑动时不错开；`<video>` 默认同层渲染（`sameLayerVideo`） |
| 沙盒目录 | 自动指到 `filesDir` / `Caches`，storage 与图片磁盘缓存才生效 |
| 生命周期 | 前后台切换、内存告警转发 |

### 产物体积（实测 arm64）

| 项 | 大小 |
|---|---|
| `libmini_render.so`（未 strip） | 41.3 MB |
| strip 之后 | 34.9 MB |

偏大，两个已知原因，都还没优化：
- `page_loader.rs` 里用 `include_str!` 把 sample-app 的页面源码当兜底内置进去了 ——
  正式 SDK 该去掉；
- openh264 / symphonia（视频与音频解码）整个链进来了，移动端本该交给系统硬解。
  `--no-default-features` 只关掉了音频**设备**，解码器还在。

要压体积的话按这两条来，另外 release 加 `strip = true` 和 `panic = "abort"` 还能再省一些。

### 只有两件事必须由你决定

1. **小程序包从哪来**：下载、校验、解包到沙盒目录（引擎按目录读，不解析 `.wxapkg`，
   也不做签名校验 —— 分包、灰度、完整性都属于宿主的事）。
2. **原生层组件谁承载**：`<web-view>` / `<video>` / `<map>` 在微信里是原生组件层
   （层级最高，只有 `cover-view` 能盖住）。引擎只负责算位置与参数，控件由你用
   `WKWebView` / `android.webkit.WebView` / ArkUI `Web()` 放上去。
   **不建议把 wry 编进引擎**，原因见第一节第 7 条。

### 怎么确认没接错

```bash
bash tools/sdk-parity.sh              # SDK 那条路 vs 桌面窗体，逐像素对比
```

SDK 用的宿主（`mini_render::host::MiniEngine`）与桌面窗体共用 `host::` 下的页面
加载、覆盖层、触摸状态机、像素合成，只有帧调度各自实现。这个脚本把每个页面用
SDK 那条路渲染出来和桌面基线逐像素比 —— 当前 **sample-app 15/15、news-app 6/6
完全一致**。哪天两条路开始分叉，它会先叫。

---

## 零、先明确引擎的定位

- 引擎是**自绘渲染器**：自己做布局（taffy）、自己光栅化（字形/路径/图片），
  最终产出一块 **RGBA 像素缓冲**。它不是 WebView，也不需要 WebView。
  对齐目标是微信的 **Skyline**（同样是原生渲染），不是 WebView 那套。
- 逻辑层是 **QuickJS**（rquickjs）跑小程序的 `app.js` / 页面 `js`，
  `wx.*` 由引擎实现，需要系统能力的部分回调到宿主。
- 所以宿主要做的事，本质上就是三件：**给它一块画布、把输入喂进去、把它要的系统能力接上**。

```
┌─────────────────────────────────────────────────────────┐
│ 宿主 App（Swift / Kotlin / ArkTS / C++）                  │
│  · 一个可显示位图的 View（UIView / SurfaceView / XComponent）│
│  · 触摸/键盘事件 → 引擎                                    │
│  · 原生层组件（web-view / video / map）                     │
│  · 沙盒目录、网络权限、剪贴板、返回键                          │
└───────────────▲───────────────────────┬─────────────────┘
                │ 像素缓冲 + 脏矩形       │ 事件 / 生命周期
┌───────────────┴───────────────────────▼─────────────────┐
│ mini-render（Rust，libmini_render.a / .so / .dylib）      │
│  布局 taffy → 绘制 → RGBA 缓冲                            │
│  wx.* 实现 / 页面栈 / 滚动与手势 / 局部重绘                  │
├─────────────────────────────────────────────────────────┤
│ QuickJS：小程序的 app.js / 各页面 js                        │
└─────────────────────────────────────────────────────────┘
```

**参考实现就在仓库里**：`src/bin/window.rs` + `src/bin/app_window/` 是一个完整的宿主
（桌面版，winit + softbuffer）。下面每一条都会指到它的对应位置 ——
移植到 iOS/Android/鸿蒙时，照着这份实现搬即可。

---

## 一、宿主要做的九件事

### 1. 准备小程序包，并告诉引擎资源根目录

引擎按**目录**读小程序（`app.json` / `pages/**` / `assets/**`），不解析 `.wxapkg`。
宿主负责下载、校验、解包到沙盒，然后：

```rust
mini_render::assets::set_app_root("/path/to/unpacked/app");   // 图片等相对路径的解析根
mini_render::data_dir::set_data_dir("/path/to/app/sandbox");  // 见第 8 条
```

- `assets::resolve()` 用它把 `/assets/a.png`、`../img/b.jpg` 解析成真实文件；
- 引擎**不做包签名校验**，也不限制包大小 —— 分包、版本灰度、完整性校验都在宿主。

### 2. 创建引擎实例

```rust
let mut app = mini_render::runtime::MiniApp::new(375, 667)?;  // 逻辑尺寸（pt/dp）
app.init()?;                                                  // 注册 wx.* 与桥
app.load_file("app.js")?;                                     // 或 register_all_modules
```

渲染器另建（页面级）：

```rust
let mut r = mini_render::renderer::WxmlRenderer::new_with_scale(
    stylesheet, 375.0, 667.0, dpr /* 2.0 / 3.0 */);
```

**尺寸约定**：传给引擎的是**逻辑尺寸**（iOS 的 pt、Android 的 dp），
`dpr` 单独给。像素缓冲的大小是 `逻辑尺寸 × dpr`。
`rpx` 由引擎按 `screen_width / 750` 换算，宿主不用管。

### 3. 每帧驱动（这一条最容易漏）

照 `src/bin/window.rs::pump_one_frame`（第 341 行起）搬，顺序不能变：

| 步骤 | 调用 | 漏了会怎样 |
|---|---|---|
| 1 | `app.update()` | 定时器、网络回调、Promise 全部不跑 |
| 2 | 取 JS 输出（`print_js_output`） | `console.log` 攒在缓冲里，排查时误判成「回调没执行」 |
| 3 | `app.take_data_dirty()` | `setData` 之后不重绘 |
| 4 | `app.drain_ui_events()` → toast / loading / modal / 下拉刷新 | `wx.showToast` 之类没反应 |
| 5 | 更新滚动（惯性、回弹、下拉刷新触发） | 松手后不再滑行 |
| 6 | `check_navigation(&mut app)` → 执行路由 | 「倒计时结束自动跳转」只打日志不换页 |
| 7 | `advance_due_swipers()` | 轮播不动 |
| 8 | `app.take_network_dirty()` / `image_net::take_dirty()` | 图片下载完不上屏（要**整帧**重绘，见坑 A） |
| 9 | 长按计时 `touch.tick(clock)` | `longpress` 永不触发 |
| 10 | 渲染 + 上屏 | — |

> **坑 A**：远程图片/网络数据刚到位那一帧必须**整帧**重绘。
> 只调渲染的话，重绘范围会被上一次 `setData` 的增量失效区收窄 ——
> 表现是「图片一直是占位图，滑一下才出来」。

驱动频率：有动画/滚动/自动播放时按屏幕刷新率（`CADisplayLink` /
`Choreographer` / `vsync`）；否则可以停下来等事件，不必空转。

### 4. 上屏

引擎产出 RGBA8 缓冲：

```rust
let px: Vec<u8> = app.to_rgba();          // 或 canvas.pixels()
```

- **iOS**：首选把一块 `CAMetalLayer` 交给引擎（`mr_app_attach_surface(MR_SURFACE_METAL_LAYER)`，GPU 合成、滚动只合成）；
  CPU 退路是 `mr_app_pixels_into(BGRA)` → `CGImage` → `CALayer.contents`（子图层要关掉隐式动画，否则每帧淡入淡出 0.25s）；
- **Android**：首选自己的 SurfaceView（GPU：`nativeAttachSurface`；CPU：`ANativeWindow_lock` 直写，零中间拷贝）；
  **别**走 `Bitmap.copyPixelsFromBuffer` + `onDraw`（每帧三次整帧拷贝 + 一次纹理上传，原生示例实测屏上重复 20%，
  见 [滑动顿挫避坑清单](滑动顿挫避坑清单.md)）；
- **鸿蒙**：`XComponent` + `OH_NativeWindow`，或 `PixelMap` 喂给 `Image`；
- **桌面**：仓库用的是 `softbuffer`。

**局部重绘**：引擎内部已经算好损伤区（`damage_clip`），只重画那一块。
宿主如果能拿到脏矩形就只贴那一块，能省一次全屏拷贝。
页面画布用的是**内容坐标**（整页高），滚动只是换一条切片、不重绘 ——
这也是滚动帧只花「上屏拷贝」时间的原因（实测滑动帧 p50 0.07ms）。

### 5. 事件注入

必须走**完整的指针三段**，不要只送「点击」：

```
按下 → on_pointer_press(x, y)
移动 → on_pointer_move(x, y)     （每个移动点都送，别抽稀）
抬起 → on_pointer_release(x, y)
```

引擎内部据此产出 `touchstart / touchmove / touchend / touchcancel /
longpress / longtap / tap`，并做手势仲裁（主轴锁定、内外层滚动交接、侧滑返回）。

> **坑 B**：只送「点击」会漏掉一整类 bug。曾经 tabBar 按下时提前 return，
> 触摸状态机没启动 → 抬手没有 tap → 真机上 tabBar 完全点不动，
> 而只用「直接调 click」的测试全绿。所以宿主也**只走指针三段**这一条路。

坐标是**逻辑坐标**（左上原点）。其余输入：

| 输入 | 说明 |
|---|---|
| 滚轮 / 触控板 | `--wheel` 那条链路，桌面端用 |
| 键盘 / IME | 先点中 `<input>` 再送字符；`enter` → `confirm` |
| 系统返回键（Android/鸿蒙） | 映射到 `navigateBack`；页面栈只剩一页时交还宿主（退出小程序） |
| 侧滑返回 | 引擎自带（`app_window/edge_back.rs`），宿主只要把手势坐标送进来 |

### 6. 路由与页面栈

逻辑层调 `wx.navigateTo` 等会写 `__pendingNavigation`，宿主每帧取一次并执行：

| 类型 | 宿主要做的 |
|---|---|
| `navigateTo` | 入栈；**入栈前给当前页留一张截图**（侧滑返回时要显示在新页下面） |
| `navigateBack` | 出栈，恢复上一页的滚动位置与数据 |
| `redirectTo` | 关掉当前页再开新页，栈深不变 |
| `reLaunch` | 清空整个栈 |
| `switchTab` | 切 tab，重置页面栈 |

换页前**必须派发 `onUnload`**，否则上一页的守护定时器会把路由顶回去
（启动页那种「N 秒后跳首页」的写法尤其明显）。
页面栈上限 10（微信语义），返回截图跟着裁，免得越走越占内存。

`app.json` 里声明了 `tabBar` 时，tabBar 由宿主外壳绘制，层叠顺序是：
**页面内容 → tabBar → 页面内 `position:fixed` 覆盖层**。
这样全屏遮罩会同时压暗 tabBar，而 tabBar 又不会被页面背景盖住。

### 7. 原生层组件（`web-view` / `video` / `map` / `camera`）

早期微信里这些是**原生组件层**，层级最高，只有 `cover-view` / `cover-image` 能盖在上面；
同层渲染之后它们插在渲染树里，按层叠顺序被普通节点、picker 面板、弹窗正常盖住。
引擎的做法：**引擎只算位置、参数与遮挡，实际控件由宿主创建**。

引擎画面是一整块不透明的纹理，控件只能摆在它上面，所以「被谁盖住」由引擎算好随
`nativeViews` 事件交给宿主（`clip` / `holes` / `mask`，见第二部分的事件表），宿主照着裁。
不裁的话 picker 面板一弹，视频就压在面板和遮罩上面。

```c
// 引擎 → 宿主：原生层组件的摆放（待新增，见第二部分）
typedef void (*mini_native_view_cb)(
    const char* kind,     // "web-view" / "video" / "map"
    const char* node_id,
    const char* payload,  // JSON: {"src":"https://...","x":0,"y":88,"w":375,"h":579}
    void* user_data);
```

宿主按平台各用自己的控件：iOS `WKWebView`、Android `android.webkit.WebView`、
鸿蒙 ArkUI `Web()`、Windows WebView2、Linux WebKitGTK。

**不建议把 wry 编进引擎**（2026-07 复核 wry 0.55.1）：

| 平台 | wry 后端 | 说明 |
|---|---|---|
| Windows | WebView2 | 可用，需 WebView2 Runtime |
| macOS / iOS | WKWebView | 可用 |
| Linux | WebKitGTK | 需 GTK；配 winit 只能 X11，Wayland 要走 tao 或 `build_gtk` |
| Android | 系统 WebView | 要求宿主 Activity 继承 `AppCompatActivity`/`WryActivity` + 环境变量生成 Kotlin + `android_setup`/`android_binding!` |
| 鸿蒙 / OpenHarmony | — | **正式版没有**：`src/` 只有 `android / webkitgtk / webview2 / wkwebview`，鸿蒙实现在未发布的 `feat/open-harmony` 分支 |

三个理由：鸿蒙断供；Android 要改宿主 Activity（集成进既有 App 基本谈不下来）；
而且 wry 给的也只是一个原生子视图，宿主自己建同样的控件反而少一层绑定。
（来源：[wry README](https://github.com/tauri-apps/wry)、[PR #1607](https://github.com/tauri-apps/wry/pull/1607)、[Tauri issue #14397](https://github.com/tauri-apps/tauri/issues/14397)；内容已改写以符合许可要求。）

桌面端**自研调试窗体**要嵌网页时，wry 是合适的。

### 8. 系统能力

| 能力 | 现状 | 宿主要做的 |
|---|---|---|
| storage | 引擎落盘到 `<数据目录>/mini-storage/<小程序名>.json` | **必须**先 `data_dir::set_data_dir(沙盒目录)`。以前写死在编译机的 `target/` 下，装到设备上会静默退化成纯内存 —— 「首次拉数据存起来、之后走缓存」的应用每次冷启都走首次分支 |
| 网络 | `wx.request` 已实现（后台线程 + 按帧取回，支持 abort） | iOS 配 ATS / Android `network_security_config` / 鸿蒙 `ohos.permission.INTERNET`；域名白名单由宿主管 |
| 图片磁盘缓存 | `<数据目录>/mini-imgcache` | 同 storage，靠 `set_data_dir` |
| 字体 | 走系统字体路径 | **各平台字体路径不同**，见第三部分「字体」 |
| 剪贴板 | 引擎里存一份；桌面窗体同步系统剪贴板（`arboard`） | 移动端收 `clipboard` 宿主事件写系统剪贴板；`getClipboardData` 要读系统剪贴板就在 `hostApis` 里点名 |
| 音视频 | 内置 symphonia + rodio + openh264 解码 | 移动端建议改用系统播放器（省电、省内存、走硬解） |
| 定位/支付/登录等 | 引擎做不了，默认立刻 `fail`（`wx.login`、`requestPayment`、`getLocation`…） | 宿主在 `hostApis` 里点名就交给宿主（`hostCall` → `mr_app_host_reply`） |
| 选图/选视频/选文件/存相册/打开文档 | 同上（要系统的选择器）；参数归一、移动端「拍照 / 从手机相册选择」菜单、回包补字段、文件进出沙盒由引擎做 | Flutter 插件**自带**（`builtinHostApis`）；桌面窗体用系统文件对话框；其它宿主点名后按下文「传文件」的约定回包 |

### 9. 生命周期与内存

| 宿主事件 | 要调 |
|---|---|
| 进前台 | `__dispatchApp('onShow')` + 恢复出帧 |
| 进后台 | `__dispatchApp('onHide')` + **停止出帧**（否则白耗电） |
| 内存告警（iOS `didReceiveMemoryWarning` / Android `onTrimMemory` / 鸿蒙 `onMemoryLevel`） | `mini_render::trim_memory()` |
| 关闭小程序 | 销毁实例 + `trim_memory()` |

内存现状（实测，见 `doc/引擎测试说明.md` 五之二）：

- 峰值 600MB~1GB，**其中 98% 是字体**（fontdue 在加载时把字体里全部
  29352 个字形几何预展开，一个 CJK 字面约 300MB）；
- 已封顶的部分：图片 64MB / 动图 32MB / 字形位图 24MB / 自定义字体文件 2 个，
  全部 LRU 逐出，可用 `MINI_IMAGE_CACHE_MB`、`MINI_GLYPH_CACHE_MB`、`MINI_FONT_CACHE_MAX` 调；
- **移动端上线前必须先解决字体内存**：换成惰性字体后端（`ab_glyph`/`swash`/`cosmic-text`）
  预计每字面 300MB → 约 23MB。这件事会改变每个字形的抗锯齿，需要单独一轮重定基线。

监控接口：

```rust
mini_render::memory_report();
// 静态图 12 张 / 31.4MB（逐出 3），动图 0 段 / 0.0MB（逐出 0），预算 64MB + 32MB；自定义字体 1 个（…Songti.ttc）
```

---

## 二、App 级 C ABI（已实现）

实现在 `src/ffi_app.rs`，声明在 `include/mini_render.h`。Kotlin 走
`src/ffi_jni.rs`（`jni` crate，只在 android 目标编译），Swift 直接调 C。

整套只有十几个函数，没有回调注册：

```c
typedef struct MRApp MRApp;

MRApp*   mr_app_create(const char* app_dir, const char* data_dir,
                       uint32_t width, uint32_t height, float dpr);
void     mr_app_destroy(MRApp*);
int      mr_app_launch(MRApp*, const char* route);   // NULL = app.json 首页
int      mr_app_navigate(MRApp*, const char* url);   // 深链

int      mr_app_pump(MRApp*, uint64_t now_ms);       // 1 = 有新画面
void     mr_app_pixel_size(MRApp*, uint32_t* w, uint32_t* h);
size_t   mr_app_pixels(MRApp*, uint8_t* out, size_t len);   // RGBA8

void     mr_app_pointer_down(MRApp*, float x, float y);
void     mr_app_pointer_move(MRApp*, float x, float y);
void     mr_app_pointer_up(MRApp*, float x, float y);
void     mr_app_pointer_cancel(MRApp*);
void     mr_app_wheel(MRApp*, float delta_y, int precise);
void     mr_app_text_input(MRApp*, const char* utf8);
void     mr_app_key(MRApp*, const char* name);

int      mr_app_back(MRApp*);            // 0 = 栈空，宿主该关掉小程序
uint32_t mr_app_page_depth(MRApp*);
void     mr_app_on_show(MRApp*);
void     mr_app_on_hide(MRApp*);
void     mr_app_trim_memory(MRApp*);
const char* mr_app_last_error(MRApp*);
const char* mr_version(void);
```

**还没有的**：损伤矩形（`mr_app_pump` 目前每次出帧都整条带重画）、输入法组字（只能提交整段文字）。
原生组件摆放、日志、软键盘、导航栏这些「引擎 → 宿主」的通知走下面的事件队列（不是回调）。

### 渲染后端（只增不改）

实现在 `src/ffi_backend.rs`，声明在 `include/mini_render.h` 末尾。引擎的内容永远由 CPU 光栅；
后端只决定「谁把各图层合成上屏」。GPU 合成器（`compositor::gpu`）与 CPU 逐字节一致（启动自检守着），
桌面窗体、无头、**SDK 的 surface 上屏**共用同一份（SDK 侧接线在 `src/host/engine_gpu.rs`）。
Android 的库默认编进 `gpu`（`build-mobile.sh`，`ANDROID_GPU=0` 关掉，arm64 实测 +5.1MB：36.3→41.4MB）；
iOS 的库也编进 gpu（`tools/build-mobile.sh ios`，`IOS_GPU=0` 关），原生 `MiniProgramView` 把自己的 CAMetalLayer 交上来；
Flutter 的 iOS 外部纹理是 CVPixelBuffer，用不上 CAMetalLayer，那条路照旧 CPU。宿主链接时要带 Metal / QuartzCore /
CoreGraphics（Package.swift、podspec、module.modulemap 已写上）。
上面那 20 个函数的签名与语义一个没改，**不调这组函数、或偏好保持 cpu 的宿主，行为与以前逐字节一致**。

```c
#define MR_BACKEND_AUTO 0
#define MR_BACKEND_CPU  1
#define MR_BACKEND_GPU  2
#define MR_SURFACE_METAL_LAYER     1   /* CAMetalLayer*（iOS / macOS） */
#define MR_SURFACE_ANDROID_WINDOW  2   /* ANativeWindow* */
#define MR_FRAME_NEW              1    /* 有新画面 */
#define MR_FRAME_PRESENTED        2    /* 引擎已画到 surface 上，宿主不要再取像素 */
#define MR_FRAME_BACKEND_CHANGED  4    /* 实际后端被引擎改了，立刻切换显示路径 */
#define MR_FRAME_PENDING          8    /* 本次没上屏，下一次 pump_ex 会补 */

int  mr_app_set_backend(MRApp*, int pref);                 // 1 成功；pref 不在 0..=2 返回 0
void mr_app_set_device_info(MRApp*, uint64_t total_mem_bytes, int low_ram, uint32_t os_api);
int  mr_app_attach_surface(MRApp*, int kind, void* native); // 2 GPU / 1 CPU / 0 参数错误
void mr_app_detach_surface(MRApp*);
int  mr_app_backend(MRApp*);                               // 1 CPU / 2 GPU；app 为空返回 0
const char* mr_app_backend_reason(MRApp*);                 // 到下一次调用 mr_app_* 之前有效
int  mr_app_pump_ex(MRApp*, uint64_t now_ms);              // MR_FRAME_* 的按位或
```

- **符号集合不随构建变化**：有没有 `gpu` feature 都导出这 7 个函数，宿主不用为两种构建写两套链接。
- **默认 CPU**：C ABI 的默认偏好是 `cpu`，老调用方从不挂 surface，行为与以前完全一样。
- **优先级**：环境变量 `MINI_BACKEND` > `mr_app_set_backend` > 默认 `cpu`。`MINI_BACKEND` 是诊断
  开关，用途是不改宿主代码也能强制某个后端；它压过宿主配置时引擎会打一行
  `MINI_BACKEND=cpu 覆盖宿主配置 gpu`。Android 用 `android.system.Os.setenv("MINI_BACKEND", "cpu", true)`，
  iOS 用 Xcode scheme 或 `setenv`，都要在创建引擎**之前**设（引擎只在创建时读一次）。
- **`mr_app_pump` 不变**：仍只返回 0 / 1，且 `mr_app_pump(app, t) == ((mr_app_pump_ex(app, t) & MR_FRAME_NEW) ? 1 : 0)`。
  挂了 surface 就改用 `pump_ex`：带 `MR_FRAME_NEW` 且**不带** `MR_FRAME_PRESENTED` 时才走 `mr_app_pixels`。
- **`MR_FRAME_BACKEND_CHANGED` 只在引擎自己改了后端时置位**（运行中回退、`set_backend` 之后重新选择）；
  宿主自己 attach / detach 不置位 —— 宿主自己知道。
- **attach 不带宽高**：引擎按 `mr_app_pixel_size` 配置，宿主把层或 `SurfaceView` 的缓冲设成同样大
  （iOS 设 `drawableSize`，Android 调 `holder.setFixedSize`）。`kind` 必须是当前目标支持的那种
  （iOS 传 2、Android 传 1 都算参数错误）。
- **所有权**：`CAMetalLayer*` 由调用方持有，必须活到 `mr_app_detach_surface` 或 `mr_app_destroy` 返回；
  `ANativeWindow*` 由引擎自己 acquire / release，调用方手里的引用照常自己释放。
- **原因串**：`mr_app_backend_reason` 说明「为什么是这个后端」。没编进 `gpu` feature 时含「未编译」；
  编进来了但没挂 surface 是「未挂载 surface（宿主需调用 mr_app_attach_surface）」；GPU 在用时是
  「强制指定」/「自动选择」；回退后以「回退：」开头；回退过的实例再挂是「本实例 GPU 已失败过：…」。
- **挂着 surface 时改偏好 / 机型信息**：引擎当场重新选。改成 cpu 会放掉合成器，改回 auto / gpu 会重建
  （含自检，几十毫秒）；两种情况下一次 `pump_ex` 都带 `MR_FRAME_BACKEND_CHANGED`。
- **回退是粘性的**：运行中设备丢失 / surface 丢失 / 校验错误当帧改走 CPU（这一帧照样出来），本实例
  之后再挂 surface 也不再试 GPU。没失败过时宿主主动的 detach / attach 照常再试。
- **GPU 帧也能取像素**：`mr_app_pixels*` 会用同一组图层补一次 CPU 合成（与屏上逐字节相同），截图、
  调试 dump 都能用；只是每帧都取就把 GPU 省下的合成又做了一遍。
- **Fifo 上屏**：GPU 在 acquire 时按 vsync 阻塞，`pump_ex` 要放在专用线程上（SDK 的约定本来就是）。
- **后台**：iOS 不允许后台提交 GPU 命令，进后台时停掉 `pump` 是**必须**的；Android 在 Surface 销毁前
  同步 `mr_app_detach_surface`。
- **`MINI_FPS=1`**：SDK 也会每秒打一行 `📊 [后端 CPU] N 帧/秒（pump M 次），pump 最慢 … p99 …，工作 …，等 surface …，帧间隔峰值 …`
  （桌面 / iOS 进 stderr，Android 进 logcat，tag `mini-render`；后端选择与回退那几行同样）。
- **`MINI_DAMAGE_LOG=1`**：每产出一帧打一行它重画了多少 —— `只合成`（滚动只换切片）/ `局部 宽x高@(x,y)`
  （CSS 动画、setData 失效范围、scroll-view 的盒子）/ `补条带`（滑出已绘制条带，只补新露出那截）/
  `整帧·原因`（结构 / 数据 / 动画 / 媒体 / 滚动区）。滑动掉帧时先看它：一串 `整帧` 就说明是谁逼的。
  整个 pump 超过 8ms 时还会多一行 `⏱ [SDK] pump`：重画之前那几段（换页留图、建页、事件回调、setData 之后的
  重建布局，后者分〔模板 / 建树 / 布局 / 换行 / 比对〕）各花了多少 —— 上面那行只算重画那一段，看不见这些。
  `MINI_NO_DAMAGE=1` 关掉局部重画、一律整帧，用来对照「局部和整帧画出来是否一样」
  （判据是 `tests/sdk_damage_equivalence.rs` 的逐帧逐字节比对）。
  `整帧·结构` 那一行末尾带 `← 源码文件:行号`（是哪一行要的整页重画，`MiniEngine::mark_full`），每行末尾是这一帧画 + 合成的毫秒数；
  GPU 上屏时中间还有 `GPU 准备 a（CPU 合成 b）等 c 传 dKB`：b 是直通帧（Toast / Modal / 操作菜单 / picker）里
  CPU 合成覆盖层那一截的时间，c 是等 surface，d 是这一帧上传的字节数。
  这几个开关在 `MiniEngine` 创建时读一次：Android 要在 `nativeCreateEx` 之前 `Os.setenv`。
  Flutter 插件与 Android 原生 `MiniProgramView`（`MiniDiag.kt`）替你做了：`adb shell setprop debug.mini.fps 1`（还有 `damage_log` / `no_damage` / `scroll_log` /
  `layout_log` / `backend` / `no_resample` / `cpu_pin` / `draw_log` / `img_log` / `no_prefetch`），重开小程序生效，不用重新打包；用完 `setprop debug.mini.fps ""` 关掉。
  测试包里也可以在代码里开（不用 adb）：`MiniProgramView.diagnostics = setOf("damage_log", "fps")`（`open` 之前设）；
  原生示例入口页上有个「诊断日志」开关就是这么接的。`damage_log` 开着时：
  - 每一帧的 🩹 行尾带 `〔慢：解码 3次 36.0ms（最慢 20.2 p_coffee.jpg 1248×832） · 降采样 … · 缩放 … · 字形 …〕`
    —— 这一帧（连同前面的建树 / 布局）里第一次解的图、第一次降采样 / 缩放、第一次光栅化的字各花了多少；
  - `整帧·数据` 后面带 `← 原因`（失效分析为什么退回整帧，如 `配不上对的 <view class=refresh-ring>（动画）…`）；
  - 关闭小程序打 `[生命周期] 关闭：主线程等放手 Surface 0.3ms` / `引擎线程上销毁引擎 18ms`。
- **屏上实际的帧**：Flutter 插件在 Android 上默认把引擎画到自己的 SurfaceView（`androidSurfaceView: true`，
  平台视图 hybrid composition），系统合成器直接取引擎交的每一帧；`false` 退回外部纹理（屏上那一帧要看 Flutter
  自己这一帧有没有按时出，主线程一忙就重复一张）。判据用 SurfaceFlinger：
  `adb shell "dumpsys SurfaceFlinger --latency '<层名>'"`（层名在 `dumpsys SurfaceFlinger --list` 里找
  `SurfaceView[包名/…](BLAST)`），每行第二列是上屏时刻，相邻两行隔了一个以上的显示器周期就是屏上重复了一张。
  宿主承载的原生组件跟不跟页面：`setprop debug.mini.pace 1` 时 Flutter 每帧另打 `🎞N <vsync> <用的引擎那一帧> <滚动> <速度> <推了多少>`
  （ColorOS 上日志多了会整段静默，看不到时临时只留这一路），`setprop debug.mini.native_lag <帧数>` 校准推算的帧数。
  外部纹理那条路的诊断：`setprop debug.mini.pace 1`（引擎 `🎞E` / 宿主 `🎞` / Flutter `🎞F` 三路），
  `python3 tools/pace-check.py pace.log` 算出每个 vsync 屏上是哪一帧。
- **建引擎之前预热**：`mr_warmup` / `MiniEngine.nativeWarmup(宽, 高, dpr, 偏好, 内存, lowRam, api)` 在建引擎之前调，
  字体与 GPU 合成器各开一条线程跟建引擎并行；设好偏好与机型信息之后调 `mr_app_prewarm_gpu` / `nativePrewarmGpu` 认领。
  一加 PLY110 上建引擎开始到 GPU 就绪 870ms → 480ms。不调就是挂 surface 时现开。
- **SurfaceView 宿主**：实现 `SurfaceHolder.Callback2.surfaceRedrawNeededAsync`，等引擎在新 Surface 上交出第一帧再回调
  （Flutter 插件 `EngineView.kt` / `EngineHost.onViewRedrawNeeded`），否则回前台那一下露黑底；第一次打开时
  SurfaceView 先 `alpha = 0`，引擎交出第一帧再显示。
- **引擎线程**：Android 上 `nativeCreateEx` 把调用它的线程（引擎线程）绑到大核（排除最慢一簇，`MINI_CPU_PIN=0` 关），
  之后只在交互（按着、惯性、转场、侧滑）或连续出帧时绑着，停 800ms 放开，
  宿主再给这条线程开一个 ADPF 会话（`FrameBoost.kt`，目标为刷新周期的一半）。所以**引擎必须在一条专用线程上创建与驱动**，
  别在 UI 线程上建。回桌面 Surface 被收走时合成器只放 surface，回来挂新 surface 不重开设备（约 25ms）。
- **动画按宿主帧时间取样**：`pump(now_ms)` 给的是 vsync 时刻（Choreographer 的 frameTimeNanos、CADisplayLink）时，
  操作菜单 / picker / `@keyframes` 这一帧都按它取样（`frame_clock.rs`）。传 0 就退回墙上时钟 —— 宿主别传「此刻」，
  画的时刻相对 vsync 忽早忽晚，动画会抖。
- **真机采样**：示例 App 的清单带 `<profileable android:shell="true"/>`，release 包也能
  `adb shell simpleperf record --app <包名> -g --duration 9 -o /data/local/tmp/perf.data`；符号用
  `target/aarch64-linux-android/release/libmini_render.so`（jniLibs 里那份是 strip 过的），simpleperf 报的
  `[+偏移]` 直接按 `llvm-nm -n` 的地址找。
- **慢拖跟手**：移动事件带时间戳（Flutter 插件自动带；原生宿主用 `mr_app_pointer_move_at` / `nativePointerMoveAt`，
  时间与 `pump` 的 `now_ms` 同一个单调时钟）时，引擎按「这一帧 − (一帧 + 5ms)」那个时刻插值手指位置来摆页面，
  不按到达顺序：事件过两道线程才到引擎线程，一帧到几个是碰运气，按到达走慢拖会一顿一顿。代价是多约一帧的跟手延迟；
  `MINI_NO_RESAMPLE=1` 关掉做对照。
  开着 `MINI_FPS` 时 Android 直写 Surface 还会每秒打一行 `📤 [直写] … 等缓冲 / 写像素 / 提交`（引擎的 📊 行不含上屏）。
  SDK 不做 `MINI_GPU_VERIFY` 的逐帧对拍（surface 读不回来），GPU 正确性靠启动自检与 `tools/gpu-parity.sh`。

Kotlin 侧对应的 JNI（`src/ffi_jni.rs`，老的 `native*` 一个没动）：`nativeSetBackend(h, pref): Boolean`、
`nativeSetDeviceInfo(h, totalMem: Long, lowRam: Boolean, api: Int)`、`nativeAttachSurface(h, surface: Surface): Int`、
`nativeDetachSurface(h)`、`nativeBackend(h): Int`、`nativeBackendReason(h): String`、`nativePumpEx(h, nowMs): Int`。
`nativeAttachSurface` 内部 `ANativeWindow_fromSurface` → `mr_app_attach_surface`（引擎 acquire 自己那份）→
release；没编 gpu 时不碰 Surface、直接返回 1。返回 1 时那块 Surface 可能已被 GPU 连过，要 CPU 直写就换一块新的。
Flutter 插件与 Android 原生 `MiniProgramView`（与插件的 `EngineHost` 共用同一个内核 `EngineCore.kt`：出帧、上屏、事件只有一份）都已接这组接口；
iOS 的 Swift 视图同一套（`sdk/ios/Sources/MiniRender/EngineSession.swift`：引擎线程 + CADisplayLink、`mr_warmup` →
`set_device_info` → `set_backend(AUTO)` → `prewarm_gpu` → `launch` → `attach_surface(CAMetalLayer)`，首帧前 Metal 层藏着，
回退时拿掉 Metal 层改 CPU，进后台同步停 DisplayLink；这串 C 调用在 macOS 上由 `tests/ffi_metal_layer.rs` 跑过，
Swift 本身只在 Mac Catalyst 下 typecheck 过，没上过 iPhone）。新的 Kotlin 配老的 `.so` 会在调用新方法时抛
`UnsatisfiedLinkError`，两者在 AAR 里一起发布，不要单独升级。

线程约定：**所有 `mr_app_*` 必须在同一个线程调用**（QuickJS 不是线程安全的）。
网络下载在引擎内部的后台线程，结果按帧取回，宿主不用管。

### 嵌入宿主接口（Flutter 插件 / 原生 View 用，只增不改）

实现在 `src/ffi_embed.rs`（Android 绑定在 `src/ffi_jni_embed.rs`），声明在 `include/mini_render.h` 末尾。
不调这组函数的宿主行为与从前一致（`tools/sdk-parity.sh` 15/15、6/6 守着）。

| 函数 | 解决什么 |
|---|---|
| `mr_app_create_ex(..., options_json)` | 系统信息、安全区、宿主 API、原生组件要在 `app.js` 执行**之前**就位（`onLaunch` 里读 `getSystemInfoSync`、用 `typeof` 探测能力是常态） |
| `mr_app_resize(w, h, dpr)` | 转屏、分屏、外层约束变化。从前尺寸创建后定死 |
| `mr_app_set_safe_area(t, r, b, l)` | `wx.getSystemInfoSync` 的 `safeArea` / `statusBarHeight` 与 WXSS 的 `env(safe-area-inset-*)` 取真值（从前写死 0 / 20） |
| `mr_app_pixels_into(dst, len, stride, format)` | 带行跨度、RGBA / BGRA 直写平台缓冲。Flutter 在 iOS 只认 32BGRA；省掉一次整帧拷贝 |
| `mr_app_poll_event()` | 事件队列（下表），取到 NULL 为止 |
| `mr_app_host_reply(id, ok, json)` | 回 `hostCall`：宿主实现的 `wx.*` |
| `mr_app_native_view_event(key, name, detail)` | 宿主承载的原生组件回传事件（`bindended` 之类） |
| `mr_app_eval(code)` | 在逻辑层执行 JS（深链参数注入、集成测试） |

`mr_app_pump_ex` 多了两位**提示位**（与后端无关）：

- `MR_FRAME_WANTS_MORE = 16`：引擎还有活要干 —— 滚动 / 惯性、scroll-view、CSS 动画、swiper、
  JS 定时器、网络请求与远程图片在飞、Toast（要到时收起）、picker 动画、聚焦的输入框（光标闪烁）、
  **按住不动的手指**（长按靠帧驱动）。宿主在下一个 vsync 再 pump；没有这一位就停掉 vsync，
  等下一个输入事件 —— 静止页面零耗电。从前没有这个信号：只在「有新画面」时续帧的宿主
  （原生 Android View 就是）会让 `setTimeout` / 网络回包在静止页面上停住。
- `MR_FRAME_EVENTS = 32`：事件队列非空。

事件（每条一个 JSON 对象，坐标都是**视口逻辑坐标**、已减滚动）：

| `type` | 何时 | 字段 |
|---|---|---|
| `page` | 当前页或栈深变了 | `route` `depth` `isTabBar` `window`（app.json `window` ← 页面 json ← 运行时导航栏改动）；tabBar 页另带 `tabBarBackgroundColor`（含 `setTabBarStyle` 改过的，宿主给系统导航栏那一截填色用） |
| `navigationBar` | `wx.setNavigationBarTitle/Color`、`show/hideNavigationBarLoading` | `title` / `frontColor` / `backgroundColor` / `loading` |
| `inputFocus` | 输入框聚焦 | `id` `tag` `value` `inputType` `password` `maxlength` `confirmType` `confirmHold` `placeholder` `rect` |
| `inputBlur` | 失焦 / 确认 | `id` `value` |
| `nativeViews` | 原生组件的摆放、遮挡或页面滚动速度变了（滚动、picker 面板滑入滑出时逐帧；停下来那一帧再报一条速度 0；静止不发） | `views[]`：`key` `tag` `x` `y` `width` `height` `fixed` `attrs` `clip` `holes` `mask`；这一帧的滚动取样 `frameTime` `scroll` `velocity` `decay` `period` `dpr`，正常流的可见区 `area`（见下文「滚动时跟住页面」） |
| `videos` | 同层渲染（创建选项 `sameLayer: ["video"]`）时页面上的视频变了：换页、`wx:if`、属性、框的大小、静音键、进出全屏 | `views[]`：`key`（`id`，没有就是 `src`）`url`（网络地址原样；包内路径已解析成本地文件绝对路径）`attrs`（`muted` 已按控制条的静音键改好；全屏的那个 `object-fit` 换成 `contain`）`box`（读回画面的设备像素 `[宽, 高]`；全屏时是全屏画面的尺寸，横屏宽高对调）。见下文「同层渲染的视频」 |
| `videoCommand` | 同层渲染的视频被点了（控制条的播放键 / 进度条、中间的播放键）或调了 `wx.createVideoContext` | `key` `command`（`play` / `pause` / `stop` / `seek` / `playbackRate`）`position`（秒）`rate` |
| `videoFullscreen` | 视频进 / 出全屏（控制条的全屏键、`requestFullScreen`、返回键 / 顶栏返回退出；两种播放器都有） | `fullScreen` `direction`（0 竖屏 / 90 / -90 横屏）`key`。宿主收起 / 放出状态栏与导航条 |
| `hostCall` | 调了宿主实现的 `wx.*` | `id` `api` `args` `tempDir` `files`（见下文「宿主与小程序之间传文件」） |
| `clipboard` | `wx.setClipboardData`（宿主写系统剪贴板） | `data` |
| `vibrate` | `wx.vibrateShort` / `vibrateLong` | `duration`（`short` / `long`）`style` |
| `app` | 启动后第一帧（只发一次） | `about`：`name` `appId` `description` `entryPage` `pageCount` `engine` |
| `menuButton` | 点了引擎画的右上角胶囊（创建选项 `menuButton: true`、`navigationStyle: custom` 的页面） | `action`：`menu`（左半 •••，查看关于）/ `close`（右半 ⊙，返回宿主） |
| `log` | 逻辑层 `console.*` | `text` |

`nativeViews` 的遮挡三项（引擎自己画的 picker 面板、弹窗、Toast / Loading、`position:fixed` 元素、
tabBar 本该盖在原生组件上面，而控件摆在引擎画面之上，所以由宿主照着裁）：

| 字段 | 含义 | 宿主怎么做 |
|---|---|---|
| `clip` `{x,y,width,height}` | 控件还露在外面的矩形 | 控件仍按 `x/y/width/height` **完整尺寸**摆（别缩放），只在 `clip` 里画、收触摸。宽或高为 0 = 整个被挡住：控件**留着别销毁**（播放进度、地图状态在里面） |
| `holes` `[{x,y,width,height}]` | `clip` 里还被盖着的块（居中的弹窗盒子、Toast、fixed 元素） | 挖掉；落在里面的触摸交给引擎 |
| `mask` | 盖在控件上的半透明黑遮罩的不透明度（picker 面板 0→0.5 渐显、showModal 0.5） | 在控件上叠一层这个不透明度的黑色；`> 0` 时控件不收触摸（遮罩吃掉触摸，点遮罩是关面板） |

横跨控件整个宽度、压住一条边的盖板（底部 picker 面板、吸底栏、tabBar）直接缩进 `clip`，
一个矩形裁剪就够；只有压在中间的才进 `holes`。已知取舍：WXML 自己写的 `position:fixed`
半透明遮罩按**不透明**算（与「点击不穿透」同一份包围盒），正常流里的视频在它下面整个被裁掉；
scroll-view 的裁剪还不在 `clip` 里。实现与判据见 `src/host/native_occlusion.rs`、
`tests/embed_native_occlusion.rs`。

**滚动时跟住页面**：别等 `nativeViews` 到了才挪控件。引擎画面与控件走两条上屏路（Android 上引擎画到自己的
SurfaceView，系统合成器直接取；控件要经事件队列 → 主线程 → 宿主的下一帧），事件到得忽早忽晚（一加 PLY110 上
晚 2~4 个 vsync），只按事件挪的话视频框比页面落后、而且每帧落后几个不一样，滑动时就是「抖、不跟着页面走」。
事件里带了**这一帧的滚动取样**，宿主在**自己每一帧**按这一帧的 vsync 推算页面此刻滚到了哪：

| 字段 | 含义 |
|---|---|
| `frameTime` | 这一帧取样用的 vsync 时刻（毫秒，`pump` 传进来的那个宿主时钟；Android 与 `Choreographer` / Flutter 的帧时间同一个单调钟） |
| `scroll` | 页面滚动位置（逻辑像素，**未取整**）；`views[].y` 是按设备像素截断后的滚动算的（与引擎合成同一个取整） |
| `velocity` | 滚动速度（逻辑像素 / 毫秒，正 = 内容往上走）。拖动时取最近 40ms 的平均（一帧没挪不报 0），惯性时是滚动控制器的瞬时速度；页面真停了报 0 |
| `decay` | 惯性的每毫秒衰减率（`v(t) = v·e^(−decay·t)`），拖动时 0 |
| `period` | vsync 周期估计（毫秒） |
| `dpr` / `area` | 设备像素比；正常流的可见区（导航栏以下、tabBar 以上，视口逻辑坐标） |

推算（`fixed` 的控件不随页面滚，不推）：`n = round((这一帧 vsync − frameTime) / period + lag)`，`n` 超出 −2~6 就不推（时钟对不上
或引擎停了）；往后 `n` 帧页面再滚 `Δ = velocity·n·period`（惯性按逐帧离散衰减：`Δ = v·P·d·(1 − dⁿ)/(1 − d)`，`d = e^(−decay·P)`）；
控件纵向挪 `trunc(scroll·dpr)/dpr − trunc((scroll + Δ)·dpr)/dpr`。`clip` 里贴着控件自己边的那几条边跟着挪、贴着遮挡物的留在原处，
再裁进 `area`；`holes` 不动。`lag` 是控件比引擎画面晚上屏几个 vsync，一加 PLY110（Flutter 宿主、独立图层）上校准为 0。
Flutter 插件已经这样做（`native_layer.dart`，`MiniProgramController.nativeViewLag`），自己摆控件的宿主照抄；
判据与真机数字见 `doc/引擎测试说明.md` 的 T30-23。

**同层渲染的视频（`sameLayer: ["video"]`，Android 默认）**：推算只能把惯性推准，拖动时引擎画面自己每帧挪多少都不匀，
盖上去的视频照样错开。同层渲染把视频**画进页面**：宿主只解码，每一帧交给引擎，滚动时视频就是页面画布里的一块像素。
Android 的 `MiniProgramView`（`sameLayerVideo`，默认开）与 Flutter 插件（`sameLayerVideo`）都自带播放器
（`sdk/android/.../VideoHost.kt`：`MediaPlayer` 硬解带声音，`SurfaceTexture` + GLES 读回 RGBA，见 `GlFrameReader.kt`
里为什么不用 `ImageReader`）。iOS 还没有：Flutter 仍是盖上去的控件，原生视图是引擎软解。自己接的话：

1. 创建选项 `"sameLayer": ["video"]`（`nativeComponents` 里也列了 `video` 时同层渲染优先，不再报进 `nativeViews`）。
2. 收 `videos`：按 `key` 建 / 换片源 / 释放播放器，`attrs` 里 `autoplay` `loop` `muted` `initial-time` 照着设。
3. 每解出一帧，按 `box` 与 `attrs["object-fit"]`（contain 默认 / fill / cover）缩好，调
   `mr_video_frame_rgba(key, w, h, stride, data)`（任何线程都行；CPU 读得到的 YUV 4:2:0 用 `mr_video_frame_yuv`），
   再让引擎出一帧。尺寸正好时引擎整行拷贝，对不上（框刚变）才逐像素缩放。
4. 执行 `videoCommand`；播放器的状态照旧当组件事件回传（`mr_app_native_view_event(key, name, detail)`，与微信同形）：
   `loadedmetadata {width, height, duration}`、`play`、`pause`、`waiting`、`timeupdate {currentTime, duration}`（250ms 一次）、
   `seekcomplete {position}`、`ended`、`error {errMsg}`。引擎据此画控制条，再派发给节点的 `bind*`。
   网络视频 seek 后要缓冲，那段时间播放器的 `isPlaying` 是 false —— 「在不在播」别看它（`VideoHost` 自己记）。
5. 收 `videoFullscreen`：收起 / 放出状态栏与导航条（`MiniProgramView.immersiveVideoFullscreen`、
   `MiniProgramController.immersiveVideoFullscreen` 默认做了）。

**同层渲染的其它组件（`sameLayer` 里除 video 以外的标签，典型是 `<map>`）**：宿主**画**它，引擎把那一帧画进页面。
盖上去的地图与当初的视频同一个病（两条上屏路，一滑就错开、一顿一顿）；画进页面以后滚动时一个像素都不错开，
宿主一帧都不用重画（只在内容变了时交）。约定：
1. 创建选项 `"sameLayer": ["map"]`（`nativeComponents` 里也列了时同层渲染优先，不再报进 `nativeViews`）。
2. 收 `sameLayerViews`（页面结构变了才发，滚动不发）：`views[]` 的 `key`（`id`，没有就是标签名）、`tag`、`attrs`、
   逻辑 `width` / `height`、设备像素 `box`、`fixed`。按 `key` 建 / 改 / 放自己的离屏控件。
3. 内容变了就按 `box` 画一帧，调 `mr_native_view_frame_rgba(key, w, h, stride, rgba)`（任何线程；JNI `nativeViewFrameRgba`），
   再让引擎出一帧：引擎只重画这一块（框的背景照画，帧拉满画在上面，圆角照节点的）。
4. 收 `nativeViewTap {key, tag, x, y}`（框里的本地逻辑坐标）：自己判点到了什么，照旧 `mr_app_native_view_event` 回传
   （`markertap {markerId}` → 节点的 `bindmarkertap`）。拖动从框里开始照样滚页面；地图自己的平移 / 缩放手势没有转。

现成的：Android `MiniProgramView.sameLayerViews["map"] = { MyMap() }`（`MiniSameLayerView`：`onDraw` 画进 Canvas、
`onTap`、`emit`、`invalidate`，示例 `DemoMapView.kt`）；Flutter `MiniProgramController(sameLayerViews: ['map'])`
（Android：`nativeViewBuilder` 造的控件在一棵离屏渲染树里 build / paint、`toImage` 读回，点击在离屏树上派发，
`same_layer.dart`；只适合纯 Flutter 画的控件，平台视图离屏画不出来）。iOS 两个视图还没有。判据见 `tests/same_layer_view_e2e.rs`。

控制条、中间的播放键、全屏都在引擎里（与引擎自己的播放器同一套：`video/chrome.rs` 画、`ui.rs` 管状态、`fullscreen.rs`），
照微信：点画面切换控制条显隐（`bindcontrolstoggle {show}`），播放中 3 秒没碰自动收起，收起后底部一条细进度
（`show-bottom-progress`）；进度条横着拖跟手、松手 seek 一次，竖着划照样滚页面；`show-play-btn`、`show-progress`
（没写时宽度大于 240 才有）、`show-mute-btn`、`show-fullscreen-btn`、`show-center-play-btn`、`title`（全屏顶栏）、
`duration` 都认。全屏方向：`requestFullScreen({direction})` > 节点的 `direction` > 画面宽高比（横的视频横屏 90）。
横屏是**引擎把全屏那一层转过来画**，不转 Activity（转 Activity 要按横屏宽度整页重排、退出再排回来）；触点按方向换算，
返回键先退出全屏。全屏期间页面不重画（标记留到退出），只在新帧 / 控制条变了时重新合成；收起状态栏报来的新安全区引擎按住不用。
还不支持：同层渲染的弹幕（`danmu-list` / `sendDanmu`）、`enable-play-gesture` 双击、亮度音量手势、小窗、投屏。

导航栏由宿主画：引擎不画导航栏（微信里它是原生外壳），`navigationStyle: custom` 的页面除外。

**右上角胶囊**（微信的「•••」菜单 +「⊙」关闭）分两处：

- `navigationStyle: custom` 的页面没有宿主导航栏，画面顶到状态栏下面：创建选项给 `"menuButton": true`，
  引擎把胶囊画在画面里，位置就是这一页 `wx.getMenuButtonBoundingClientRect()` 返回的矩形
  （默认 87×32、距右 7、`statusBarHeight + 4`；宿主在 `systemInfo.menuButton` 给了真机的值就用它）。
  它压在页面与 `position:fixed` 元素之上、Toast / Modal / picker 之下；按下整串触摸归它（不漏到下面的页面），
  抬手还在按下的那一半里才发 `menuButton` 事件，滑出去松手不算。`navigationBarTextStyle: white` 用半透明黑底白图标，
  否则半透明白底黑图标；`wx.setNavigationBarColor` 改了前景色下一帧跟着换。
- 普通页面的胶囊画在**宿主自己的导航栏**右侧（Flutter 插件的 `MiniProgramScaffold` 已经这样做，几何与配色同上）。
- 语义：「•••」= **查看关于**，「⊙」= **返回宿主**。引擎只发事件，做什么由宿主定。关于信息在启动后第一帧的
  `app` 事件里（`about`：`name` / `appId` / `description` / `entryPage` / `pageCount` / `engine`，取自
  project.config.json 与 app.json，见 `src/app_about.rs`），没开胶囊也发。
- 现成的宿主都留了口子、带了默认实现（演示用）：

  | 宿主 | 「•••」查看关于 | 「⊙」返回宿主 |
  |---|---|---|
  | iOS `MiniProgramView` | `onAbout: (MiniProgramAbout) -> Void`；不设时弹 `UIAlertController` | `onClose`；不设时 pop（在导航栈里）或 dismiss 所在的控制器 |
  | Android `MiniProgramView` | `onAbout: (MiniProgramAbout) -> Unit`；不设时弹 `AlertDialog` | `onClose`；不设时 `finish()` 所在的 Activity |
  | Flutter `MiniProgramScaffold` | `onMenu`（信息在 `controller.about`）；不设时什么也不做，`example/` 弹关于框 | `onExit`；不设时 `Navigator.pop` 关掉整个小程序页面（不管小程序里叠了几页；小程序是根页面时退出 App） |
  | 桌面窗体 | 弹「关于…」Modal | 关窗口（无头模式只打日志） |
  | H5 导出 | 先派发可取消的 `wx-menu-button` 事件（`detail: {action, about}`），没人拦就 `wx.showModal` 关于 | 同上，默认 `history.back()` |

  iOS / Android 的视图还改成了 `create_ex`（带 `menuButton` 与安全区，`menuButton` 属性在 `open` 之前设）+ `pump_ex` 取事件，
  `onEvent` 能拿到每一条原始事件。要让胶囊落在状态栏下面，视图得铺到状态栏下（安全区会自动交给引擎）。
- 实现与判据见 `src/host/capsule.rs`、`tests/menu_button_e2e.rs`、`doc/引擎测试说明.md` T30。

创建选项：

```json
{
  "systemInfo": {"platform": "android", "model": "Pixel 8", "language": "zh_CN"},
  "safeArea": {"top": 24, "right": 0, "bottom": 16, "left": 0},
  "hostApis": ["chooseImage", "getLocation", "setClipboardData"],
  "nativeComponents": ["video", "map", "web-view"],
  "menuButton": true
}
```

- `hostApis` 点名的装进 `wx`（调用 → `hostCall` 事件 → 宿主 `mr_app_host_reply`）。**微信官方的 503 个 `wx.*`
  全都在**（对照表与实现分层见 `.agents/tasks/wx-gap-2026-10-23/gap.md`）：引擎能做的真做（存储、路由、TCP / UDP、
  Worker、WebAudio、worklet、canvas …）；要宿主的（`login` / `requestPayment` / `getLocation` / 蓝牙 / NFC / 生物认证 …）
  默认**立刻 `fail`**，errMsg 写明「当前宿主未实现」，`wx.canIUse` 对它返回 false —— 框架包装层
  （`uni.getLocation → wx.getLocation`）拿到 undefined 会直接 TypeError，马上失败的函数页面的 fail 分支接得住。
  宿主在 `hostApis` 里点名哪个就接管哪个。官方名单之外的名字仍是 `undefined`。
- **宿主承载的对象**（`MapContext` / `CameraContext` / `LivePlayerContext` / `LivePusherContext` / NFC / `VideoDecoder` …）
  的方法同样走 `hostCall`，名字带点：`MapContext.moveToLocation`，`args.contextId` 是 `createMapContext(id)` 的 id。
  宿主在 `hostApis` 里点名 `MapContext.moveToLocation` 就交给宿主，没点名的立刻 fail。
- **宿主往小程序推事件**：`mr_app_dispatch_event(app, name, json)`（JNI `nativeDispatchEvent`、Flutter
  `controller.dispatchEvent(name, payload)`、iOS / Android 视图同名方法）。`name` 写 `accelerometerChange` 或
  `onAccelerometerChange` 都行，`json` 原样交给 `wx.onAccelerometerChange` 登记的回调；特殊的几个：
  `networkStatusChange`（同时更新 `getNetworkType`）、`keyboardHeightChange`、`themeChange`、`error`、
  `recorderStart` / `recorderStop` / `recorderFrame …`（RecorderManager）、`nfcDiscovered`、
  `contextEvent`（`{contextId, event, detail}`：地图 / 相机 / 直播控件上的事件派发到节点自己的 bind 上）。返回值是收到它的回调个数。
- **宿主与小程序之间传文件**（`chooseImage` / `chooseMedia` / `chooseVideo` / `chooseMessageFile` /
  `saveImageToPhotosAlbum` / `openDocument` 这一类，实现在 `src/runtime/app_host.rs`）：
  - 小程序只认虚拟路径 `wxfile://tmp/…`，宿主只认真实路径，翻译由引擎做；
  - `hostCall` 事件带 `tempDir`：宿主可以直接写的临时目录（= 沙盒里的 `wxfile://tmp/`）；
  - 宿主回包里 **`file://` 开头的字符串**是交给小程序的文件：引擎把它换成 `wxfile://tmp/…`。写在 `tempDir`
    里的只换名字，别处的拷一份进来（桌面选的是用户自己的照片，不能挪走）。移动端请直接写进 `tempDir`，
    否则大视频的拷贝发生在引擎线程上。`file://` 后面直接拼绝对路径即可，百分号转义的标准 URI 也认；
  - `args` 里出现的 `wxfile://` 路径，事件的 `files` 字段给出真实路径（`saveImageToPhotosAlbum({filePath})`
    要读的文件）；文件不存在的不列；
  - 引擎画图不读 EXIF 方向：交照片之前按方向摆正（手机竖拍的 JPEG 几乎都是「横着存 + 方向 6」）；
    HEIC 等引擎解不了的格式转成 JPEG。
- 这几个 API 宿主点名之后，引擎还替宿主做三件平台无关的事（`js/prelude/host-media.js`）：参数按官方默认值
  补齐、夹到合法范围；移动端（`platform` 是 android / ios / ohos）`sourceType` 同时有 album 与 camera 时先弹
  引擎画的「拍照 / 从手机相册选择」菜单，选完把 `sourceType` 收窄成一项再发 `hostCall`（取消 = `<api>:fail cancel`，
  宿主收不到）；回包按官方形状补齐（`tempFilePaths` ↔ `tempFiles` 互推、`chooseMedia` 的 `type` / `fileType`、
  `chooseMessageFile` 的 `name` / `type` / `time`）。参数错误（`chooseMessageFile` 不给 `count`、存相册不给
  `filePath`）直接 fail，不发给宿主。
- 剪贴板与振动不用点名：`wx.setClipboardData` 发宿主事件 `{type:'clipboard', data}`（宿主写系统剪贴板），
  `wx.vibrateShort/Long` 发 `{type:'vibrate', duration, style}`。真实的系统设置 / 授权状态放在
  `systemInfo.systemSetting` / `systemInfo.appAuthorizeSetting` 里，`wx.getSystemSetting()` /
  `wx.getAppAuthorizeSetting()` 逐项覆盖默认值。
- `nativeComponents` 里的标签引擎只报位置、属性与遮挡，不自己画（只留一块底色，video 是黑底）；`video` 连解码都不做。
  **没点名的** `map` / `camera` / `live-player` / `live-pusher` / `voip-room` / `web-view` / `channel-*` 按官方默认尺寸
  （map 300×150、live-player 300×225、web-view 铺满）画一块浅灰占位并写上组件名（`renderer/components/native_placeholder.rs`），
  不再是一块看不出是什么的空白。
- **一个进程同一时刻只能有一个实例**（小程序根目录、数据目录、tabBar 高度、网络回包队列都是进程级的）。
  宿主要保证上一个 destroy 之后再建下一个。

---

## Flutter（`sdk/flutter`）

插件名 `mini_render`，Android 与 iOS。画面走 `Texture`（不是平台视图）：

- **线程**：引擎跑在原生侧的专用线程上，vsync 用这条线程上的 Choreographer / CADisplayLink。
  不能让 Dart 同步调引擎：Flutter 3.29 起 Dart 就跑在平台主线程上；也不能交给 Dart 的
  isolate（不保证一直在同一条系统线程上，而引擎有线程本地状态）。
- **Android 上屏**：`SurfaceProducer`。先试引擎直接 `ANativeWindow_lock` 写进 Surface（零中间拷贝）；
  锁不上（API 29+ 底层是 `ImageReader(PRIVATE, GPU_SAMPLED)`，不保证能被 CPU 锁）就换一块新的
  SurfaceProducer，改走 Bitmap + `lockHardwareCanvas`（Flutter 自己的平台视图就这么画）。
  被 CPU 连过的 Surface 销毁前不能再给 GPU 生产者用，所以必须换新的（纹理 id 会变，Dart 侧自动跟上）。
- **Android GPU 上屏（可选）**：`MiniProgramController(backend: MiniBackend.auto / gpu)` 时 Surface 先交给引擎的
  GPU 合成器（`nativeAttachSurface`，wgpu Vulkan / GLES），引擎自己上屏，插件只在 `FRAME_PRESENTED` 时通知 Flutter
  取帧。没选上（门槛 / 自检）或运行中回退（`FRAME_BACKEND_CHANGED`）就换一块新的 SurfaceProducer 走上面的 CPU 路。
  默认 `MiniBackend.cpu`。实际后端与原因在 `controller.activeBackend`。
- **iOS 上屏**：`FlutterTexture.copyPixelBuffer`，IOSurface 支撑的 32BGRA CVPixelBuffer 池（3 张），
  始终交出最新一张。`backend` 在 iOS 上不起作用（恒为 CPU）。
- **输入**：`Listener` 只送指针三段、只认第一根手指，并用 `EagerGestureRecognizer` 立刻赢下手势竞技场
  （否则外层 PageView / 侧滑返回会一起响应）。原生组件盖在画面上、按 `clip` / `holes` / `mask` 裁剪：
  露出的部分触摸归控件，被面板 / 弹窗盖住的部分归引擎。控件的 Widget 结构不随遮挡变，面板开合不会重建播放器。
- **软键盘**：`inputFocus` → `TextInput.attach`（按 `type` / `password` / `confirm-type` 选键盘），
  输入法给的文本与上次按码点差分成 backspace + 整串文字；键盘弹起时不缩画面，而是像微信
  `adjust-position` 那样整体上移露出输入框（`MiniProgramScaffold` 已设 `resizeToAvoidBottomInset: false`）。
- **安全区**：按视图与屏幕刘海 / 底部横条真正重叠的部分算（放在 AppBar 下面时上边是 0）。
- **尺寸**：引擎是整数逻辑尺寸；纹理按 `pixelWidth / dpr` 摆、最近邻采样 —— 一个设备像素对一个纹理像素。

```bash
bash tools/build-android-abi.sh            # 只编 arm64-v8a（调试用；发版用 build-mobile.sh android）
cd sdk/flutter/example && flutter run       # 例子：内置一个演示小程序
# 用仓库里的示例小程序：
adb push sample/sample-app /sdcard/Android/data/dev.minirender.mini_render_example/files/
```

验收（模拟器 / 真机上跑，判据可判定）：

1. 像素：`--dart-define=MINI_AUTODUMP=1` 打开例子，3 秒后引擎把当前帧存成 `engine_frame.png`、
   日志打出纹理位置；与 `adb exec-out screencap -p` 裁出的同一块逐像素比（`tools/pixdiff.py`），
   差异为 0 才说明纹理没被拉伸、颜色通道没反。一条命令：`BUILD=1 bash tools/flutter-android-check.sh`；
   `BACKEND=gpu` 再跑一次：GPU 上屏时存帧是同一组图层的 CPU 合成，差异为 0 就是 GPU 输出在真机链路上逐字节对。
2. 输入：tabBar 每个 tab 都能点通；横划小程序里的轮播时外层页面不动；输入框弹出数字键盘、
   输入能进去、确认能收键盘。
3. 静止省电：静止页面上 `MR_FRAME_WANTS_MORE` 为 0 时 vsync 停掉；有 `setInterval` 的页面照常走。
4. Android 13+：Flutter 3.41.7 的 SurfaceProducer 有每帧泄漏一个文件描述符的问题
   （flutter#188161）。持续出帧的页面挂 30 分钟，`/proc/<pid>/fd` 的数量要稳定。

---

## 三、各平台步骤与限制

### 通用构建

`Cargo.toml` 已经配好三种产物：

```toml
crate-type = ["cdylib", "staticlib", "rlib"]
```

- `cdylib` → Android/Linux/Windows/macOS 的动态库
- `staticlib` → iOS 的 `.a`（XCFramework）
- `rlib` → Rust 宿主直接依赖

### iOS

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo build --release --target aarch64-apple-ios
# 产物 target/aarch64-apple-ios/release/libmini_render.a → 打成 XCFramework
```

- 上屏：原生 `MiniProgramView` 默认 GPU（CAMetalLayer），CPU 退路是 `CALayer.contents`（`CGImage`）；
  ProMotion 机型要 120Hz 除了 `preferredFrameRateRange` 还要 Info.plist 的 `CADisableMinimumFrameDurationOnPhone = YES`；
- 事件：`UIView` 的 `touchesBegan/Moved/Ended/Cancelled` → 指针三段；
- 输入法：`UITextField` 隐藏在下面接 IME，把字符送 `mr_app_text_input`；
- 数据目录：`NSCachesDirectory`（可清理）或 Application Support（要备份的）；
- 字体：iOS **没有** `/System/Library/Fonts/Hiragino Sans GB.ttc` 这类路径。
  要么把字体打进包（注意版权），要么用 `CTFontManagerCopyAvailableFontFamilyNames`
  拿到路径喂给引擎。**当前 `text_family.rs` 的候选路径表是 macOS 的，iOS 上要另配一张。**
- 限制：`devtools` 之类的私有 API 别开（上架审核）。

### Android

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi
# 用 cargo-ndk 最省事
cargo ndk -t arm64-v8a -t armeabi-v7a -o ./jniLibs build --release
```

- 上屏：`SurfaceView` + `ANativeWindow_lock/unlockAndPost`（少一次拷贝），
  或 `TextureView` + `Bitmap.copyPixelsFromBuffer`；
- 事件：`onTouchEvent` 的 `ACTION_DOWN/MOVE/UP/CANCEL` → 指针三段
  （注意 `ACTION_MOVE` 要用 `getHistoricalX/Y` 把历史点都送进去，别抽稀）；
- 返回键：`onBackPressed` → `mr_app_navigate_back`，返回 false 时再自己 finish；
- 数据目录：`context.getFilesDir()`；
- 字体：`/system/fonts/NotoSansCJK-Regular.ttc` 等，路径按 ROM 有差异，
  建议用 `SystemFonts.getAvailableFonts()`（API 29+）取路径；
- JNI：`mr_app_*` 全部在同一个线程（建议专门起一个渲染线程 + Handler）。

### 鸿蒙 / OpenHarmony

```bash
rustup target add aarch64-unknown-linux-ohos armv7-unknown-linux-ohos
# 需要 OpenHarmony SDK 的 clang 与 sysroot（用 ohos-rs 工具链最省事）
```

- 上屏：`XComponent` + `OH_NativeWindow`（`OH_NativeWindow_NativeWindowRequestBuffer`
  拿 buffer 写 RGBA 再 `FlushBuffer`），或产出 `PixelMap` 交给 ArkUI `Image`；
- 事件：`XComponent` 的 `OH_NativeXComponent_RegisterTouchEventCallback` → 指针三段；
- 返回：`onBackPress` → `mr_app_navigate_back`；
- 数据目录：`context.filesDir` / `context.cacheDir`；
- `web-view` 用 ArkUI 的 `Web()` 组件（`registerJavaScriptProxy` / `runJavaScript`
  做双向通信）；
- 限制：**鸿蒙没有 GTK/WebView2/WKWebView**，所以第 7 条那套「宿主提供控件」的
  设计是鸿蒙能跑起来的前提。

### Windows / macOS / Linux（桌面）

仓库自带的 `mini-app-window` 就是桌面宿主（winit 0.30 + softbuffer），直接跑：

```bash
cargo build --release
./target/release/mini-app-window sample-app
```

Linux 上如果要嵌 WebView（`web-view` 组件）才需要 GTK；纯渲染不需要。

---

## 四、与微信的差距（上线前要知道）

完整清单在 `doc/引擎测试说明.md` 第六节（G-1 ~ G-13），影响集成决策的几条：

| 编号 | 差距 | 对宿主的影响 |
|---|---|---|
| G-2 | 无 `wx:key` / `template` / `import`/`include` / `slot` / `wxs` | 用这些写法的小程序结构会缺失 —— 上线前先拿目标小程序跑一遍 |
| G-4/G-5 | `scroll-view` 只读 `scroll-x`，不派发 `bindscroll`/`scrolltolower` | 内层容器驱动的无限列表加载不了下一页（页面级 `onReachBottom` 可用） |
| G-6 | `video` 只读 `src`/`autoplay`/`loop`，不派发播放事件 | 视频类小程序需要先补 |
| G-10 | `wx.login` / `requestPayment` / `getLocation` 等要宿主接（选图 / 选文件 / 存相册 Flutter 插件、原生 Android 视图与桌面窗体已自带） | 涉及登录与支付的小程序要宿主在 `hostApis` 里接上自己的 SDK（三个视图都有 `hostApis`）；原生 iOS 视图还没自带选图 / 选文件那一组 |
| — | `web-view` 组件本身还没有 | 见第 7 条，要连原生层回调一起做 |
| — | 字体内存 | 见第 9 条，移动端的硬约束 |
| — | 2026-10 对照补齐时写明的降级 | `snapshot.takeSnapshot` 失败；`editor` 只有纯文本；手势组件单指（scale 恒为 1、force-press 不触发）；`movable-view` 不能双指缩放；`page-meta` 的 page-style / root-font-size 不生效；Worker 与页面同一个 QuickJS；`sticky-*` 不吸顶；WebAudio 只有声源出声（滤波 / 压缩 / 声像节点不改变声音）、`createMediaAudioPlayer` 无声；`wx.router` 的自定义转场不参与（照常用默认转场）；worklet 跑在逻辑线程（共享值改动经一次 setData 上屏） |

---

## 五、接完之后怎么验收

按顺序过一遍，全绿才算接通：

1. **能出图**：目标小程序首页与微信截图并排看，主要区块位置一致；
2. **能滚**：上下滑跟手、越界橡皮筋、松手回弹不振荡、惯性撞边界能停住；
3. **能点**：tabBar 每个 tab 都能切、列表项能进详情、按压态有反馈；
4. **能输入**：点输入框弹键盘、逐字符进去、`confirm` 能触发；
5. **能跳**：`navigateTo` 进得去、系统返回键退得出、`redirectTo` 退不回被关掉的页；
6. **能持久**：杀掉进程重开，`wx.setStorageSync` 存的东西还在（验证 `set_data_dir` 接对了）；
7. **能联网**：`wx.request` 通，4xx 也走 `success`；
8. **内存**：切 20 个页面 + 滚到底，`memory_report()` 里的缓存占用**不持续增长**；
   收到内存告警后 `trim_memory()` 能把它降下来；
9. **能后台**：切后台停止出帧（用系统能耗面板确认），回前台画面正常。

引擎自身的回归请跑 `doc/引擎测试说明.md` 第一节那一套。
