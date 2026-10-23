// 同层渲染的组件（sameLayerViews）：nativeViewBuilder 造的控件离屏画成一帧交给引擎（viewFrame），
// 不再盖在画面上；引擎报点在框里（nativeViewTap）时控件的 onTap 照常触发。原生侧用假通道顶替
import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart' show MethodCall;
import 'package:flutter_test/flutter_test.dart';
import 'package:mini_render/mini_render.dart';
import 'support/fake_native.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('map 离屏画成帧交给引擎、选项里不再算盖上去的组件；点在框里转给控件', (t) async {
    final n = FakeNative()..install();
    var taps = 0;
    final c = MiniProgramController(
      appDir: '/app',
      nativeComponents: const ['video', 'map'],
      sameLayerViews: const ['map'],
      backend: MiniBackend.cpu,
    );
    addTearDown(c.dispose);
    await t.pumpWidget(MaterialApp(
      home: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 300,
          height: 400,
          child: MiniProgramView(
            controller: c,
            nativeViewBuilder: (context, v, _) => v.tag == 'map'
                ? GestureDetector(
                    onTap: () => taps++,
                    // 上半红、下半蓝：看得出读回来没颠倒
                    child: const Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
                      Expanded(child: ColoredBox(color: Color(0xFFFF0000))),
                      Expanded(child: ColoredBox(color: Color(0xFF0000FF))),
                    ]),
                  )
                : null,
          ),
        ),
      ),
    ));
    await t.pump();
    await t.pump();
    final opts = n.named('create').single.arguments['options'] as String;
    expect(opts, contains('"sameLayer":["video","map"]'));
    expect(opts, contains('"nativeComponents":[]'), reason: 'video 与 map 都走同层渲染，没有要盖上去的');

    // 引擎报页面上有一个地图：40×20 逻辑像素
    await n.emit([
      {
        'type': 'sameLayerViews',
        'views': [
          {'key': 'map1', 'tag': 'map', 'attrs': {'latitude': '22.5'}, 'width': 40, 'height': 20, 'box': [120, 60], 'fixed': false},
        ],
      },
    ]);
    // 离屏画、toImage 读回是异步的（要真的让出事件循环）
    MethodCall? frame;
    for (var i = 0; i < 20 && frame == null; i++) {
      await t.runAsync(() => Future<void>.delayed(const Duration(milliseconds: 20)));
      await t.pump();
      frame = n.named('viewFrame').firstOrNull;
    }
    expect(frame, isNotNull, reason: '要把画好的一帧交给引擎');
    final a = frame!.arguments as Map;
    expect(a['key'], 'map1');
    final (w, h) = (a['width'] as int, a['height'] as int);
    final dpr = t.view.devicePixelRatio;
    expect(w, closeTo(40 * dpr, 1), reason: '按设备像素画（一个像素对一个像素）');
    expect(h, closeTo(20 * dpr, 1));
    final px = a['data'] as Uint8List;
    expect(px.length, w * h * 4);
    List<int> at(int x, int y) => px.sublist((y * w + x) * 4, (y * w + x) * 4 + 4);
    expect(at(w ~/ 2, 1), [255, 0, 0, 255], reason: '上半是红的');
    expect(at(w ~/ 2, h - 2), [0, 0, 255, 255], reason: '下半是蓝的');
    // 盖在画面上的那一层里没有它（离屏树不在屏幕的控件树里）
    expect(find.byWidgetPredicate((w) => w is ColoredBox && w.color == const Color(0xFFFF0000)), findsNothing);

    // 引擎报点在框里：控件的 onTap 照常触发
    await n.emit([
      {'type': 'nativeViewTap', 'key': 'map1', 'tag': 'map', 'x': 20, 'y': 10},
    ]);
    await t.pump();
    expect(taps, 1);
  }, variant: TargetPlatformVariant.only(TargetPlatform.android));
}
