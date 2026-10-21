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
  builder: (_) => MiniProgramScaffold(         // 导航栏 + 返回键 + 画面
    controller: c,
    nativeViewBuilder: (context, view, c) => view.tag == 'video' ? MyPlayer(view.attrs['src']) : null,
  ),
));
// 关掉时：c.dispose()
```

先编引擎库：`bash tools/build-mobile.sh android`（或只编一个 ABI：`bash tools/build-android-abi.sh`）、
`bash tools/build-mobile.sh ios`。例子在 `example/`。

原生组件按引擎报的遮挡裁剪（picker 面板、弹窗、fixed 元素、tabBar 盖在视频上面，与微信同层渲染一致），
`nativeViewBuilder` 只管造控件，不用自己处理层级。

约定、事件、限制与验收见 [`doc/原生App集成指南.md`](../../doc/原生App集成指南.md) 的「Flutter」一节。
