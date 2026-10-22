# mini_render（Flutter 插件）

在 Flutter 里打开微信小程序。引擎是仓库根目录的 mini-render（Rust 自绘，对标 Skyline），
跑在原生侧的专用线程上，画面经 `Texture` 上屏（Android：SurfaceProducer；iOS：CVPixelBuffer）。

```dart
final c = MiniProgramController(
  appDir: unpackedDir,                         // 解包后的小程序目录（含 app.json）
  hostApis: {'chooseImage': (args) async => {'tempFilePaths': await pick()}},
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

先编引擎库：`bash tools/build-mobile.sh android`（或只编一个 ABI：`bash tools/build-android-abi.sh`）、
`bash tools/build-mobile.sh ios`。例子在 `example/`。

导航栏默认由引擎画（`MiniProgramController(navigationBar: true)`，带转场 `routeAnimation`）：画面铺满整屏，普通页 ↔ `navigationStyle: custom` 页切换不改尺寸，不会拉伸、跳动；`navigationBar: false` 回到用 AppBar 画导航栏的旧做法。

底部的系统导航栏：**Android** 上视图铺到虚拟导航键 / 手势条下面时，那一截不交给引擎（与微信安卓一致，`windowHeight` 不含它），
由 `MiniProgramView` 填 `navigationBarAreaColor`（`MiniProgramScaffold` 在 tabBar 页给 tabBar 底色、其它页给页面底色），
并关掉系统的半透明灰罩、导航键图标按底色深浅反色；按在那一截上的触摸不进引擎。页面最底下一截与吸底栏不再被导航键盖住，
`env(safe-area-inset-bottom)` 在 Android 上是 0。**iOS** 照样铺满 Home 指示条（与微信 iOS 一致），底部安全区交给 `env()` 与原生 tabBar。

原生组件按引擎报的遮挡裁剪（picker 面板、弹窗、fixed 元素、tabBar 盖在视频上面，与微信同层渲染一致），
`nativeViewBuilder` 只管造控件，不用自己处理层级。

右上角胶囊（「•••」+「⊙」，87×32、距右 7）：普通页面画在导航栏右侧（`MiniCapsule`），
`navigationStyle: custom` 的页面由引擎画在画面里，位置就是 `wx.getMenuButtonBoundingClientRect()`。
「•••」是查看关于、「⊙」是返回宿主：两处的点击都从 `controller.menuButtonTaps` 出来，
`MiniProgramScaffold` 接成 `onMenu` / `onExit`（`onExit` 不给时直接关掉整个小程序页面，小程序里叠了几页都一样；关于信息在 `controller.about`，
`example/` 里弹了一个关于框）。导航栏文字白色时是深色胶囊（白图标）。`MiniProgramController(menuButton: false)` 两处都不画。

约定、事件、限制与验收见 [`doc/原生App集成指南.md`](../../doc/原生App集成指南.md) 的「Flutter」一节。
