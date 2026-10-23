# 对照微信：API 与组件差距（2026-10-23）

口径：官方 API 目录 https://developers.weixin.qq.com/miniprogram/dev/api/ 抽出的 `wx.*` 共 503 个；
组件目录 https://developers.weixin.qq.com/miniprogram/dev/component/ 。引擎现状取运行时 `Object.keys(wx)`
（桌面窗体，demo 小程序）与 `renderer/components/registry.rs`。

## 一、API

开工前：wx 上 175 个键，其中 24 个是「立刻 fail」的默认实现；**缺 341 个**。按官方分类：

| 分类 | 缺 | 典型 |
|---|---|---|
| 基础 | 43 | getSystemInfoAsync、getSkylineInfo(Sync)、getRendererUserAgent、路由事件 7 对、requestIdleCallback、onAudioInterruption*、onLazyLoadError、getUserCryptoManager、preload* |
| 路由 / 跳转 | 14 | router（Skyline 自定义路由）、rewriteRoute、exitMiniProgram、restartMiniProgram、openEmbeddedMiniProgram、open*（公众号 / 视频号） |
| 转发 / 聊天工具 | 16 | onCopyUrl、shareFileMessage、shareVideoMessage、shareToOfficialAccount、*ToGroup |
| 界面 | 18 | enable/disableAlertBeforeUnload、hideHomeButton、setTopBarText、loadBuiltInFontFace、setWindowSize、onWindowStateChange、**worklet** |
| 网络 | 12 | createTCPSocket、createUDPSocket、mDNS（start/stopLocalServiceDiscovery + 4 对事件） |
| 支付 | 9 | requestPluginPayment、requestVirtualPayment、requestCommonPayment、createGlobalPayment… |
| 数据缓存 | 12 | getStorageInfo、batch{Set,Get}Storage(Sync)、createBufferURL、revokeBufferURL、backgroundFetch、createCacheManager |
| 数据分析 / 画布 / 文件 | 4 | getExptInfoSync、getCommonConfig、createOffscreenCanvas、saveFileToDisk |
| 媒体 | 46 | createMapContext、createCameraContext、createLive{Player,Pusher}Context、previewMedia、getVideoInfo、compressVideo、editImage、cropImage、WebAudio、MediaAudioPlayer、旧版背景音频 / 录音 / 语音、VoIP 一族 |
| 位置 | 9 | startLocationUpdate(Background)、onLocationChange(Error)、getFuzzyLocation、choosePoi |
| 开放接口 | 42 | 隐私（requirePrivacyAuthorize、getPrivacySetting、onNeedPrivacyAuthorization、openPrivacyContract）、soter 检查、卡券、发票抬头、视频号、收藏… |
| 设备 | 105 | 蓝牙 / BLE 外设 / iBeacon / NFC / HCE / Wi-Fi / 日历 / 加速计 / 罗盘 / 陀螺仪 / 设备方向 / getRandomValues / getLocalIPAddress / onKeyDown / sendSms / 录屏状态… |
| AI / Worker / 广告 | 11 | createInferenceSession、VKSession、faceDetect、createWorker、createRewardedVideoAd、createInterstitialAd |

已有类的方法缺口（运行时探测）：
- RequestTask：onChunkReceived / offChunkReceived
- Animation：left/top/right/bottom、rotate3d/rotateX/rotateY、translate3d/translateZ、scale3d/scaleZ、skewX/skewY、matrix/matrix3d
- CanvasContext（旧版）：arcTo、clip、createPattern、measureText、rect、setLineDash、setMiterLimit、setTransform、transform
- VideoContext：startCasting / switchCasting / exitCasting / reconnectCasting
- RealtimeLogManager：getCurrentState、in；NodesRef：ref
- 页面：getOpenerEventChannel（EventChannel 整个没有）
- 顺带发现的 bug：`wx.onAppShow/onAppHide` 的监听存在 `__appHooks.onAppShow`，而派发读的是 `__appHooks.onShow` —— 从没被调过

## 二、组件

已登记 24 个；navigator / form / label 走兜底 + 命中逻辑；map / web-view / camera 等只在 SDK 点名 `nativeComponents` 时交给宿主，否则画成空 view。

| 缺的组件 | 怎么补 |
|---|---|
| cover-view / cover-image | 别名（view / image） |
| match-media | 渲染器按窗口尺寸判，不成立就不建子树 |
| movable-area / movable-view | 渲染器 + 手势仲裁（跟手、惯性、越界回弹、bindchange / bindscale） |
| page-container | 内置组件（WXML+JS），返回键 / 侧滑先关它 |
| root-portal | 脱离文档流（fixed 层） |
| page-meta / navigation-bar | 渲染器读属性，转成导航栏 / 背景 / 页面滚动 / 根字号 |
| grid-view（aligned / masonry）/ list-view / sticky-section / sticky-header / nested-scroll-* / list-builder / grid-builder | grid-view 渲染器布局；其余按语义降级为容器（写明） |
| span | 内联容器 |
| draggable-sheet | 内置组件（拖动、吸附 snap-sizes）+ DraggableSheetContext.scrollTo |
| 8 个手势组件（tap / pan / scale / long-press / double-tap / force-press / horizontal-drag / vertical-drag） | 内置组件，按触摸算 state / delta / velocity，回调 `on-gesture-event` |
| share-element / open-container / snapshot / selection / editor / editor-portal / keyboard-accessory / functional-page-navigator | 容器降级；snapshot.takeSnapshot、EditorContext 给最小实现 |
| audio（旧版） | 内置组件（InnerAudioContext + 播放条） |
| camera / live-player / live-pusher / voip-room / channel-* / web-view / map / ad / ad-custom / official-account* / open-data* / reward / store-* | 原生组件：宿主没接时画占位（官方默认尺寸 + 标注），点名 `nativeComponents` 时照旧交给宿主 |

## 三、实现分层

1. **引擎里能真做的就真做**：路由事件、EventChannel、alertBeforeUnload、存储批量 / getStorageInfo、
   getRandomValues、TCP / UDP、getLocalIPAddress、getVideoInfo（解析 mp4）、createWorker（同一 QuickJS 里的隔离作用域）、
   createMediaQueryObserver、createOffscreenCanvas、旧版 CanvasContext 补齐、Animation 补齐、隐私接口（引擎不收集隐私，按「已同意」回）。
2. **要宿主的**：统一走 hostCall（宿主在 `hostApis` 点名就交给宿主，没点名立刻 fail 写明原因），
   事件类 `wx.onXxx` 统一登记，宿主用 `mr_app_dispatch_event(name, json)` 推。
3. **组件**：别名 / 降级的在 registry；行为在 Rust 的（movable、match-media、grid-view、page-meta、原生占位）；
   能用内置 WXML+JS 组件做的（page-container、draggable-sheet、手势、audio）走「引擎内置组件」（新机制：页面用到才注入，页面自己声明的同名组件优先）。

## 四、完成情况（2026-10-23 收尾）
- API：运行时 `wx` 键 516 个，对照官方 503 个只剩 `onShow`（官方清单误抓，不是 wx API）。
- 组件：渲染器 grid-view / cover-image（别名）/ 占位与默认样式（native_placeholder）/ movable-area 默认样式；
  内置组件加 movable-view（`virtualHost` + 引擎专用的 `__mergeHost`：模板根节点并入宿主）。
- 动画：`this.animate` / `clearAnimation`（含滚动时间线）、`wx.worklet` 全套、`applyAnimatedStyle`：写 data 的 `__nodeStyles`，
  模板展开时拼到命中节点的 style（parser/template/node_styles.rs）。
- WebAudio：节点图全形状，出声走 InnerAudioContext（decodeAudioData 原生解码；改过的缓冲 / 振荡器编成 WAV 播）。
- 顺带修的引擎语义：touchmove / touchend / touchcancel 锁定在 touchstart 的节点上（wxml_renderer/touch_capture.rs）；
  `TouchTracker::identifier()` 抬起后不再给 0。
- 测试：`tests/wx_compat.rs` 9 个、`src/tests/skyline_component_tests.rs` 7 个、node_styles / builtin_components 单测。
