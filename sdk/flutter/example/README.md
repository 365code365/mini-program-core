# mini_render_example

用 `mini_render` 插件在 Flutter 里打开小程序。启动页的「内置示例」是 `assets/demo` 里的小程序，
覆盖引擎目前实现的绝大部分能力：

| tab | 内容 |
|---|---|
| 首页 | 语法与样式（WXML / WXSS / 事件 / 动画）、页面形态（自定义导航栏、综合性能场景、静态详情页） |
| 组件 | 基础内容、表单、选择器、轮播、滚动容器、媒体、画布、自定义组件 |
| 接口 | 界面反馈、路由与生命周期、数据缓存、网络、节点查询与页面滚动 |
| 宿主 | hostApis（选图、登录、定位、扫码、支付、拨号）、原生 map、剪贴板与振动、tabBar 运行时接口、前后台生命周期、系统信息 |

宿主侧（`lib/main.dart`）对应的接法：

- `hostApis`：返回假数据；`requestPayment` / `makePhoneCall` 弹 Flutter 对话框，取消时走 `fail`。
- `nativeComponents: ['video', 'map']`：
  - `video` 由 `video_player`（系统硬解）出画面，见 `lib/host_video.dart`。支持 `src`（https / 包内路径）、
    `autoplay` / `loop` / `muted` / `controls` / `object-fit` / `poster`，回传 `loadedmetadata` / `play` / `pause` /
    `waiting` / `timeupdate` / `ended` / `error`。网络地址要 https：Android 9+ 默认禁明文 HTTP，iOS 有 ATS。
  - `map` 是占位控件，点标记回传 `markertap`。
- `clipboard` / `vibrate` 事件：写系统剪贴板、触感反馈。

小程序里加了目录，记得在 `pubspec.yaml` 的 `assets` 里加一行（Flutter 的目录条目不递归）。

```bash
flutter run                                   # 默认 CPU 后端
flutter run --dart-define=MINI_BACKEND=auto   # 试 GPU 上屏（仅 Android）
```

桌面上预览同一份小程序（不经过 Flutter）：

```bash
./target/release/mini-app-window sdk/flutter/example/assets/demo --route pages/index/index
```
