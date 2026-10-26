// 示例里的假地图（宿主承载的 <map>）：真实 App 在这里放 google_maps_flutter / 高德。
import 'package:flutter/material.dart';
import 'package:mini_render/mini_render.dart';

/// 假地图：按 latitude / longitude 属性画一张网格，中间一个标记；点标记回传 markertap {markerId}
class FakeMap extends StatelessWidget {
  const FakeMap({super.key, required this.view, required this.controller});

  final MiniNativeView view;
  final MiniProgramController controller;

  @override
  Widget build(BuildContext context) {
    final lat = double.tryParse(view.attrs['latitude'] ?? '') ?? 0;
    final lng = double.tryParse(view.attrs['longitude'] ?? '') ?? 0;
    return ClipRRect(
      borderRadius: BorderRadius.circular(6),
      child: Stack(
        children: [
          Positioned.fill(child: CustomPaint(painter: _GridPainter())),
          Center(
            child: GestureDetector(
              onTap: () => controller.sendNativeViewEvent(view.key, 'markertap', {'markerId': 1}),
              child: const Icon(Icons.location_on, color: Color(0xFFE64340), size: 40),
            ),
          ),
          Positioned(
            left: 8,
            bottom: 6,
            child: Text(
              '宿主原生 map（假地图） ${lat.toStringAsFixed(4)}, ${lng.toStringAsFixed(4)}',
              style: const TextStyle(color: Colors.black54, fontSize: 11),
            ),
          ),
        ],
      ),
    );
  }
}

class _GridPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = const Color(0xFFE8EEF5));
    final line = Paint()
      ..color = const Color(0xFFCAD6E3)
      ..strokeWidth = 1;
    for (double x = 0; x < size.width; x += 24) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), line);
    }
    for (double y = 0; y < size.height; y += 24) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), line);
    }
    final road = Paint()
      ..color = Colors.white
      ..strokeWidth = 8;
    canvas.drawLine(Offset(0, size.height * 0.62), Offset(size.width, size.height * 0.38), road);
    canvas.drawLine(Offset(size.width * 0.3, 0), Offset(size.width * 0.45, size.height), road);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
