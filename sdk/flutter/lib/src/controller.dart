import 'dart:async';
import 'dart:convert';
import 'dart:io' show Platform;

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'events.dart';

/// 一个小程序实例。**一个进程同一时刻只能有一个**（引擎的小程序根目录、数据目录、
/// 网络回包队列都是进程级的）：在上一个 [dispose] 之前再建会报 `ALREADY_RUNNING`。
///
/// 用法：
/// ```dart
/// final c = MiniProgramController(
///   appDir: unpackedDir,
///   hostApis: {'chooseImage': (args) async => {'tempFilePaths': await pick()}},
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
    this.nativeComponents = const [],
    this.systemInfo = const {},
    this.backend = MiniBackend.cpu,
    this.menuButton = true,
    this.navigationBar = true,
    this.routeAnimation = true,
  });

  /// 解包后的小程序目录（含 `app.json`）
  final String appDir;

  /// 可写数据目录（storage / 图片缓存）。不给就用插件默认的 `filesDir/mini-render`
  final String? dataDir;

  /// 起始页（`pages/x/x?id=1`），不给就是 app.json 首页
  final String? route;

  /// 宿主实现的 `wx.*`：名字会装进小程序的 `wx`，调用转到这里
  final Map<String, MiniHostApi> hostApis;

  /// 由宿主用原生控件承载的组件标签（见 [MiniProgramView.nativeViewBuilder]）
  final List<String> nativeComponents;

  /// 合并进 `wx.getSystemInfoSync()` 的字段（`model` / `brand` …）。尺寸、像素比、
  /// 安全区、平台、语言、主题由插件自动给，这里给了会覆盖。
  final Map<String, Object?> systemInfo;

  /// 渲染后端偏好。默认 [MiniBackend.cpu]；Android 选 auto / gpu 才会试 GPU 上屏
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

  static const MethodChannel _channel = MethodChannel('dev.minirender/engine');
  static MiniProgramController? _active;
  static bool _initialized = false;

  /// 引擎画面所在的纹理（建好之前是 null；Android 换像素出口时会换一次）
  final ValueNotifier<int?> textureId = ValueNotifier(null);

  /// 引擎画面的物理像素尺寸（= 逻辑尺寸 × dpr 取整）
  final ValueNotifier<Size?> pixelSize = ValueNotifier(null);

  /// 当前页面（导航栏标题、颜色、栈深）
  final ValueNotifier<MiniPage?> page = ValueNotifier(null);

  /// 宿主承载的原生组件在视口里的位置
  final ValueNotifier<List<MiniNativeView>> nativeViews = ValueNotifier(const []);

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
      'hostApis': hostApis.keys.toList(),
      'nativeComponents': nativeComponents,
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
    if (r.containsKey('backend')) activeBackend.value = MiniBackendStatus.fromTextureInfo(r);
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
  void pointer(int phase, double x, double y) {
    if (!_created || _disposed) return;
    _channel.invokeMethod<void>('pointer', {'phase': phase, 'x': x, 'y': y});
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

  /// 宿主承载的原生组件发生了事件（派发给节点上的 `bind<name>`）
  Future<void> sendNativeViewEvent(String key, String name, [Map<String, Object?> detail = const {}]) =>
      _call('nativeViewEvent', {'key': key, 'name': name, 'detail': jsonEncode(detail)});

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
        nativeViews.value = [
          for (final v in (ev['views'] as List? ?? const []))
            if (v is Map) MiniNativeView.fromJson(v.cast<String, dynamic>()),
        ];
      case 'hostCall':
        _onHostCall(MiniHostCall.fromJson(ev));
      case 'menuButton':
        final a = MiniMenuButtonAction.parse(ev['action']);
        if (a != null) handleMenuButton(a);
      case 'app':
        final a = ev['about'];
        if (a is Map) about.value = MiniAbout.fromJson(a.cast<String, dynamic>());
      case 'log':
        if (!_logs.isClosed) _logs.add('${ev['text']}');
    }
  }

  Future<void> _onHostCall(MiniHostCall call) async {
    final api = hostApis[call.api];
    var ok = false;
    Map<String, dynamic> res;
    if (api == null) {
      res = {'errMsg': '${call.api}:fail 宿主没有实现'};
    } else {
      try {
        res = await api(call.args);
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
