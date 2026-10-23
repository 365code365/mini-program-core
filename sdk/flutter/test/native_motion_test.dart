// 宿主承载的原生组件跟着页面滚：每一帧按这一帧的 vsync 推算位置（native_layer.dart），
// 而不是等 nativeViews 事件到了才挪（事件晚到一帧就停一格、一帧到两条就跳两格 —— 真机上视频框抖、跟不上页面）
import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mini_render/mini_render.dart';
import 'support/fake_native.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  double nowMs() => SchedulerBinding.instance.currentSystemFrameTimeStamp.inMicroseconds / 1000.0;

  Map<String, Object?> event(double frameTime, double y, double velocity, {double scroll = 50, Map<String, Object?>? clip}) => {
        'type': 'nativeViews',
        'views': [
          {
            'key': 'v1', 'tag': 'video', 'x': 10, 'y': y, 'width': 200, 'height': 100, 'fixed': false,
            'attrs': {'src': 'a.mp4'}, if (clip != null) 'clip': clip,
          },
        ],
        'frameTime': frameTime, 'scroll': scroll, 'velocity': velocity, 'period': 10, 'dpr': 1,
        'area': {'x': 0, 'y': 100, 'width': 393, 'height': 400},
      };

  final finder = find.byKey(const ValueKey('native-v1'));

  testWidgets('滚动中每帧按 vsync 推算位置，事件不来也接着走；速度归零就按事件摆', (t) async {
    final n = FakeNative()..install();
    final c = await pumpView(t, n);
    final t0 = nowMs();
    // 引擎那一帧：视频在 y=300，页面以 0.5px/ms 往上走（每个 10ms 的 vsync 走 5px）
    await n.emit([event(t0, 300, 0.5)]);
    await t.pump(const Duration(milliseconds: 10));
    expect(t.getRect(finder).top, 295, reason: '下一帧：比事件里的位置再往上 5px');
    final element = t.element(finder);
    // 事件没来：照样往前推一格
    await t.pump(const Duration(milliseconds: 10));
    expect(t.getRect(finder).top, 290);
    expect(identical(t.element(finder), element), isTrue, reason: '逐帧挪的只是外层，控件不重建');
    // 新事件到了（引擎第二帧：y=290，同一速度）：按它推，不会因为事件晚到跳格
    await n.emit([event(t0 + 20, 290, 0.5, scroll: 60)]);
    await t.pump(const Duration(milliseconds: 10));
    expect(t.getRect(finder).top, 285);
    // 停了：速度 0 的那条一到就按事件摆
    await n.emit([event(t0 + 30, 285, 0, scroll: 65)]);
    await t.pump(const Duration(milliseconds: 10));
    expect(t.getRect(finder).top, 285);
    await t.pump(const Duration(milliseconds: 10));
    expect(t.getRect(finder).top, 285, reason: '静止不再推');
    expect(t.hasRunningAnimations, isFalse, reason: '停下来之后不再逐帧空转');
    await c.dispose();
  });

  testWidgets('宿主比引擎晚上屏的帧数按 nativeViewLag 多推；时钟对不上（隔太久）就不推', (t) async {
    final n = FakeNative()..install();
    final c = MiniProgramController(appDir: '/app', nativeComponents: const ['video'], backend: MiniBackend.cpu, nativeViewLag: 1);
    addTearDown(c.dispose);
    await t.pumpWidget(MaterialApp(
      home: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: 393.6,
          height: 500,
          child: MiniProgramView(
            controller: c,
            nativeViewBuilder: (context, v, _) => Container(key: ValueKey('native-${v.key}'), color: Colors.black),
          ),
        ),
      ),
    ));
    await t.pump();
    await t.pump();
    final t0 = nowMs();
    await n.emit([event(t0, 300, 0.5)]);
    await t.pump(const Duration(milliseconds: 10));
    expect(t.getRect(finder).top, 290, reason: '下一帧 + 晚上屏的 1 帧 = 推 2 格');
    // 引擎那边的时刻是另一个钟（差了一秒）：不推，按事件摆
    await n.emit([event(t0 - 1000, 300, 0.5)]);
    await t.pump(const Duration(milliseconds: 10));
    expect(t.getRect(finder).top, 300);
    await c.dispose();
  });

  testWidgets('推算后的裁剪：贴着控件自己的边跟着挪，贴着导航栏的边留在可见区上沿', (t) async {
    final n = FakeNative()..install();
    final c = await pumpView(t, n);
    Future<bool> reachesEngine(Offset p) async {
      final before = n.named('pointer').length;
      await t.tapAt(p);
      return n.named('pointer').length > before;
    }

    final t0 = nowMs();
    // 视频顶在 y=120，可见区从 100 开始；以 1.5px/ms 往上走：下一帧顶到 105，再下一帧 90（钻进导航栏下面）
    await n.emit([event(t0, 120, 1.5)]);
    await t.pump(const Duration(milliseconds: 10));
    expect(t.getRect(finder).top, 105);
    await t.pump(const Duration(milliseconds: 10));
    expect(t.getRect(finder).top, 90, reason: '控件按完整尺寸摆');
    expect(await reachesEngine(const Offset(50, 95)), isTrue, reason: '可见区上沿以上（导航栏）被裁掉，触摸归引擎');
    expect(await reachesEngine(const Offset(50, 150)), isFalse, reason: '露出来的部分归控件');
    await c.dispose();
  });

  test('惯性按引擎逐帧的离散衰减外推（先衰减再走），拖动按匀速', () {
    const drag = MiniNativeMotion(frameTimeMs: 0, scroll: 0, velocity: 1, periodMs: 10, dpr: 1);
    expect(drag.distance(3), 30);
    const fling = MiniNativeMotion(frameTimeMs: 0, scroll: 0, velocity: 1, periodMs: 10, dpr: 1, decay: 0.005);
    // 逐帧模拟引擎：v ← v·e^(−0.05)，s ← s + v·10
    var v = 1.0, s = 0.0;
    for (var k = 1; k <= 5; k++) {
      v *= 0.951229424500714;
      s += v * 10;
      expect(fling.distance(k.toDouble()), closeTo(s, 1e-9));
    }
    // 3 帧后滚了 27.17px，位置按设备像素截断（dpr 1）成 27：控件往上挪 27
    expect(fling.shiftAt(30, 0), -27);
  });

  test('MiniNativeView.shifted：fixed 不动；被挡住的那条边不跟着挪', () {
    const r = Rect.fromLTWH(0, 200, 100, 100);
    // 下沿被吸底栏挡在 260
    const v = MiniNativeView(key: 'k', tag: 'video', rect: r, attrs: {}, fixed: false, clip: Rect.fromLTWH(0, 200, 100, 60));
    final s = v.shifted(-20, null);
    expect(s.rect, const Rect.fromLTWH(0, 180, 100, 100));
    expect(s.clip, const Rect.fromLTRB(0, 180, 100, 260), reason: '上沿跟控件走，下沿留在吸底栏顶边');
    const f = MiniNativeView(key: 'k', tag: 'video', rect: r, attrs: {}, fixed: true);
    expect(identical(f.shifted(-20, null), f), isTrue);
  });
}
