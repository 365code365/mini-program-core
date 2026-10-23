# 原生示例补宿主能力 / 地图同层渲染 / iOS 视图接 GPU（2026-10-23，另一个会话）

给同时在改这些文件的会话看：下面这些是我加的，改到同一处时请保留。

## 共享文件里加的东西
- `sdk/android/.../MiniProgramView.kt`：`hostApis` / `builtinHostApis`（HostCallRouter.kt）、`sameLayerViews`
  （MiniSameLayerView.kt）、`clipboard` 写系统剪贴板。`sameLayer` 选项合并了 `sameLayerVideo` 的 video 与
  `sameLayerViews` 的标签（别再单独 `options.put("sameLayer", ["video"])`，会把地图冲掉）。
- 新文件（SDK）：MiniHostCall.kt、HostCallRouter.kt、MiniMediaApis.kt、MiniMediaActivity.kt、MiniFileProvider.kt、
  MediaImport.kt（Flutter 那份的拷贝，包名 dev.minirender）、MiniSameLayerView.kt。清单：sdk/android/src/main/AndroidManifest.xml。
- 引擎：`components/host_view.rs`（通用同层组件的帧表）、`host/engine_host_views.rs`（`sameLayerViews` /
  `nativeViewTap` 事件、回传事件）、`EngineOptions::same_layer_views()`；`animated_image.rs` 的 mark 多了 `view`；
  `draw.rs::draw_component` 末尾画同层组件的帧；`tap.rs` 两处 `engine_host_views::on_tap`；
  `engine_events.rs` 的 `sync_host_views()` 与 `dispatch_native_view_event` 的回退；
  FFI `mr_native_view_frame_rgba` / JNI `nativeViewFrameRgba`。
- iOS：`sdk/ios/Sources/MiniRender/{MiniProgramView,EngineSession,MiniHostCall}.swift` 重写（引擎线程 + CAMetalLayer）；
  头文件补了 `mr_warmup` / `mr_app_prewarm_gpu` / `mr_native_view_frame_rgba`；build-mobile.sh 的 iOS 编进 gpu。
