import 'dart:math' as math;
import 'dart:ui' show PlatformDispatcher;

import 'package:flutter/foundation.dart' show listEquals;
import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';

import 'controller.dart';
import 'events.dart';
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
/// - 画面走 `Texture`（不是平台视图）：Android 是 SurfaceProducer，iOS 是 CVPixelBuffer。
/// - 触摸只送**指针三段**、只认第一根手指，并且立刻赢下手势竞技场 —— 否则外层的
///   PageView / 侧滑返回会和小程序同时响应。
/// - 原生组件按引擎报的位置盖在画面上，并按引擎报的遮挡裁掉被 picker 面板、弹窗、
///   Toast、fixed 元素、tabBar 盖住的部分（与微信同层渲染的层叠结果一致：面板在视频上面）。
///   没被盖住的部分触摸归控件，被盖住的部分触摸归引擎。
/// - 键盘弹起时**不改尺寸**，而是像微信的 `adjust-position` 那样把画面整体上移，
///   让聚焦的输入框露出来（放进 `Scaffold` 时请设 `resizeToAvoidBottomInset: false`，
///   否则键盘动画的每一帧都会让引擎重建一次布局）。
class MiniProgramView extends StatefulWidget {
  const MiniProgramView({
    super.key,
    required this.controller,
    this.nativeViewBuilder,
    this.backgroundColor = const Color(0xFFFFFFFF),
    this.placeholder,
    this.onError,
  });

  final MiniProgramController controller;
  final MiniNativeViewBuilder? nativeViewBuilder;

  /// 画面没铺满的那一两个像素（逻辑尺寸取整）与加载前的底色
  final Color backgroundColor;

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
    final size = box.size;
    final dpr = mq.devicePixelRatio;
    final origin = box.localToGlobal(Offset.zero);
    final screen = mq.size;
    final pad = mq.viewPadding;
    _gapToScreenBottom = math.max(0, screen.height - (origin.dy + size.height));
    // 只算视图与屏幕刘海 / 底部横条真正重叠的那部分（放在 AppBar 下面时上边是 0）
    final insets = [
      math.max(0.0, pad.top - origin.dy),
      math.max(0.0, (origin.dx + size.width) - (screen.width - pad.right)),
      math.max(0.0, (origin.dy + size.height) - (screen.height - pad.bottom)),
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
    if (f == null) return 0;
    final kb = MediaQuery.viewInsetsOf(context).bottom;
    final overlap = math.max(0.0, kb - _gapToScreenBottom);
    if (overlap <= 0) return 0;
    const spacing = 10.0; // 输入框与键盘之间留一点（微信默认 cursor-spacing）
    final need = f.rect.bottom + spacing - (size.height - overlap);
    return need.clamp(0.0, overlap);
  }

  void _down(PointerDownEvent e) {
    if (_pointer != null) return; // 只认第一根手指：引擎的触摸状态机是单指的
    _pointer = e.pointer;
    _c.pointer(0, e.localPosition.dx, e.localPosition.dy);
  }

  void _move(PointerMoveEvent e) {
    if (e.pointer != _pointer) return;
    _c.pointer(1, e.localPosition.dx, e.localPosition.dy);
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
                if (widget.nativeViewBuilder != null) _nativeViews(),
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

  Widget _nativeViews() {
    return ValueListenableBuilder<List<MiniNativeView>>(
      valueListenable: _c.nativeViews,
      builder: (context, views, _) => Stack(
        children: [
          for (final v in views)
            if (widget.nativeViewBuilder!(context, v, _c) case final Widget child)
              Positioned.fromRect(
                rect: v.rect,
                child: KeyedSubtree(key: ValueKey(v.key), child: _Occluded(view: v, child: child)),
              ),
        ],
      ),
    );
  }
}

/// 按引擎报的遮挡摆原生组件：只在 `clip` 里、`holes` 之外画和收触摸，上面叠 `mask` 的黑色。
///
/// 引擎画面是一整块不透明纹理，控件只能摆在它上面；引擎自己画的 picker 面板、弹窗、
/// fixed 元素本该盖住控件，所以由引擎算出被盖住的几何、这里裁掉 —— 裁掉的地方露出的
/// 就是引擎画面里的那几层。
///
/// 三层结构**不随遮挡变**（ClipPath → IgnorePointer → DecoratedBox 一直都在）：
/// 中间少一层，Flutter 就会把底下的控件当成新的重建，视频播放器被销毁、面板一关进度就没了。
class _Occluded extends StatelessWidget {
  const _Occluded({required this.view, required this.child});

  final MiniNativeView view;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final v = view;
    final origin = v.rect.topLeft;
    final whole = v.holes.isEmpty && _near(v.clip, v.rect);
    return ClipPath(
      clipper: _OcclusionClipper(v.clip.shift(-origin), [for (final h in v.holes) h.shift(-origin)]),
      // 没被挡就不建裁剪层；命中测试不看 clipBehavior，照样按 clipper 判
      clipBehavior: whole ? Clip.none : Clip.hardEdge,
      child: IgnorePointer(
        // 遮罩盖着时触摸归引擎（点遮罩关面板），不归视频
        ignoring: v.mask > 0,
        child: DecoratedBox(
          position: DecorationPosition.foreground,
          decoration: BoxDecoration(color: Color.fromRGBO(0, 0, 0, v.mask.clamp(0.0, 1.0))),
          child: child,
        ),
      ),
    );
  }

  static bool _near(Rect a, Rect b) =>
      (a.left - b.left).abs() < 0.01 &&
      (a.top - b.top).abs() < 0.01 &&
      (a.right - b.right).abs() < 0.01 &&
      (a.bottom - b.bottom).abs() < 0.01;
}

/// 露出的部分 = `clip` 减去 `holes`（控件本地坐标）
class _OcclusionClipper extends CustomClipper<Path> {
  _OcclusionClipper(this.clip, this.holes);

  final Rect clip;
  final List<Rect> holes;

  @override
  Path getClip(Size size) {
    final keep = Path()..addRect(clip);
    if (holes.isEmpty || clip.isEmpty) return keep;
    final cut = Path();
    for (final h in holes) {
      cut.addRect(h);
    }
    return Path.combine(PathOperation.difference, keep, cut);
  }

  @override
  bool shouldReclip(_OcclusionClipper oldClipper) =>
      oldClipper.clip != clip || !listEquals(oldClipper.holes, holes);
}
