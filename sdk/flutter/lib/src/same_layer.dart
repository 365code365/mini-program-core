import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';

import 'controller.dart';
import 'view.dart' show MiniNativeViewBuilder;

/// 点在同层渲染的组件框里（引擎的 `nativeViewTap`）
@immutable
class MiniNativeViewTap {
  const MiniNativeViewTap(this.key, this.position);

  /// 组件的键（`id`，没有就是标签名）
  final String key;

  /// 框里的本地逻辑坐标
  final Offset position;
}

/// 同层渲染的原生组件（[MiniProgramController.sameLayerViews] 里的标签，如 `map`）：
/// [MiniProgramView.nativeViewBuilder] 造的控件**离屏**画成一帧，交给引擎画进页面。
///
/// ## 为什么不盖在画面上（[NativeViewLayer] 那条路）
/// 盖上去的控件与引擎画面走两条上屏路：Android 上引擎画到自己的 SurfaceView（系统合成器直接取），控件要经
/// 事件 → 主线程 → Dart → Flutter 的下一帧，晚 2~4 个 vsync、每帧晚几个还不一样。按 vsync 推算只推得准惯性，
/// 拖动时引擎画面自己每帧挪 0 / 2.8 / 0 / 4.3px —— 一滑页面地图框就跟页面错开、一顿一顿。
/// 画进页面以后它就是页面画布里的一块像素，滚动时跟文字在同一张图里挪，这里一帧都不用重画。
///
/// ## 怎么做的
/// 每个组件一棵独立的渲染树（自己的 [BuildOwner] / [PipelineOwner]，不挂在屏幕上），控件要重画时
/// （setState、动画、属性变了）走一遍 build / layout / paint，`toImage` 读回 RGBA 交给引擎
/// （`mr_native_view_frame_rgba`），引擎只重画页面上的这一块。点击：引擎发 `nativeViewTap`（框里的本地坐标），
/// 这里在离屏树上做命中测试、派发一次按下 / 抬起，`GestureDetector.onTap` 照常触发。
///
/// 取舍：拖动从框里开始照样滚页面（与同层渲染的视频一致），只转点击；控件里的平台视图（google_maps_flutter
/// 这类）离屏画不出来 —— 那种控件别列进 `sameLayerViews`，照旧盖在画面上。
class SameLayerHost extends StatefulWidget {
  const SameLayerHost({super.key, required this.controller, required this.builder});

  final MiniProgramController controller;
  final MiniNativeViewBuilder builder;

  @override
  State<SameLayerHost> createState() => _SameLayerHostState();
}

class _SameLayerHostState extends State<SameLayerHost> {
  final Map<String, _Offscreen> _views = {};
  StreamSubscription<MiniNativeViewTap>? _taps;

  MiniProgramController get _c => widget.controller;

  @override
  void initState() {
    super.initState();
    _c.sameLayerViewList.addListener(_sync);
    _taps = _c.nativeViewTaps.listen((t) => _views[t.key]?.tap(t.position));
    // 第一次同步要等 context 能查 Theme / MediaQuery（build 之后）
    WidgetsBinding.instance.addPostFrameCallback((_) => _sync());
  }

  @override
  void dispose() {
    _c.sameLayerViewList.removeListener(_sync);
    _taps?.cancel();
    for (final v in _views.values) {
      v.dispose();
    }
    _views.clear();
    super.dispose();
  }

  void _sync() {
    if (!mounted) return;
    final view = View.of(context);
    final dpr = view.devicePixelRatio;
    final live = <String>{};
    for (final v in _c.sameLayerViewList.value) {
      final child = widget.builder(context, v, _c);
      if (child == null) continue;
      live.add(v.key);
      // 尺寸按引擎给的设备像素 box：读回来正好一个像素对一个像素，引擎只剩整行拷贝
      final size = Size(v.rect.width, v.rect.height);
      final wrapped = InheritedTheme.captureAll(
        context,
        MediaQuery(
          data: MediaQueryData(size: size, devicePixelRatio: dpr),
          child: Directionality(textDirection: Directionality.maybeOf(context) ?? TextDirection.ltr, child: child),
        ),
      );
      (_views[v.key] ??= _Offscreen(v.key, _c, view)).update(wrapped, size, dpr);
    }
    for (final k in _views.keys.where((k) => !live.contains(k)).toList()) {
      _views.remove(k)?.dispose();
    }
  }

  @override
  Widget build(BuildContext context) => const SizedBox.shrink();
}

/// 一个同层组件的离屏渲染树
class _Offscreen {
  _Offscreen(this.key, this._c, ui.FlutterView flutterView) {
    _pipeline = PipelineOwner(onNeedVisualUpdate: _schedule);
    _view = RenderView(
      view: flutterView,
      configuration: const ViewConfiguration(),
      child: _boundary,
    );
    _pipeline.rootNode = _view;
    _view.prepareInitialFrame();
    _build = BuildOwner(focusManager: _focus, onBuildScheduled: _schedule);
  }

  final String key;
  final MiniProgramController _c;
  final RenderRepaintBoundary _boundary = RenderRepaintBoundary();
  final FocusManager _focus = FocusManager();
  late final RenderView _view;
  late final PipelineOwner _pipeline;
  late final BuildOwner _build;
  RenderObjectToWidgetElement<RenderBox>? _root;
  double _dpr = 1;
  bool _scheduled = false;
  bool _capturing = false;
  bool _again = false;
  bool _disposed = false;

  /// 离屏树上的指针 id：与真实触摸错开
  static int _nextPointer = 0x7f000000;

  void update(Widget child, Size size, double dpr) {
    _dpr = dpr;
    _view.configuration = ViewConfiguration(
      logicalConstraints: BoxConstraints.tight(size),
      physicalConstraints: BoxConstraints.tight(size * dpr),
      devicePixelRatio: dpr,
    );
    _root = RenderObjectToWidgetAdapter<RenderBox>(container: _boundary, child: child).attachToRenderTree(_build, _root);
    _schedule();
  }

  /// 控件要重画（setState / 动画 / 属性变了）：这一帧末尾画一次（同一帧里多次只画一次）
  void _schedule() {
    if (_scheduled || _disposed) return;
    _scheduled = true;
    SchedulerBinding.instance.addPostFrameCallback((_) => _capture());
    SchedulerBinding.instance.ensureVisualUpdate();
  }

  Future<void> _capture() async {
    _scheduled = false;
    final root = _root;
    if (_disposed || root == null) return;
    _build.buildScope(root);
    _build.finalizeTree();
    _pipeline.flushLayout();
    _pipeline.flushCompositingBits();
    _pipeline.flushPaint();
    // 上一帧还在读回：读完再画一次（不并发读，交给引擎的顺序就是画的顺序）
    if (_capturing) {
      _again = true;
      return;
    }
    _capturing = true;
    try {
      final img = await _boundary.toImage(pixelRatio: _dpr);
      final data = await img.toByteData(format: ui.ImageByteFormat.rawRgba);
      final (w, h) = (img.width, img.height);
      img.dispose();
      if (data != null && !_disposed) await _c.pushViewFrame(key, w, h, data.buffer.asUint8List());
    } finally {
      _capturing = false;
      if (_again && !_disposed) {
        _again = false;
        _schedule();
      }
    }
  }

  /// 引擎报点在框里（本地逻辑坐标）：离屏树上命中测试，派发一次按下 / 抬起
  void tap(Offset p) {
    if (_disposed || _root == null) return;
    final result = BoxHitTestResult();
    if (!_boundary.hitTest(result, position: p)) return;
    // 手势竞技场在 GestureBinding 自己的 handleEvent 里关 / 清（真实触摸也是它排在命中链最后）
    result.add(HitTestEntry(GestureBinding.instance));
    final id = _nextPointer++;
    final t = Duration(microseconds: DateTime.now().microsecondsSinceEpoch);
    final g = GestureBinding.instance;
    g.dispatchEvent(PointerDownEvent(pointer: id, position: p, kind: PointerDeviceKind.touch, timeStamp: t), result);
    g.dispatchEvent(PointerUpEvent(pointer: id, position: p, kind: PointerDeviceKind.touch, timeStamp: t), result);
  }

  void dispose() {
    if (_disposed) return;
    _disposed = true;
    final root = _root;
    if (root != null) {
      // 卸掉整棵控件树（State.dispose 照常调）
      RenderObjectToWidgetAdapter<RenderBox>(container: _boundary).attachToRenderTree(_build, root);
      _build.buildScope(root);
      _build.finalizeTree();
    }
    _root = null;
    _focus.dispose();
    // PipelineOwner.dispose 要求先摘掉根
    _pipeline.rootNode = null;
    _pipeline.dispose();
  }
}
