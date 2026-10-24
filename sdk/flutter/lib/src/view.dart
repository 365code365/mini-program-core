import 'dart:math' as math;
import 'dart:ui' show PlatformDispatcher;

import 'package:flutter/foundation.dart' show Factory, TargetPlatform, defaultTargetPlatform;
import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart' show PlatformViewHitTestBehavior;
import 'package:flutter/services.dart' show AndroidViewController, PlatformViewsService, StandardMessageCodec;
import 'package:flutter/widgets.dart';

import 'controller.dart';
import 'events.dart';
import 'same_layer.dart';
import 'native_layer.dart';
import 'text_input.dart';

/// 给宿主承载的原生组件（`video` / `map` / `web-view` …）造一个 Widget。
/// 返回 null 表示这个组件不显示（引擎那边只留一块底色）。
typedef MiniNativeViewBuilder = Widget? Function(
  BuildContext context,
  MiniNativeView view,
  MiniProgramController controller,
);

/// 显示一个小程序。尺寸由外层约束决定（必须是有限的），首次布局后才真正创建引擎。
///
/// - 画面：Android 默认画到引擎自己的 SurfaceView（平台视图，见 [MiniProgramController.androidSurfaceView]），
///   iOS 与 `androidSurfaceView: false` 走 `Texture`（CVPixelBuffer / SurfaceProducer）。
/// - 触摸只送**指针三段**、只认第一根手指，并且立刻赢下手势竞技场 —— 否则外层的
///   PageView / 侧滑返回会和小程序同时响应。
/// - 原生组件按引擎报的位置盖在画面上，并按引擎报的遮挡裁掉被 picker 面板、弹窗、
///   Toast、fixed 元素、tabBar 盖住的部分（与微信同层渲染的层叠结果一致：面板在视频上面）。
///   没被盖住的部分触摸归控件，被盖住的部分触摸归引擎。页面滚动时控件每帧按 vsync 推算位置，
///   跟着页面走（native_layer.dart）。
/// - 键盘弹起时**不改尺寸**，而是像微信的 `adjust-position` 那样把画面整体上移，
///   让聚焦的输入框露出来（放进 `Scaffold` 时请设 `resizeToAvoidBottomInset: false`，
///   否则键盘动画的每一帧都会让引擎重建一次布局）。
class MiniProgramView extends StatefulWidget {
  const MiniProgramView({
    super.key,
    required this.controller,
    this.nativeViewBuilder,
    this.backgroundColor = const Color(0xFFFFFFFF),
    this.navigationBarAreaColor,
    this.placeholder,
    this.onError,
  });

  final MiniProgramController controller;
  final MiniNativeViewBuilder? nativeViewBuilder;

  /// 画面没铺满的那一两个像素（逻辑尺寸取整）与加载前的底色
  final Color backgroundColor;

  /// Android 上视图铺到系统导航栏下面时，那一截不交给引擎画、填这个色（缺省同 [backgroundColor]）。
  /// [MiniProgramScaffold] 在 tabBar 页给 tabBar 底色、其它页给页面底色，与上面的画面连成一片
  final Color? navigationBarAreaColor;

  /// 引擎第一帧出来之前显示的东西
  final Widget? placeholder;
  final void Function(Object error)? onError;

  @override
  State<MiniProgramView> createState() => _MiniProgramViewState();
}

class _MiniProgramViewState extends State<MiniProgramView> with WidgetsBindingObserver {
  late final MiniTextInput _input = MiniTextInput(widget.controller);
  final GlobalKey _box = GlobalKey();
  int? _pointer;
  Size? _size;
  double? _dpr;
  List<double>? _insets;
  bool _syncQueued = false;
  Object? _error;

  /// 视图底边到屏幕底边的距离（键盘从屏幕底部升起，算遮挡要用）
  double _gapToScreenBottom = 0;

  /// 底部留给 Android 系统导航栏（三键 / 手势条）的那一截，不交给引擎画。
  ///
  /// 微信安卓的小程序窗口不伸到虚拟导航键下面（`windowHeight` 不含它）。Android 15 起应用一律
  /// edge-to-edge，视图铺到屏幕底边：不留的话页面最底下一截、吸底的操作栏都压在导航键下面
  /// （荣耀三键导航实测底部 38 被盖住），而样例页大多没写 `env(safe-area-inset-bottom)`。
  /// iOS 不留：Home 指示条只是一条细线，与微信 iOS 一样画面铺到底，底部安全区交给 `env()` 与 tabBar。
  double _reservedBottom = 0;

  static bool get _reservesNavigationBar => defaultTargetPlatform == TargetPlatform.android;

  MiniProgramController get _c => widget.controller;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _c.focusedInput.addListener(_onFocus);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _c.focusedInput.removeListener(_onFocus);
    _input.close();
    super.dispose();
  }

  void _onFocus() {
    final f = _c.focusedInput.value;
    if (f == null) {
      _input.close();
    } else {
      _input.open(f);
    }
    if (mounted) setState(() {}); // 键盘位移跟着输入框走
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    switch (state) {
      case AppLifecycleState.resumed:
        _c.lifecycle(0);
      case AppLifecycleState.paused:
      case AppLifecycleState.hidden:
        _c.lifecycle(1); // 进后台：派发 onHide 并停止出帧
      default:
    }
  }

  @override
  void didHaveMemoryPressure() => _c.lifecycle(2);

  /// 布局之后：第一次就建引擎，之后尺寸 / 像素比 / 安全区变了再同步
  void _queueSync() {
    if (_syncQueued) return;
    _syncQueued = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _syncQueued = false;
      if (mounted) _sync();
    });
  }

  void _sync() {
    final box = _box.currentContext?.findRenderObject() as RenderBox?;
    if (box == null || !box.hasSize) return;
    final mq = MediaQuery.of(context);
    final dpr = mq.devicePixelRatio;
    final origin = box.localToGlobal(Offset.zero);
    final screen = mq.size;
    final pad = mq.viewPadding;
    _gapToScreenBottom = math.max(0, screen.height - (origin.dy + box.size.height));
    // 视图与系统导航栏真正重叠的那一截。用 View 上未经裁剪的值：祖先去掉了 MediaQuery 的底部
    // padding、视图却照样铺到屏幕底边时，读 `pad` 会得到 0
    final viewPad = MediaQueryData.fromView(View.of(context)).viewPadding;
    final navOverlap = math.max(0.0, (origin.dy + box.size.height) - (screen.height - viewPad.bottom));
    final reserve = _reservesNavigationBar ? navOverlap : 0.0;
    if ((reserve - _reservedBottom).abs() > 0.5) setState(() => _reservedBottom = reserve);
    // 交给引擎的是去掉那一截之后的尺寸（不随换页变：切页照样不 resize）
    final size = Size(box.size.width, math.max(1.0, box.size.height - reserve));
    // 只算视图与屏幕刘海 / 底部横条真正重叠的那部分（放在 AppBar 下面时上边是 0）
    final insets = [
      math.max(0.0, pad.top - origin.dy),
      math.max(0.0, (origin.dx + size.width) - (screen.width - pad.right)),
      reserve > 0 ? 0.0 : math.max(0.0, (origin.dy + size.height) - (screen.height - pad.bottom)),
      math.max(0.0, pad.left - origin.dx),
    ];
    if (_size == null) {
      _size = size;
      _dpr = dpr;
      _insets = insets;
      final locale = PlatformDispatcher.instance.locale;
      _c
          .attach(
            logicalSize: size,
            devicePixelRatio: dpr,
            safeArea: insets,
            autoSystemInfo: {
              'language': locale.countryCode == null
                  ? locale.languageCode
                  : '${locale.languageCode}_${locale.countryCode}',
              'theme': mq.platformBrightness == Brightness.dark ? 'dark' : 'light',
              'fontSizeSetting': (16 * mq.textScaler.scale(1)).roundToDouble(),
              // 状态栏高度是设备的，不是这块视图与状态栏的重叠（放在 AppBar 下面时重叠是 0，
              // 但自定义导航栏的算式要的是真实状态栏高）。也不能用上面的 `pad`：Scaffold 有
              // AppBar 时会把 body 里 MediaQuery 的顶部 padding / viewPadding 一起去掉，那里读到 0，
              // 而引擎只在创建时收一次 —— 首页带导航栏，后面 navigationStyle: custom 的页面就全拿 0。
              // 所以从 View 直接取未经裁剪的值
              'statusBarHeight': MediaQueryData.fromView(View.of(context)).viewPadding.top,
            },
          )
          .catchError(_fail);
      return;
    }
    if (size.width.floor() != _size!.width.floor() || size.height.floor() != _size!.height.floor() || dpr != _dpr) {
      _size = size;
      _dpr = dpr;
      _c.resize(size, dpr).catchError(_fail);
    }
    if (!_sameInsets(insets, _insets!)) {
      _insets = insets;
      _c.setSafeArea(insets).catchError(_fail);
    }
  }

  static bool _sameInsets(List<double> a, List<double> b) {
    for (var i = 0; i < 4; i++) {
      if ((a[i] - b[i]).abs() > 0.5) return false;
    }
    return true;
  }

  void _fail(Object e) {
    widget.onError?.call(e);
    if (mounted) setState(() => _error = e);
  }

  /// 键盘挡住聚焦的输入框时，画面整体上移多少（微信 `adjust-position` 的行为）
  double _keyboardShift(Size size) {
    final f = _c.focusedInput.value;
    if (f == null || !f.adjustPosition) return 0;
    final kb = MediaQuery.viewInsetsOf(context).bottom;
    final overlap = math.max(0.0, kb - _gapToScreenBottom);
    if (overlap <= 0) return 0;
    const spacing = 10.0; // 输入框与键盘之间留一点（微信默认 cursor-spacing）
    final need = f.rect.bottom + f.cursorSpacing + spacing - (size.height - overlap);
    return need.clamp(0.0, overlap);
  }

  void _down(PointerDownEvent e) {
    if (_pointer != null) return; // 只认第一根手指：引擎的触摸状态机是单指的
    // 按在留给系统导航栏的那一截上（手势导航时这条收得到触摸）：不属于小程序
    if (_reservedBottom > 0 && e.localPosition.dy >= (_size?.height ?? double.infinity)) return;
    _pointer = e.pointer;
    _c.pointer(0, e.localPosition.dx, e.localPosition.dy);
  }

  void _move(PointerMoveEvent e) {
    if (e.pointer != _pointer) return;
    // 带上事件时间：安卓上 timeStamp 就是 MotionEvent.getEventTime()，与引擎 pump 的帧时间同一个钟
    _c.pointer(1, e.localPosition.dx, e.localPosition.dy, timeMs: e.timeStamp.inMicroseconds / 1000.0);
  }

  void _up(PointerUpEvent e) {
    if (e.pointer != _pointer) return;
    _pointer = null;
    _c.pointer(2, e.localPosition.dx, e.localPosition.dy);
  }

  void _cancel(PointerCancelEvent e) {
    if (e.pointer != _pointer) return;
    _pointer = null;
    _c.pointer(3, 0, 0);
  }

  void _signal(PointerSignalEvent e) {
    if (e is PointerScrollEvent) _c.wheel(e.scrollDelta.dy);
  }

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      return ColoredBox(
        color: widget.backgroundColor,
        child: Center(child: Text('小程序打不开：$_error', textDirection: TextDirection.ltr)),
      );
    }
    return LayoutBuilder(builder: (context, constraints) {
      assert(constraints.hasBoundedWidth && constraints.hasBoundedHeight, 'MiniProgramView 需要有限的宽高');
      _queueSync();
      final size = constraints.biggest;
      final shift = _keyboardShift(size);
      return ColoredBox(
        key: _box,
        color: widget.backgroundColor,
        child: ClipRect(
          child: Transform.translate(
            offset: Offset(0, -shift),
            child: Stack(
              fit: StackFit.expand,
              children: [
                RawGestureDetector(
                  behavior: HitTestBehavior.opaque,
                  gestures: <Type, GestureRecognizerFactory>{
                    // 一按下就赢下竞技场：外层的滚动 / 侧滑返回不再抢这串触摸
                    EagerGestureRecognizer: GestureRecognizerFactoryWithHandlers<EagerGestureRecognizer>(
                      EagerGestureRecognizer.new,
                      (_) {},
                    ),
                  },
                  child: Listener(
                    behavior: HitTestBehavior.opaque,
                    onPointerDown: _down,
                    onPointerMove: _move,
                    onPointerUp: _up,
                    onPointerCancel: _cancel,
                    onPointerSignal: _signal,
                    child: _frame(context),
                  ),
                ),
                if (_reservedBottom > 0)
                  Positioned(
                    left: 0,
                    right: 0,
                    bottom: 0,
                    height: _reservedBottom,
                    child: IgnorePointer(
                      child: ColoredBox(color: widget.navigationBarAreaColor ?? widget.backgroundColor),
                    ),
                  ),
                if (widget.nativeViewBuilder != null)
                  NativeViewLayer(controller: _c, builder: widget.nativeViewBuilder!),
                // 同层渲染的组件（sameLayerViews）：离屏画成帧交给引擎，这里不占位置
                if (widget.nativeViewBuilder != null)
                  SameLayerHost(controller: _c, builder: widget.nativeViewBuilder!),
              ],
            ),
          ),
        ),
      );
    });
  }

  /// 引擎画面：纹理按**物理像素**对齐（逻辑尺寸取整后 × dpr），最近邻采样 —— 一个设备像素
  /// 对一个纹理像素，文字不会因为重采样发虚
  Widget _frame(BuildContext context) {
    return ValueListenableBuilder<bool>(
      valueListenable: _c.surfaceView,
      builder: (context, onView, _) => onView ? _engineSurface(context) : _texture(context),
    );
  }

  /// Android 独立图层（[MiniProgramController.androidSurfaceView]）：引擎自己的 SurfaceView，
  /// 以 hybrid composition 嵌进来 —— 系统合成器直接取引擎交的每一帧，Flutter 这一帧出没出都不影响它。
  /// 尺寸与纹理一样按物理像素对齐；触摸不进原生视图（transparent），照旧由外面的 Listener 收
  Widget _engineSurface(BuildContext context) {
    return ValueListenableBuilder<Size?>(
      valueListenable: _c.pixelSize,
      builder: (context, px, _) {
        final dpr = MediaQuery.devicePixelRatioOf(context);
        return Align(
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: px == null ? null : px.width / dpr,
            height: px == null ? null : px.height / dpr,
            child: PlatformViewLink(
              viewType: _engineViewType,
              surfaceFactory: (context, controller) => AndroidViewSurface(
                controller: controller as AndroidViewController,
                gestureRecognizers: const <Factory<OneSequenceGestureRecognizer>>{},
                hitTestBehavior: PlatformViewHitTestBehavior.transparent,
              ),
              onCreatePlatformView: (params) => PlatformViewsService.initExpensiveAndroidView(
                id: params.id,
                viewType: _engineViewType,
                layoutDirection: TextDirection.ltr,
                creationParamsCodec: const StandardMessageCodec(),
              )
                ..addOnPlatformViewCreatedListener(params.onPlatformViewCreated)
                ..create(),
            ),
          ),
        );
      },
    );
  }

  static const _engineViewType = 'dev.minirender/engine-view';

  Widget _texture(BuildContext context) {
    return ValueListenableBuilder<int?>(
      valueListenable: _c.textureId,
      builder: (context, id, _) {
        if (id == null) return widget.placeholder ?? const SizedBox.expand();
        return ValueListenableBuilder<Size?>(
          valueListenable: _c.pixelSize,
          builder: (context, px, _) {
            final dpr = MediaQuery.devicePixelRatioOf(context);
            return Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: px == null ? null : px.width / dpr,
                height: px == null ? null : px.height / dpr,
                child: Texture(textureId: id, filterQuality: FilterQuality.none),
              ),
            );
          },
        );
      },
    );
  }
}
