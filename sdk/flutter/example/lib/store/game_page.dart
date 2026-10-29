// 商店里的小游戏页：对应原生示例的 GameActivity —— 全屏沉浸（藏状态栏、导航栏，边缘上滑临时呼出），
// 包的 game.json 方向由插件锁（退出时还原）；wx.exitMiniProgram 关掉这一页，wx.restartMiniProgram 换一个新实例。
// 右上角胶囊由插件的原生视图画（与 wx.getMenuButtonBoundingClientRect 同一个位置，横竖屏都避开刘海 / 横条）：
// 「⊙」退出本页，「•••」弹与小程序同一个微信式底部菜单（转发 / 重新进入 / 关于）。返回键 / 返回手势也退出。
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mini_render/mini_render.dart';

import 'mini_store.dart';
import 'store_sheets.dart';

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
      case 'menuButton':
        if (e['action'] == 'close') {
          Navigator.of(context).maybePop();
        } else {
          _showMenu();
        }
      default:
        debugPrint('[小游戏] ${widget.app.id} $type $e');
    }
  }

  /// 胶囊「•••」：与小程序页（main.dart 的 _showMenu）同一个底部菜单
  void _showMenu() {
    final app = widget.app;
    void about() => showDialog<void>(
          context: context,
          builder: (ctx) => AlertDialog(
            title: Text(app.name),
            content: Text('小游戏 · ${app.key}\nv${app.version}，由 mini-render 引擎运行'),
            actions: [TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('确定'))],
          ),
        );
    showCapsuleMenu(context, id: app.id, name: app.name, subtitle: '小游戏 · v${app.version}', onHeader: about, top: [
      CapsuleMenuItem(Icons.share_outlined, '转发给朋友', () => copyMiniLink(context, app), glyph: '↗'),
    ], bottom: [
      CapsuleMenuItem(Icons.refresh, '重新进入\n小游戏', () => setState(() => _generation++), glyph: '↻'),
      CapsuleMenuItem(Icons.info_outline, '关于', about, glyph: 'ⓘ'),
    ]);
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
          // 跑不满 60 时稳在 30（帧间隔均匀，比 40fps 上下跳更顺）
          'adaptiveFps': true,
        },
        onHostEvent: _onHostEvent,
        onFirstFrame: _ready,
      ),
    );
  }
}
