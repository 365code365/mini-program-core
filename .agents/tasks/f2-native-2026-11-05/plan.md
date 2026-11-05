# F2 与上一轮留下的项（2026-11-05 起）

上一轮（`.agents/tasks/component-gap-2026-11-03/plan.md`）明确写了「下一轮」的：
WP-F2（camera、live-pusher、voip-room、Flutter iOS 的 map / web-view / 同层视频）、scroll-view `clip=false`、canvas `bind:error`（同页 canvas-id 重复）。
另带两条「下一步」：下拉刷新拉着时 setData 退回整帧、地图虚线多边形每帧重切。

## 做法（沿用已有的通路，不另起一套）
- `<camera>` / `<live-pusher>` / `<voip-room>` 走**通用同层**（`sameLayer` 选项 → `sameLayerViews` 事件 → 宿主按框交 RGBA → `host_view::draw`）。
  宿主在自己的 GL 线程上把相机画面按框裁好（cover）读回，引擎只剩整行拷贝；事件经 `nativeViewEvent` → `bind*`；
  上下文方法经 `hostCall`（`CameraContext.*` / `LivePusherContext.*`，`SameLayerViews.contextCall` 按 contextClass 找实例）。
- Android：Camera2 → SurfaceTexture（`GlFrameReader`，加裁剪）→ 读回给引擎；录像 / 推流共用一个 H.264（Surface 输入）+ AAC 编码器，
  录像进 `MediaMuxer`，推流进自写的 RTMP（FLV tag）发送端。权限借 `MiniMediaActivity` 申请。
- iOS 原生：AVCaptureSession → BGRA 帧 → 裁剪换序 → `mr_native_view_frame_rgba`；拍照 / 录像（AVAssetWriter）；推流同一套 RTMP（Swift）+ VideoToolbox / AudioConverter。
- voip-room：Skyline 不渲染它（官方「渲染框架支持情况：WebView」），房间要 `wx.joinVoIPChat`（微信后台）。mode=camera 显示本机相机画面，mode=video 黑底 + `binderror`。
- scanCode：引擎在宿主交来的相机帧上解二维码（纯 Rust 解码器），节流，命中发 `bindscancode`。
- Flutter iOS：podspec 带上 `sdk/ios/Sources/MiniRender` 里同层 / 视频 / 地图 / web-view 那几份（与 Android 插件直接编原生 SDK 源码同一个思路），
  `EngineHost.swift` 补选项合并与事件过滤，Dart 去掉只限 Android 的开关。

## 验收
- 定向单测：引擎侧事件 / 上下文路由 / scanCode / clip=false 局部重画 vs 整帧逐字节 / canvas bind:error。
- 真机（一加 PLY110）：演示页 `pages/camera`、`pages/pusher` 的 `NC-CHECK`（`android-native-components.sh` 加两项）；
  推流用本机 `ffmpeg -listen` 当 RTMP 服务端（`adb reverse tcp:1935`），`ffprobe` 验收到的流有 H.264 + AAC；顿挫 `host-slow` 不回退。
- iOS：模拟器（没有相机：走 binderror 那条）+ Flutter iOS 模拟器 `NC-CHECK map / webview / live`。
- 全量回归、site 16 种语言、测试场景总表一轮记录。

## 进度

### 2026-11-05 · 全部做完的那一版

**引擎（Rust）**
- `<camera>` / `<live-pusher>` / `<voip-room>` 走通用同层（`sameLayer` 点名即用；没接的照旧画占位）。live-pusher 默认样式改成官方的「宽 100%、无默认高」。
- `host/engine_camera.rs`（新）：onCameraFrame 由引擎从宿主交来的预览里取（frame-size：small 360 / medium 640 / large 1280 长边，不放大；
  `bin_stash` 种类 3 直通 ArrayBuffer，不过 base64）；`mode="scanCode"` 在预览上解二维码（rqrr，后台线程、至少隔 250ms、同一结果 2 秒内不重发，
  detail `{type:"QR_CODE", result, charSet, rawData}`，官方没写 detail，待验）；voip-room `mode="video"` 派一次 `binderror {errCode:-1}`。
- prelude：CameraContext.startRecord 的 `timeoutCallback`（宿主到点推 `recordTimeout`，逻辑层再要一次 stopRecord 的结果）；`fields({context})` 认 camera / live-pusher；
  canIUse 认宿主接了的原生组件（`__hostComponents`）。
- scroll-view `clip="{{false}}"`：四条绘制路径都不裁、主轴两侧各多画一个盒子长；滚动的损伤区 = 盒子 ∪ 盒外画着的那圈（`clip_false_paint`，
  SDK 与桌面同一个 `scroll_area_damage_bounds`）；里面 setData 变了整帧（swiper clip=false 的同一个缺口一起补了）。
- `<canvas binderror>`：同页 canvas-id 重复，第二个起各报一次（`host/engine_canvas_check.rs`，按布局代数查）。
- scroll-view 自己的属性变了、盒子没动（下拉刷新回调里 setData `refresher-triggered`）：只重画盒子，不再「子树绘制坐标≠布局坐标」整帧。
  盒子超过视口 0.34 时照旧按整帧画（engine_damage_data.rs 的老规矩），所以真机那页（占屏 37%）还是整帧，小一点的 scroll-view 受益。

**Android（原生 SDK 与 Flutter 插件同一份 Kotlin）**
- `CameraPipe.kt`：Camera2 → SurfaceTexture（`GlFrameReader` 加了裁剪 / 镜像矩阵、画进窗口 Surface）；每帧按框 cover 裁好读回给引擎；
  预览尺寸按 4:3（相机）/ 16:9（推流）挑（按框比例挑会挑到 1200×1200 这种，照片也成方的）；拍照取预览那一帧存 JPEG，flash=on 先开手电 450ms；
  重开相机（换前后置、回前台）编码器的输入留着；编码帧按要的帧率取、时间戳至少隔一个帧间隔。
- `AvEncoder.kt`：H.264（Surface 输入）+ AAC（AudioRecord，PCM 按小端写进输入缓冲 —— 默认大端全是噪声）；`Mp4Sink`（录像）。
- `RtmpPublisher.kt`：握手 → connect → createStream → publish → onMetaData → 序列头 → 音视频 tag；断线重连 3 次、停推发 FCUnpublish / deleteStream；
  官方状态码 1001 / 1002 / 1009 / 1102 / 3001~3005 / -1307，netstatus 每秒一次。
- `MiniCameraView.kt` / `MiniLivePusherView.kt` / `CameraPermissions.kt`（借 MiniMediaActivity 申请；弹权限框那一下的「进后台」不关相机）。
  Flutter：`builtinCamera`（默认开），NativeComponents 接 CameraContext / LivePusherContext 的 hostCall。示例清单加 CAMERA / RECORD_AUDIO。
- 真机（一加 PLY110）：`android-native-components.sh` 原生与 Flutter 都是 map / webview / live / **camera / pusher** 全过；本机 ffmpeg 收到的流
  `h264 360×640 + aac`、4.1~4.2 秒、`ffmpeg -f null` 解码无报错。ColorOS 上 `pm grant` 不让用：装包用 `adb install -r -g`。

**iOS**
- 原生 SDK：`CameraCapture.swift`（AVCaptureSession → vImage 裁缩换序）、`MiniCameraView.swift`（PNG 拍照、AVAssetWriter 录像）、
  `MiniLivePusherView.swift` + `LiveEncoder.swift`（VideoToolbox + AVAudioConverter）+ `RtmpPublisher.swift`（同一套协议）。
  模拟器：camera / pusher 走「没有可用的摄像头」/ -1301 那条（ok:false 是对的），map / webview / live 过。**没上 iPhone**（真机没连）。
- Flutter iOS：podspec 软链 sdk/ios 的同层 / 地图 / web-view / 视频 / 相机源码（`Classes/Native/`），`NativeComponents.swift` 补选项与事件过滤，
  web-view 摆在 Dart 报来的 `viewRect` 上（只接 web-view 那几块触摸的容器）；Dart 去掉只限 Android 的开关；podspec 补 `compression`（引擎读 emjc 表情字体要，
  从前 Flutter iOS 就链不过）。模拟器 `flutter run`：NC-CHECK map / webview / live 全过。

**没做**：live-pusher 的美颜 / 滤镜 / BGM / 小窗 / 自定义渲染；voip-room 真的通话（要微信后台）；scroll-view 双向同时滚；scanCode 只认二维码（条形码没做）；
地图虚线多边形缓存（量过只占 ~1ms，没动）。

## F3（同日继续）：用户点名的四项

| 项 | 结论 |
|---|---|
| 扫码只认二维码 | ✅ rqrr 换成 rxing（ZXing 移植）：QR / EAN / UPC / Code 128 / 39 / 93 / Codabar / ITF / Data Matrix / PDF417 / Aztec；`type` 用 `QR_CODE` / `EAN_13` 这套名字（待验）；scanCode 模式下 takePhoto 回微信那句 fail |
| scroll-view 横竖同时滚 | ✅ 主控制器管纵向、`"{id}#x"` 管横向（`ui/interaction/scroll_2d.rs`）；手势「哪个方向先动都归它」、滚轮、绘制（live 路、不分块）、SelectorQuery / 组件 id 的偏移、bindscroll 的 scrollLeft / deltaX、左右阈值、scroll-left、scrollTo({left, top})、scrollIntoView、scrollOffset；用例 `wpd::scroll_2d` 2 个 |
| live-pusher 美颜 / 滤镜 / BGM / 小窗 | ✅ Android `FrameEffects.kt`（GL 着色器）+ `BgmPlayer.kt`（解码 → AudioTrack + 混进 AvEncoder 的麦克风 PCM）；iOS `FrameEffects.swift`（Core Image 颜色核）+ `BgmMixer.swift`（AVAudioPlayer + 按 currentTime 读文件混进 LiveEncoder）；小窗在引擎（`components/pip.rs` 画在合成那一步、`host/engine_pip.rs`），四份宿主不用改；真机（PLY110，原生 + Flutter）收端：怀旧滤镜 R−B 4.8→49.3 / 4.5→44.4，背景音 RMS −37.4→−21.1 / −41.7→−24.1 dB，小窗那段最大帧间隔 0.12s；iOS 只编过（没有 iPhone，模拟器没相机） |
| voip-room 真正的通话 | ⚠️ 微信后台在微信外进不去（signature / groupId 由微信服务器签、媒体走微信）。做的是接口：`joinVoIPChat` 等进宿主 hostApis（原有通道），宿主接了时引擎不再派「进不了房」；`mode="video"` 的画面向宿主的 `MiniVoip.video`（Android / iOS）要；用例 `voip_tests` 2 个。没有可用的后台，端到端通话没验 |
