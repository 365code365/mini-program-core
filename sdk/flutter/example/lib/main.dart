import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mini_render/mini_render.dart';

import 'host_video.dart';

/// 例子：从 Flutter 打开小程序。
///
/// - 「内置示例」是 `assets/demo` 里的小程序（首次运行时拷到应用私有目录）；
/// - 外部目录里有 `sample-app` / `news-app` 时（`adb push sample/sample-app
///   /sdcard/Android/data/dev.minirender.mini_render_example/files/`）也列出来。
///
/// `--dart-define=MINI_AUTODUMP=1`：打开内置示例 3 秒后把引擎当前帧存成 PNG（外部目录），
/// 并在日志里打出纹理的屏幕位置 —— 与 `adb exec-out screencap` 逐像素对，确认纹理没被拉伸。
///
/// `--dart-define=MINI_ROUTE=pages/media/media`：启动后直接打开内置示例的这一页（真机上验某一页
/// 不用一路点进去）。
///
/// `--dart-define=MINI_BACKEND=auto|cpu|gpu`：渲染后端偏好（默认 cpu）。实际后端打在
/// 日志的「[后端]」行里；GPU 上屏时存帧取的是同一组图层的 CPU 合成，与屏幕逐字节对得上
/// 才说明 GPU 输出没偏。
void main() => runApp(const ExampleApp());

class ExampleApp extends StatelessWidget {
  const ExampleApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
        title: 'mini_render 示例',
        theme: ThemeData(colorSchemeSeed: const Color(0xFF07C160), useMaterial3: true),
        home: const Launcher(),
      );
}

class Launcher extends StatefulWidget {
  const Launcher({super.key});

  @override
  State<Launcher> createState() => _LauncherState();
}

class _LauncherState extends State<Launcher> {
  final List<(String, String)> _apps = [];
  String _status = '准备中…';
  // bool.fromEnvironment 只认 "true"；文档里写的是 =1，两种都认
  static const _autoDumpRaw = String.fromEnvironment('MINI_AUTODUMP');
  static const _autoDump = _autoDumpRaw == '1' || _autoDumpRaw == 'true';
  static const _startRoute = String.fromEnvironment('MINI_ROUTE');

  @override
  void initState() {
    super.initState();
    _prepare();
  }

  Future<void> _prepare() async {
    final paths = await MiniProgramController.paths();
    final demo = '${paths['files']}/demo';
    await _unpackDemo(demo);
    final apps = <(String, String)>[('内置示例', demo)];
    final ext = paths['external'] ?? '';
    for (final name in ['sample-app', 'news-app', 'tea-app']) {
      if (ext.isNotEmpty && File('$ext/$name/app.json').existsSync()) apps.add((name, '$ext/$name'));
    }
    setState(() {
      _apps
        ..clear()
        ..addAll(apps);
      _status = '';
    });
    // 对图用静态的详情页（首页有定时器与旋转动画，屏幕截图与存帧之间差几十毫秒就对不上）
    if (_autoDump && mounted) {
      _open(demo, dumpTo: ext.isNotEmpty ? ext : paths['files']!, route: 'pages/detail/detail?from=dump');
    } else if (_startRoute.isNotEmpty && mounted) {
      _open(demo, route: _startRoute);
    }
  }

  /// 把 `assets/demo/**` 拷成一个普通目录（引擎按目录读小程序）
  static Future<void> _unpackDemo(String dir) async {
    final manifest = await AssetManifest.loadFromAssetBundle(rootBundle);
    for (final key in manifest.listAssets().where((k) => k.startsWith('assets/demo/'))) {
      final data = await rootBundle.load(key);
      final out = File('$dir/${key.substring('assets/demo/'.length)}');
      await out.parent.create(recursive: true);
      await out.writeAsBytes(data.buffer.asUint8List(data.offsetInBytes, data.lengthInBytes));
    }
  }

  void _open(String dir, {String? dumpTo, String? route}) {
    Navigator.of(context)
        .push(MaterialPageRoute(builder: (_) => MiniAppPage(appDir: dir, dumpTo: dumpTo, route: route)));
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('mini_render 示例')),
      body: ListView(children: [
        if (_status.isNotEmpty) ListTile(title: Text(_status)),
        for (final (name, dir) in _apps)
          ListTile(title: Text(name), subtitle: Text(dir), onTap: () => _open(dir), trailing: const Icon(Icons.chevron_right)),
      ]),
    );
  }
}

/// 一个小程序页：导航栏、返回键、原生组件、宿主能力都接上
class MiniAppPage extends StatefulWidget {
  const MiniAppPage({super.key, required this.appDir, this.dumpTo, this.route});

  final String appDir;
  final String? dumpTo;
  final String? route;

  @override
  State<MiniAppPage> createState() => _MiniPageState();
}

class _MiniPageState extends State<MiniAppPage> {
  // 缺省 auto：有 GPU 就用 GPU（`--dart-define=MINI_BACKEND=cpu` 强制软件渲染）
  static const _backendName = String.fromEnvironment('MINI_BACKEND', defaultValue: 'auto');
  static final _backend =
      MiniBackend.values.firstWhere((b) => b.name == _backendName, orElse: () => MiniBackend.auto);

  late final MiniProgramController _c = MiniProgramController(
    appDir: widget.appDir,
    route: widget.route,
    backend: _backend,
    // 内置示例里 media / perf 页有 <video>，宿主 tab 有 <map>
    nativeComponents: const ['video', 'map'],
    // 小程序侧的演示见内置示例的「宿主」tab。真实 App 在这里接 image_picker / 定位 / 支付 SDK；
    // 示例给假数据，requestPayment 与 makePhoneCall 弹一个 Flutter 对话框，表明确实走到了宿主。
    // 注意：别在这里点名 setClipboardData —— 点名会**替换**引擎自带的实现（弹「内容已复制」、
    // getClipboardData 读回），剪贴板改由下面监听 `clipboard` 事件来写系统剪贴板。
    hostApis: {
      'chooseImage': (args) async {
        final n = (args['count'] as num?)?.toInt() ?? 1;
        return {
          'tempFilePaths': [for (var i = 1; i <= n; i++) '/fake/photo_$i.jpg'],
          'tempFiles': [for (var i = 1; i <= n; i++) {'path': '/fake/photo_$i.jpg', 'size': 102400 * i}],
        };
      },
      'login': (args) async => {'code': 'demo_${DateTime.now().millisecondsSinceEpoch.toRadixString(36)}'},
      'getUserProfile': (args) async => {
            'userInfo': {'nickName': 'Flutter 用户', 'avatarUrl': '', 'gender': 0, 'language': 'zh_CN'},
          },
      'getLocation': (args) async => {
            'latitude': 22.5431,
            'longitude': 114.0579,
            'speed': 0,
            'accuracy': 30,
            'type': '${args['type'] ?? 'wgs84'}',
          },
      'scanCode': (args) async => {'result': 'https://example.com/qr/demo', 'scanType': 'QR_CODE', 'charSet': 'UTF-8'},
      'requestPayment': (args) async {
        final ok = await _confirm('模拟支付', '小程序请求支付 ¥${((args['total'] as num? ?? 0) / 100).toStringAsFixed(2)}', '支付');
        if (!ok) throw const MiniHostApiError('requestPayment:fail cancel');
        return {};
      },
      'makePhoneCall': (args) async {
        final ok = await _confirm('拨打电话', '${args['phoneNumber'] ?? ''}', '呼叫');
        if (!ok) throw const MiniHostApiError('makePhoneCall:fail cancel');
        return {};
      },
    },
  );

  /// 宿主自己的对话框（宿主能力的「真实感」：小程序调 API → Flutter UI → 结果回小程序）
  Future<bool> _confirm(String title, String content, String okText) async {
    if (!mounted) return false;
    final r = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(title),
        content: Text(content),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: Text(okText)),
        ],
      ),
    );
    return r ?? false;
  }

  @override
  void initState() {
    super.initState();
    _c.logs.listen((l) => debugPrint('[小程序] $l'));
    // 例子里把引擎事件打进日志（logcat 里 grep「[事件]」），排查与自动化验收都靠它
    _c.events.listen((e) {
      if (e['type'] != 'log') debugPrint('[事件] ${e['type']} ${_brief(e)}');
      switch (e['type']) {
        // wx.setClipboardData：引擎自己存一份并弹 Toast，宿主负责写系统剪贴板
        case 'clipboard':
          Clipboard.setData(ClipboardData(text: '${e['data'] ?? ''}'));
        // wx.vibrateShort / vibrateLong
        case 'vibrate':
          if (e['duration'] == 'long') {
            HapticFeedback.vibrate();
          } else {
            switch (e['style']) {
              case 'heavy':
                HapticFeedback.heavyImpact();
              case 'light':
                HapticFeedback.lightImpact();
              default:
                HapticFeedback.mediumImpact();
            }
          }
      }
    });
    _c.activeBackend.addListener(() => debugPrint('[后端] 偏好 ${_backend.name} → ${_c.activeBackend.value}'));
    if (widget.dumpTo != null) _scheduleDump(widget.dumpTo!);
  }

  static String _brief(Map<String, dynamic> e) {
    switch (e['type']) {
      case 'page':
        return '${e['route']} depth=${e['depth']} title=${(e['window'] as Map?)?['navigationBarTitleText']}';
      case 'inputFocus':
        return 'id=${e['id']} type=${e['inputType']} maxlength=${e['maxlength']} rect=${e['rect']}';
      case 'nativeViews':
        return [for (final v in (e['views'] as List)) '${v['key']}@${v['x']},${v['y']} ${v['width']}x${v['height']}'].join(' ');
      default:
        final m = Map.of(e)..remove('type');
        return '$m';
    }
  }

  /// 第一条 page 事件之后再等 2 秒（引擎建实例要加载字体，冷启动可能好几秒）
  void _scheduleDump(String dir) {
    void onPage() {
      if (_c.page.value == null) return;
      _c.page.removeListener(onPage);
      Future<void>.delayed(const Duration(seconds: 2), () => _dump(dir));
    }

    _c.page.addListener(onPage);
  }

  Future<void> _dump(String dir) async {
    {
      if (!mounted) return;
      // 画面在 Scaffold body 里（AppBar 下面）：先把对图要的几何量取出来，再去存图
      final dpr = MediaQuery.devicePixelRatioOf(context);
      final top = kToolbarHeight + MediaQuery.paddingOf(context).top;
      final ok = await _c.debugDumpFrame('$dir/engine_frame.png');
      final gpu = _c.activeBackend.value?.gpu ?? false;
      debugPrint('MINI_DUMP ok=$ok dpr=$dpr bodyTop=$top pixel=${_c.pixelSize.value} backend=${gpu ? 'gpu' : 'cpu'}');
    }
  }

  /// 胶囊的「•••」：查看关于。SDK 只留了口子（`onMenu`），展示什么由宿主定；
  /// 示例弹一个关于框（信息来自引擎的 `app` 事件，见 `controller.about`）。
  /// 「⊙」不用接：`onExit` 不给时默认关掉整个小程序页面，回到打开小程序的那一页
  void _showAbout() {
    if (!mounted) return;
    final a = _c.about.value;
    showDialog<void>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(a?.dialogTitle ?? '关于小程序'),
        content: Text(a?.dialogMessage ?? '小程序还在加载'),
        actions: [TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('确定'))],
      ),
    );
  }

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MiniProgramScaffold(
      controller: _c,
      onMenu: _showAbout,
      nativeViewBuilder: (context, view, c) {
        switch (view.tag) {
          // video_player 出画面（系统硬解），事件按微信 <video> 的形状回传，见 host_video.dart
          case 'video':
            return HostVideo(view: view, controller: c);
          // 真实 App 在这里放 google_maps_flutter / 高德；示例画一张网格和一个标记
          case 'map':
            return _FakeMap(view: view, controller: c);
        }
        return null;
      },
    );
  }
}

/// 假地图：按 latitude / longitude 属性画一张网格，中间一个标记；点标记回传 markertap {markerId}
class _FakeMap extends StatelessWidget {
  const _FakeMap({required this.view, required this.controller});

  final MiniNativeView view;
  final MiniProgramController controller;

  @override
  Widget build(BuildContext context) {
    final lat = double.tryParse(view.attrs['latitude'] ?? '') ?? 0;
    final lng = double.tryParse(view.attrs['longitude'] ?? '') ?? 0;
    return ClipRRect(
      borderRadius: BorderRadius.circular(6),
      child: Stack(children: [
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
          child: Text('宿主原生 map（假地图） ${lat.toStringAsFixed(4)}, ${lng.toStringAsFixed(4)}',
              style: const TextStyle(color: Colors.black54, fontSize: 11)),
        ),
      ]),
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
