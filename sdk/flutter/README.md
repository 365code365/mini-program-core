# mini_render（Flutter 插件）

在 Flutter 里打开微信小程序。引擎是仓库根目录的 mini-render（Rust 自绘，对标 Skyline），
跑在原生侧的专用线程上。Android 默认画到引擎自己的 SurfaceView（平台视图，系统合成器直接上屏，不经过 Flutter 的帧；`androidSurfaceView: false` 退回 `Texture`）；iOS 经 `Texture` 上屏（CVPixelBuffer）。

```dart
final c = MiniProgramController(
  appDir: unpackedDir,                         // 解包后的小程序目录（含 app.json）
  hostApis: {'login': (args) async => {'code': await myLogin()}},  // 宿主自己的 wx.* 能力
  nativeComponents: const ['video'],           // 由宿主用原生控件承载的组件
);
Navigator.push(context, MaterialPageRoute(
  builder: (_) => MiniProgramScaffold(         // 导航栏 + 返回键 + 右上角胶囊 + 画面
    controller: c,
    nativeViewBuilder: (context, view, c) => view.tag == 'video' ? MyPlayer(view.attrs['src']) : null,
    onMenu: () => showMyShareSheet(),           // 胶囊「•••」：弹什么由宿主决定（不给就什么也不做）
    onExit: () => Navigator.pop(context),      // 胶囊「⊙」/ 只剩一页时的返回（不给就关掉这个页面）
  ),
));
// 关掉时：c.dispose()
```

**插件自带的宿主能力**（`builtinHostApis`，默认开，见 `MiniBuiltinHostApis`）：`wx.chooseImage` / `chooseMedia` /
`chooseVideo` / `chooseMessageFile` / `saveImageToPhotosAlbum` / `saveVideoToPhotosAlbum` / `openDocument`，
走系统自己的界面 —— Android 照片选择器（不要存储权限）/ 系统相机 / 文件选择器 / MediaStore，iOS
`PHPickerViewController` / `UIImagePickerController` / `UIDocumentPickerViewController` / `PHPhotoLibrary`。
选到的文件写进引擎给的临时目录，小程序拿到 `wxfile://tmp/…`，`<image>`、`uploadFile`、`saveFile`、
`getFileSystemManager` 直接能用；照片按 EXIF 摆正、HEIC 转 JPEG。`hostApis` 里给了同名的用宿主的。
iOS 宿主要在 Info.plist 里写 `NSCameraUsageDescription`（录像另要 `NSMicrophoneUsageDescription`）与
`NSPhotoLibraryAddUsageDescription`，缺了的那一项走 fail；Android 宿主清单里声明了 CAMERA 权限时插件会先申请。

**宿主往小程序推事件**：`await c.dispatchEvent('accelerometerChange', {'x': 0, 'y': 0, 'z': -1})` 交给
`wx.onAccelerometerChange` 登记的回调（名字写 `onAccelerometerChange` 也行），返回收到它的回调个数。传感器、
网络变化、键盘高度、主题、录音帧、NFC、地图 / 相机控件事件（`contextEvent`）都走这一条，名单见集成指南的「创建选项」。

先编引擎库：`bash tools/build-mobile.sh android`（或只编一个 ABI：`bash tools/build-android-abi.sh`）、
`bash tools/build-mobile.sh ios`。例子在 `example/`。

导航栏默认由引擎画（`MiniProgramController(navigationBar: true)`，带转场 `routeAnimation`）：画面铺满整屏，普通页 ↔ `navigationStyle: custom` 页切换不改尺寸，不会拉伸、跳动；`navigationBar: false` 回到用 AppBar 画导航栏的旧做法。

底部的系统导航栏：**Android** 上视图铺到虚拟导航键 / 手势条下面时，那一截不交给引擎（与微信安卓一致，`windowHeight` 不含它），
由 `MiniProgramView` 填 `navigationBarAreaColor`（`MiniProgramScaffold` 在 tabBar 页给 tabBar 底色、其它页给页面底色），
并关掉系统的半透明灰罩、导航键图标按底色深浅反色；按在那一截上的触摸不进引擎。页面最底下一截与吸底栏不再被导航键盖住，
`env(safe-area-inset-bottom)` 在 Android 上是 0。**iOS** 照样铺满 Home 指示条（与微信 iOS 一致），底部安全区交给 `env()` 与原生 tabBar。

**`<video>` 在 Android 上走同层渲染**（`nativeComponents` 列了 `video`、`sameLayerVideo: true` 默认）：插件自带的
系统播放器（MediaPlayer，硬解带声音）解码，帧交给引擎画进页面 —— 滑动时视频就是页面里的一块像素，不会与页面错开；
不走 `nativeViewBuilder`。控制条照微信（点画面显隐、播放中 3 秒自动收起、拖进度、静音键 `show-mute-btn`、全屏键），
全屏横 / 竖屏都由引擎画（横屏是把那一层转过来画，不转 Activity），全屏时插件收起状态栏（`immersiveVideoFullscreen`，
退出恢复成 edgeToEdge；`controller.videoFullscreen` 可以自己听）。iOS 暂时仍是盖上去的控件（`nativeViewBuilder`）。

**别的组件也可以同层渲染**（Android）：`MiniProgramController(nativeComponents: ['map'], sameLayerViews: ['map'])`。
`nativeViewBuilder` 造的控件不盖在画面上，而是在一棵**离屏**渲染树里 build / paint、`toImage` 读回、交给引擎画进页面
（`lib/src/same_layer.dart`）：滑动时它是页面里的一块像素，跟页面一起动（盖上去的控件慢拖时实测错开最多 15px，
同层渲染 0，见 `doc/滑动顿挫避坑清单.md`）；只在控件要重画时（setState、动画、属性变了）读回一次，滚动不读。
点击：引擎发 `nativeViewTap`，插件在离屏树上派发一次按下 / 抬起，`GestureDetector.onTap` 照常触发；拖动滚页面。
只适合纯 Flutter 画的控件 —— 平台视图（google_maps_flutter 这类）离屏画不出来，别列进 `sameLayerViews`。

原生组件按引擎报的遮挡裁剪（picker 面板、弹窗、fixed 元素、tabBar 盖在视频上面，与微信同层渲染一致），
`nativeViewBuilder` 只管造控件，不用自己处理层级。页面滚动时控件每帧按 vsync 推算位置跟住页面（不等事件到了才挪）；
某台机型上视频框落在页面后面 / 跑到前面时，用 `adb shell setprop debug.mini.native_lag <帧数>` 校准
（调大 = 往前推，重开小程序生效），再把数写进 `MiniProgramController(nativeViewLag: …)`。

右上角胶囊（「•••」+「⊙」，87×32、距右 7）：普通页面画在导航栏右侧（`MiniCapsule`），
`navigationStyle: custom` 的页面由引擎画在画面里，位置就是 `wx.getMenuButtonBoundingClientRect()`。
「•••」是查看关于、「⊙」是返回宿主：两处的点击都从 `controller.menuButtonTaps` 出来，
`MiniProgramScaffold` 接成 `onMenu` / `onExit`（`onExit` 不给时直接关掉整个小程序页面，小程序里叠了几页都一样；关于信息在 `controller.about`，
`example/` 里弹了一个关于框）。导航栏文字白色时是深色胶囊（白图标）。`MiniProgramController(menuButton: false)` 两处都不画。

约定、事件、限制与验收见 [`doc/原生App集成指南.md`](../../doc/原生App集成指南.md) 的「Flutter」一节。
