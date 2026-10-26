# 03 API 规格

逐条列出小游戏要提供的 API。列含义：

- **语义要点**：实现必须对齐的行为（含易错点）。标「取证」的以 G0.2 真机取证为准，取证前按括号里的默认实现。
- **实现**：`复用` = 小程序已有实现可直接装进游戏 profile；`改` = 已有但要改；`新` = 新写；`宿主` = 走 `hostApis` 代理（宿主不点名就立即 fail，`canIUse` 报 false）。
- **期**：归属的实现期（04）。

通用约定（全部异步 API）：
- 回调式：`success/fail/complete`，**下一轮事件循环**回调（与小程序一致，`__asyncOk/__asyncFail`）；`errMsg` 形如 `<api>:ok` / `<api>:fail <原因>`。
- 不传任何回调时返回 Promise（基础库 2.10.2+ 行为），`fail` 对应 reject。
- 同步版 `xxxSync` 失败抛 `Error`，`message` 与异步版 `errMsg` 相同。
- 参数类型错误：`fail`，`errMsg` 为 `<api>:fail parameter error: parameter.<字段> should be <类型> instead of <类型>;`（与基础库格式一致——取证）。

## 1. 基础与生命周期

| API | 语义要点 | 实现 | 期 |
|---|---|---|---|
| `GameGlobal` | 全局对象 | 新 | G1 |
| `wx.env.USER_DATA_PATH` | `wxfile://usr` | 复用 | G1 |
| `wx.canIUse(schema)` | 按游戏 profile 的 API 表判断；宿主未点名的代理 API 为 false | 改（游戏版表） | G1 |
| `wx.getSystemInfoSync/getSystemInfo/getSystemInfoAsync` | **`platform` 由引擎按编译目标给真实值（`android/ios/mac/windows`），宿主不能覆盖成 `devtools`**（现有默认值就是 `devtools`，见 09 §2.3）；`SDKVersion` = 常量 `GAME_SDK_VERSION`（D-9），与 `canIUse` 一致；横屏时 `safeArea` 按旋转后的坐标给；字段：`brand, model, pixelRatio, screenWidth/Height, windowWidth/Height, statusBarHeight, language, version, system, platform, fontSizeSetting, SDKVersion, benchmarkLevel, safeArea, deviceOrientation…`；横屏游戏里 `windowWidth > windowHeight` | 改（游戏字段、方向） | G1/G6 |
| `wx.getWindowInfo/getDeviceInfo/getAppBaseInfo/getSystemSetting/getAppAuthorizeSetting` | 拆分版系统信息 | 复用 + 改 | G1 |
| `wx.getDeviceBenchmarkInfo` | `{benchmarkLevel, modelLevel}`；用 `DeviceInfo` 分级（-2/-1/0~50），规则写进代码注释 | 新 | G6 |
| `wx.onShow/offShow` | `{scene, query, shareTicket, referrerInfo{appId, extraData, chatType}}`；回前台时触发；冷启动**不**触发（取证） | 新 | G6 |
| `wx.onHide/offHide` | 进后台触发 | 新 | G6 |
| `wx.getLaunchOptionsSync` | 冷启动参数，`scene` 默认 1001（取证）；来自创建选项 `launchOptions` | 改 | G6 |
| `wx.getEnterOptionsSync` | 本次进入参数（热启动更新） | 改 | G6 |
| `wx.exitMiniProgram` | 发 `exit` 事件给宿主；宿主关闭视图 | 新 | G6 |
| `wx.restartMiniProgram` | 发 `restart` 事件；宿主销毁重建实例 | 新 | G6 |
| `wx.onError/offError` | `{message, stack}`；所有回调外层捕获 | 改 | G1 |
| `wx.onUnhandledRejection/off…` | `{reason, promise}` | 改 | G1 |
| `wx.getUpdateManager` | `onCheckForUpdate/onUpdateReady/onUpdateFailed/applyUpdate`；默认 `hasUpdate:false` | 复用 | G6 |
| `wx.setEnableDebug` | 打开 vConsole 等价物：引擎只记日志开关 | 改 | G7 |
| `wx.getLogManager/getRealtimeLogManager` | 写宿主日志 | 复用 | G7 |
| `wx.triggerGC` | `Runtime::run_gc` | 新 | G1 |
| `wx.onMemoryWarning/off…` | `{level}`（Android 有，iOS 无 level——取证） | 复用 | G1 |
| `wx.getPerformance()` | `now()` 单调微秒精度毫秒；`getEntries` 等返回空数组 | 改 | G2 |
| `wx.reportPerformance(id, value, dimensions)` | 写日志 / 宿主事件 | 新 | G7 |
| `wx.getAccountInfoSync` | `{miniProgram:{appId, envVersion, version}}` | 复用 | G1 |

## 2. 渲染

### 2.1 画布

| API | 语义要点 | 实现 | 期 |
|---|---|---|---|
| `wx.createCanvas()` | 首次上屏、之后离屏（官方）；上屏默认尺寸 = 窗口逻辑尺寸（取证） | 新 | G2 |
| `Canvas.width/height` | 设值清空后备缓冲、状态复位；主画布显示区不变、缩放填满 | 新 | G2 |
| `Canvas.getContext(type, attrs)` | `2d` / `webgl` / `webgl2`；一个画布只能一种；`attrs`：`antialias`(false)、`preserveDrawingBuffer`(false)、`antialiasSamples`(2，仅 iOS)、`alpha`(false，2.11.0+) | 新 | G2/G8 |
| `Canvas.toDataURL()` | PNG data URI（读后备缓冲） | 新（复用 `encode`） | G2 |
| `Canvas.toTempFilePath(obj)` / `toTempFilePathSync` | `x,y,width,height,destWidth,destHeight,fileType(png/jpg),quality`；文件落 `wxfile://tmp_…` | 新（复用 `canvas_save` 逻辑） | G2 |
| `wx.createOffscreenCanvas` | 小游戏里也存在（基础库 2.16.1+ 支持 `type`/`width`/`height`——取证） | 改（接游戏画布） | G2 |
| `requestAnimationFrame(cb)` / `cancelAnimationFrame(id)` | 全局；`cb(timestamp)`；默认 60fps | 新 | G2 |
| `wx.setPreferredFramesPerSecond(fps)` | 1~60，超范围夹到边界（取证：是否夹） | 新 | G2 |

### 2.2 2D 上下文（`CanvasRenderingContext2D`）

全部按 HTML Canvas 2D 规范，下列为必须实现且有回归用例的成员：

| 分类 | 成员 | 期 |
|---|---|---|
| 状态 | `save/restore`（深度上限 1024）、`reset`（取证是否存在） | G2 |
| 变换 | `translate/rotate/scale/transform/setTransform(6 参或矩阵)/resetTransform/getTransform` | G2 |
| 合成 | `globalAlpha`、`globalCompositeOperation`（26 种，现有 fx2d 覆盖哪些在 G2.1 列表核对，缺的列为已知差距） | G2 |
| 填充描边 | `fillStyle/strokeStyle`（色串、`CanvasGradient`、`CanvasPattern`）、`createLinearGradient/createRadialGradient/createConicGradient/createPattern(image, repetition)` | G2 |
| 线 | `lineWidth/lineCap/lineJoin/miterLimit/setLineDash/getLineDash/lineDashOffset` | G2 |
| 阴影 | `shadowBlur/shadowColor/shadowOffsetX/shadowOffsetY` | G2 |
| 矩形 | `clearRect/fillRect/strokeRect` | G2 |
| 路径 | `beginPath/closePath/moveTo/lineTo/bezierCurveTo/quadraticCurveTo/arc/arcTo/ellipse/rect/roundRect`（取证） | G2 |
| 绘制路径 | `fill(fillRule?)/stroke/clip(fillRule?)`、`Path2D` 版本、`isPointInPath/isPointInStroke` | G2 |
| 文本 | `font/textAlign/textBaseline/direction`、`fillText/strokeText(maxWidth?)`、`measureText → TextMetrics` | G4 |
| 图像 | `drawImage`（3/5/9 参，来源 Image / Canvas）、`imageSmoothingEnabled/imageSmoothingQuality` | G2/G4 |
| 像素 | `createImageData/getImageData/putImageData(dirty 参数)`，`data` 为 `Uint8ClampedArray`（二进制桥） | G2 |
| 滤镜 | `filter`：CPU 不支持就忽略并打一次日志（steering 10 取舍要写明） | G9 |
| 属性 | `canvas`（回指画布） | G2 |

### 2.3 图片与字体

| API | 语义要点 | 实现 | 期 |
|---|---|---|---|
| `wx.createImage()` | `Image{src, width, height, onload, onerror, referrerPolicy}`；`src` 支持包内路径、`wxfile://`、`http(s)`、base64 data URI（2.11.0+）；`width/height` 为真实尺寸 | 新 | G4 |
| `wx.loadFont(path)` | 同步返回 family 字符串，失败 `null`；支持本地路径、代码包路径（官方） | 新（接 `text_faces`） | G4 |
| `wx.getTextLineHeight({fontStyle, fontWeight, fontSize, fontFamily, text})` | 返回行高 number（取证计算口径） | 新 | G4 |
| `wx.getImageInfo` | 复用 | 复用 | G4 |
| `wx.previewImage/saveImageToPhotosAlbum` | 宿主 | 宿主 | G7 |

### 2.4 WebGL（G8）

`WebGLRenderingContext` / `WebGL2RenderingContext` 全部方法与常量，按 Khronos WebGL 1.0 / 2.0 规范；必须支持的扩展（以 ANGLE 实际能力为上限）：
`ANGLE_instanced_arrays, OES_vertex_array_object, OES_element_index_uint, OES_standard_derivatives, OES_texture_float, OES_texture_half_float, OES_texture_float_linear, EXT_texture_filter_anisotropic, WEBGL_depth_texture, WEBGL_lose_context, WEBGL_compressed_texture_{etc,etc1,astc,pvrtc,s3tc}`（按设备）、`EXT_color_buffer_float`（WebGL2）。
`getContextAttributes`、`isContextLost`、`webglcontextlost/restored` 事件（画布上 `addEventListener`——取证小游戏画布是否支持 `addEventListener`）。

## 3. 输入

| API | 语义要点 | 实现 | 期 |
|---|---|---|---|
| `wx.onTouchStart/Move/End/Cancel` | `{touches, changedTouches, timeStamp}`；`Touch{identifier, screenX, screenY, clientX, clientY, pageX, pageY, force}`；`identifier` 在一次触摸过程中不变（官方） | 新 | G3 |
| `wx.offTouchStart/…(listener?)` | 不传参数移除全部监听（官方） | 新 | G3 |
| `wx.onKeyDown/onKeyUp`（PC） | `{key, code, timeStamp}` | 新 | G3 |
| `wx.onMouseDown/onMouseUp/onMouseMove/onWheel`（PC） | 字段取证 | 新 | G3 |
| `wx.onDeviceOrientationChange/off…` | `{value:'portrait'\|'landscape'\|'landscapeReverse'}`（取证） | 新 | G6 |
| `wx.onWindowResize/off…` | `{windowWidth, windowHeight}` | 新 | G6 |
| `wx.setDeviceOrientation({value})` | `landscape` / `portrait`；需 `resizable` 或全平台？（取证）；改方向后发 `onWindowResize` | 新 + 宿主 | G6 |
| `wx.startAccelerometer/onAccelerometerChange`、陀螺仪、罗盘、`DeviceMotion` | 已有监听框架；**SDK 自己采集**（SensorManager / CoreMotion），`interval: game(20ms)/ui(60ms)/normal(200ms)` | 改（SDK 采集） | G7 |
| `wx.vibrateShort({type})/vibrateLong` | `type: heavy/medium/light`；复用 | 复用 | G7 |

## 4. 系统 UI

| API | 语义要点 | 实现 | 期 |
|---|---|---|---|
| `wx.showToast/hideToast/showLoading/hideLoading/showModal/showActionSheet` | 盖在游戏层上；Modal / ActionSheet / mask 拦截触摸 | 复用绘制 + 新命中 | G7 |
| `wx.showKeyboard({defaultValue, maxLength, multiple, confirmHold, confirmType})` | 宿主弹 IME；引擎画输入条（取证：微信是否显示输入条） | 新 + 宿主 | G7 |
| `wx.hideKeyboard/updateKeyboard({value})` | | 新 + 宿主 | G7 |
| `wx.onKeyboardInput/Confirm/Complete/HeightChange` + `off…` | `{value}`；`Complete` 在收起时 | 新 | G7 |
| `wx.getMenuButtonBoundingClientRect` | 胶囊位置；与引擎画的胶囊一致 | 复用 | G1 |
| `wx.setMenuStyle({style:'light'\|'dark'})` | 改胶囊配色 | 新 | G7 |
| `wx.setStatusBarStyle` | 发 `statusBar` 事件 | 新 + 宿主 | G7 |
| `wx.setKeepScreenOn/setScreenBrightness/getScreenBrightness` | 复用 | 复用 | G7 |
| `wx.createUserInfoButton/createOpenSettingButton/createFeedbackButton/createGameClubButton` | `{type:'text'\|'image', text, image, style{left, top, width, height, backgroundColor, borderColor, borderWidth, borderRadius, color, textAlign, fontSize, lineHeight}, withCredentials, lang}`（官方）；`show/hide/destroy/onTap/offTap`；按钮消费触摸（取证） | 新（引擎画）+ 宿主（点击后的动作） | G7 |

## 5. 包与多线程

| API | 语义要点 | 实现 | 期 |
|---|---|---|---|
| `wx.loadSubpackage({name, success, fail, complete})` | `name` 可填 name 或 root；独立分包里 `__GAME__` 表示主包；加载完**自动执行**分包的 `game.js`（官方）；返回 `LoadSubpackageTask{onProgressUpdate(cb{progress, totalBytesWritten, totalBytesExpectedToWrite})}` | 新 | G6 |
| `wx.preDownloadSubpackage` | 只下载不执行（官方区别） | 改 | G6 |
| `wx.createWorker(path, {useExperimentalWorker})` | 最多一个（官方）；`postMessage/onMessage/terminate/onProcessKilled`；需 `game.json.workers` | 改（真线程） | G6 |
| Worker 内 `worker.onMessage/postMessage`、`worker.getCameraFrameData`（不支持） | | 新 | G6 |
| `wx.createSharedArrayBuffer`（Worker 共享内存，取证是否存在） | QuickJS 支持 `SharedArrayBuffer`（未核实）；不支持就返回 `undefined` 并记入差距 | 待定 | G6 |

## 6. 网络、文件、存储

| API | 语义要点 | 实现 | 期 |
|---|---|---|---|
| `wx.request` | 加 `responseType:'arraybuffer'`、`data` 为 ArrayBuffer、`enableChunked` + `onChunkReceived`、`onHeadersReceived`、`timeout` 默认取 `game.json.networkTimeout.request`；`dataType:'json'` 时解析失败返回原文（与微信一致） | 改 | G4 |
| `wx.downloadFile` | `onProgressUpdate{progress, totalBytesWritten, totalBytesExpectedToWrite}`；`onHeadersReceived` | 改 | G4 |
| `wx.uploadFile` | 进度回调 | 改 | G4 |
| `wx.connectSocket` / `SocketTask` | 二进制帧走二进制桥（不再 base64） | 改 | G4 |
| `wx.createTCPSocket/createUDPSocket` | 同上 | 改 | G4 |
| `wx.getFileSystemManager()` | 现有方法全部保留；`readFile` 不给 encoding 返回 ArrayBuffer（二进制桥）；新增 `readCompressedFile/readCompressedFileSync`（`compressionAlgorithm:'br'`，取证）、`readZipEntry`；`unzip` 保留 | 改 | G4 |
| 文件配额 | 本地用户文件总量上限（微信 200MB——取证），超出 `fail ... the maximum size of the file storage limit is exceeded`（文案取证） | 新 | G4 |
| `wx.setStorage*/getStorage*/removeStorage*/clearStorage*/getStorageInfo*` | 复用；补单键 1MB / 总量 10MB 检查（文案取证） | 改 | G4 |
| `TextEncoder/TextDecoder`、`atob/btoa`、`wx.arrayBufferToBase64/base64ToArrayBuffer` | 存在性取证；实现原生 | 新 | G1 |

## 7. 音频

| API | 语义要点 | 实现 | 期 |
|---|---|---|---|
| `wx.createInnerAudioContext({useWebAudioImplement})` | 属性：`src, startTime, autoplay, loop, obeyMuteSwitch, volume, playbackRate(0.5~2.0), currentTime, duration, paused, buffered, referrerPolicy`；方法 `play/pause/stop/seek/destroy`；事件 `onCanplay/Play/Pause/Stop/Ended/TimeUpdate/Error/Waiting/Seeking/Seeked` 及 `off…`；`onError` 的 `errCode`（10001 系统错误 / 10002 网络 / 10003 文件 / 10004 格式 / -1 未知） | 改（新音频引擎） | G5 |
| `wx.setInnerAudioOption({mixWithOther, obeyMuteSwitch, speakerOn})` | 全局 | 新 + 宿主 | G5 |
| `wx.getAvailableAudioSources` | `{audioSources:['auto', …]}` | 新 | G5 |
| `wx.onAudioInterruptionBegin/End` + `off…` | 来电、闹钟、其他 App | 改（宿主推） | G5 |
| `wx.createWebAudioContext()` | `currentTime, destination, listener, sampleRate, state`；`close/resume/suspend/decodeAudioData(ArrayBuffer, ok, err)`（也返回 Promise）；`createBuffer/createBufferSource/createGain/createBiquadFilter/createChannelMerger/createChannelSplitter/createConstantSource/createDelay/createDynamicsCompressor/createIIRFilter/createOscillator/createPanner/createPeriodicWave/createScriptProcessor/createWaveShaper/createAnalyser`；`AudioParam` 自动化 `setValueAtTime/linearRampToValueAtTime/exponentialRampToValueAtTime/setTargetAtTime/setValueCurveAtTime/cancelScheduledValues` | 改（真混音图，§8.1）；第一批真实现：BufferSource、Gain、Panner(stereo)、Oscillator、ConstantSource、Delay、BiquadFilter、ChannelMerger/Splitter、Analyser；其余保留现有降级并在文件头写明 | G5 |
| `wx.createMediaAudioPlayer` | 视频音轨，小游戏用得少 | 保持空壳，`canIUse` false | — |
| `wx.getRecorderManager` | 宿主 | 宿主 | G7 |

## 8. 开放能力与商业化（宿主代理）

全部走现有 `hostApi` 机制，默认立即 fail（`errMsg: <api>:fail not supported in this host`——文案沿用现有实现）。游戏 profile 的名单：

| 类 | API |
|---|---|
| 登录与用户 | `login, checkSession, getUserInfo, getUserProfile, getSetting, openSetting, authorize, requirePrivacyAuthorize, getPrivacySetting, onNeedPrivacyAuthorization` |
| 分享 | `showShareMenu, hideShareMenu, updateShareMenu, shareAppMessage, onShareAppMessage/off, onShareTimeline/off, onAddToFavorites/off, getShareInfo, authPrivateMessage, setMessageToFriendQuery, showShareImageMenu` |
| 开放数据 | `setUserCloudStorage, removeUserCloudStorage`（W，宿主后端）；`getOpenDataContext`、`getSharedCanvas`、`onMessage`（E，11 B-6） |
| 支付 | `requestMidasPayment`、`requestVirtualPayment`（取证当前推荐哪个） |
| 广告 | `createRewardedVideoAd, createInterstitialAd, createBannerAd, createCustomAd, createGridAd`：返回对象的 `show/load/destroy/onLoad/onError/onClose/onResize` 形状必须完整（游戏普遍不判 null）；宿主没接时 `onError({errCode:1000…})`——取证错误码 |
| 跳转 | `navigateToMiniProgram, exitMiniProgram, restartMiniProgram` |
| 游戏圈 / 客服 / 其他 | `createGameClubButton, openCustomerServiceConversation, getGameRecorder, createGamePortal, createGameIcon, getChannelsLiveInfo …`（`getGameRecorder` 返回对象形状完整、`start` 立即 `onError`） |
| 云开发 | `wx.cloud`：宿主提供；引擎不内置 |

宿主没接的 API 一律**存在且立即 fail**，不是 `undefined`：游戏引擎的适配层大量直接调用、不做 `typeof` 判断（`host-capabilities.js` 文件头已写明这条原则）。

## 9. 不能由引擎独立完成的部分（形状照做，见 11 §6）

> 第三轮审查后，`WXWebAssembly`、`wx.createVideo`、开放数据域已从本表移出，分别进 G10、G7、G9；全部 382 个 API 的逐项状态以 10 为准。

| API | 原因 |
|---|---|
| `wx.getGameServerManager`（帧同步） | 依赖微信后台 |
| 关系链数据本身 | 依赖微信关系链；接口与开放数据域环境照做（11 B-6），数据由宿主后端提供 |
| `wx.createCamera`、`createVKSession`、`createInferenceSession` | 计划外 |
