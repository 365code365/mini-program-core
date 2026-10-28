// 小游戏视图（game.json + game.js 的包）：Android 上嵌原生 SDK 的 MiniGameView（插件 GameView.kt），
// 与原生示例的 GameActivity 同一套 —— 独立 SurfaceView、引擎线程上屏、按 game.json 锁方向、前后台自动暂停。
//
// 与 [MiniProgramView] 不同：触摸不经 Flutter 转成引擎事件，而是让平台视图直接收 MotionEvent
// （MiniGameView.onTouchEvent 自己拆多指、带历史采样），所以这里用 EagerGestureRecognizer 把整块区域的指针全交出去。
//
// iOS 还没有小游戏宿主（原生 iOS SDK 也没有），这里画一行提示，不黑屏。
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

  @override
  State<MiniGameView> createState() => _MiniGameViewState();
}

class _MiniGameViewState extends State<MiniGameView> {
  MethodChannel? _ch;

  void _attach(int id) {
    final ch = MethodChannel('dev.minirender/game_$id');
    ch.setMethodCallHandler((call) async {
      if (call.method == 'firstFrame') {
        if (mounted) widget.onFirstFrame?.call();
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
    _ch?.setMethodCallHandler(null);
    widget.controller?._ch = null;
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (kIsWeb || !Platform.isAndroid) {
      return const ColoredBox(
        color: Colors.black,
        child: Center(child: Text('这个平台还不支持小游戏（目前只有 Android）', style: TextStyle(color: Colors.white70))),
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
        creationParams: {
          'dir': widget.gameDir,
          'dataDir': widget.dataDir ?? '',
          'options': jsonEncode(widget.options),
        },
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
