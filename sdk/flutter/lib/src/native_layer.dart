import 'package:flutter/foundation.dart' show listEquals;
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';
import 'controller.dart';
import 'events.dart';
import 'view.dart' show MiniNativeViewBuilder;

/// 宿主承载的原生组件那一层（[MiniProgramView] 盖在引擎画面上面）：按引擎报的位置与遮挡摆控件，
/// 页面滚动时**每一帧**按这一帧的 vsync 推算位置（[MiniNativeMotion.shiftAt]）。
///
/// 从前只在 `nativeViews` 事件到的时候挪：事件要经主线程、MethodChannel 才进 Dart，到得忽早忽晚，
/// 一帧到两条就跳两格、一条没到就停一格；Android 上引擎画面又是系统合成器直接取的，比控件早上屏。
/// 视频框于是比页面落后 1~3 帧、每帧落后几个还不一样 —— 滑动时视频抖、不跟着页面走。
///
/// 控件本身（[MiniNativeViewBuilder] 造的那个）只在事件到的时候建一次；每帧只挪外面那层
/// Positioned / 裁剪，视频播放器不会因为滚动被反复重建。
class NativeViewLayer extends StatefulWidget {
  const NativeViewLayer({super.key, required this.controller, required this.builder});

  final MiniProgramController controller;
  final MiniNativeViewBuilder builder;

  @override
  State<NativeViewLayer> createState() => _NativeViewLayerState();
}

class _NativeViewLayerState extends State<NativeViewLayer> with SingleTickerProviderStateMixin {
  late final Ticker _ticker = createTicker((_) => _tick());

  /// 这一帧控件相对事件里的位置还要往下挪多少（逻辑像素）
  final ValueNotifier<double> _dy = ValueNotifier(0);

  MiniProgramController get _c => widget.controller;

  @override
  void initState() {
    super.initState();
    _c.nativeViews.addListener(_onViews);
    _onViews();
  }

  @override
  void didUpdateWidget(NativeViewLayer old) {
    super.didUpdateWidget(old);
    if (!identical(old.controller, widget.controller)) {
      old.controller.nativeViews.removeListener(_onViews);
      widget.controller.nativeViews.addListener(_onViews);
      _onViews();
    }
  }

  @override
  void dispose() {
    _c.nativeViews.removeListener(_onViews);
    _ticker.dispose();
    _dy.dispose();
    super.dispose();
  }

  /// 新的一条 `nativeViews`：在动就逐帧推（下一帧的 tick 先于 build，用的就是这条取样），停了就按事件摆
  void _onViews() {
    final m = _c.nativeMotion;
    if (m != null && m.velocity != 0) {
      if (!_ticker.isActive) _ticker.start();
    } else {
      if (_ticker.isActive) _ticker.stop();
      _dy.value = 0;
    }
  }

  void _tick() {
    final m = _c.nativeMotion;
    if (m == null || m.velocity == 0) {
      _dy.value = 0;
      return;
    }
    final vsync = SchedulerBinding.instance.currentSystemFrameTimeStamp.inMicroseconds / 1000.0;
    // 引擎停了（停下来那条事件会把速度报成 0；这里兜住事件丢了的情况）：别一直空转
    if (vsync - m.frameTimeMs > (MiniNativeMotion.maxAheadFrames + 4) * m.periodMs) {
      _ticker.stop();
      _dy.value = 0;
      return;
    }
    _dy.value = m.shiftAt(vsync, _c.effectiveNativeViewLag);
    // `debug.mini.pace`：每帧一行 `🎞N <这一帧 vsync> <用的引擎那一帧> <滚动> <速度> <推了多少> lag=`。
    // 注意 ColorOS 上 App 日志一多会整段静默（连 🎞F 一起开时常见），只看这一行时可以临时去掉 🎞F
    if (_c.paceLog) {
      debugPrint('🎞N ${vsync.toStringAsFixed(1)} ${m.frameTimeMs.toStringAsFixed(0)} ${m.scroll.toStringAsFixed(2)} '
          '${m.velocity.toStringAsFixed(3)} ${_dy.value.toStringAsFixed(2)} lag=${_c.effectiveNativeViewLag}');
    }
  }

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<List<MiniNativeView>>(
      valueListenable: _c.nativeViews,
      builder: (context, views, _) {
        final built = <(MiniNativeView, Widget)>[
          for (final v in views)
            if (widget.builder(context, v, _c) case final Widget child) (v, child),
        ];
        final area = _c.nativeMotion?.area;
        // 内层每帧重建，用的还是这里建好的同一个控件实例：Flutter 见到同一个 Widget 就不往下重建
        return ValueListenableBuilder<double>(
          valueListenable: _dy,
          builder: (context, dy, _) => Stack(
            children: [
              for (final (v, child) in built) _place(v.shifted(dy, area), child),
            ],
          ),
        );
      },
    );
  }

  static Widget _place(MiniNativeView v, Widget child) => Positioned.fromRect(
        rect: v.rect,
        child: KeyedSubtree(key: ValueKey(v.key), child: _Occluded(view: v, child: child)),
      );
}

// ── 以下从 view.dart 原样搬来（那边要超 500 行了） ──

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
