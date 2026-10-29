// 小游戏视图（game.json + game.js 的包）：Android 上嵌原生 SDK 的 MiniGameView（插件 GameView.kt），
// 与原生示例的 GameActivity 同一套 —— 独立 SurfaceView、引擎线程上屏、按 game.json 锁方向、前后台自动暂停。
//
// 与 [MiniProgramView] 不同：触摸不经 Flutter 转成引擎事件，而是让平台视图直接收 MotionEvent
// （MiniGameView.onTouchEvent 自己拆多指、带历史采样），所以这里用 EagerGestureRecognizer 把整块区域的指针全交出去。
//
// iOS 上嵌原生 iOS SDK 的 MiniGameView（插件 GameViewFactory.swift）：引擎线程上屏，WebGL 主画布经 ANGLE 直接 swap 进
// CAMetalLayer。转屏要由 Dart 做（Flutter 的控制器只认 SystemChrome 设的方向）：原生读 game.json / wx.setDeviceOrientation
// 之后发 `orientation`，这里 setPreferredOrientations，退出时还原成 App 的默认（空列表 = Info.plist 里的那几种）。
// 其它平台画一行提示，不黑屏。
import 'dart:convert';
import 'dart:io' show Platform;

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';

/// 小游戏发给宿主的事件：`exit`（wx.exitMiniProgram）、`restart`（wx.restartMiniProgram）、
/// `hostCall`（创建选项 hostApis 里点名的 API，要 [MiniGameController.hostReply] 回）等，[json] 是原始事件
typedef MiniGameHostEvent = void Function(String type, Map<String, dynamic> json);

/// 往运行中的小游戏发东西：回 hostCall、推事件、诊断 eval。视图建好之前调用的会被丢掉
class MiniGameController {
  MethodChannel? _ch;

  bool get attached => _ch != null;

  Future<void> hostReply(int id, bool ok, Map<String, dynamic> result) async =>
      _ch?.invokeMethod('hostReply', {'id': id, 'ok': ok, 'json': jsonEncode(result)});

  Future<void> dispatchEvent(String name, Map<String, dynamic> detail) async =>
      _ch?.invokeMethod('dispatchEvent', {'name': name, 'json': jsonEncode(detail)});

  Future<String?> eval(String js) async => _ch?.invokeMethod<String>('eval', {'js': js});
}

class MiniGameView extends StatefulWidget {
  const MiniGameView({
    super.key,
    required this.gameDir,
    this.dataDir,
    this.options = const {},
    this.controller,
    this.onHostEvent,
    this.onFirstFrame,
  });

  /// 解好的小游戏包目录（根上有 game.json 与 game.js）
  final String gameDir;

  /// 存档 / 缓存目录；不给用应用私有目录下的 game-data（与原生示例同一处）
  final String? dataDir;

  /// mr_game_create 的选项：appId（存档按它分）、launchOptions、systemInfo、hostApis、rotateInEngine …
  final Map<String, Object?> options;

  final MiniGameController? controller;
  final MiniGameHostEvent? onHostEvent;

  /// 游戏第一帧真正上屏了（只回调一次）。建实例 + 跑 game.js + 第一帧真机上要 0.5s 上下，
  /// 这之前平台视图里是空的 SurfaceView（黑底）：loading 层要等这个回调再撤，别在视图建好时就撤
  final VoidCallback? onFirstFrame;

  static const viewType = 'dev.minirender/game-view';

  /// 下载 / 解包完就在后台把包里的大脚本编成字节码（第一次打开也不用解析）。立刻返回；马上打开也没关系，
  /// 实例碰到正在编的脚本会等它编完。[dataDir] 与之后建视图给的同一个（不给用默认的 game-data）
  static Future<void> precompile(String gameDir, {String? dataDir}) async {
    if (kIsWeb || !(Platform.isAndroid || Platform.isIOS)) return;
    await const MethodChannel('dev.minirender/engine').invokeMethod<void>('precompileGame', {'dir': gameDir, 'dataDir': dataDir ?? ''});
  }

  @override
  State<MiniGameView> createState() => _MiniGameViewState();
}

class _MiniGameViewState extends State<MiniGameView> {
  MethodChannel? _ch;

  /// iOS：游戏要的方向（game.json / wx.setDeviceOrientation）→ SystemChrome
  bool _oriented = false;

  void _orient(Object? value) {
    if (!Platform.isIOS || !mounted) return;
    _oriented = true;
    SystemChrome.setPreferredOrientations(switch ('$value') {
      'landscape' => const [DeviceOrientation.landscapeLeft, DeviceOrientation.landscapeRight],
      // 官方：landscapeLeft = home 键在左；Flutter 引擎在 iOS 上把 DeviceOrientation.landscapeLeft 直接映射成
      // UIInterfaceOrientationMaskLandscapeLeft（也是 home 键在左）
      'landscapeLeft' => const [DeviceOrientation.landscapeLeft],
      'landscapeRight' => const [DeviceOrientation.landscapeRight],
      _ => const [DeviceOrientation.portraitUp],
    });
  }

  void _attach(int id) {
    final ch = MethodChannel('dev.minirender/game_$id');
    ch.setMethodCallHandler((call) async {
      if (call.method == 'firstFrame') {
        if (mounted) widget.onFirstFrame?.call();
        return;
      }
      if (call.method == 'orientation') {
        _orient((call.arguments as Map?)?['value']);
        return;
      }
      if (call.method != 'event') return;
      final a = (call.arguments as Map).cast<String, Object?>();
      Map<String, dynamic> j;
      try {
        j = (jsonDecode('${a['json']}') as Map).cast<String, dynamic>();
      } catch (_) {
        j = {};
      }
      widget.onHostEvent?.call('${a['type']}', j);
    });
    _ch = ch;
    widget.controller?._ch = ch;
    // 初始方向原生建视图时就从 game.json 读好了，那时通道还没接上：问一次
    if (Platform.isIOS) ch.invokeMethod<String>('orientation').then(_orient, onError: (_) {});
  }

  @override
  void didUpdateWidget(MiniGameView old) {
    super.didUpdateWidget(old);
    if (old.controller != widget.controller) {
      old.controller?._ch = null;
      widget.controller?._ch = _ch;
    }
  }

  @override
  void dispose() {
    // iOS：让原生马上销毁实例（Android 的 PlatformView.dispose 由 Flutter 同步调，不用这一句）
    if (Platform.isIOS) _ch?.invokeMethod<void>('dispose').catchError((_) {});
    _ch?.setMethodCallHandler(null);
    widget.controller?._ch = null;
    // 还原成 App 的默认方向（Android 由 GameView.kt 还原 Activity 的 requestedOrientation）
    if (_oriented) SystemChrome.setPreferredOrientations(const []);
    super.dispose();
  }

  Map<String, Object?> get _creationParams => {
        'dir': widget.gameDir,
        'dataDir': widget.dataDir ?? '',
        'options': jsonEncode(widget.options),
      };

  @override
  Widget build(BuildContext context) {
    if (!kIsWeb && Platform.isIOS) {
      return UiKitView(
        viewType: MiniGameView.viewType,
        creationParams: _creationParams,
        creationParamsCodec: const StandardMessageCodec(),
        // 指针一按下就整个交给原生视图（同 Android）：不等 Flutter 的手势竞技场裁决
        gestureRecognizers: <Factory<OneSequenceGestureRecognizer>>{
          Factory<OneSequenceGestureRecognizer>(() => EagerGestureRecognizer()),
        },
        onPlatformViewCreated: (id) {
          if (mounted) _attach(id);
        },
      );
    }
    if (kIsWeb || !Platform.isAndroid) {
      // 外面多半没有 Material：不给 DefaultTextStyle 的话 Text 带调试用的黄色双下划线
      return const ColoredBox(
        color: Colors.black,
        child: Center(
          child: DefaultTextStyle(
            style: TextStyle(color: Colors.white70, fontSize: 14, decoration: TextDecoration.none),
            child: Text('这个平台还不支持小游戏（目前有 Android 与 iOS）'),
          ),
        ),
      );
    }
    return PlatformViewLink(
      viewType: MiniGameView.viewType,
      surfaceFactory: (context, controller) => AndroidViewSurface(
        controller: controller as AndroidViewController,
        // 指针一按下就整个交给原生视图：不进 Flutter 的手势竞技场等裁决（那要等到抬手或移动够远，游戏的按下就晚了）
        gestureRecognizers: <Factory<OneSequenceGestureRecognizer>>{
          Factory<OneSequenceGestureRecognizer>(() => EagerGestureRecognizer()),
        },
        hitTestBehavior: PlatformViewHitTestBehavior.opaque,
      ),
      onCreatePlatformView: (params) => PlatformViewsService.initExpensiveAndroidView(
        id: params.id,
        viewType: MiniGameView.viewType,
        layoutDirection: TextDirection.ltr,
        creationParams: _creationParams,
        creationParamsCodec: const StandardMessageCodec(),
      )
        ..addOnPlatformViewCreatedListener((id) {
          params.onPlatformViewCreated(id);
          if (mounted) _attach(id);
        })
        ..create(),
    );
  }
}
