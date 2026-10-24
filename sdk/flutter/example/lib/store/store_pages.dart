// 商店的界面：入口页里的「小程序」宫格（像微信下拉出来的那排圆图标）+ 点开后的 loading 页。
//
// loading 页的流程与微信一致：先盖一层「图标 + 名字 + 三个跳动的点」，后面同时下载、解包、建引擎；
// 引擎的第一条 page 事件到了才把这层淡出 —— 冷启动加载字体那几百毫秒也在 loading 底下，不露白屏。
import 'package:flutter/material.dart';

import '../main.dart' show MiniAppPage;
import 'mini_store.dart';
import 'store_links.dart';

const _iconColors = [
  Color(0xFF07C160), Color(0xFF10AEFF), Color(0xFFFA9D3B), Color(0xFFE64340),
  Color(0xFF6467F0), Color(0xFF1485EE), Color(0xFF91D300), Color(0xFF576B95),
];

/// 圆形图标：取名字的第一个字，底色按 id 算（与管理页 admin.js 同一个哈希，两边颜色一样）
class StoreAppIcon extends StatelessWidget {
  const StoreAppIcon({super.key, required this.id, required this.name, this.size = 56});

  StoreAppIcon.of(StoreApp app, {Key? key, double size = 56}) : this(key: key, id: app.id, name: app.name, size: size);

  final String id;
  final String name;
  final double size;

  static int _hash(String s) {
    var h = 0;
    for (final r in s.runes) {
      h = (h * 31 + r) & 0xFFFFFFFF;
    }
    return h;
  }

  @override
  Widget build(BuildContext context) {
    final ch = name.isEmpty ? '?' : String.fromCharCode(name.runes.first);
    return Container(
      width: size,
      height: size,
      alignment: Alignment.center,
      decoration: BoxDecoration(color: _iconColors[_hash(id) % _iconColors.length], shape: BoxShape.circle),
      child: Text(ch, style: TextStyle(color: Colors.white, fontSize: size * 0.42, fontWeight: FontWeight.w600)),
    );
  }
}

/// 入口页里的一段：服务器上的小程序，四列宫格
class StoreSection extends StatefulWidget {
  const StoreSection({super.key});

  @override
  State<StoreSection> createState() => _StoreSectionState();
}

class _StoreSectionState extends State<StoreSection> {
  final _store = StoreLinks.store;
  StoreList? _list;
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  Future<void> _refresh() async {
    setState(() => _loading = true);
    final l = await _store.list();
    if (!mounted) return;
    setState(() {
      _list = l;
      _loading = false;
    });
  }

  // 场景值 1001：发现栏小程序主入口（微信「小程序」列表点进去就是它）
  void _open(StoreApp app) => StoreLinks.open(StoreLink(key: app.key, scene: 1001), hint: app);

  @override
  Widget build(BuildContext context) {
    final l = _list;
    final sub = Theme.of(context).textTheme.bodySmall?.copyWith(color: Colors.black45);
    return Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: [
      Padding(
        padding: const EdgeInsets.fromLTRB(16, 20, 8, 4),
        child: Row(children: [
          const Expanded(child: Text('小程序', style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600))),
          if (_loading)
            const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))
          else
            IconButton(tooltip: '刷新列表', icon: const Icon(Icons.refresh), onPressed: _refresh),
        ]),
      ),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Text(
          l == null
              ? '正在连接 $kMiniStoreUrl …'
              : l.offline
                  ? '服务器连不上，显示上次的列表（${l.error}）'
                  : '来自 $kMiniStoreUrl',
          style: sub,
        ),
      ),
      if (l != null && l.apps.isEmpty)
        const Padding(padding: EdgeInsets.all(16), child: Text('服务器上还没有小程序')),
      if (l != null && l.apps.isNotEmpty)
        GridView.count(
          crossAxisCount: 4,
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(8, 12, 8, 8),
          childAspectRatio: 0.8,
          children: [
            for (final app in l.apps)
              Semantics(
                button: true,
                label: '打开小程序 ${app.name}',
                child: InkWell(
                  borderRadius: BorderRadius.circular(8),
                  onTap: () => _open(app),
                  child: Column(mainAxisAlignment: MainAxisAlignment.center, children: [
                    StoreAppIcon.of(app),
                    const SizedBox(height: 8),
                    Text(app.name, maxLines: 1, overflow: TextOverflow.ellipsis, style: const TextStyle(fontSize: 12)),
                  ]),
                ),
              ),
          ],
        ),
    ]);
  }
}

/// 点开一个小程序：loading 层盖着，后面「检查更新 → 下载 → 解包 → 建引擎」；第一页出来了再淡出
class MiniStoreLaunchPage extends StatefulWidget {
  const MiniStoreLaunchPage({super.key, required this.store, required this.link, this.hint});

  final MiniStore store;
  final StoreLink link;

  /// 列表里点的：先拿列表里的名字 / 图标画 loading，最新版信息到了再换
  final StoreApp? hint;

  /// 从下往上推出来（与微信点开小程序一致）
  static Route<void> route({required MiniStore store, required StoreLink link, StoreApp? hint}) => PageRouteBuilder<void>(
        transitionDuration: const Duration(milliseconds: 250),
        pageBuilder: (_, _, _) => MiniStoreLaunchPage(store: store, link: link, hint: hint),
        transitionsBuilder: (_, a, _, child) => SlideTransition(
          position: Tween(begin: const Offset(0, 1), end: Offset.zero)
              .animate(CurvedAnimation(parent: a, curve: Curves.easeOutCubic)),
          child: child,
        ),
      );

  @override
  State<MiniStoreLaunchPage> createState() => _LaunchState();
}

class _LaunchState extends State<MiniStoreLaunchPage> {
  StoreApp? _app;
  String? _dir;
  bool _ready = false;
  bool _coverGone = false;
  Object? _error;
  String _status = '';
  String _progress = '';

  @override
  void initState() {
    super.initState();
    _app = widget.hint;
    _install();
  }

  Future<void> _install() async {
    setState(() {
      _error = null;
      _progress = '';
    });
    // 上一个小程序页（深链 / navigateToMiniProgram 换下来的）还没销毁完就建新的，会撞上「单实例」
    await MiniAppPage.waitAllClosed();
    // 深链只带 AppID：装过的话先用本地记录的名字 / 图标画 loading，不等网络
    if (_app == null) {
      final known = await widget.store.local(widget.link.key);
      if (known != null && mounted) setState(() => _app = known);
    }
    try {
      final p = await widget.store.prepare(
        widget.link.key,
        onStatus: (s) => mounted ? setState(() => _status = s) : null,
        onProgress: (got, total) {
          if (!mounted) return;
          String mb(int b) => (b / 1048576).toStringAsFixed(1);
          setState(() => _progress = total > 0 ? '${mb(got)} / ${mb(total)} MB' : '${mb(got)} MB');
        },
      );
      final why = p.stale == null ? (p.updated ? '已更新' : '已是最新') : '连不上服务器，用本地版（${p.stale}）';
      debugPrint('[商店] 打开 ${p.app.id} ${p.app.appId} v${p.app.version} $why → ${p.dir} 起始页 ${widget.link.route ?? '首页'}');
      StoreLinks.currentAppId = p.app.appId;
      if (mounted) {
        setState(() {
          _app = p.app;
          _dir = p.dir;
        });
      }
    } catch (e) {
      debugPrint('[商店] ${widget.link.key} 打不开：$e');
      if (mounted) setState(() => _error = e);
    }
  }

  @override
  Widget build(BuildContext context) {
    final app = _app;
    return Stack(children: [
      if (_dir != null)
        MiniAppPage(
          appDir: _dir!,
          route: widget.link.route,
          launchOptions: widget.link.launchOptions,
          onReady: () => mounted ? setState(() => _ready = true) : null,
        ),
      // 淡出之后整层拿掉：留在树里的话三个点的动画每帧都让 Flutter 出一帧，小程序滑起来就多一份主线程负担
      if (!_coverGone)
        IgnorePointer(
          ignoring: _ready,
          child: AnimatedOpacity(
            opacity: _ready ? 0 : 1,
            duration: const Duration(milliseconds: 200),
            onEnd: () => _ready && mounted ? setState(() => _coverGone = true) : null,
            child: _LoadingCover(
              id: app?.id ?? widget.link.key,
              name: app?.name ?? '小程序',
              status: _status,
              progress: _progress,
              error: _error,
              onRetry: _install,
            ),
          ),
        ),
    ]);
  }
}

class _LoadingCover extends StatelessWidget {
  const _LoadingCover({
    required this.id,
    required this.name,
    required this.status,
    required this.progress,
    required this.error,
    required this.onRetry,
  });

  final String id;
  final String name;
  final String status;
  final String progress;
  final Object? error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Colors.white,
      child: SafeArea(
        child: Stack(children: [
          // 右上角：关闭（微信胶囊的「⊙」位置）
          Positioned(
            top: 6,
            right: 8,
            child: IconButton(
              tooltip: '关闭',
              icon: const Icon(Icons.close, color: Colors.black54),
              onPressed: () => Navigator.of(context).maybePop(),
            ),
          ),
          Align(
            alignment: const Alignment(0, -0.35),
            child: Column(mainAxisSize: MainAxisSize.min, children: [
              StoreAppIcon(id: id, name: name, size: 64),
              const SizedBox(height: 14),
              Text(name, style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w500)),
              const SizedBox(height: 28),
              if (error == null) ...[
                const _Dots(),
                const SizedBox(height: 12),
                Text([status, progress].where((t) => t.isNotEmpty).join('  '),
                    style: const TextStyle(fontSize: 12, color: Colors.black38)),
              ] else ...[
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 32),
                  child: Text('打不开：$error', textAlign: TextAlign.center, style: const TextStyle(color: Color(0xFFE64340))),
                ),
                const SizedBox(height: 12),
                OutlinedButton(onPressed: onRetry, child: const Text('重试')),
              ],
            ]),
          ),
        ]),
      ),
    );
  }
}

/// 微信 loading 的三个点：依次变深
class _Dots extends StatefulWidget {
  const _Dots();

  @override
  State<_Dots> createState() => _DotsState();
}

class _DotsState extends State<_Dots> with SingleTickerProviderStateMixin {
  late final AnimationController _a =
      AnimationController(vsync: this, duration: const Duration(milliseconds: 900))..repeat();

  @override
  void dispose() {
    _a.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: '加载中',
      child: AnimatedBuilder(
        animation: _a,
        builder: (_, _) => Row(mainAxisSize: MainAxisSize.min, children: [
          for (var i = 0; i < 3; i++)
            Container(
              width: 8,
              height: 8,
              margin: const EdgeInsets.symmetric(horizontal: 4),
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: ((_a.value * 3).floor() % 3 == i) ? const Color(0xFF07C160) : const Color(0xFFD8D8D8),
              ),
            ),
        ]),
      ),
    );
  }
}
