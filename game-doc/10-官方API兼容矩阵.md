# 10 官方 API 兼容矩阵（逐项）

来源：微信小游戏 API 总览页（developers.weixin.qq.com/minigame/dev/api/，2026-10-26 抓取，快照 `取证/official-api-index-2026-10-26.json`）。
本表由脚本从快照生成（`tools/game-api-matrix.py`，G0 落地），**官方每列一个 API，这里就有一行**；官方新增 API 时重跑脚本，新行自动标「本轮补入」。

等级：
- **E** 引擎实现：行为以取证为准逐项对齐，可无头验证；
- **H** 宿主代理：引擎提供完整的函数 / 对象形状，宿主在 `hostApis` 点名后由宿主实现；未点名时与微信「不支持」同形 fail（`canIUse` 为 false）；
- **E+H** 引擎负责绘制 / 事件 / 状态，系统能力（输入法、状态栏、亮度）由宿主做；
- **W** 依赖微信后台或关系链：引擎提供完整形状与错误语义，数据由宿主后端可选提供（`hostApis` 点名），否则返回与微信无授权时同形的错误。

状态：「已有」= 现有 prelude 已定义（游戏 profile 装上即可，仍要过该行的用例）；「已规划」= 01~09 已写到；「本轮补入」= 第三轮审查发现计划里漏掉、现在补进对应期。

统计：共 382 项；等级 E 147、E+H 3、H 192、W 40；状态 已有 247、已规划 64、本轮补入 71。

通用验收（对每一行都成立，用例 GT-API1~3，见 11 §4）：函数存在且 `typeof` 与官方一致；同步 / 异步 / Promise 形态一致；回调参数与 errMsg 形状与取证一致。


## 基础

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.env` | 环境变量 | E | 已有 | G1 | GT-API1 |
| `wx.getAppAuthorizeSetting` | 获取微信APP授权设置 更新 | E | 已有 | G1 | GT-API1 |
| `wx.getAppBaseInfo` | 获取微信APP基础信息 | E | 已有 | G1 | GT-API1 |
| `wx.getDeviceBenchmarkInfo` | 获取设备性能得分和机型档位数据 | E | 已有 | G6 | GT-API1 |
| `wx.getDeviceInfo` | 获取设备基础信息 | E | 已有 | G1 | GT-API1 |
| `wx.getEnterOptionsSync` | 获取小游戏打开的参数（包括冷启动和热启动） 应用级事件 | E | 已有 | G6 | GT-LC |
| `wx.getLaunchOptionsSync` | 获取小游戏冷启动时的参数 | E | 已有 | G6 | GT-LC |
| `wx.getLogManager` | 获取日志管理器对象 | E | 已有 | G1 | GT-API1 |
| `wx.getPerformance` | 获取性能管理器 | E | 已有 | G2 | GT-K/GT-S |
| `wx.getRealtimeLogManager` | 获取实时日志管理器对象 | E | 已有 | G1 | GT-API1 |
| `wx.getSystemInfo` | 获取系统信息 | E | 已有 | G1 | GT-API1 |
| `wx.getSystemInfoAsync` | 异步获取系统信息 | E | 已有 | G1 | GT-API1 |
| `wx.getSystemInfoSync` | wx.getSystemInfo 的同步版本 | E | 已有 | G1 | GT-API1 |
| `wx.getSystemSetting` | 获取设备设置 | E | 已有 | G1 | GT-API1 |
| `wx.getUpdateManager` | 获取全局唯一的版本更新管理器，用于管理小程序更新 | E | 已有 | G6 | GT-API1 |
| `wx.getUserCryptoManager` | 获取用户加密模块 | E | 已有 | G1 | GT-API1 |
| `wx.getWindowInfo` | 获取窗口信息 | E | 已有 | G1 | GT-API1 |
| `wx.loadSubpackage` | 触发分包加载，详见 分包加载 | E | 已规划 | G6 | GT-P |
| `wx.offAudioInterruptionBegin` | 移除音频因为受到系统占用而被中断开始事件的监听函数 性能 | E | 已有 | G1 | GT-AU |
| `wx.offAudioInterruptionEnd` | 移除音频中断结束事件的监听函数 | E | 已有 | G1 | GT-AU |
| `wx.offError` | 移除全局错误事件的监听函数 | E | 已规划 | G1 | GT-S |
| `wx.offHide` | 移除小游戏隐藏到后台事件的监听函数 | E | 已规划 | G6 | GT-LC |
| `wx.offShow` | 移除小游戏回到前台的事件的监听函数 | E | 已规划 | G6 | GT-LC |
| `wx.offUnhandledRejection` | 移除未处理的 Promise 拒绝事件的监听函数 | E | 已规划 | G1 | GT-S |
| `wx.onAudioInterruptionBegin` | 监听音频因为受到系统占用而被中断开始事件 | E | 已有 | G1 | GT-AU |
| `wx.onAudioInterruptionEnd` | 监听音频中断结束事件 | E | 已有 | G1 | GT-AU |
| `wx.onError` | 监听全局错误事件 | E | 已有 | G1 | GT-S |
| `wx.onHide` | 监听小游戏隐藏到后台事件 | E | 已有 | G6 | GT-LC |
| `wx.onShow` | 监听小游戏回到前台的事件 | E | 已有 | G6 | GT-LC |
| `wx.onUnhandledRejection` | 监听未处理的 Promise 拒绝事件 | E | 已有 | G1 | GT-S |
| `wx.openAppAuthorizeSetting` | 跳转系统微信授权管理页 | E | 已有 | G1 | GT-API1 |
| `wx.openSystemBluetoothSetting` | 跳转系统蓝牙设置页 | E | 已有 | G1 | GT-API1 |
| `wx.preDownloadSubpackage` | 触发分包预下载 | E | 已有 | G6 | GT-P |
| `wx.reportPerformance` | 小程序测速上报 | E | 已有 | G1 | GT-API1 |
| `wx.setEnableDebug` | 设置是否打开调试开关 | E | 已有 | G1 | GT-API1 |
| `wx.triggerGC` | 加快触发 JavaScriptCore 垃圾回收（Garbage Collect | E | 已规划 | G2 | GT-K/GT-S |
| `wx.updateWeChatApp` | 更新客户端版本 | E | 已有 | G1 | GT-API1 |

## 渲染

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.createCanvas` | 创建一个画布对象 | E | 已规划 | G2 | GT-C/GT-M |
| `wx.createImage` | 创建一个图片对象 | E | 已规划 | G4 | GT-I |
| `wx.createImageData` | 这里有两种使用方法, 一种是指定ImageData的宽和高, 另外一种使用已有的 | E | 已规划 | G2 | GT-C/GT-M |
| `wx.createPath2D` | 创建一个 Path2D 路径对象 | E | **本轮补入** | G2 | GT-C/GT-M |
| `wx.exitPointerLock` | 解除锁定鼠标指针 | E | **本轮补入** | G3 | GT-T |
| `wx.getGPU` | 获取 WebGPU 的 GPU 对象，等价于 Web 端的 navigator. | E | **本轮补入** | G8 | GT-WG |
| `wx.getTextLineHeight` | 获取一行文本的行高 | E | 已规划 | G4 | GT-F |
| `wx.isPointerLocked` | 检查鼠标指针是否被锁定 | E | **本轮补入** | G3 | GT-T |
| `wx.loadFont` | 加载自定义字体文件 | E | 已规划 | G4 | GT-F |
| `wx.requestPointerLock` | 锁定鼠标指针 | E | **本轮补入** | G3 | GT-T |
| `wx.setCursor` | 加载自定义光标，仅支持 PC 平台 | E | **本轮补入** | G3 | GT-T |
| `wx.setPreferredFramesPerSecond` | 可以修改渲染帧率 | E | 已规划 | G2 | GT-K/GT-S |

## 设备

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.closeBLEConnection` | 断开与蓝牙低功耗设备的连接 蓝牙-通用 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.closeBluetoothAdapter` | 关闭蓝牙模块 电量 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.createBLEConnection` | 连接蓝牙低功耗设备 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.createBLEPeripheralServer` | 建立本地作为蓝牙低功耗外围设备的服务端，可创建多个 BLEPeripheralS | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getBLEDeviceCharacteristics` | 获取蓝牙低功耗设备某个服务中所有特征 (characteristic) | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getBLEDeviceRSSI` | 获取蓝牙低功耗设备的信号强度 (Received Signal Strength | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getBLEDeviceServices` | 获取蓝牙低功耗设备所有服务 (service) | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getBLEMTU` | 获取蓝牙低功耗的最大传输单元 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getBatteryInfo` | 获取设备电量 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getBatteryInfoSync` | wx.getBatteryInfo 的同步版本 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getBeacons` | 获取所有已搜索到的 Beacon 设备 BeaconInfo Beacon 设备 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getBluetoothAdapterState` | 获取本机蓝牙适配器状态 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getBluetoothDevices` | 获取在蓝牙模块生效期间所有搜索到的蓝牙设备 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getClipboardData` | 获取系统剪贴板的内容 游戏手柄 | E | 已有 | G7 | GT-API1 |
| `wx.getConnectedBluetoothDevices` | 根据主服务 UUID 获取已连接的蓝牙设备 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getGamepads` | 获取已连接的游戏手柄信息，仅在 PC 平台支持 网络 | E | **本轮补入** | G3 | GT-T |
| `wx.getLocalIPAddress` | 获取局域网IP地址 屏幕 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getNetworkType` | 获取网络类型 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getScreenBrightness` | 获取屏幕亮度 加速计 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getScreenRecordingState` | 查询用户是否在录屏 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.hideKeyboard` | 隐藏键盘 触摸 | H | 已有 | G7 | GT-H1/GT-H3/GT-U |
| `wx.isBluetoothDevicePaired` | 查询蓝牙设备是否配对，仅安卓支持 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.makeBluetoothPair` | 蓝牙配对接口，仅安卓支持 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.notifyBLECharacteristicValueChange` | 启用蓝牙低功耗设备特征值变化时的 notify 功能，订阅特征 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offAccelerometerChange` | 移除加速度数据事件的监听函数 罗盘 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offBLECharacteristicValueChange` | 移除蓝牙低功耗设备的特征值变化事件的全部监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offBLEConnectionStateChange` | 移除蓝牙低功耗连接状态改变事件的监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offBLEMTUChange` | 移除蓝牙低功耗的最大传输单元变化事件的监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offBLEPeripheralConnectionStateChanged` | 移除当前外围设备被连接或断开连接事件的监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offBeaconServiceChange` | 移除 Beacon 服务状态变化事件的全部监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offBeaconUpdate` | 移除 Beacon 设备更新事件的全部监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offBluetoothAdapterStateChange` | 移除蓝牙适配器状态变化事件的全部监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offBluetoothDeviceFound` | 移除搜索到新设备的事件的全部监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offCompassChange` | 移除罗盘数据变化事件的监听函数 设备方向 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offDeviceMotionChange` | 移除设备方向变化事件的监听函数 转屏 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offDeviceOrientationChange` | 移除屏幕转向切换事件的监听函数 陀螺仪 | E | 已规划 | G6 | GT-O |
| `wx.offGyroscopeChange` | 移除陀螺仪数据变化事件的监听函数 内存 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offKeyDown` | 移除键盘按键按下事件的监听函数 | E | 已有 | G3 | GT-T |
| `wx.offKeyUp` | 移除键盘按键弹起事件的监听函数 | E | 已有 | G3 | GT-T |
| `wx.offKeyboardComplete` | 移除键盘收起的事件的监听函数 | E | **本轮补入** | G3 | GT-T |
| `wx.offKeyboardConfirm` | 移除用户点击键盘 Confirm 按钮时的事件的监听函数 | E | **本轮补入** | G3 | GT-T |
| `wx.offKeyboardHeightChange` | 移除键盘高度变化事件的监听函数 | E | 已有 | G3 | GT-T |
| `wx.offKeyboardInput` | 移除键盘输入事件的监听函数 | E | 已规划 | G3 | GT-T |
| `wx.offMemoryWarning` | 移除内存不足告警事件的监听函数 扫码 | H | 已有 | G6 | GT-H1/GT-H3/GT-S |
| `wx.offMouseDown` | 移除鼠标按键按下事件的监听函数 滚轮 | E | 已规划 | G3 | GT-T |
| `wx.offMouseMove` | 移除鼠标移动事件的监听函数 | E | 已规划 | G3 | GT-T |
| `wx.offMouseUp` | 移除鼠标按键弹起事件的监听函数 | E | 已规划 | G3 | GT-T |
| `wx.offNetworkStatusChange` | 移除网络状态变化事件的监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offNetworkWeakChange` | 移除弱网状态变化事件的监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offScreenRecordingStateChanged` | 移除用户录屏事件的监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offTouchCancel` | 移除触点失效事件的监听函数 Touch 在触控设备上的触摸点 鼠标 | E | 已规划 | G3 | GT-T |
| `wx.offTouchEnd` | 移除触摸结束事件的监听函数 | E | 已规划 | G3 | GT-T |
| `wx.offTouchMove` | 移除触点移动事件的监听函数 | E | 已规划 | G3 | GT-T |
| `wx.offTouchStart` | 移除开始触摸事件的监听函数 | E | 已规划 | G3 | GT-T |
| `wx.offUserCaptureScreen` | 用户主动截屏事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offWheel` | 移除鼠标滚轮事件的监听函数 蓝牙-信标(Beacon) | E | 已规划 | G3 | GT-T |
| `wx.onAccelerometerChange` | 监听加速度数据事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onBLECharacteristicValueChange` | 监听蓝牙低功耗设备的特征值变化事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onBLEConnectionStateChange` | 监听蓝牙低功耗连接状态改变事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onBLEMTUChange` | 监听蓝牙低功耗的最大传输单元变化事件（仅安卓触发） | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onBLEPeripheralConnectionStateChanged` | 监听当前外围设备被连接或断开连接事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onBeaconServiceChange` | 监听 Beacon 服务状态变化事件，仅能注册一个监听 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onBeaconUpdate` | 监听 Beacon 设备更新事件，仅能注册一个监听 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onBluetoothAdapterStateChange` | 监听蓝牙适配器状态变化事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onBluetoothDeviceFound` | 监听搜索到新设备的事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onCompassChange` | 监听罗盘数据变化事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onDeviceMotionChange` | 监听设备方向变化事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onDeviceOrientationChange` | 监听屏幕转向切换事件 | E | 已规划 | G6 | GT-O |
| `wx.onGamepadConnected` | 监听用户已连接游戏手柄的事件 | E | **本轮补入** | G3 | GT-T |
| `wx.onGamepadDisconnected` | 监听用户断开游戏手柄的事件 | E | **本轮补入** | G3 | GT-T |
| `wx.onGyroscopeChange` | 监听陀螺仪数据变化事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onKeyDown` | 监听键盘按键按下事件，仅适用于 PC 平台 | E | 已有 | G3 | GT-T |
| `wx.onKeyUp` | 监听键盘按键弹起事件，仅适用于 PC 平台 | E | 已有 | G3 | GT-T |
| `wx.onKeyboardComplete` | 监听键盘收起的事件 | E | **本轮补入** | G3 | GT-T |
| `wx.onKeyboardConfirm` | 监听用户点击键盘 Confirm 按钮时的事件 | E | **本轮补入** | G3 | GT-T |
| `wx.onKeyboardHeightChange` | 监听键盘高度变化事件 | E | 已有 | G3 | GT-T |
| `wx.onKeyboardInput` | 监听键盘输入事件 | E | 已规划 | G3 | GT-T |
| `wx.onMemoryWarning` | 监听内存不足告警事件 | H | 已有 | G6 | GT-H1/GT-H3/GT-S |
| `wx.onMouseDown` | 监听鼠标按键按下事件 | E | 已规划 | G3 | GT-T |
| `wx.onMouseMove` | 监听鼠标移动事件 | E | 已规划 | G3 | GT-T |
| `wx.onMouseUp` | 监听鼠标按键弹起事件 | E | 已规划 | G3 | GT-T |
| `wx.onNetworkStatusChange` | 监听网络状态变化事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onNetworkWeakChange` | 监听弱网状态变化事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onScreenRecordingStateChanged` | 监听用户录屏事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onTouchCancel` | 监听触点失效事件 | E | 已规划 | G3 | GT-T |
| `wx.onTouchEnd` | 监听触摸结束事件 | E | 已规划 | G3 | GT-T |
| `wx.onTouchMove` | 监听触点移动事件 | E | 已规划 | G3 | GT-T |
| `wx.onTouchStart` | 监听开始触摸事件 | E | 已规划 | G3 | GT-T |
| `wx.onUserCaptureScreen` | 监听用户主动截屏事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onWheel` | 监听鼠标滚轮事件 | E | 已规划 | G3 | GT-T |
| `wx.openBluetoothAdapter` | 初始化蓝牙模块 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.readBLECharacteristicValue` | 读取蓝牙低功耗设备特征值的二进制数据 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.scanCode` | 调起客户端扫码界面进行扫码 振动 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.setBLEMTU` | 协商设置蓝牙低功耗的最大传输单元 (Maximum Transmission U | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.setClipboardData` | 设置系统剪贴板的内容 | E | 已有 | G7 | GT-API1 |
| `wx.setDeviceOrientation` | 切换横竖屏 | H | 已规划 | G6 | GT-H1/GT-H3/GT-O |
| `wx.setKeepScreenOn` | 设置是否保持常亮状态 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.setScreenBrightness` | 设置屏幕亮度 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.setVisualEffectOnCapture` | 设置截屏/录屏时屏幕表现 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.showKeyboard` | 显示键盘 | H | 已规划 | G7 | GT-H1/GT-H3/GT-U |
| `wx.startAccelerometer` | 开始监听加速度数据 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.startBeaconDiscovery` | 开始搜索附近的 Beacon 设备 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.startBluetoothDevicesDiscovery` | 开始搜寻附近的蓝牙外围设备 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.startCompass` | 开始监听罗盘数据 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.startDeviceMotionListening` | 开始监听设备方向的变化 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.startGyroscope` | 开始监听陀螺仪数据 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.stopAccelerometer` | 停止监听加速度数据 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.stopBeaconDiscovery` | 停止搜索附近的 Beacon 设备 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.stopBluetoothDevicesDiscovery` | 停止搜寻附近的蓝牙外围设备 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.stopCompass` | 停止监听罗盘数据 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.stopDeviceMotionListening` | 停止监听设备方向的变化 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.stopGyroscope` | 停止监听陀螺仪数据 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.updateKeyboard` | 更新键盘输入框内容 | H | 已规划 | G7 | GT-H1/GT-H3/GT-U |
| `wx.vibrateLong` | 使手机发生较长时间的振动（400 ms) | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.vibrateShort` | 使手机发生较短时间的振动（15 ms） | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.writeBLECharacteristicValue` | 向蓝牙低功耗设备特征值中写入二进制数据 | H | 已有 | G7 | GT-H1/GT-H3 |

## 界面

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.getMenuButtonBoundingClientRect` | 获取菜单按钮（右上角胶囊按钮）的布局位置信息 状态栏 | E+H | 已有 | G7 | GT-U |
| `wx.getOfficialComponentsInfo` | 获取所有官方组件的相关信息 | E | **本轮补入** | G7 | GT-API1 |
| `wx.hideLoading` | 隐藏 loading 提示框 菜单 | E | 已有 | G7 | GT-U |
| `wx.hideToast` | 隐藏消息提示框 | E | 已有 | G7 | GT-U |
| `wx.offOfficialComponentsInfoChange` | 移除官方组件信息变化事件的监听函数 | E | **本轮补入** | G7 | GT-API1 |
| `wx.offWindowResize` | 移除窗口尺寸变化事件的监听函数 | E | 已有 | G6 | GT-O |
| `wx.offWindowStateChange` | 移除小程序窗口状态变化事件的监听函数 | E | 已有 | G7 | GT-API1 |
| `wx.onOfficialComponentsInfoChange` | 监听官方组件信息变化事件 | E | **本轮补入** | G7 | GT-API1 |
| `wx.onWindowResize` | 监听窗口尺寸变化事件 | E | 已有 | G6 | GT-O |
| `wx.onWindowStateChange` | 监听小程序窗口状态变化事件 | E | 已有 | G7 | GT-API1 |
| `wx.setMenuStyle` | 动态设置通过右上角按钮拉起的菜单的样式 | E+H | 已规划 | G7 | GT-U |
| `wx.setStatusBarStyle` | 当在配置中设置 showStatusBar 时，屏幕顶部会显示状态栏 | E+H | 已规划 | G7 | GT-U |
| `wx.setWindowSize` | 设置窗口大小，该接口仅适用于 PC 平台，使用细则请参见指南 | E | 已有 | G3 | GT-T |
| `wx.showActionSheet` | 显示操作菜单 | E | 已有 | G7 | GT-U |
| `wx.showLoading` | 显示 loading 提示框 | E | 已有 | G7 | GT-U |
| `wx.showModal` | 显示模态对话框 | E | 已有 | G7 | GT-U |
| `wx.showToast` | 显示消息提示框 | E | 已有 | G7 | GT-U |

## 媒体

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.chooseImage` | 从本地相册选择图片或使用相机拍照 录音 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.chooseMedia` | 拍摄或从手机相册中选择图片或视频 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.chooseMessageFile` | 从客户端会话选择文件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.compressImage` | 压缩图片接口，可选压缩质量 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.createCamera` | 创建相机 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.createInnerAudioContext` | 创建内部 audio 上下文 | E | 已有 | G5 | GT-AU |
| `wx.createMediaAudioPlayer` | 创建媒体音频播放器对象 | E | 已有 | G5 | GT-AU |
| `wx.createVideo` | 创建视频 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.createVideoDecoder` | 创建视频解码器，可逐帧获取解码后的数据 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.createWebAudioContext` | 创建 WebAudio 上下文 | E | 已有 | G5 | GT-AU |
| `wx.exitVoIPChat` | 退出（销毁）实时语音通话 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getAvailableAudioSources` | 获取当前支持的音频输入源 | E | 已有 | G5 | GT-AU |
| `wx.getRecorderManager` | 获取全局唯一的录音管理器 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.joinVoIPChat` | 加入 (创建) 实时语音通话，更多信息可见 实时语音指南 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offVoIPChatInterrupted` | 移除被动断开实时语音通话事件的监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offVoIPChatMembersChanged` | 移除实时语音通话成员在线状态变化事件的监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offVoIPChatSpeakersChanged` | 移除实时语音通话成员通话状态变化事件的监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offVoIPChatStateChanged` | 移除房间状态变化事件的监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onVoIPChatInterrupted` | 监听被动断开实时语音通话事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onVoIPChatMembersChanged` | 监听实时语音通话成员在线状态变化事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onVoIPChatSpeakersChanged` | 监听实时语音通话成员通话状态变化事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onVoIPChatStateChanged` | 监听房间状态变化事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.previewImage` | 在新页面中全屏预览图片 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.previewMedia` | 预览图片和视频 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.saveImageToPhotosAlbum` | 保存图片到系统相册 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.setInnerAudioOption` | 设置 InnerAudioContext 的播放选项 | E | 已有 | G5 | GT-AU |
| `wx.updateVoIPChatMuteConfig` | 更新实时语音静音设置 | H | 已有 | G7 | GT-H1/GT-H3 |

## 网络

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.closeSocket` | 关闭 WebSocket 连接 | E | 已有 | G4 | GT-N |
| `wx.connectSocket` | 创建一个 WebSocket 连接 | E | 已有 | G4 | GT-N |
| `wx.createTCPSocket` | 创建一个 TCP Socket 实例 | E | 已有 | G4 | GT-N |
| `wx.createUDPSocket` | 创建一个 UDP Socket 实例 | E | 已有 | G4 | GT-N |
| `wx.downloadFile` | 下载文件资源到本地 | E | 已有 | G4 | GT-N |
| `wx.onSocketClose` | 监听 WebSocket 连接关闭事件 | E | **本轮补入** | G4 | GT-N |
| `wx.onSocketError` | 监听 WebSocket 错误事件 | E | **本轮补入** | G4 | GT-N |
| `wx.onSocketMessage` | 监听 WebSocket 接收到服务器的消息事件 | E | **本轮补入** | G4 | GT-N |
| `wx.onSocketOpen` | 监听 WebSocket 连接打开事件 | E | **本轮补入** | G4 | GT-N |
| `wx.request` | 发起 HTTPS 网络请求 | E | 已有 | G4 | GT-N |
| `wx.sendSocketMessage` | 通过 WebSocket 连接发送数据 | E | 已有 | G4 | GT-N |
| `wx.uploadFile` | 将本地资源上传到服务器 | E | 已有 | G4 | GT-N |

## 文件

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.getFileSystemManager` | 获取全局唯一的文件管理器 FileStats 每个 FileStats 对象包含 | E | 已有 | G4 | GT-FS |
| `wx.saveFileToDisk` | 保存文件系统的文件到用户磁盘，仅在 PC 端支持 | E | 已有 | G4 | GT-FS |

## 数据缓存

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.clearStorage` | 清理本地数据缓存 | E | 已有 | G4 | GT-FS |
| `wx.clearStorageSync` | wx.clearStorage 的同步版本 | E | 已有 | G4 | GT-FS |
| `wx.createBufferURL` | 根据传入的 buffer 创建一个唯一的 URL 存在内存中 | E | 已有 | G4 | GT-API1 |
| `wx.getBackgroundFetchData` | 拉取 backgroundFetch 客户端缓存数据 | E | 已有 | G4 | GT-API1 |
| `wx.getBackgroundFetchToken` | 获取设置过的自定义登录态 | E | 已有 | G4 | GT-API1 |
| `wx.getStorage` | 从本地缓存中异步获取指定 key 的内容 | E | 已有 | G4 | GT-FS |
| `wx.getStorageInfo` | 异步获取当前 storage 的相关信息 | E | 已有 | G4 | GT-FS |
| `wx.getStorageInfoSync` | wx.getStorageInfo 的同步版本 | E | 已有 | G4 | GT-FS |
| `wx.getStorageSync` | 从本地缓存中同步获取指定 key 的内容 | E | 已有 | G4 | GT-FS |
| `wx.onBackgroundFetchData` | 监听收到 backgroundFetch 数据事件 | E | 已有 | G4 | GT-API1 |
| `wx.removeStorage` | 从本地缓存中移除指定 key | E | 已有 | G4 | GT-FS |
| `wx.removeStorageSync` | wx.removeStorage 的同步版本 | E | 已有 | G4 | GT-FS |
| `wx.revokeBufferURL` | 根据 URL 销毁存在内存中的数据 | E | 已有 | G4 | GT-API1 |
| `wx.setBackgroundFetchToken` | 设置自定义登录态，在周期性拉取数据时带上，便于第三方服务器验证请求合法性 | E | 已有 | G4 | GT-API1 |
| `wx.setStorage` | 将数据存储在本地缓存中指定的 key 中 | E | 已有 | G4 | GT-FS |
| `wx.setStorageSync` | 将数据存储在本地缓存中指定的 key 中 | E | 已有 | G4 | GT-FS |

## Worker

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.createWorker` | 创建一个 Worker 线程 | E | 已有 | G6 | GT-W |

## 工具

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.decode` | 将 ArrayBuffer 按照指定的编码格式解码成字符串 | E | 已规划 | G4 | GT-B |
| `wx.enableOfflineModeDebug` | 开启或关闭离线模式调试 | E | **本轮补入** | G4 | GT-API1 |
| `wx.encode` | 将字符串按照指定的编码格式编码成 ArrayBuffer | E | 已规划 | G4 | GT-B |
| `wx.offOfflineModeStateChange` | 取消监听离线模式状态变化事件 | E | **本轮补入** | G4 | GT-API1 |
| `wx.onOfflineModeStateChange` | 监听离线模式状态变化事件 | E | **本轮补入** | G4 | GT-API1 |

## 数据分析

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.getExptInfoSync` | 给定实验参数数组，获取对应的实验参数值 | W | 已有 | G1 | GT-H1/GT-OD |
| `wx.getGameExptInfo` | 给定实验参数数组，获取对应的实验参数值 | W | **本轮补入** | G1 | GT-H1/GT-OD |
| `wx.getGameLogManager` | 初始化并返回一个游戏日志管理器实例，用于记录和管理游戏日志 | E | **本轮补入** | G1 | GT-API1 |
| `wx.getMiniReportManager` | 初始化并返回一个MiniReportManager实例，用于记录和管理小游戏上报 | E | **本轮补入** | G1 | GT-API1 |
| `wx.getScenePerformanceManager` | 该接口专门用于协助开发者分析场景化的运行性能数据 | E | **本轮补入** | G1 | GT-API1 |
| `wx.reportEvent` | 事件上报 | E | 已有 | G1 | GT-API1 |
| `wx.reportMonitor` | 自定义业务数据监控上报接口 | E | 已有 | G1 | GT-API1 |
| `wx.reportScene` | 用于游戏启动阶段的自定义场景上报 | E | **本轮补入** | G1 | GT-API1 |
| `wx.reportUserBehaviorBranchAnalytics` | 上报场景分析，用于UI组件（一般是按钮）相关事件的上报，事件目前有曝光、点击两种 | W | **本轮补入** | G1 | GT-H1/GT-OD |

## 位置

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.getFuzzyLocation` | 获取当前的模糊地理位置 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getLocation` | 获取当前的地理位置、速度 | H | 已有 | G7 | GT-H1/GT-H3 |

## 开放接口

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.addCard` | 批量添加卡券 我的小程序 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.authorize` | 提前向用户发起授权请求 开放数据 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.checkIsAddedToMyMiniProgram` | 检查小程序是否被添加至 「我的小程序」 人脸检测 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.checkIsSupportFacialRecognition` | 检查当前设备是否支持人脸识别能力 账号信息 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.checkSession` | 检查登录态 session_key 是否过期 授权 | W | 已有 | G7 | GT-H1/GT-OD |
| `wx.createFeedbackButton` | 创建打开意见反馈页面的按钮 | H | 已规划 | G7 | GT-H1/GT-H3/GT-U |
| `wx.createGameClubButton` | 创建游戏圈按钮 | H | 已规划 | G7 | GT-H1/GT-H3/GT-U |
| `wx.createOpenSettingButton` | 创建打开设置页面的按钮 AuthSetting 用户授权设置信息，详情参考权限 | H | 已规划 | G7 | GT-H1/GT-H3/GT-U |
| `wx.createPageManager` | 小游戏开放页面管理器，用于启动微信内置的各种小游戏活动、功能页面 PageMan | H | **本轮补入** | G7 | GT-H1/GT-H3 |
| `wx.createStoreGift` | 创建蓝包组件 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.createUserInfoButton` | 创建用户信息按钮 UserInfo 用户信息 | H | 已规划 | G7 | GT-H1/GT-H3/GT-U |
| `wx.getAccountInfoSync` | 获取当前账号信息 视频号 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getChannelsLiveInfo` | 获取视频号直播信息 微信群 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getChannelsLiveNoticeInfo` | 获取视频号直播预告信息 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getFriendCloudStorage` | 拉取当前用户所有同玩好友的托管数据 FriendInfo 用户信息 Friend | W | **本轮补入** | G7 | GT-H1/GT-OD/GT-FS |
| `wx.getFriendSendGiftStatus` | 指定一批 openid 查询好友送礼状态，该接口只在开放数据域下可用 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.getGameClubData` | 获取游戏圈数据 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.getGroupCloudStorage` | 获取群同玩成员的游戏数据 | W | **本轮补入** | G7 | GT-H1/GT-OD/GT-FS |
| `wx.getGroupEnterInfo` | 获取微信群聊场景下的小程序启动信息 隐私信息授权 | W | 已有 | G7 | GT-H1/GT-OD |
| `wx.getGroupInfo` | 获取群信息 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.getGroupMembersInfo` | 获取所选群成员的头像、昵称，自行在开放数据域中渲染 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.getOpenDataContext` | 获取开放数据域 | E | 已规划 | G9 | GT-H1/GT-OD |
| `wx.getPhoneNumber` | 手机号快速验证，向用户申请，并在用户同意后，快速填写和验证手机 具体说明 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.getPotentialFriendList` | 获取可能对游戏感兴趣的未注册的好友名单 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.getPrivacySetting` | 查询隐私授权情况 微信客服 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getRankManager` | 初始化并返回一个擂台赛管理器实例，用于管理游戏得分存取、得分排行榜（开发中）以及 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.getRelationFriendList` | 获取小游戏同玩互动好友列表 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.getSetting` | 获取用户的当前设置 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getSharedCanvas` | 获取主域和开放数据域共享的 sharedCanvas | E | **本轮补入** | G9 | GT-H1/GT-OD |
| `wx.getUserCloudStorage` | 获取当前用户托管数据当中对应 key 的数据 | W | **本轮补入** | G7 | GT-H1/GT-OD/GT-FS |
| `wx.getUserCloudStorageKeys` | 获取当前用户托管数据当中所有的 key | W | **本轮补入** | G7 | GT-H1/GT-OD/GT-FS |
| `wx.getUserInfo` | 获取用户信息 | W | 已有 | G7 | GT-H1/GT-OD |
| `wx.getUserInteractiveStorage` | 获取当前用户互动型托管数据对应 key 的数据 | W | **本轮补入** | G7 | GT-H1/GT-OD/GT-FS |
| `wx.getWeRunData` | 获取用户过去三十一天微信运动步数 OPENLINK | W | 已有 | G7 | GT-H1/GT-OD |
| `wx.login` | 调用接口获取登录凭证（code） | W | 已有 | G7 | GT-H1/GT-OD |
| `wx.modifyFriendInteractiveStorage` | 修改好友的互动型托管数据，该接口只可在开放数据域下使用 | W | **本轮补入** | G7 | GT-H1/GT-OD/GT-FS |
| `wx.offInteractiveStorageModified` | 取消监听成功修改好友的互动型托管数据事件，该接口在游戏主域使用 | W | **本轮补入** | G7 | GT-H1/GT-OD/GT-FS |
| `wx.onInteractiveStorageModified` | 监听成功修改好友的互动型托管数据事件，该接口在游戏主域使用 | W | **本轮补入** | G7 | GT-H1/GT-OD/GT-FS |
| `wx.onMessage` | 监听主域发送的消息 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.onNeedPrivacyAuthorization` | 监听隐私接口需要用户授权事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.openCard` | 查看微信卡包中的卡券 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.openChannelsActivity` | 打开视频号视频 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.openChannelsEvent` | 打开视频号活动页 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.openChannelsLive` | 打开视频号直播 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.openChannelsUserProfile` | 打开视频号主页 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.openCustomerServiceChat` | 打开微信客服，页面产生点击事件后才可调用 订阅消息 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.openCustomerServiceConversation` | 进入客服会话 微信运动 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.openPrivacyContract` | 跳转至隐私协议页面 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.openSetting` | 调起客户端小程序设置界面，返回用户设置的操作结果 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.removeUserCloudStorage` | 删除用户托管数据当中对应 key 的数据 | W | 已规划 | G7 | GT-H1/GT-OD/GT-FS |
| `wx.requestFacialRecognition` | 腾讯游戏人脸识别验证功能是基于健康系统防沉迷体系，用于识别疑似未成年人冒用成年人 | H | **本轮补入** | G7 | GT-H1/GT-H3/GT-N |
| `wx.requestFacialVerify` | 对用户实名信息进行基于生物识别的人脸核身验证 | H | 已有 | G7 | GT-H1/GT-H3/GT-N |
| `wx.requestSubscribeMessage` | 调起客户端小游戏订阅消息界面，返回用户订阅消息的操作结果 | W | 已有 | G7 | GT-H1/GT-OD/GT-N |
| `wx.requestSubscribeSystemMessage` | 调起小游戏系统订阅消息界面，返回用户订阅消息的操作结果 | W | **本轮补入** | G7 | GT-H1/GT-OD/GT-N |
| `wx.requirePrivacyAuthorize` | 模拟隐私接口调用，并触发隐私弹窗逻辑 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.reserveChannelsLive` | 预约视频号直播 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.sendGiftToFriend` | 指定 openid 给他好友送礼，该接口只在开放数据域下可用 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.setUserCloudStorage` | 对用户托管数据进行写数据操作 | W | 已规划 | G7 | GT-H1/GT-OD/GT-FS |

## 转发

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.authPrivateMessage` | 验证私密消息 | W | 已有 | G7 | GT-H1/GT-OD |
| `wx.checkHandoffEnabled` | 检查是否可以进行接力，该接口需要在开放数据域调用 | H | **本轮补入** | G7 | GT-H1/GT-H3 |
| `wx.getShareInfo` | 获取转发详细信息（主要是获取群ID） | W | 已有 | G7 | GT-H1/GT-OD |
| `wx.hideShareMenu` | 隐藏当前页面的转发按钮 | H | 已有 | G7 | GT-H1/GT-H3/GT-U |
| `wx.offAddToFavorites` | 移除用户点击菜单「收藏」按钮时触发的事件的全部监听函数 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.offCopyUrl` | 移除用户点击右上角菜单的「复制链接」按钮时触发的事件的全部监听函数 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offHandoff` | 移除用户点击菜单「在电脑上打开」按钮时触发的事件的全部监听函数 | H | **本轮补入** | G7 | GT-H1/GT-H3 |
| `wx.offShareAppMessage` | 移除用户点击右上角菜单的「转发」按钮时触发的事件的监听函数 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.offShareMessageToFriend` | 移除主域接收wx.shareMessageToFriend接口的成功失败通知事件 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.offShareTimeline` | 移除用户点击右上角菜单的「分享到朋友圈」按钮时触发的事件的监听函数 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.onAddToFavorites` | 监听用户点击菜单「收藏」按钮时触发的事件 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.onCopyUrl` | 监听用户点击右上角菜单的「复制链接」按钮时触发的事件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.onHandoff` | 监听用户点击菜单「在电脑上打开」按钮时触发的事件 | H | **本轮补入** | G7 | GT-H1/GT-H3 |
| `wx.onShareAppMessage` | 监听用户点击右上角菜单的「转发」按钮时触发的事件 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.onShareMessageToFriend` | 监听主域接收wx.shareMessageToFriend接口的成功失败通知事件 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.onShareTimeline` | 监听用户点击右上角菜单的「分享到朋友圈」按钮时触发的事件 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.setHandoffQuery` | 设置接力参数，该接口需要在游戏域调用 | H | **本轮补入** | G7 | GT-H1/GT-H3 |
| `wx.setMessageToFriendQuery` | 设置 wx.shareMessageToFriend 接口 query 字段的值 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.shareAppMessage` | 主动拉起转发，进入选择通讯录界面 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.shareMessageToFriend` | 接口 query 字段的值 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.showShareImageMenu` | 打开分享图片弹窗，可以将图片发送给朋友、收藏或下载 | H | 已有 | G7 | GT-H1/GT-H3/GT-U |
| `wx.showShareMenu` | 设置右上角点开的详情界面中的分享按钮是否可用 | H | 已有 | G7 | GT-H1/GT-H3/GT-U |
| `wx.startHandoff` | 开始进行接力，该接口需要在开放数据域调用 | H | **本轮补入** | G7 | GT-H1/GT-H3 |
| `wx.updateShareMenu` | 更新转发属性 | H | 已有 | G7 | GT-H1/GT-H3/GT-U |

## 虚拟支付

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.checkIsSupportMidasPayment` | 检查当前环境是否支持虚拟支付 | W | **本轮补入** | G7 | GT-H1/GT-OD |
| `wx.requestMidasFriendPayment` | 发起米大师朋友礼物索要 | W | **本轮补入** | G7 | GT-H1/GT-OD/GT-N |
| `wx.requestMidasPayment` | 发起购买游戏币支付请求，可参考虚拟支付2.0游戏币，虚拟支付全流程可参考技术手册 | W | 已规划 | G7 | GT-H1/GT-OD/GT-N |
| `wx.requestMidasPaymentGameItem` | 发起道具直购支付请求，可参考虚拟支付2.0道具直购，虚拟支付全流程可参考技术手册 | W | **本轮补入** | G7 | GT-H1/GT-OD/GT-N |

## 广告

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.createAdSkipCard` | 创建并返回广告跳过卡（畅玩卡）管理器 AdSkipCard 实例（单例） Ban | H | **本轮补入** | G7 | GT-H1/GT-H3 |
| `wx.createBannerAd` | 创建 banner 广告组件 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.createCustomAd` | 创建原生模板广告组件 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.createGridAd` | 创建 grid(格子) | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.createInterstitialAd` | 创建插屏广告组件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.createRewardedVideoAd` | 创建激励视频广告组件 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getDirectAdStatusSync` | 获取直玩广告组件展示状态 | H | **本轮补入** | G7 | GT-H1/GT-H3 |
| `wx.getShowSplashAdStatus` | 获取封面广告组件展示状态 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.offDirectAdStatusChange` | 移除监听直玩广告状态变化的监听函数 | H | **本轮补入** | G7 | GT-H1/GT-H3 |
| `wx.onDirectAdStatusChange` | 监听监听直玩广告状态变化 | H | **本轮补入** | G7 | GT-H1/GT-H3 |

## 游戏服务

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.getGameServerManager` | 获取 全局唯一 的游戏服务管理器 | W | 已规划 | G7 | GT-H1/GT-OD |

## 游戏对局回放

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.createGameRecorderShareButton` | 创建游戏对局回放分享按钮，返回一个单例对象 | H | **本轮补入** | G7 | GT-H1/GT-H3/GT-U |
| `wx.getGameRecorder` | 获取全局唯一的游戏画面录制对象 | H | 已规划 | G7 | GT-H1/GT-H3 |
| `wx.operateGameRecorderVideo` | 分享游戏对局回放 | H | **本轮补入** | G7 | GT-H1/GT-H3 |

## 聊天工具

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.enterChatToolMode` | 进入聊天工具开放能力模式 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.exitChatTool` | 退出聊天工具开放能力模式 | H | **本轮补入** | G7 | GT-H1/GT-H3 |
| `wx.getChatToolInfo` | 获取聊天工具模式下的群聊信息 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.isChatTool` | 是否处于聊天工具模式 | H | **本轮补入** | G7 | GT-H1/GT-H3 |
| `wx.notifyGroupMembers` | 提醒用户完成任务，标题长度不超过 30 个字符，支持中英文和数字，中文算2个字符 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.openChatTool` | 进入聊天工具模式 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.selectGroupMembers` | 选择聊天室的成员，并返回选择成员的 group_openid | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.shareAppMessageToGroup` | 转发小程序卡片到聊天 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.shareEmojiToGroup` | 转发表情到聊天 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.shareImageToGroup` | 转发图片到聊天 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.shareTextToGroup` | 转发文本到聊天 | H | **本轮补入** | G7 | GT-H1/GT-H3 |
| `wx.shareVideoToGroup` | 转发视频到聊天 | H | 已有 | G7 | GT-H1/GT-H3 |

## 跳转

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.exitMiniProgram` | 退出当前小程序 | H | 已有 | G6 | GT-H1/GT-H3/GT-LC |
| `wx.navigateBackMiniProgram` | 返回到上一个小程序 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.navigateToMiniProgram` | 打开另一个小程序 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.openPhotoTopicView` | 打开微信官方贴图话题发布页 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.restartMiniProgram` | 重启当前小程序 | H | 已有 | G6 | GT-H1/GT-H3/GT-LC |

## 第三方平台

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.getExtConfig` | 获取第三方平台自定义的数据字段 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getExtConfigSync` | wx.getExtConfig 的同步版本 | H | 已有 | G7 | GT-H1/GT-H3 |

## AI

| API | 官方说明 | 等级 | 状态 | 期 | 用例 |
|---|---|---|---|---|---|
| `wx.createInferenceSession` | 创建 AI 推理 Session | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.createVKSession` | 创建 vision kit 会话对象 VKBodyAnchor 人体 ancho | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.faceDetect` | 人脸检测，使用前需要通过 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.getInferenceEnvInfo` | 获取通用AI推理引擎版本 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.initFaceDetect` | 初始化人脸检测 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.isVKSupport` | 判断支持版本 | H | 已有 | G7 | GT-H1/GT-H3 |
| `wx.stopFaceDetect` | 停止人脸检测 | H | 已有 | G7 | GT-H1/GT-H3 |

## 对象与方法

每个对象的**全部方法**都必须存在（哪怕等级是 H、宿主没接）：游戏引擎拿到对象后直接调方法，不判 `typeof`。用例 GT-API2 遍历这张表。

| 对象 | 等级 | 期 | 方法（官方列出的全部） |
|---|---|---|---|
| `AudioBuffer` | E | G5 | copyFromChannel, copyToChannel, getChannelData |
| `BLEPeripheralServer` | H | G7 | addService, offCharacteristicReadRequest, offCharacteristicSubscribed, offCharacteristicUnsubscribed, offCharacteristicWriteRequest, onCharacteristicReadRequest, onCharacteristicSubscribed, onCharacteristicUnsubscribed, onCharacteristicWriteRequest, removeService, startAdvertising, stopAdvertising, writeCharacteristicValue |
| `BannerAd` | H | G7 | destroy, hide, offError, offLoad, offResize, onError, onLoad, onResize, show |
| `BufferSourceNode` | E | G5 | connect, disconnect, start, stop |
| `Camera` | H | G7 | closeFrameChange, destroy, listenFrameChange, onAuthCancel, onCameraFrame, onStop, setZoom, startRecord, stopRecord, takePhoto |
| `Canvas` | E | G2 | getContext, toDataURL, toTempFilePath, toTempFilePathSync |
| `CustomAd` | H | G7 | destroy, hide, isShow, offClose, offError, offHide, offLoad, offResize, onClose, onError, onHide, onLoad, onResize, show |
| `DownloadTask` | E | G4 | abort, offHeadersReceived, offProgressUpdate, onHeadersReceived, onProgressUpdate |
| `FeedbackButton` | E+H | G7 | destroy, hide, offTap, onTap, show |
| `FileSystemManager` | E | G4 | access, accessSync, appendFile, appendFileSync, close, closeSync, copyFile, copyFileSync, fstat, fstatSync, ftruncate, ftruncateSync, getFileInfo, getSavedFileList, mkdir, mkdirSync, open, openSync, read, readCompressedFile, readCompressedFileSync, readFile, readFileSync, readSync, readZipEntry, readdir, readdirSync, removeSavedFile, rename, renameSync, rmdir, rmdirSync, saveFile, saveFileSync, stat, statSync, truncate, truncateSync, unlink, unlinkSync, unzip, write, writeFile, writeFileSync, writeSync |
| `GameClubButton` | E+H | G7 | destroy, hide, offTap, onTap, show |
| `GameLogManager` | E+H | G7 | getCommonInfo, log, tag, updateCommonInfo |
| `GameRecorder` | H | G7 | abort, isAtempoSupported, isFrameSupported, isSoundSupported, isVolumeSupported, off, on, pause, resume, start, stop |
| `GameRecorderShareButton` | E+H | G7 | hide, offTap, onTap, show |
| `GameServerManager` | W | G7 | broadcastInRoom, cancelMatch, changeSeat, createRoom, endGame, endStateService, getFriendsStateData, getJoinVoIPChatSignature, getLastRoomInfo, getLostFrames, inviteFriend, joinRoom, kickoutMember, login, logout, memberLeaveRoom, offBeKickedOut, offBroadcast, offDisconnect, offGameEnd, offGameStart, offInvite, offLockStepError, offLogout, offMatch, offRoomInfoChange, offStateUpdate, offSyncFrame, onBeKickedOut, onBroadcast, onDisconnect, onGameEnd, onGameStart, onInvite, onLockStepError, onLogout, onMatch, onRoomInfoChange, onStateUpdate, onSyncFrame, ownerLeaveRoom, reconnect, restart, setInviteData, setState, startGame, startMatch, startStateService, updateReadyStatus, uploadFrame |
| `GridAd` | H | G7 | destroy, hide, offError, offLoad, offResize, onError, onLoad, onResize, show |
| `InferenceSession` | H | G7 | destroy, offError, offLoad, onError, onLoad, run |
| `InnerAudioContext` | E | G5 | destroy, offCanplay, offEnded, offError, offPause, offPlay, offSeeked, offSeeking, offStop, offTimeUpdate, offWaiting, onCanplay, onEnded, onError, onPause, onPlay, onSeeked, onSeeking, onStop, onTimeUpdate, onWaiting, pause, play, seek, stop |
| `InterstitialAd` | H | G7 | destroy, load, offClose, offError, offLoad, onClose, onError, onLoad, show |
| `LoadSubpackageTask` | E | G6 | onProgressUpdate |
| `LogManager` | E | G7 | debug, info, log, warn |
| `MediaAudioPlayer` | E | G5 | addAudioSource, destroy, removeAudioSource, start, stop |
| `MiniReportManager` | H | G7 | report |
| `OpenDataContext` | E | G9 | postMessage |
| `OpenSettingButton` | E+H | G7 | destroy, hide, offTap, onTap, show |
| `PageManager` | H | G7 | destroy, load, off, on, show |
| `Performance` | E | G2 | now |
| `PreDownloadSubpackageTask` | E | G6 | onProgressUpdate |
| `RankManager` | W | G7 | abort, createChallenge, getScore, middleUpdate, offChallengeStart, onChallengeStart, update |
| `RealtimeLogManager` | E+H | G7 | addFilterMsg, error, info, setFilterMsg, warn |
| `RecorderManager` | H | G7 | onError, onFrameRecorded, onInterruptionBegin, onInterruptionEnd, onPause, onResume, onStart, onStop, pause, resume, start, stop |
| `RequestTask` | E | G4 | abort, offChunkReceived, offHeadersReceived, onChunkReceived, onHeadersReceived |
| `RewardedVideoAd` | H | G7 | destroy, load, offClose, offError, offLoad, onClose, onError, onLoad, setMiniGameAutoIntelligenceInfo, setServerSideVerificationData, show |
| `ScenePerformanceManager` | E | G7 | getCommonInfo, setCommonInfo, setData |
| `SocketTask` | E | G4 | close, onClose, onError, onMessage, onOpen, send |
| `Stats` | E | G7 | isDirectory, isFile |
| `StoreGift` | W | G7 | getOrderInfo, isSupported, open |
| `TCPSocket` | E | G4 | bindWifi, close, connect, offBindWifi, offClose, offConnect, offError, offMessage, onBindWifi, onClose, onConnect, onError, onMessage, write |
| `UDPSocket` | E | G4 | bind, close, connect, offClose, offError, offListening, offMessage, onClose, onError, onListening, onMessage, send, setTTL, write |
| `UpdateManager` | E | G6 | applyUpdate, onCheckForUpdate, onUpdateFailed, onUpdateReady |
| `UploadTask` | E | G4 | abort, offHeadersReceived, offProgressUpdate, onHeadersReceived, onProgressUpdate |
| `UserCryptoManager` | E | G4 | getLatestUserKey, getRandomValues |
| `UserInfoButton` | E+H | G7 | destroy, hide, offTap, onTap, show |
| `VKCamera` | H | G7 | getProjectionMatrix |
| `VKFrame` | H | G7 | getCameraBuffer, getCameraJpgBuffer, getCameraTexture, getDepthBuffer, getDisplayTransform, getLegSegmentBuffer |
| `VKSession` | H | G7 | addMarker, addOSDMarker, cancelAnimationFrame, destroy, detectBody, detectDepth, detectFace, detectHand, getAllMarker, getAllOSDMarker, getVKFrame, hitTest, off, on, removeMarker, removeOSDMarker, requestAnimationFrame, runOCR, setDepthOccRange, start, stop, update3DMode, updateMaskMode, updateOSDThreshold |
| `Video` | E+H | G7 | destroy, exitFullScreen, offEnded, offError, offPause, offPlay, offProgress, offTimeUpdate, offWaiting, onEnded, onError, onPause, onPlay, onProgress, onTimeUpdate, onWaiting, pause, play, requestFullScreen, seek, stop |
| `VideoDecoder` | H | G7 | getFrameData, off, on, remove, seek, start, stop |
| `WebAudioContext` | E | G5 | close, createAnalyser, createBiquadFilter, createBuffer, createBufferSource, createChannelMerger, createChannelSplitter, createConstantSource, createDelay, createDynamicsCompressor, createGain, createIIRFilter, createOscillator, createPanner, createPeriodicWave, createScriptProcessor, createWaveShaper, decodeAudioData, resume, suspend |
| `WebGLRenderingContext` | E | G8 | WebGL 1/2 全部标准方法（Khronos）+ 微信扩展 `wxBindCanvasTexture`（把一个 Canvas 的纹理绑定到 WebGL 上下文，11 §2.6） |
| `Worker` | E | G6 | getCameraFrameData, onError, onMessage, onProcessKilled, postMessage, terminate, testOnProcessKilled |
