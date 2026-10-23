// 测试共用的假原生侧（从 mini_render_test.dart 原样搬出，native_motion_test.dart 也要用）
import 'dart:convert';
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
