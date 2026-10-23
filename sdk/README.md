# mini-render SDK

给原生 App 用的极简封装。集成方只需要一个 View。

```
sdk/
├── android/
│   ├── MiniEngine.kt        # JNI 绑定（内部用，别直接调）
│   ├── MiniProgramView.kt   # ← 集成方只用这个（FrameLayout + 一块 SurfaceView）
│   ├── EngineSession.kt     # 引擎线程、vsync 出帧、GPU → CPU 直写 → Bitmap 三级上屏（内部）
│   ├── FrameBoost.kt / MiniDiag.kt      # ADPF 性能提示 / `debug.mini.*` 诊断开关（内部）
│   ├── MiniHostCall.kt / HostCallRouter.kt  # `hostApis`：宿主实现的 wx.*（登录、支付、定位…）
│   ├── MiniMediaApis.kt / MiniMediaActivity.kt / MiniFileProvider.kt / MediaImport.kt
│   │                        # 自带的选图 / 拍照 / 选视频 / 选文件 / 存相册 / 打开文档（清单见 src/main/AndroidManifest.xml）
│   ├── MiniSameLayerView.kt # 同层渲染的组件（`<map>` 这类）：宿主画、引擎画进页面，滑动不错开
│   ├── example/             # 纯原生示例 App（./gradlew installRelease 量流畅度）
│   └── jniLibs/<abi>/libmini_render.so   # 由 tools/build-mobile.sh android 产出
└── ios/
    ├── Sources/MiniRender/MiniProgramView.swift   # ← 集成方只用这个（CAMetalLayer，GPU 合成，失败回退 CPU）
    ├── Sources/MiniRender/EngineSession.swift     # 引擎线程、CADisplayLink、GPU / CPU 上屏（内部）
    ├── module.modulemap                 # 让 Swift 看见 C ABI
    └── MiniRender.xcframework           # 由 tools/build-mobile.sh ios 产出
```

## 编库

```bash
bash tools/build-mobile.sh android    # arm64-v8a / armeabi-v7a / x86_64
bash tools/build-mobile.sh ios        # 真机 + 模拟器，打成 XCFramework
bash tools/build-mobile.sh check      # 只做交叉编译检查，不产出
```

## 三行接入

Android：

```kotlin
val mini = MiniProgramView(this)
setContentView(mini)
mini.open(File(filesDir, "my-mini-app"))
// 返回键：if (!mini.goBack()) super.onBackPressed()
```

iOS：

```swift
let mini = MiniProgramView(frame: view.bounds)
view.addSubview(mini)
mini.open(appDir: unpackedDir)
```

完整说明（生命周期、原生层组件、鸿蒙、验收清单）见
[`doc/原生App集成指南.md`](../doc/原生App集成指南.md)。

## 三条必须知道的约定

1. **线程**：引擎不是线程安全的（逻辑层是 QuickJS）。两个 View 内部都用一条专用
   线程，自己直接调 C ABI 时也要保证同线程。
2. **触摸送三段**：按下 / 移动 / 抬起，不要在宿主侧合成"点击"。
   `touchstart/touchmove/touchend/longpress/tap` 都由引擎的触摸状态机产出，
   绕过它会出现「测试全绿、真机点不动」的问题。
3. **数据目录必须给**：`storage` 与图片磁盘缓存都落在那里。不给不会报错，
   只是**静默失效** —— 「首次拉数据存起来、之后走缓存」的小程序每次冷启都走首次分支。
   两个 View 已经默认指到 `filesDir` / `Caches`。

## 一致性

SDK 用的宿主是 `mini_render::host::MiniEngine`，与桌面窗体共用 `host::` 下的页面
加载、覆盖层、触摸状态机、像素合成，只有帧调度各自实现。

```bash
bash tools/sdk-parity.sh                          # sample-app：15/15 逐像素一致
bash tools/sdk-parity.sh sample/news-app target/fin2_news   # news-app：6/6
```
