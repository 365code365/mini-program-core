import 'package:flutter/material.dart';

/// 微信右上角的胶囊按钮：左半「•••」（菜单）、右半「⊙」（关闭）。
///
/// [MiniProgramScaffold] 把它放在普通页面的导航栏右侧；`navigationStyle: custom` 的页面没有
/// 宿主导航栏，胶囊由引擎画在画面里（`menuButton` 选项，见引擎的 `src/host/capsule.rs`）。
/// 两边的尺寸、配色、图标几何是同一组数：87×32、`navigationBarTextStyle: white` 时半透明黑底
/// 白图标，否则半透明白底黑图标；按着哪一半压暗哪一半，滑出去松手不算点。
class MiniCapsule extends StatelessWidget {
  const MiniCapsule({super.key, this.dark = false, this.onMenu, this.onClose});

  /// 深色导航栏（白图标）
  final bool dark;
  final VoidCallback? onMenu;
  final VoidCallback? onClose;

  static const double width = 87;
  static const double height = 32;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: width,
      height: height,
      child: _Halves(dark: dark, onMenu: onMenu, onClose: onClose),
    );
  }
}

/// 按压态要分左右两半记，所以两半各是一个手势区，共用一张画
class _Halves extends StatefulWidget {
  const _Halves({required this.dark, this.onMenu, this.onClose});

  final bool dark;
  final VoidCallback? onMenu;
  final VoidCallback? onClose;

  @override
  State<_Halves> createState() => _HalvesState();
}

/// 0 = 没按，1 = 按着左半，2 = 按着右半
class _HalvesState extends State<_Halves> {
  int _pressed = 0;

  Widget _half(int which, String label, VoidCallback? onTap) {
    return Expanded(
      child: Semantics(
        button: true,
        label: label,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapDown: (_) => setState(() => _pressed = which),
          onTapCancel: () => setState(() => _pressed = 0),
          onTapUp: (_) => setState(() => _pressed = 0),
          onTap: onTap,
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      painter: CapsulePainter(dark: widget.dark, pressed: _pressed),
      child: Row(children: [
        _half(1, '更多', widget.onMenu),
        _half(2, '关闭', widget.onClose),
      ]),
    );
  }
}

/// 胶囊的画法（与引擎 `capsule.rs` 的 `draw` 同一组几何与配色）
class CapsulePainter extends CustomPainter {
  const CapsulePainter({required this.dark, this.pressed = 0});

  final bool dark;

  /// 0 = 没按，1 = 左半，2 = 右半
  final int pressed;

  // 配色（ARGB）
  Color get _bg => dark ? const Color(0x26000000) : const Color(0x99FFFFFF);
  Color get _border => dark ? const Color(0x40FFFFFF) : const Color(0x14000000);
  Color get _divider => dark ? const Color(0x40FFFFFF) : const Color(0x33000000);
  Color get _icon => dark ? const Color(0xFFFFFFFF) : const Color(0xFF000000);
  Color get _press => dark ? const Color(0x2EFFFFFF) : const Color(0x1A000000);

  // 图标几何（逻辑像素）
  static const double _dotGap = 6.2, _dotR = 1.7, _dotRMid = 2.4;
  static const double _ringROut = 7.4, _ringStroke = 1.9, _ringDotR = 2.7;
  static const double _dividerFrac = 18 / 32;

  @override
  void paint(Canvas canvas, Size size) {
    final w = size.width, h = size.height;
    final pill = RRect.fromRectAndRadius(Offset.zero & size, Radius.circular(h / 2));
    canvas.drawRRect(pill, Paint()..color = _bg);
    if (pressed != 0) {
      canvas.save();
      canvas.clipRRect(pill);
      final half = pressed == 1 ? Rect.fromLTWH(0, 0, w / 2, h) : Rect.fromLTWH(w / 2, 0, w / 2, h);
      canvas.drawRect(half, Paint()..color = _press);
      canvas.restore();
    }
    // 描边：0.5 宽，整条画在轮廓里面
    canvas.drawRRect(
      pill.deflate(0.25),
      Paint()
        ..color = _border
        ..style = PaintingStyle.stroke
        ..strokeWidth = 0.5,
    );
    final dh = h * _dividerFrac;
    canvas.drawRect(Rect.fromLTWH(w / 2 - 0.25, (h - dh) / 2, 0.5, dh), Paint()..color = _divider);
    final icon = Paint()
      ..color = _icon
      ..isAntiAlias = true;
    final cy = h / 2, mx = w / 4;
    canvas.drawCircle(Offset(mx - _dotGap, cy), _dotR, icon);
    canvas.drawCircle(Offset(mx, cy), _dotRMid, icon);
    canvas.drawCircle(Offset(mx + _dotGap, cy), _dotR, icon);
    final cx = w * 0.75;
    canvas.drawCircle(
      Offset(cx, cy),
      _ringROut - _ringStroke / 2,
      Paint()
        ..color = _icon
        ..style = PaintingStyle.stroke
        ..strokeWidth = _ringStroke,
    );
    canvas.drawCircle(Offset(cx, cy), _ringDotR, icon);
  }

  @override
  bool shouldRepaint(CapsulePainter old) => old.dark != dark || old.pressed != pressed;
}
