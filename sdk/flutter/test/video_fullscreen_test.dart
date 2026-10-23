// 视频全屏（引擎的 `videoFullscreen` 事件）：插件收起 / 放出系统栏，`videoFullscreen` 跟着变。
// 横屏是引擎自己把那一层转过来画，插件不转屏（不调 setPreferredOrientations）
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mini_render/mini_render.dart';
import 'support/fake_native.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('进全屏收起系统栏、退出放出来，不转屏', (t) async {
    final platform = <MethodCall>[];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, (call) async {
      platform.add(call);
      return null;
    });
    addTearDown(() => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger.setMockMethodCallHandler(SystemChannels.platform, null));
    final n = FakeNative()..install();
    final c = await pumpView(t, n);
    Iterable<Object?> modes() => platform.where((m) => m.method == 'SystemChrome.setEnabledSystemUIMode').map((m) => m.arguments);

    await n.emit([{'type': 'videoFullscreen', 'fullScreen': true, 'direction': 90, 'key': 'v1'}]);
    expect(c.videoFullscreen.value, isTrue);
    expect(modes().last, 'SystemUiMode.immersiveSticky');
    await n.emit([{'type': 'videoFullscreen', 'fullScreen': false, 'direction': 90, 'key': 'v1'}]);
    expect(c.videoFullscreen.value, isFalse);
    expect(modes().last, 'SystemUiMode.edgeToEdge');
    expect(platform.where((m) => m.method == 'SystemChrome.setPreferredOrientations'), isEmpty, reason: '不转屏');
    await c.dispose();
  });
}
