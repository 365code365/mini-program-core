// 插件自带的宿主能力（MiniBuiltinHostApis）：hostCall 的参数怎么变成原生方法的参数、
// 原生回的文件怎么拼成微信的回包形状、取消 / 没权限怎么变成 errMsg。原生侧用假通道顶替
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mini_render/mini_render.dart';

import 'support/fake_native.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const ch = MethodChannel('dev.minirender/test-media');
  final calls = <MethodCall>[];
  Object? Function(MethodCall) reply = (_) => null;

  setUp(() {
    calls.clear();
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger.setMockMethodCallHandler(ch, (c) async {
      calls.add(c);
      return reply(c);
    });
  });

  MiniHostCall call(String api, Map<String, dynamic> args, {Map<String, String> files = const {}}) =>
      MiniHostCall(id: 1, api: api, args: args, tempDir: '/data/wxfile/demo/tmp', files: files);

  test('chooseImage：相册来源、只要压缩图；回包 tempFilePaths 与 tempFiles 一一对应', () async {
    reply = (_) => [
          {'path': 'file:///data/wxfile/demo/tmp/host_1.jpg', 'size': 10, 'width': 4, 'height': 3, 'fileType': 'image'},
          {'path': 'file:///data/wxfile/demo/tmp/host_2.jpg', 'size': 20, 'width': 4, 'height': 3, 'fileType': 'image'},
        ];
    final res = await MiniBuiltinHostApis.handle(
      call('chooseImage', {'count': 2, 'sizeType': ['compressed'], 'sourceType': ['album']}),
      channel: ch,
    );
    final m = calls.single;
    expect(m.method, 'pickMedia');
    expect(m.arguments, containsPair('dir', '/data/wxfile/demo/tmp'));
    expect(m.arguments, containsPair('kind', 'image'));
    expect(m.arguments, containsPair('source', 'album'));
    expect(m.arguments, containsPair('count', 2));
    expect(m.arguments, containsPair('compressed', true));
    expect(res['tempFilePaths'], ['file:///data/wxfile/demo/tmp/host_1.jpg', 'file:///data/wxfile/demo/tmp/host_2.jpg']);
    expect(res['tempFiles'], [
      {'path': 'file:///data/wxfile/demo/tmp/host_1.jpg', 'size': 10},
      {'path': 'file:///data/wxfile/demo/tmp/host_2.jpg', 'size': 20},
    ]);
  });

  test('chooseMedia：图和视频都要是 mix、只有相机就开相机；视频带时长和封面', () async {
    reply = (_) => [
          {'path': 'file:///t/v.mp4', 'size': 99, 'width': 640, 'height': 360, 'duration': 2.5, 'thumb': 'file:///t/v.jpg', 'fileType': 'video'},
        ];
    final res = await MiniBuiltinHostApis.handle(
      call('chooseMedia', {'count': 9, 'mediaType': ['image', 'video'], 'sourceType': ['camera'], 'maxDuration': 15, 'camera': 'front'}),
      channel: ch,
    );
    expect(calls.single.arguments, allOf(containsPair('kind', 'mix'), containsPair('source', 'camera'), containsPair('maxDuration', 15), containsPair('camera', 'front')));
    expect(res['tempFiles'], [
      {'tempFilePath': 'file:///t/v.mp4', 'size': 99, 'width': 640, 'height': 360, 'fileType': 'video', 'duration': 2.5, 'thumbTempFilePath': 'file:///t/v.jpg'},
    ]);
  });

  test('chooseMessageFile 走系统文件选择器，名字与类型原样带回', () async {
    reply = (_) => [
          {'path': 'file:///t/a.pdf', 'size': 8, 'name': '报告.pdf', 'fileType': 'file', 'time': 1700000000},
        ];
    final res = await MiniBuiltinHostApis.handle(
      call('chooseMessageFile', {'count': 3, 'type': 'file', 'extension': ['pdf']}),
      channel: ch,
    );
    expect(calls.single.method, 'pickFiles');
    expect(calls.single.arguments, allOf(containsPair('count', 3), containsPair('type', 'file'), containsPair('extension', ['pdf'])));
    expect(res['tempFiles'], [
      {'path': 'file:///t/a.pdf', 'size': 8, 'name': '报告.pdf', 'type': 'file', 'time': 1700000000},
    ]);
  });

  test('存相册用引擎给的真实路径；找不到文件、取消、没权限都是微信的 errMsg', () async {
    reply = (_) => null;
    await MiniBuiltinHostApis.handle(
      call('saveImageToPhotosAlbum', {'filePath': 'wxfile://tmp/a.jpg'}, files: {'wxfile://tmp/a.jpg': '/data/wxfile/demo/tmp/a.jpg'}),
      channel: ch,
    );
    expect(calls.single.method, 'saveToAlbum');
    expect(calls.single.arguments, {'path': '/data/wxfile/demo/tmp/a.jpg', 'kind': 'image'});

    Future<String> errOf(MiniHostCall c) async {
      try {
        await MiniBuiltinHostApis.handle(c, channel: ch);
      } on MiniHostApiError catch (e) {
        return e.errMsg;
      }
      return 'no error';
    }

    expect(await errOf(call('saveVideoToPhotosAlbum', {'filePath': 'wxfile://tmp/none.mp4'})), 'saveVideoToPhotosAlbum:fail file not found');
    reply = (_) => throw PlatformException(code: 'cancel');
    expect(await errOf(call('chooseImage', {'sourceType': ['album']})), 'chooseImage:fail cancel');
    reply = (_) => throw PlatformException(code: 'denied', message: 'camera');
    expect(await errOf(call('chooseVideo', {'sourceType': ['camera']})), 'chooseVideo:fail auth deny（camera）');
    // 原生回了空列表（用户什么都没选就返回）也是取消
    reply = (_) => <Object>[];
    expect(await errOf(call('chooseImage', {'sourceType': ['album']})), 'chooseImage:fail cancel');
  });

  testWidgets('控制器：宿主没给的内置 API 走插件，宿主给了的用宿主的；关掉 builtinHostApis 就都不点名', (t) async {
    final n = FakeNative()..install();
    final c = MiniProgramController(appDir: '/app', navigationBar: false, hostApis: {
      'chooseVideo': (a) async => {'tempFilePath': 'mine.mp4'},
    });
    addTearDown(c.dispose);
    await t.pumpWidget(MaterialApp(home: MiniProgramScaffold(controller: c)));
    await t.pump();
    await t.pump();
    final opts = jsonDecode((n.named('create').single.arguments as Map)['options'] as String) as Map;
    expect(opts['hostApis'], containsAll(MiniBuiltinHostApis.names));
    expect((opts['hostApis'] as List).where((x) => x == 'chooseVideo'), hasLength(1));

    await n.emit([
      {'type': 'hostCall', 'id': 1, 'api': 'chooseVideo', 'args': {'sourceType': ['album']}, 'tempDir': '/t'},
      // 内置的这个走插件的 pickMedia（假原生回 null = 取消）
      {'type': 'hostCall', 'id': 2, 'api': 'chooseImage', 'args': {'sourceType': ['album']}, 'tempDir': '/t'},
    ]);
    await t.pump();
    final replies = {for (final r in n.named('hostReply')) (r.arguments as Map)['id']: r.arguments as Map};
    expect(jsonDecode(replies[1]!['result'] as String), {'tempFilePath': 'mine.mp4'});
    expect(n.named('pickMedia'), hasLength(1), reason: 'chooseImage 由插件处理');
    expect(replies[2]!['ok'], false);
    expect(jsonDecode(replies[2]!['result'] as String)['errMsg'], 'chooseImage:fail cancel');
    await c.dispose();
  });

  testWidgets('关掉 builtinHostApis：插件自带的一个都不点名（回到「没点名 = 立刻 fail」）', (t) async {
    final n = FakeNative()..install();
    final off = MiniProgramController(appDir: '/app', navigationBar: false, builtinHostApis: false);
    addTearDown(off.dispose);
    await t.pumpWidget(MaterialApp(home: MiniProgramScaffold(controller: off)));
    await t.pump();
    await t.pump();
    final opts = jsonDecode((n.named('create').single.arguments as Map)['options'] as String) as Map;
    expect(opts['hostApis'], isEmpty);
  });
}
