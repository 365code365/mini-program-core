import 'dart:async';
import 'dart:convert';
import 'dart:io' show Platform;
import 'dart:ui' show FramePhase, Offset;

import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';

import 'events.dart';
import 'media.dart';
import 'same_layer.dart' show MiniNativeViewTap;

/// 一个小程序实例。**一个进程同一时刻只能有一个**（引擎的小程序根目录、数据目录、
/// 网络回包队列都是进程级的）：在上一个 [dispose] 之前再建会报 `ALREADY_RUNNING`。
///
/// 用法：
/// ```dart
/// final c = MiniProgramController(
///   appDir: unpackedDir,
///   // 选图 / 选文件 / 存相册插件自带（builtinHostApis），这里只接宿主自己的能力
///   hostApis: {'login': (args) async => {'code': await myLogin()}},
///   nativeComponents: const ['video'],
/// );
/// // 放进 MiniProgramScaffold(controller: c) 或 MiniProgramView(controller: c)
/// ```
///
/// 引擎跑在原生侧的专用线程上（Flutter 3.29 起 Dart 就跑在平台主线程，引擎的一帧
/// 动辄几毫秒到十几毫秒，同步调会让整个 App 一起卡）。这里的方法全是异步投递。
class MiniProgramController {
  MiniProgramController({
    required this.appDir,
    this.dataDir,
    this.route,
    this.hostApis = const {},
    this.builtinHostApis = true,
    this.nativeComponents = const [],
    this.systemInfo = const {},
    this.backend = MiniBackend.auto,
    this.menuButton = true,
    this.navigationBar = true,
    this.routeAnimation = true,
    this.androidSurfaceView = true,
    this.nativeViewLag = 0,
    this.sameLayerVideo = true,
    this.immersiveVideoFullscreen = true,
    this.sameLayerViews = const [],
  });

  /// [nativeComponents] 里哪些标签走**同层渲染**（Android）：[MiniProgramView.nativeViewBuilder] 造的控件不再盖在
  /// 画面上，而是离屏画成一帧交给引擎画进页面 —— 滚动时它就是页面里的一块像素，跟页面一起动，不会像盖上去的控件
  /// 那样一滑就错开、一顿一顿。点击照常到控件（`GestureDetector.onTap`），拖动滚页面。
  ///
  /// 只适合纯 Flutter 画的控件（CustomPaint、图片、文字）：平台视图（google_maps_flutter 这类）离屏画不出来，
  /// 别列进来。`video` 不在这里（见 [sameLayerVideo]）。iOS 上暂时照旧盖在画面上
  final List<String> sameLayerViews;

  List<String> get _sameLayerTags => defaultTargetPlatform == TargetPlatform.android
      ? [for (final t in sameLayerViews) if (t != 'video' && nativeComponents.contains(t)) t]
      : const [];

  /// 视频全屏（控制条的全屏键 / `requestFullScreen`；横屏由引擎把那一层转过来画）时收起状态栏与导航条，
  /// 退出时恢复成 [SystemUiMode.edgeToEdge]。宿主自己的系统栏模式不是这个就关掉它、听 [videoFullscreen] 自己处理
  final bool immersiveVideoFullscreen;

  /// 有视频在全屏（引擎的 `videoFullscreen` 事件）
  final ValueNotifier<bool> videoFullscreen = ValueNotifier(false);

  /// `nativeComponents` 列了 `video` 时，Android 上走**同层渲染**（默认）：插件自带的系统播放器
  /// （MediaPlayer）解码，帧交给引擎画进页面 —— 滚动时视频就是页面里的一块像素，跟页面一起动；
  /// 封面、播放键、控制条由引擎画，点击与 `wx.createVideoContext` 的命令交给插件的播放器，
  /// `bindplay` / `bindtimeupdate` / `bindended` 等照常触发。这时 `video` 不再走 [MiniProgramView.nativeViewBuilder]。
  ///
  /// false = 旧做法（宿主用 Flutter 控件盖在画面上，滑动时会与页面错开）。iOS 上暂时总是旧做法（插件还没带播放器）
  final bool sameLayerVideo;

  bool get _sameLayerVideo =>
      sameLayerVideo && defaultTargetPlatform == TargetPlatform.android && nativeComponents.contains('video');

  /// 解包后的小程序目录（含 `app.json`）
  final String appDir;

  /// 可写数据目录（storage / 图片缓存）。不给就用插件默认的 `filesDir/mini-render`
  final String? dataDir;

  /// 起始页（`pages/x/x?id=1`），不给就是 app.json 首页
  final String? route;

  /// 宿主实现的 `wx.*`：名字会装进小程序的 `wx`，调用转到这里
  final Map<String, MiniHostApi> hostApis;

  /// 插件自带的宿主能力（默认开）：选图 / 选视频 / 选文件 / 存相册 / 打开文档，走系统自己的选择器，
  /// 见 [MiniBuiltinHostApis]。[hostApis] 里给了同名的用宿主的；关掉后这几个回到「没点名 = 立刻 fail」
  final bool builtinHostApis;

  /// 装进小程序 `wx` 的宿主 API：宿主给的 + 插件自带的
  List<String> get _hostApiNames => {
        ...hostApis.keys,
        if (builtinHostApis) ...MiniBuiltinHostApis.names,
      }.toList();

  /// 由宿主用原生控件承载的组件标签（见 [MiniProgramView.nativeViewBuilder]）
  final List<String> nativeComponents;

  /// 合并进 `wx.getSystemInfoSync()` 的字段（`model` / `brand` …）。尺寸、像素比、
  /// 安全区、平台、语言、主题由插件自动给，这里给了会覆盖。
  final Map<String, Object?> systemInfo;

  /// 渲染后端偏好。默认 [MiniBackend.auto]：设备有可用的 GPU（启动自检逐字节一致）就用 GPU，
  /// 否则 / 运行中出错就回退 CPU。要强制软件渲染传 [MiniBackend.cpu]。
  /// iOS 的 Flutter 外部纹理是 CVPixelBuffer，没有可上屏的 Metal 层，那边恒走 CPU
  final MiniBackend backend;

  /// 右上角胶囊（微信的「•••」+「⊙」）。`navigationStyle: custom` 的页面由引擎画在画面里，
  /// 普通页面由 [MiniProgramScaffold] 画在导航栏右侧；点击都从 [menuButtonTaps] 出来。
  /// 关掉后两处都不画
  final bool menuButton;

  /// 导航栏由引擎画（默认）：画面铺满整屏、顶到状态栏，普通页面顶上是引擎画的导航栏
  /// （标题 / 颜色 / 返回键 / loading / 胶囊），custom 页面没有。切页时**画面尺寸不变** ——
  /// 从前由 [MiniProgramScaffold] 用 AppBar 画，普通页 ↔ custom 页一切换 AppBar 就要出现 / 消失、
  /// 画面跟着改尺寸重排：进 custom 页先拉伸一下、返回时跳一下，`onLoad` 读到的 `windowHeight` 也不对。
  /// false = 旧做法（宿主自己画导航栏，[MiniProgramScaffold] 用 AppBar）
  final bool navigationBar;

  /// 页面转场（默认开）：navigateTo 新页从右边滑入、返回时当前页向右滑出，300ms（与微信一致）
  final bool routeAnimation;

  /// Android：引擎画到自己的 SurfaceView（平台视图，hybrid composition），由系统合成器直接上屏，
  /// **不经过 Flutter 的帧**（默认）。false = 画进 Flutter 外部纹理（旧做法）。
  ///
  /// 外部纹理那条路上，屏上那一帧取决于 Flutter 自己这一帧有没有按时出：主线程一忙，Flutter 这个
  /// vsync 不出帧，屏上就重复上一张 —— 引擎每帧都按时交了，慢拖、弹菜单照样一顿一顿
  /// （一加 PLY110 慢拖 3% 的 vsync 这样丢，SurfaceFlinger 量的）。iOS 不受影响（一直是纹理）
  final bool androidSurfaceView;

  /// 宿主承载的原生组件（[nativeComponents]）比引擎画面晚上屏几个 vsync。页面滚动时控件按
  /// 「自己这一帧的 vsync + 这么多帧」推算页面滚到了哪（[MiniNativeMotion.shiftAt]），与引擎画面对齐。
  ///
  /// 默认 0：一加 PLY110（90Hz，Android 独立图层，GPU）上甩动截屏校准，0 时 24 张里 22 张视频框与页面
  /// 差 ≤2 物理像素（均值 0.8），1 时视频框反而跑到页面前面（均值 7.8）—— 控件与引擎画面是同一个 vsync
  /// 上屏的，晚到的只是事件，而那部分正是按 vsync 推算消掉的。别的机型跟不上 / 跑到前面时用
  /// `adb shell setprop debug.mini.native_lag <帧数>` 校准（负数写成 `' -2'`；小于 -2 等于关掉推算、
  /// 只按事件摆），改完重开小程序生效，压过这里的值：视频框落在页面后面就调大，跑到前面就调小
  final double nativeViewLag;

  static const MethodChannel _channel = MethodChannel('dev.minirender/engine');
  static MiniProgramController? _active;
  static bool _initialized = false;

  /// 引擎画面所在的纹理（建好之前是 null；Android 换像素出口时会换一次）
  final ValueNotifier<int?> textureId = ValueNotifier(null);

  /// 引擎画面在独立图层上（Android [androidSurfaceView]）：视图建平台视图，不建 Texture
  final ValueNotifier<bool> surfaceView = ValueNotifier(false);

  /// 引擎画面的物理像素尺寸（= 逻辑尺寸 × dpr 取整）
  final ValueNotifier<Size?> pixelSize = ValueNotifier(null);

  /// 当前页面（导航栏标题、颜色、栈深）
  final ValueNotifier<MiniPage?> page = ValueNotifier(null);

  /// 宿主承载的原生组件在视口里的位置（引擎那一帧的；滚动中按 [nativeMotion] 推算此刻的位置）
  final ValueNotifier<List<MiniNativeView>> nativeViews = ValueNotifier(const []);

  /// 走同层渲染的组件（[sameLayerViews]）：键、标签、属性、框的逻辑尺寸（`rect` 的原点是 0）。
  /// 只在页面结构变了时更新（滚动不更新：画面在引擎里跟着页面走）
  final ValueNotifier<List<MiniNativeView>> sameLayerViewList = ValueNotifier(const []);
  /// 点在同层渲染的组件框里（引擎的 `nativeViewTap`，本地逻辑坐标）
  Stream<MiniNativeViewTap> get nativeViewTaps => _viewTaps.stream;
  final StreamController<MiniNativeViewTap> _viewTaps = StreamController.broadcast();
  /// 与 [nativeViews] 同一条事件带的滚动取样（旧版引擎不带时为 null）。
  /// 先于 [nativeViews] 更新：监听 [nativeViews] 的一方拿到的就是同一帧的取样
  MiniNativeMotion? get nativeMotion => _nativeMotion;
  MiniNativeMotion? _nativeMotion;

  /// 实际生效的 [nativeViewLag]：调试属性 `debug.mini.native_lag`（Android）压过构造参数
  double get effectiveNativeViewLag => _nativeLagOverride ?? nativeViewLag;
  double? _nativeLagOverride;

  /// 当前聚焦的输入框（null = 键盘该收起）
  final ValueNotifier<MiniInputFocus?> focusedInput = ValueNotifier(null);

  /// 实际在用的后端（建好之前是 null；GPU 运行中回退时会变）
  final ValueNotifier<MiniBackendStatus?> activeBackend = ValueNotifier(null);

  /// 小程序的关于信息（启动后第一帧到；胶囊「•••」展示它）
  final ValueNotifier<MiniAbout?> about = ValueNotifier(null);

  final StreamController<String> _logs = StreamController.broadcast();
  final StreamController<Map<String, dynamic>> _events = StreamController.broadcast();
  final StreamController<MiniMenuButtonAction> _menuTaps = StreamController.broadcast();

  /// 小程序的 `console.*` 输出
  Stream<String> get logs => _logs.stream;

  /// 点了右上角胶囊（引擎画的那个与导航栏里的那个都从这里出来）
  Stream<MiniMenuButtonAction> get menuButtonTaps => _menuTaps.stream;

  /// 宿主自己画的胶囊被点了（如放在原生导航栏里）：送进 [menuButtonTaps]，
  /// 与引擎画的那个走同一条路，监听的一方不用分两处
  void handleMenuButton(MiniMenuButtonAction action) {
    if (!_menuTaps.isClosed) _menuTaps.add(action);
  }

  /// 全部原始事件（JSON 对象）
  Stream<Map<String, dynamic>> get events => _events.stream;

  bool _created = false;
  bool _disposed = false;
  Future<void>? _creating;

  /// 页面栈只剩一页时为 false（系统返回键该交给宿主）
  bool get canGoBack => (page.value?.depth ?? 0) > 1;

  static Future<void> _ensureInit() async {
    // 每次都挂一下：宿主（或测试框架）可能换过二进制信使，挂重复了也只是覆盖同一个处理函数
    _channel.setMethodCallHandler(_onNativeCall);
    if (_initialized) return;
    _initialized = true;
    // 热重启后 Dart 状态清空了，原生侧的旧实例还在跑：先清掉
    await _channel.invokeMethod<void>('init');
  }

  /// 插件给的默认目录：`files`（应用私有、持久）、`cache`，Android 还有 `external`
  /// （`/sdcard/Android/data/<包名>/files`，`adb push` 能直接放进去）
  static Future<Map<String, String>> paths() async {
    await _ensureInit();
    final m = await _channel.invokeMapMethod<String, String>('paths');
    return m ?? const {};
  }

  /// 由 [MiniProgramView] 在第一次布局后调用：逻辑尺寸、像素比、安全区都在这时才知道
  Future<void> attach({
    required Size logicalSize,
    required double devicePixelRatio,
    required List<double> safeArea,
    Map<String, Object?> autoSystemInfo = const {},
  }) {
    return _creating ??= _create(logicalSize, devicePixelRatio, safeArea, autoSystemInfo);
  }

  Future<void> _create(Size size, double dpr, List<double> safeArea, Map<String, Object?> auto) async {
    await _ensureInit();
    if (_disposed) return;
    if (_active != null && !identical(_active, this)) {
      throw StateError('同一进程同时只能有一个小程序（先 dispose 上一个）');
    }
    _active = this;
    final options = <String, Object?>{
      'systemInfo': {
        'platform': Platform.isIOS ? 'ios' : Platform.isAndroid ? 'android' : Platform.operatingSystem,
        ...auto,
        ...systemInfo,
      },
      'safeArea': {'top': safeArea[0], 'right': safeArea[1], 'bottom': safeArea[2], 'left': safeArea[3]},
      'hostApis': _hostApiNames,
      // 同层渲染的视频不再是盖上去的控件（不走 nativeViewBuilder）
      'nativeComponents': [
        for (final t in nativeComponents)
          if (!(t == 'video' && _sameLayerVideo) && !_sameLayerTags.contains(t)) t,
      ],
      if (_sameLayerVideo || _sameLayerTags.isNotEmpty) 'sameLayer': [if (_sameLayerVideo) 'video', ..._sameLayerTags],
      'menuButton': menuButton,
      'navigationBar': navigationBar,
      'routeAnimation': routeAnimation,
    };
    final Map<String, Object?>? r;
    try {
      r = await _channel.invokeMapMethod<String, Object?>('create', {
        'appDir': appDir,
        'dataDir': dataDir,
        'route': route,
        'width': size.width.floor(),
        'height': size.height.floor(),
        'dpr': dpr,
        'options': jsonEncode(options),
        'backend': backend.code,
        'presentation': androidSurfaceView ? 'view' : 'texture',
      });
    } catch (_) {
      // 失败了要让出「唯一实例」的位置，宿主才能换个目录再建
      if (identical(_active, this)) _active = null;
      _creating = null;
      rethrow;
    }
    if (r == null) throw StateError('mini_render: create 没有返回');
    _created = true;
    _applyTexture(r);
    if (_disposed) await _channel.invokeMethod<void>('dispose');
  }

  void _applyTexture(Map<dynamic, dynamic> r) {
    final w = (r['pixelWidth'] as num?)?.toDouble();
    final h = (r['pixelHeight'] as num?)?.toDouble();
    if (w != null && h != null) pixelSize.value = Size(w, h);
    final id = (r['textureId'] as num?)?.toInt();
    if (id != null) textureId.value = id;
    if (r['view'] == true) surfaceView.value = true;
    if (r.containsKey('backend')) activeBackend.value = MiniBackendStatus.fromTextureInfo(r);
    if (r['pace'] == true) _startPaceLog();
    final lag = r['nativeLag'];
    if (lag is num) _nativeLagOverride = lag.toDouble();
  }

  static bool _paceOn = false;

  /// `debug.mini.pace` 开着（原生组件那一层据此每帧打一行 `🎞N`，见 native_layer.dart）
  bool get paceLog => _paceOn;

  /// `adb shell setprop debug.mini.pace 1`：Flutter 每一帧打一行 `🎞F <vsync µs> <光栅开始 µs> <光栅结束 µs>`。
  ///
  /// 引擎画进的是 Flutter 的外部纹理，**屏上那一帧是 Flutter 光栅线程取纹理那一刻最新的那张**。
  /// 两边各跟各的 vsync 出帧：引擎这一帧交得比 Flutter 取纹理晚一点，这个 vsync 就重复上一张，
  /// 下一个 vsync 一次跳两张 —— 引擎侧的帧耗时、位移全都均匀，屏上照样一顿一顿。
  /// 与引擎那边的 `🎞` 行（交出时刻）对起来才看得出来（`tools/pace-check.py`）。
  /// 时间戳与 Android `System.nanoTime` 同一个单调钟（微秒）
  void _startPaceLog() {
    if (_paceOn) return;
    _paceOn = true;
    SchedulerBinding.instance.addTimingsCallback((timings) {
      for (final t in timings) {
        debugPrint('🎞F ${t.timestampInMicroseconds(FramePhase.vsyncStart)} '
            '${t.timestampInMicroseconds(FramePhase.rasterStart)} '
            '${t.timestampInMicroseconds(FramePhase.rasterFinish)}');
      }
    });
  }

  Future<void> _call(String method, [Object? args]) async {
    if (!_created || _disposed) return;
    await _channel.invokeMethod<void>(method, args);
  }

  /// 尺寸 / 像素比变了（转屏、分屏）
  Future<void> resize(Size logicalSize, double dpr) async {
    if (!_created || _disposed) return;
    final r = await _channel.invokeMapMethod<String, Object?>('resize', {
      'width': logicalSize.width.floor(),
      'height': logicalSize.height.floor(),
      'dpr': dpr,
    });
    if (r != null) _applyTexture(r);
  }

  /// 安全区（逻辑像素：上 右 下 左）
  Future<void> setSafeArea(List<double> insets) =>
      _call('safeArea', {'top': insets[0], 'right': insets[1], 'bottom': insets[2], 'left': insets[3]});

  /// 指针三段：0 按下 / 1 移动 / 2 抬起 / 3 取消（视口逻辑坐标）。
  /// **别在宿主侧合成「点击」**：tap / longpress 都由引擎的触摸状态机产出。
  ///
  /// [timeMs]：事件自己的时间（`PointerEvent.timeStamp` 的毫秒）。给了的话，引擎按它重采样页面拖动
  /// （过 MethodChannel 再转引擎线程，一帧收到几个移动是碰运气，按到达时刻走慢拖会一顿一顿）。
  void pointer(int phase, double x, double y, {double? timeMs}) {
    if (!_created || _disposed) return;
    _channel.invokeMethod<void>('pointer', {'phase': phase, 'x': x, 'y': y, if (timeMs != null) 't': timeMs});
  }

  void wheel(double dy, {bool precise = true}) {
    if (!_created || _disposed) return;
    _channel.invokeMethod<void>('wheel', {'dy': dy, 'precise': precise});
  }

  /// 往聚焦的输入框送文字（输入法提交的整串）
  Future<void> sendText(String text) => _call('text', {'text': text});

  /// 功能键（引擎 `KEY_NAMES` 那张表）：enter / newline / backspace / delete / left / right /
  /// up / down / home / end、`shift+…` 扩展选区、word-left / word-right、doc-start / doc-end、
  /// delete-word-back、selectall / copy / cut / undo / redo / escape
  Future<void> sendKey(String name) => _call('key', {'name': name});

  /// 返回上一页。false = 页面栈只剩一页（宿主该关掉小程序）
  Future<bool> back() async {
    if (!_created || _disposed) return false;
    return await _channel.invokeMethod<bool>('back') ?? false;
  }

  /// 跳到某页（深链）
  Future<void> navigate(String url) => _call('navigate', {'url': url});

  /// 同层渲染的组件画好的一帧（RGBA 不预乘，[width] × [height] 设备像素）：引擎只重画页面上的这一块
  Future<void> pushViewFrame(String key, int width, int height, Uint8List rgba) =>
      _call('viewFrame', {'key': key, 'width': width, 'height': height, 'data': rgba});

  /// 宿主承载的原生组件发生了事件（派发给节点上的 `bind<name>`）
  Future<void> sendNativeViewEvent(String key, String name, [Map<String, Object?> detail = const {}]) =>
      _call('nativeViewEvent', {'key': key, 'name': name, 'detail': jsonEncode(detail)});

  /// 宿主推事件给小程序的 `wx.onXxx` 监听者：`name` 如 `accelerometerChange`、`locationChange`、
  /// `bluetoothDeviceFound`、`networkStatusChange`、`themeChange`、`keyboardHeightChange`、
  /// `contextEvent`（`MapContext.on` / `CameraContext.onCameraFrame` 等对象自己的事件）。
  /// 返回收到它的监听者个数（0 = 小程序没在听，可以停掉对应的传感器）
  Future<int> dispatchEvent(String name, [Map<String, Object?> payload = const {}]) async {
    if (_disposed || !_created) return 0;
    final n = await _channel.invokeMethod<int>('dispatchEvent', {'name': name, 'payload': jsonEncode(payload)});
    return n ?? 0;
  }

  /// 进前台 / 进后台 / 内存告警（[MiniProgramView] 已自动转发）
  Future<void> lifecycle(int what) => _call('lifecycle', {'what': what});

  /// 在逻辑层执行一段 JS（调试与集成测试用）
  Future<String> debugEval(String code) async {
    if (!_created || _disposed) return '';
    return await _channel.invokeMethod<String>('eval', {'code': code}) ?? '';
  }

  /// 把引擎当前帧存成 PNG（调试与集成测试用：与屏幕截图逐像素对，确认纹理没被拉伸、颜色没变）
  Future<bool> debugDumpFrame(String path) async {
    if (!_created || _disposed) return false;
    return await _channel.invokeMethod<bool>('dumpFrame', {'path': path}) ?? false;
  }

  Future<void> dispose() async {
    if (_disposed) return;
    _disposed = true;
    if (identical(_active, this)) _active = null;
    if (_created) await _channel.invokeMethod<void>('dispose');
    await _logs.close();
    await _events.close();
    await _viewTaps.close();
    await _menuTaps.close();
  }

  // ───────────────────────── 原生 → Dart ─────────────────────────

  static Future<Object?> _onNativeCall(MethodCall call) async {
    final c = _active;
    if (c == null || c._disposed) return null;
    switch (call.method) {
      case 'events':
        for (final raw in (call.arguments as List).cast<String>()) {
          final ev = jsonDecode(raw);
          if (ev is Map<String, dynamic>) c._onEvent(ev);
        }
      case 'texture':
        c._applyTexture(call.arguments as Map);
    }
    return null;
  }

  void _onEvent(Map<String, dynamic> ev) {
    if (!_events.isClosed) _events.add(ev);
    switch (ev['type']) {
      case 'page':
        page.value = MiniPage.fromJson(ev);
      case 'navigationBar':
        final p = page.value;
        if (p != null) {
          final w = Map<String, dynamic>.of(p.window);
          if (ev.containsKey('title')) w['navigationBarTitleText'] = ev['title'];
          if (ev.containsKey('backgroundColor')) w['navigationBarBackgroundColor'] = ev['backgroundColor'];
          if (ev.containsKey('frontColor')) {
            w['navigationBarTextStyle'] = '${ev['frontColor']}'.toLowerCase() == '#ffffff' ? 'white' : 'black';
          }
          if (ev.containsKey('loading')) w['navigationBarLoading'] = ev['loading'];
          page.value = MiniPage(route: p.route, depth: p.depth, isTabBar: p.isTabBar, window: w);
        }
      case 'inputFocus':
        focusedInput.value = MiniInputFocus.fromJson(ev);
      case 'inputBlur':
        focusedInput.value = null;
      case 'clipboard':
        // wx.setClipboardData、输入框里的 copy / cut：写系统剪贴板
        Clipboard.setData(ClipboardData(text: '${ev['data'] ?? ''}'));
      case 'nativeViews':
        _nativeMotion = MiniNativeMotion.fromJson(ev);
        nativeViews.value = [
          for (final v in (ev['views'] as List? ?? const []))
            if (v is Map) MiniNativeView.fromJson(v.cast<String, dynamic>()),
        ];
      case 'hostCall':
        _onHostCall(MiniHostCall.fromJson(ev));
      case 'sameLayerViews':
        sameLayerViewList.value = [
          for (final v in (ev['views'] as List? ?? const []))
            if (v is Map)
              MiniNativeView.fromJson({
                ...v.cast<String, dynamic>(),
                'x': 0,
                'y': 0,
              }),
        ];
      case 'nativeViewTap':
        if (!_viewTaps.isClosed) {
          _viewTaps.add(MiniNativeViewTap(
            '${ev['key'] ?? ''}',
            Offset((ev['x'] as num?)?.toDouble() ?? 0, (ev['y'] as num?)?.toDouble() ?? 0),
          ));
        }
      case 'menuButton':
        final a = MiniMenuButtonAction.parse(ev['action']);
        if (a != null) handleMenuButton(a);
      case 'app':
        final a = ev['about'];
        if (a is Map) about.value = MiniAbout.fromJson(a.cast<String, dynamic>());
      case 'log':
        if (!_logs.isClosed) _logs.add('${ev['text']}');
      case 'videoFullscreen':
        final on = ev['fullScreen'] == true;
        videoFullscreen.value = on;
        // 收起时系统报的新安全区引擎按住不用（不重排页面），退出后放出来的还是原来那份
        if (immersiveVideoFullscreen) {
          SystemChrome.setEnabledSystemUIMode(on ? SystemUiMode.immersiveSticky : SystemUiMode.edgeToEdge);
        }
    }
  }

  Future<void> _onHostCall(MiniHostCall call) async {
    final api = hostApis[call.api];
    final builtin = api == null && builtinHostApis && MiniBuiltinHostApis.names.contains(call.api);
    var ok = false;
    Map<String, dynamic> res;
    if (api == null && !builtin) {
      res = {'errMsg': '${call.api}:fail 宿主没有实现'};
    } else {
      try {
        res = api != null ? await api(call.args) : await MiniBuiltinHostApis.handle(call, channel: _channel);
        ok = true;
      } on MiniHostApiError catch (e) {
        res = {...e.extra, 'errMsg': e.errMsg};
      } catch (e) {
        res = {'errMsg': '${call.api}:fail $e'};
      }
    }
    await _call('hostReply', {'id': call.id, 'ok': ok, 'result': jsonEncode(res)});
  }
}
