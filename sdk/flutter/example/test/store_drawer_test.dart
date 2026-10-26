// 下拉面板（lib/store/store_drawer.dart）的手势：滚到顶往下拉过阈值展开、没过弹回、
// 列表不在顶上时下拉是普通滚动、展开后点底下那一截 / 返回键收起。
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mini_render_example/store/store_drawer.dart';

void main() {
  late PullDownDrawerState drawer;

  Future<ScrollController> pump(WidgetTester t) async {
    late ScrollController sc;
    await t.pumpWidget(MaterialApp(
      home: Scaffold(
        body: PullDownDrawer(
          panel: const ColoredBox(color: Colors.black, child: Center(child: Text('面板'))),
          builder: (context, scroll, physics) {
            sc = scroll;
            return ListView(controller: scroll, physics: physics, children: [
              for (var i = 0; i < 60; i++) SizedBox(height: 60, child: Text('行$i')),
            ]);
          },
        ),
      ),
    ));
    drawer = t.state(find.byType(PullDownDrawer));
    return sc;
  }

  testWidgets('滚到顶往下拉过阈值：展开，列表不动；点底下那一截：收起', (t) async {
    final sc = await pump(t);
    await t.dragFrom(const Offset(200, 200), const Offset(0, 260));
    await t.pumpAndSettle();
    expect(drawer.isOpen, isTrue);
    expect(sc.offset, 0);
    // 首页沉到底：底下那一截在屏幕最下方
    await t.tapAt(Offset(200, t.view.physicalSize.height / t.view.devicePixelRatio - 20));
    await t.pumpAndSettle();
    expect(drawer.isOpen, isFalse);
  });

  testWidgets('拉得不够：弹回', (t) async {
    await pump(t);
    await t.dragFrom(const Offset(200, 200), const Offset(0, 60));
    await t.pumpAndSettle();
    expect(drawer.isOpen, isFalse);
  });

  testWidgets('列表不在顶上：下拉是普通滚动，不出面板', (t) async {
    final sc = await pump(t);
    sc.jumpTo(600);
    await t.pump();
    await t.dragFrom(const Offset(200, 300), const Offset(0, 200));
    await t.pumpAndSettle();
    expect(drawer.isOpen, isFalse);
    expect(sc.offset, lessThan(600));
  });

  testWidgets('展开时按返回键：收起面板而不是退出页面', (t) async {
    await pump(t);
    drawer.open();
    await t.pumpAndSettle();
    expect(drawer.isOpen, isTrue);
    await t.binding.handlePopRoute();
    await t.pumpAndSettle();
    expect(drawer.isOpen, isFalse);
    expect(find.text('行0'), findsOneWidget);
  });
}
