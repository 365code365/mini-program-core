import 'dart:convert';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mini_render/mini_render.dart';

/// 假的原生侧：记下每一次通道调用，`create` 回一个纹理
class FakeNative {
  final List<MethodCall> calls = [];
  static const channel = MethodChannel('dev.minirender/engine');

  void install() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger.setMockMethodCallHandler(channel, (call) async {
      calls.add(call);
      switch (call.method) {
        case 'create':
          final a = call.arguments as Map;
          final dpr = (a['dpr'] as num).toDouble();
          return {
            'textureId': 7,
            'pixelWidth': ((a['width'] as int) * dpr).floor(),
            'pixelHeight': ((a['height'] as int) * dpr).floor(),
            // 与 Android 插件一致：create 返回时还没挂 surface
            'backend': 1,
            'backendReason': '未挂载 surface',
          };
        case 'back':
          return true;
        case 'paths':
          return {'files': '/f', 'cache': '/c'};
      }
      return null;
    });
  }

  Iterable<MethodCall> named(String m) => calls.where((c) => c.method == m);

  /// 原生 → Dart：纹理 / 实际后端变了
  Future<void> emitTexture(Map<String, Object?> info) async {
    final data = const StandardMethodCodec().encodeMethodCall(MethodCall('texture', info));
    await TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .handlePlatformMessage(channel.name, data, (_) {});
  }

  /// 原生 → Dart 的事件
  Future<void> emit(List<Map<String, Object?>> events) async {
    final data = const StandardMethodCodec()
        .encodeMethodCall(MethodCall('events', events.map(jsonEncode).toList()));
    await TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .handlePlatformMessage(channel.name, data, (_) {});
  }
}

Future<MiniProgramController> pumpView(
  WidgetTester t,
  FakeNative n, {
  Map<String, MiniHostApi> hostApis = const {},
  MiniBackend backend = MiniBackend.cpu,
}) async {
  final c = MiniProgramController(appDir: '/app', hostApis: hostApis, nativeComponents: const ['video'], backend: backend);
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
  await t.pump(); // 首帧之后的 post-frame 回调里建引擎
  await t.pump();
  return c;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('首次布局后按取整后的逻辑尺寸建引擎，选项带上宿主 API 与原生组件', (t) async {
    final n = FakeNative()..install();
    final c = await pumpView(t, n, hostApis: {'chooseImage': (a) async => {}});
    final create = n.named('create').single.arguments as Map;
    expect(create['width'], 393, reason: '逻辑宽要取整（引擎是整数逻辑尺寸）');
    expect(create['height'], 500);
    final opts = jsonDecode(create['options'] as String) as Map;
    expect(opts['hostApis'], ['chooseImage']);
    expect(opts['nativeComponents'], ['video']);
    expect((opts['safeArea'] as Map).keys, containsAll(['top', 'right', 'bottom', 'left']));
    expect(c.textureId.value, 7);
    expect(find.byType(Texture), findsOneWidget);
    // 纹理按物理像素对齐：宽 = pixelWidth / dpr，不是外层的 393.6
    final tex = t.getSize(find.byType(Texture));
    final dpr = t.view.devicePixelRatio;
    expect(tex.width, closeTo((393 * dpr).floor() / dpr, 1e-6));
    await c.dispose();
  });

  testWidgets('后端偏好默认 cpu', (t) async {
    final n = FakeNative()..install();
    final c = await pumpView(t, n);
    expect((n.named('create').single.arguments as Map)['backend'], MiniBackend.cpu.code);
    await c.dispose();
  });

  testWidgets('选 gpu 时 create 带 2；实际后端跟着原生侧的纹理信息走', (t) async {
    final n2 = FakeNative()..install();
    final g = await pumpView(t, n2, backend: MiniBackend.gpu);
    expect((n2.named('create').single.arguments as Map)['backend'], 2);
    expect(g.activeBackend.value, const MiniBackendStatus(gpu: false, reason: '未挂载 surface'));
    // 挂上 surface、GPU 选上了：纹理没换，只更新后端
    await n2.emitTexture({'textureId': 7, 'pixelWidth': 10, 'pixelHeight': 10, 'backend': 2, 'backendReason': '强制指定'});
    expect(g.activeBackend.value?.gpu, isTrue);
    expect(g.textureId.value, 7);
    // 运行中回退：换了一块新纹理走 CPU
    await n2.emitTexture({'textureId': 8, 'pixelWidth': 10, 'pixelHeight': 10, 'backend': 1, 'backendReason': '回退：设备丢失'});
    expect(g.activeBackend.value, const MiniBackendStatus(gpu: false, reason: '回退：设备丢失'));
    expect(g.textureId.value, 8);
    expect(MiniBackend.values.map((b) => b.code), [0, 1, 2], reason: '与 MR_BACKEND_AUTO/CPU/GPU 一致');
    await g.dispose();
  });

  testWidgets('只送指针三段、只认第一根手指、坐标是视图内逻辑坐标', (t) async {
    final n = FakeNative()..install();
    final c = await pumpView(t, n);
    final g1 = await t.startGesture(const Offset(100, 200), pointer: 1);
    await g1.moveTo(const Offset(100, 150));
    final g2 = await t.startGesture(const Offset(300, 300), pointer: 2); // 第二根手指：忽略
    await g2.moveTo(const Offset(310, 310));
    await g2.up();
    await g1.up();
    final ptr = n.named('pointer').map((c) => (c.arguments['phase'], c.arguments['x'], c.arguments['y'])).toList();
    expect(ptr.first, (0, 100.0, 200.0));
    expect(ptr.last, (2, 100.0, 150.0));
    expect(ptr.where((p) => p.$1 == 0).length, 1, reason: '第二根手指的按下不该送');
    expect(ptr.any((p) => p.$2 == 310.0), isFalse, reason: '第二根手指的移动不该送');
    expect(n.calls.where((c) => c.method == 'tap'), isEmpty, reason: '宿主不合成点击');
    await c.dispose();
  });

  testWidgets('赢下手势竞技场：外层 PageView 不会被一起拖动', (t) async {
    FakeNative().install();
    final c = MiniProgramController(appDir: '/app');
    addTearDown(c.dispose);
    final pc = PageController();
    await t.pumpWidget(MaterialApp(
      home: PageView(controller: pc, children: [
        MiniProgramView(controller: c),
        const ColoredBox(color: Colors.red),
      ]),
    ));
    await t.pump();
    await t.pump();
    await t.drag(find.byType(MiniProgramView), const Offset(-300, 0));
    await t.pumpAndSettle();
    expect(pc.page, 0, reason: '横划给了小程序，外层翻页不该动');
    await c.dispose();
  });

  testWidgets('页面事件驱动导航栏与返回键；hostCall 调宿主再回包', (t) async {
    final n = FakeNative()..install();
    var asked = <String, dynamic>{};
    final c = MiniProgramController(appDir: '/app', navigationBar: false, hostApis: {
      'chooseImage': (a) async {
        asked = a;
        return {'tempFilePaths': ['/x.jpg']};
      },
    });
    addTearDown(c.dispose);
    await t.pumpWidget(MaterialApp(home: MiniProgramScaffold(controller: c)));
    await t.pump();
    await t.pump();
    await n.emit([
      {
        'type': 'page', 'route': 'pages/a/a', 'depth': 2, 'isTabBar': false,
        'window': {'navigationBarTitleText': '标题甲', 'navigationBarBackgroundColor': '#07c160', 'navigationBarTextStyle': 'white'},
      },
      {'type': 'hostCall', 'id': 5, 'api': 'chooseImage', 'args': {'count': 2}},
    ]);
    await t.pump();
    expect(find.text('标题甲'), findsOneWidget);
    expect(find.byType(BackButton), findsOneWidget, reason: '栈深 2 要有返回键');
    expect(c.canGoBack, isTrue);
    expect(asked['count'], 2);
    final reply = n.named('hostReply').single.arguments as Map;
    expect(reply['id'], 5);
    expect(reply['ok'], true);
    expect(jsonDecode(reply['result'] as String)['tempFilePaths'], ['/x.jpg']);

    // 导航栏改标题
    await n.emit([{'type': 'navigationBar', 'title': '标题乙'}]);
    await t.pump();
    expect(find.text('标题乙'), findsOneWidget);

    // 系统返回键：栈深 > 1 时交给小程序，不关掉宿主页面
    await t.binding.handlePopRoute();
    await t.pump();
    expect(n.named('back'), isNotEmpty);
    await c.dispose();
  });

  testWidgets('原生组件按引擎报的位置摆，点在上面的触摸不进引擎', (t) async {
    final n = FakeNative()..install();
    final c = await pumpView(t, n);
    await n.emit([
      {
        'type': 'nativeViews',
        'views': [
          {'key': 'v1', 'tag': 'video', 'x': 10, 'y': 300, 'width': 200, 'height': 100, 'fixed': false, 'attrs': {'src': 'a.mp4'}},
        ],
      },
    ]);
    await t.pump();
    final r = t.getRect(find.byKey(const ValueKey('native-v1')));
    expect(r, const Rect.fromLTWH(10, 300, 200, 100));
    final before = n.named('pointer').length;
    await t.tapAt(const Offset(50, 350));
    expect(n.named('pointer').length, before, reason: '没被挡住的原生组件吃掉触摸，不该穿到页面');
    await c.dispose();
  });

  testWidgets('原生组件按引擎报的遮挡裁：被盖住的部分触摸归引擎，控件不重建', (t) async {
    final n = FakeNative()..install();
    final c = await pumpView(t, n);
    Map<String, Object?> view({required List<double> clip, List<List<double>> holes = const [], double mask = 0}) {
      Map<String, double> r(List<double> v) => {'x': v[0], 'y': v[1], 'width': v[2], 'height': v[3]};
      return {
        'type': 'nativeViews',
        'views': [
          {
            'key': 'v1', 'tag': 'video', 'x': 10, 'y': 300, 'width': 200, 'height': 100, 'fixed': false,
            'attrs': {'src': 'a.mp4'}, 'clip': r(clip), 'holes': [for (final h in holes) r(h)], 'mask': mask,
          },
        ],
      };
    }

    final finder = find.byKey(const ValueKey('native-v1'));
    Future<bool> reachesEngine(Offset p) async {
      final before = n.named('pointer').length;
      await t.tapAt(p);
      return n.named('pointer').length > before;
    }

    await n.emit([view(clip: [10, 300, 200, 100])]);
    await t.pump();
    final element = t.element(finder);
    expect(await reachesEngine(const Offset(50, 350)), isFalse, reason: '没被挡：触摸归控件');

    // picker 面板顶边在 340、遮罩 0.5：控件仍按完整尺寸摆，触摸全归引擎（点遮罩关面板）
    await n.emit([view(clip: [10, 300, 200, 40], mask: 0.5)]);
    await t.pump();
    expect(t.getRect(finder), const Rect.fromLTWH(10, 300, 200, 100), reason: '被挡住时不缩放控件');
    expect(await reachesEngine(const Offset(50, 320)), isTrue, reason: '遮罩盖着时露出的部分也不收触摸');
    expect(await reachesEngine(const Offset(50, 370)), isTrue, reason: '面板盖住的部分归引擎');

    // 只剩一个居中的盒子挖洞（没有遮罩）：洞里归引擎，洞外归控件
    await n.emit([view(clip: [10, 300, 200, 100], holes: [[60, 320, 50, 40]])]);
    await t.pump();
    expect(await reachesEngine(const Offset(80, 340)), isTrue, reason: '洞里露出的是引擎画的弹窗');
    expect(await reachesEngine(const Offset(30, 340)), isFalse);

    // 整个被挡住：控件留在树上（播放进度在里面），只是不收触摸
    await n.emit([view(clip: [10, 300, 0, 0], mask: 0.5)]);
    await t.pump();
    expect(finder, findsOneWidget);
    expect(await reachesEngine(const Offset(50, 350)), isTrue);

    expect(identical(t.element(finder), element), isTrue, reason: '遮挡变化不该让控件重建（视频会被销毁）');

    // 旧版引擎不带遮挡字段：当成整个露在外面
    final legacy = MiniNativeView.fromJson({'key': 'k', 'tag': 'video', 'x': 1, 'y': 2, 'width': 3, 'height': 4});
    expect(legacy.clip, const Rect.fromLTWH(1, 2, 3, 4));
    expect(legacy.holes, isEmpty);
    expect(legacy.mask, 0);
    await c.dispose();
  });

  test('软键盘差分：删的用 backspace、加的整串送（按码点比）', () async {
    final n = FakeNative()..install();
    final c = MiniProgramController(appDir: '/app');
    addTearDown(c.dispose);
    await c.attach(logicalSize: const Size(300, 600), devicePixelRatio: 2, safeArea: const [0, 0, 0, 0]);
    final input = MiniTextInput(c);
    input.updateEditingValue(const TextEditingValue(text: 'ab'));
    input.updateEditingValue(const TextEditingValue(text: 'a😀'));
    await Future<void>.delayed(Duration.zero);
    final seq = n.calls
        .where((m) => m.method == 'text' || m.method == 'key')
        .map((m) => m.method == 'text' ? 'text:${m.arguments['text']}' : 'key:${m.arguments['name']}')
        .toList();
    expect(seq, ['text:ab', 'key:backspace', 'text:😀']);
    await c.dispose();
  });

  List<String> keySeq(FakeNative n) => n.calls
      .where((m) => m.method == 'text' || m.method == 'key')
      .map((m) => m.method == 'text' ? 'text:${m.arguments['text']}' : 'key:${m.arguments['name']}')
      .toList();

  test('软键盘差分：中间插字、纠错改前面的词，都先把引擎光标挪到改动处', () async {
    final n = FakeNative()..install();
    final c = MiniProgramController(appDir: '/app');
    addTearDown(c.dispose);
    await c.attach(logicalSize: const Size(300, 600), devicePixelRatio: 2, safeArea: const [0, 0, 0, 0]);
    final input = MiniTextInput(c);
    input.updateEditingValue(const TextEditingValue(text: 'teh ', selection: TextSelection.collapsed(offset: 4)));
    // 输入法把 teh 纠成 the：只换中间两个字，光标仍在末尾
    input.updateEditingValue(const TextEditingValue(text: 'the ', selection: TextSelection.collapsed(offset: 4)));
    // 硬件方向键挪到 t 后面（只改选区），再打一个字
    input.updateEditingValue(const TextEditingValue(text: 'the ', selection: TextSelection.collapsed(offset: 1)));
    input.updateEditingValue(const TextEditingValue(text: 'tXhe ', selection: TextSelection.collapsed(offset: 2)));
    await Future<void>.delayed(Duration.zero);
    expect(keySeq(n), [
      'text:teh ',
      'key:left', 'key:backspace', 'key:backspace', 'text:he', 'key:doc-end',
      'key:left', 'key:left', 'key:left',
      'text:X',
    ]);
    await c.dispose();
  });

  testWidgets('textarea 默认右下角是换行键，换行不重复送；写了 send 才是确认键', (t) async {
    final n = FakeNative()..install();
    final c = MiniProgramController(appDir: '/app');
    addTearDown(c.dispose);
    await c.attach(logicalSize: const Size(300, 600), devicePixelRatio: 2, safeArea: const [0, 0, 0, 0]);
    final input = MiniTextInput(c);
    input.open(MiniInputFocus.fromJson({'id': 'ta', 'tag': 'textarea', 'value': 'ab', 'cursor': 1}));
    final cfg = t.testTextInput.log.lastWhere((m) => m.method == 'TextInput.setClient').arguments[1] as Map;
    expect(cfg['inputAction'], 'TextInputAction.newline');
    expect(t.testTextInput.editingState?['selectionBase'], 1, reason: '输入法从引擎的光标处开始编辑，不是末尾');
    // 平台已把换行写进编辑值；performAction(newline) 再送一次就是两个换行
    input.performAction(TextInputAction.newline);
    input.updateEditingValue(const TextEditingValue(text: 'a\nb', selection: TextSelection.collapsed(offset: 2)));
    await t.pump();
    expect(keySeq(n), ['text:\n']);
    input.close();

    input.open(MiniInputFocus.fromJson({'id': 'ta2', 'tag': 'textarea', 'value': '', 'confirmType': 'send'}));
    final cfg2 = t.testTextInput.log.lastWhere((m) => m.method == 'TextInput.setClient').arguments[1] as Map;
    expect(cfg2['inputAction'], 'TextInputAction.send');
    input.close();
    await c.dispose();
  });

  testWidgets('硬件退格与 Ctrl+A：本地改值、回给输入法、按同一条差分送给引擎', (t) async {
    final n = FakeNative()..install();
    final c = MiniProgramController(appDir: '/app');
    addTearDown(c.dispose);
    await c.attach(logicalSize: const Size(300, 600), devicePixelRatio: 2, safeArea: const [0, 0, 0, 0]);
    final input = MiniTextInput(c);
    input.open(MiniInputFocus.fromJson({'id': 'in', 'tag': 'input', 'value': 'abc', 'cursor': 3}));
    await t.sendKeyEvent(LogicalKeyboardKey.backspace);
    expect(t.testTextInput.editingState?['text'], 'ab');
    await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await t.sendKeyEvent(LogicalKeyboardKey.keyA);
    await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await t.pump();
    expect(keySeq(n), ['key:backspace', 'key:selectall']);
    input.close();
    await c.dispose();
  });

  testWidgets('胶囊：默认让引擎画（create 的选项带 menuButton: true）', (t) async {
    final n = FakeNative()..install();
    final c = await pumpView(t, n);
    final opts = jsonDecode((n.named('create').single.arguments as Map)['options'] as String) as Map;
    expect(opts['menuButton'], true);
    expect(MiniMenuButtonAction.parse('menu'), MiniMenuButtonAction.menu);
    expect(MiniMenuButtonAction.parse('close'), MiniMenuButtonAction.close);
    expect(MiniMenuButtonAction.parse('不认识'), isNull);
    await c.dispose();
  });

  testWidgets('胶囊：普通页面画在导航栏右侧，点左半 onMenu、右半 onExit；custom 页面导航栏不画、引擎报的照样接上', (t) async {
    final n = FakeNative()..install();
    final c = MiniProgramController(appDir: '/app', navigationBar: false);
    addTearDown(c.dispose);
    var menus = 0, exits = 0;
    await t.pumpWidget(MaterialApp(
      home: MiniProgramScaffold(controller: c, onMenu: () => menus++, onExit: () => exits++),
    ));
    await t.pump();
    await t.pump();
    await n.emit([
      {
        'type': 'page', 'route': 'pages/a/a', 'depth': 1, 'isTabBar': false,
        'window': {'navigationBarTitleText': '甲', 'navigationBarTextStyle': 'white', 'navigationBarBackgroundColor': '#07c160'},
      },
    ]);
    await t.pump();
    // 启动后第一帧的 app 事件：关于信息（「•••」展示它）
    await n.emit([
      {
        'type': 'app',
        'about': {'name': '示例', 'appId': '', 'description': '', 'entryPage': 'pages/a/a', 'pageCount': 3, 'engine': 'mini-render 0.1.0'},
      },
    ]);
    await t.pump();
    expect(c.about.value?.dialogTitle, '关于示例');
    expect(c.about.value?.dialogMessage, 'AppID：未设置\n入口：pages/a/a（共 3 页）\n引擎：mini-render 0.1.0');
    final capsule = find.byType(MiniCapsule);
    expect(capsule, findsOneWidget);
    expect(find.descendant(of: find.byType(AppBar), matching: capsule), findsOneWidget);
    expect(t.widget<MiniCapsule>(capsule).dark, isTrue, reason: '白色导航栏文字配深色胶囊');
    expect(t.getSize(capsule), const Size(87, 32));
    // 距右 7
    final appBarRight = t.getRect(find.byType(AppBar)).right;
    expect(appBarRight - t.getRect(capsule).right, closeTo(7, 0.01));
    final r = t.getRect(capsule);
    await t.tapAt(Offset(r.left + 20, r.center.dy));
    await t.pump();
    expect((menus, exits), (1, 0));
    await t.tapAt(Offset(r.right - 20, r.center.dy));
    await t.pump();
    expect((menus, exits), (1, 1));

    // custom 页面：没有 AppBar，胶囊在引擎画面里，点击由引擎事件报上来
    await n.emit([
      {'type': 'page', 'route': 'pages/b/b', 'depth': 2, 'isTabBar': false, 'window': {'navigationStyle': 'custom'}},
    ]);
    await t.pump();
    expect(find.byType(AppBar), findsNothing);
    expect(find.byType(MiniCapsule), findsNothing);
    await n.emit([
      {'type': 'menuButton', 'action': 'close'},
    ]);
    await t.pump();
    expect(exits, 2);
    await c.dispose();
  });

  testWidgets('胶囊「⊙」默认返回宿主：小程序叠了两页也是整个关掉，不是退一页', (t) async {
    final n = FakeNative()..install();
    final c = MiniProgramController(appDir: '/app');
    addTearDown(c.dispose);
    await t.pumpWidget(MaterialApp(
      home: Builder(
        builder: (ctx) => TextButton(
          onPressed: () => Navigator.of(ctx).push(MaterialPageRoute(builder: (_) => MiniProgramScaffold(controller: c))),
          child: const Text('宿主首页'),
        ),
      ),
    ));
    await t.tap(find.text('宿主首页'));
    await t.pumpAndSettle();
    await t.pump();
    await n.emit([
      {'type': 'page', 'route': 'pages/b/b', 'depth': 2, 'isTabBar': false, 'window': {'navigationStyle': 'custom'}},
    ]);
    // 先让 PopScope 按栈深 2 重建（真机上 page 事件早就到了）；同一批里一起发的话还是旧的 canPop
    await t.pump();
    await n.emit([
      {'type': 'menuButton', 'action': 'close'},
    ]);
    await t.pumpAndSettle();
    expect(find.byType(MiniProgramScaffold), findsNothing, reason: '整个小程序页面关掉了');
    expect(find.text('宿主首页'), findsOneWidget);
    expect(n.named('back'), isEmpty, reason: '不能被 PopScope 拦成小程序内的返回（真机上踩过）');
    await c.dispose();
  });

  testWidgets('导航栏默认由引擎画：不画 AppBar、选项带 navigationBar / routeAnimation，切 custom 页画面不改尺寸', (t) async {
    final n = FakeNative()..install();
    final c = MiniProgramController(appDir: '/app');
    addTearDown(c.dispose);
    await t.pumpWidget(MaterialApp(home: MiniProgramScaffold(controller: c)));
    await t.pump();
    await t.pump();
    final opts = jsonDecode((n.named('create').single.arguments as Map)['options'] as String) as Map;
    expect((opts['navigationBar'], opts['routeAnimation']), (true, true));
    final size0 = t.getSize(find.byType(MiniProgramView));
    for (final w in [<String, Object?>{'navigationBarTitleText': '甲'}, <String, Object?>{'navigationStyle': 'custom'}]) {
      await n.emit([
        {'type': 'page', 'route': 'pages/a/a', 'depth': 2, 'isTabBar': false, 'window': w},
      ]);
      await t.pump();
      await t.pump();
      expect(find.byType(AppBar), findsNothing);
      expect(t.getSize(find.byType(MiniProgramView)), size0, reason: '普通页 ↔ custom 页画面尺寸不变');
    }
    expect(n.named('resize'), isEmpty, reason: '切页不再改引擎尺寸（从前一切就 resize：进 custom 页拉伸、返回时跳）');
    await c.dispose();
  });

  testWidgets('胶囊：控制器关掉 menuButton 时两处都不画', (t) async {
    final n = FakeNative()..install();
    final c = MiniProgramController(appDir: '/app', menuButton: false);
    addTearDown(c.dispose);
    await t.pumpWidget(MaterialApp(home: MiniProgramScaffold(controller: c)));
    await t.pump();
    await t.pump();
    expect(find.byType(MiniCapsule), findsNothing);
    final opts = jsonDecode((n.named('create').single.arguments as Map)['options'] as String) as Map;
    expect(opts['menuButton'], false);
    await c.dispose();
  });

  testWidgets('滚轮转成 wheel', (t) async {
    final n = FakeNative()..install();
    final c = await pumpView(t, n);
    final p = TestPointer(1, PointerDeviceKind.mouse);
    await t.sendEventToBinding(p.hover(const Offset(100, 100)));
    await t.sendEventToBinding(p.scroll(const Offset(0, 40)));
    expect(n.named('wheel').single.arguments['dy'], 40.0);
    await c.dispose();
  });
}
