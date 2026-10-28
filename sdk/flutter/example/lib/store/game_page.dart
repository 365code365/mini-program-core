// 商店里的小游戏页：对应原生示例的 GameActivity —— 全屏沉浸（藏状态栏、导航栏，边缘上滑临时呼出），
// 包的 game.json 方向由插件锁（退出时还原）；wx.exitMiniProgram 关掉这一页，wx.restartMiniProgram 换一个新实例。
// 与原生示例一样没有胶囊菜单：游戏全屏，返回键 / 返回手势退出。
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mini_render/mini_render.dart';

import 'mini_store.dart';

class MiniGamePage extends StatefulWidget {
  const MiniGamePage({super.key, required this.gameDir, required this.app, required this.launchOptions, this.onReady});

  final String gameDir;
  final StoreApp app;
  final Map<String, Object?> launchOptions;

  /// 游戏第一帧上屏了（loading 层可以淡出）
  final VoidCallback? onReady;

  @override
  State<MiniGamePage> createState() => _MiniGamePageState();
}

class _MiniGamePageState extends State<MiniGamePage> {
  /// restartMiniProgram：换 key 让平台视图整个重建（旧的 MiniGameView 销毁、新实例从 game.js 重新跑）
  var _generation = 0;
  bool _readySent = false;

  /// 兜底：游戏一直不出帧（脚本报错、卡在加载）也别让 loading 永远盖着
  Timer? _fallback;

  @override
  void initState() {
    super.initState();
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.immersiveSticky);
    // 等游戏第一帧上屏（MiniGameView.onFirstFrame）再撤 loading：平台视图建好时 SurfaceView 还是空的，撤早了露黑底
    _fallback = Timer(const Duration(seconds: 10), _ready);
  }

  void _ready() {
    if (_readySent || !mounted) return;
    _readySent = true;
    _fallback?.cancel();
    widget.onReady?.call();
  }

  @override
  void dispose() {
    _fallback?.cancel();
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
    super.dispose();
  }

  void _onHostEvent(String type, Map<String, dynamic> e) {
    switch (type) {
      case 'exit':
        Navigator.of(context).maybePop();
      case 'restart':
        setState(() => _generation++);
      default:
        debugPrint('[小游戏] ${widget.app.id} $type $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    return ColoredBox(
      color: Colors.black,
      child: MiniGameView(
        key: ValueKey(_generation),
        gameDir: widget.gameDir,
        options: {
          // 存档按 appId 分：用商店的真 AppID，换了解包目录也不丢档（与原生示例一致）
          'appId': widget.app.key,
          'launchOptions': widget.launchOptions,
        },
        onHostEvent: _onHostEvent,
        onFirstFrame: _ready,
      ),
    );
  }
}
