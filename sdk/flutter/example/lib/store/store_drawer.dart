// 微信式的下拉小程序面板：首页在最上面时往下拉，整页跟着手指下沉、露出后面深色的面板；
// 松手时拉过 [_openDistance] 就展开（首页沉到底，只露一截 [_peek]），否则弹回。
// 展开后点 / 上推底下那一截、在面板里拉到底再往上推、按返回键，都收起来。
//
// 为什么不用 RefreshIndicator / 越界通知做：越界通知在手指回推时不会回来，列表会自己滚走
// （面板没收、内容先动）。这里在指针层（[Listener]）接管位移，同时让列表的 physics 在下拉时
// 吃掉位移、不起惯性（[_PullPhysics]）—— 一次拖动里面板与列表只有一个在动。
// 原生示例的 PullDrawerLayout.kt 是同一套手感与阈值。
import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// 松手时拉过这么多（逻辑像素，已经算上阻尼）就展开
const _openDistance = 72.0;

/// 展开后首页露在底下的那一截
const _peek = 64.0;

/// 下拉的阻尼：手指走 1，页面走 0.55（拉起来发沉，与微信一样）
const _damping = 0.55;

class PullDownDrawer extends StatefulWidget {
  const PullDownDrawer({super.key, required this.panel, required this.builder});

  /// 后面的面板（铺满整屏；底下 [peekHeight] 那一截会被首页盖住）
  final Widget panel;

  /// 首页：把 [controller] 与 [physics] 交给它最外层的那个列表（下拉只在它滚到顶时生效）
  final Widget Function(BuildContext context, ScrollController controller, ScrollPhysics physics) builder;

  /// 首页露在底下的高度（面板的内容要让出这么多）
  static double peekHeight(BuildContext context) => _peek + MediaQuery.paddingOf(context).bottom;

  static PullDownDrawerState? of(BuildContext context) => context.findAncestorStateOfType<PullDownDrawerState>();

  @override
  State<PullDownDrawer> createState() => PullDownDrawerState();
}

class _PullGate {
  bool pulling = false;
}

/// 下拉进行中：列表不跟手、不起惯性（位移全交给面板）
class _PullPhysics extends ClampingScrollPhysics {
  const _PullPhysics(this.gate, {super.parent});

  final _PullGate gate;

  @override
  _PullPhysics applyTo(ScrollPhysics? ancestor) => _PullPhysics(gate, parent: buildParent(ancestor));

  @override
  double applyPhysicsToUserOffset(ScrollMetrics position, double offset) =>
      gate.pulling ? 0 : super.applyPhysicsToUserOffset(position, offset);

  @override
  Simulation? createBallisticSimulation(ScrollMetrics position, double velocity) =>
      gate.pulling ? null : super.createBallisticSimulation(position, velocity);
}

class PullDownDrawerState extends State<PullDownDrawer> with SingleTickerProviderStateMixin {
  /// 0 = 收起，1 = 展开
  late final AnimationController _t = AnimationController(vsync: this, duration: const Duration(milliseconds: 300));
  final _scroll = ScrollController();
  final _gate = _PullGate();
  late final _physics = _PullPhysics(_gate);
  int? _pointer;
  double _travel = 1;

  bool get isOpen => _t.value > 0;

  void open() {
    HapticFeedback.lightImpact();
    _t.animateTo(1, curve: Curves.easeOutCubic);
  }

  void close() => _t.animateTo(0, curve: Curves.easeOutCubic);

  void _settle({double velocity = 0}) {
    // 往下甩 / 拉够了就展开；往上甩就收
    if (velocity < -400) return close();
    if (velocity > 400 || _t.value * _travel >= _openDistance && _t.value > 0) {
      if (_t.value < 1) open();
    } else {
      close();
    }
  }

  void _onDown(PointerDownEvent e) => _pointer ??= e.pointer;

  void _onMove(PointerMoveEvent e) {
    if (e.pointer != _pointer || _t.isAnimating) return;
    if (!_gate.pulling) {
      // 只在首页滚到顶、面板收着、手指往下时接管
      if (_t.value > 0 || e.delta.dy <= 0 || !_scroll.hasClients || _scroll.position.pixels > 0) return;
      _gate.pulling = true;
    }
    _t.value = (_t.value + e.delta.dy * _damping / _travel).clamp(0.0, 1.0);
    // 推回到顶：交还给列表（同一次拖动里再往上推就是普通滚动）
    if (_t.value == 0 && e.delta.dy < 0) _gate.pulling = false;
  }

  void _onUp(PointerEvent e) {
    if (e.pointer != _pointer) return;
    _pointer = null;
    if (!_gate.pulling) return;
    // 列表的拖动结束（建惯性）排在这一条指针事件之后：等它过去再放开，否则松手那一下列表会甩出去
    // setState：已经拉到底（不再有动画帧）时也要把「点一下收起」那层换上来
    scheduleMicrotask(() {
      _gate.pulling = false;
      if (mounted) setState(() {});
    });
    _settle();
  }

  /// 面板里的列表拉到底再往上推：面板跟着收（越界的位移交给面板）
  bool _onPanelScroll(ScrollNotification n) {
    if (n.depth != 0 || _t.isAnimating) return false;
    if (n is OverscrollNotification && n.dragDetails != null && n.overscroll > 0) {
      _t.value = (_t.value - n.overscroll / _travel).clamp(0.0, 1.0);
    } else if (n is ScrollEndNotification && _t.value < 1 && _t.value > 0) {
      _t.value < 0.85 ? close() : open();
    }
    return false;
  }

  @override
  void dispose() {
    _t.dispose();
    _scroll.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final peek = PullDownDrawer.peekHeight(context);
    final home = widget.builder(context, _scroll, _physics);
    return LayoutBuilder(builder: (context, box) {
      _travel = math.max(1, box.maxHeight - peek);
      return AnimatedBuilder(
        animation: _t,
        builder: (context, _) {
          final v = _t.value;
          final shown = v > 0;
          return PopScope(
            canPop: !shown,
            onPopInvokedWithResult: (didPop, _) => didPop ? null : close(),
            child: Stack(children: [
              // 面板：从上方一点淡入、略放大到原尺寸（微信下拉时面板也是这样「浮」出来的）
              Positioned.fill(
                child: Visibility(
                  visible: shown,
                  maintainState: true,
                  child: AnnotatedRegion<SystemUiOverlayStyle>(
                    value: SystemUiOverlayStyle.light.copyWith(statusBarColor: Colors.transparent),
                    child: NotificationListener<ScrollNotification>(
                      onNotification: _onPanelScroll,
                      child: Opacity(
                        opacity: Curves.easeOut.transform(math.min(1, v * 1.6)),
                        child: Transform.translate(
                          offset: Offset(0, -48 * (1 - v)),
                          child: Transform.scale(scale: 0.94 + 0.06 * v, child: widget.panel),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
              Transform.translate(
                offset: Offset(0, v * _travel),
                child: Listener(
                  onPointerDown: _onDown,
                  onPointerMove: _onMove,
                  onPointerUp: _onUp,
                  onPointerCancel: _onUp,
                  child: Stack(children: [
                    DecoratedBox(
                      position: DecorationPosition.foreground,
                      decoration: BoxDecoration(color: Colors.black.withValues(alpha: 0.25 * v)),
                      child: ClipRRect(borderRadius: BorderRadius.vertical(top: Radius.circular(16 * v)), child: home),
                    ),
                    // 展开（或正在收 / 展）时首页不接受操作：点一下收起，上推跟手收起
                    if (shown && !_gate.pulling)
                      Positioned.fill(
                        child: Semantics(
                          button: true,
                          label: '收起小程序面板',
                          child: GestureDetector(
                            behavior: HitTestBehavior.opaque,
                            onTap: close,
                            onVerticalDragUpdate: (d) => _t.value = (_t.value + d.delta.dy / _travel).clamp(0.0, 1.0),
                            onVerticalDragEnd: (d) {
                              final vy = d.velocity.pixelsPerSecond.dy;
                              vy < -300 || _t.value < 0.85 ? close() : open();
                            },
                            child: Align(
                              alignment: Alignment.topCenter,
                              child: Padding(
                                padding: const EdgeInsets.only(top: 8),
                                child: Opacity(
                                  opacity: v,
                                  child: Container(
                                    width: 36,
                                    height: 4,
                                    decoration: BoxDecoration(
                                        color: Colors.white70, borderRadius: BorderRadius.circular(2)),
                                  ),
                                ),
                              ),
                            ),
                          ),
                        ),
                      ),
                  ]),
                ),
              ),
            ]),
          );
        },
      );
    });
  }
}
