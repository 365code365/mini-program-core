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
| 出帧 | 只在引擎回报「有新画面」时才上屏；静止时不出帧（不耗电） |
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

- **iOS**：`CGDataProvider` + `CGImageCreate` → `CALayer.contents`，
  或直接写 `CVPixelBuffer` / Metal 纹理；
- **Android**：`Bitmap.copyPixelsFromBuffer` + `Canvas.drawBitmap`，
  或 `ANativeWindow_lock/unlockAndPost`（避开一次拷贝）；
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

微信里这些是**原生组件层**，层级最高，只有 `cover-view` / `cover-image` 能盖在上面。
引擎的做法应当一致：**引擎只算位置与参数，实际控件由宿主创建**。

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
| 剪贴板 | 桌面用 `arboard` | 移动端要接系统剪贴板 |
| 音视频 | 内置 symphonia + rodio + openh264 解码 | 移动端建议改用系统播放器（省电、省内存、走硬解） |
| 定位/支付/登录等 | **未实现**（`wx.login`、`requestPayment`、`getLocation`…） | 未实现的 API 保持 `undefined`，框架能正确降级；要用就得新增实现 |

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

**还没有的**：原生层组件的回调（`web-view`/`video` 的摆放，第 7 条那个
`mr_native_view_cb`）、日志回调、损伤矩形（`mr_app_pump` 目前每帧整帧重绘）。

线程约定：**所有 `mr_app_*` 必须在同一个线程调用**（QuickJS 不是线程安全的）。
网络下载在引擎内部的后台线程，结果按帧取回，宿主不用管。

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

- 上屏：`CALayer.contents`（`CGImage`）最省事；追求帧率用 Metal 纹理；
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
| G-10 | `wx.login` / `requestPayment` / `chooseImage` / `getLocation` 等未实现 | 涉及登录与支付的小程序跑不通，这部分必须宿主 + 引擎一起补 |
| — | `web-view` 组件本身还没有 | 见第 7 条，要连原生层回调一起做 |
| — | 字体内存 | 见第 9 条，移动端的硬约束 |

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
