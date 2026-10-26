import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mini_render/mini_render.dart';

import 'diag_upload.dart';
import 'fake_map.dart';
import 'host_apis.dart';
import 'host_video.dart';
import 'native_sheet.dart';
import 'store/mini_store.dart' show StoreApp;
import 'store/store_drawer.dart';
import 'store/store_home.dart';
import 'store/store_links.dart';
import 'store/store_panel.dart';
import 'store/store_sheets.dart';

/// 例子：从 Flutter 打开小程序。
///
/// - 「内置示例」是 `assets/demo` 里的小程序（首次运行时拷到应用私有目录）；
/// - 外部目录里有 `sample-app` / `news-app` 时（`adb push sample/sample-app
///   /sdcard/Android/data/dev.minirender.mini_render_example/files/`）也列出来；
/// - 下面一段「小程序」是服务器上的（tools/mini-store，`--dart-define=MINI_STORE=http://host:port` 换地址），
///   点开先下载解包，见 lib/store/。
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
void main() {
  // 「上传诊断日志」用：debugPrint 的每一行留一份（lib/diag_upload.dart）
  DiagLog.install();
  runApp(const ExampleApp());
}

class ExampleApp extends StatelessWidget {
  const ExampleApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
        title: '灵犀',
        theme: ThemeData(colorSchemeSeed: const Color(0xFF07C160), useMaterial3: true),
        // 外部链接 / navigateToMiniProgram 要在任何页面上都能推出小程序页（store/store_links.dart）
        navigatorKey: StoreLinks.navigatorKey,
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
    // 外部 App / H5 拉起时带的链接（minirender://open?appid=…）
    StoreLinks.init();
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

  /// 胶囊菜单「重新进入小程序」（本地的）：先收掉这一页，等实例销毁完再开一个新的（单实例）
  Future<void> _relaunch(String dir) async {
    Navigator.of(context).popUntil((r) => r.isFirst);
    await MiniAppPage.waitAllClosed();
    if (mounted) _open(dir);
  }

  void _open(String dir, {String? dumpTo, String? route}) {
    Navigator.of(context)
        .push(PageRouteBuilder<void>(
          pageBuilder: (_, _, _) =>
              MiniAppPage(appDir: dir, dumpTo: dumpTo, route: route, onRelaunch: () => _relaunch(dir)),
          transitionsBuilder: (_, a, _, child) => SlideTransition(
            position: Tween(begin: const Offset(1, 0), end: Offset.zero)
                .animate(CurvedAnimation(parent: a, curve: Curves.easeOutCubic)),
            child: child,
          ),
          // 退出不做动画：理由见 store/store_pages.dart 的 MiniStoreLaunchPage.route
          reverseTransitionDuration: Duration.zero,
        ));
  }

  static const _pageBg = Color(0xFFF2F3F5);

  // 高度定死 88：tools/android-jank.sh 点的 (500,400) 要落在第一行里（模拟器 420dpi 上标题栏到 y=210，
  // 72 高的话这一行到 399 就完了，正好点空）
  Widget _localTile(String name, String dir) => ListTile(
        minTileHeight: 88,
        leading: Container(
          width: 40,
          height: 40,
          decoration: BoxDecoration(color: const Color(0xFF07C160), borderRadius: BorderRadius.circular(10)),
          child: const Icon(Icons.widgets_outlined, color: Colors.white, size: 22),
        ),
        title: Text(name, style: const TextStyle(fontWeight: FontWeight.w500)),
        subtitle: Text(name == '内置示例' ? 'APK 里的演示小程序 · 组件 / 宿主能力 / 性能页' : dir,
            maxLines: 1, overflow: TextOverflow.ellipsis),
        onTap: () => _open(dir),
        trailing: const Icon(Icons.chevron_right, color: Colors.black26),
      );

  @override
  Widget build(BuildContext context) {
    // 外层 Scaffold 不带 AppBar：提示条（SnackBar）挂在它上面，面板展开、首页沉到底时也看得见
    return Scaffold(
      backgroundColor: const Color(0xFF1C1C25),
      // 微信式下拉面板：首页滚到顶再往下拉（或点标题栏的宫格图标），露出「最近使用」「我的小程序」
      body: PullDownDrawer(
        panel: const MiniProgramPanel(),
        builder: (context, scroll, physics) => Material(
          color: _pageBg,
          child: Column(children: [
            AppBar(
              backgroundColor: Colors.white,
              surfaceTintColor: Colors.white,
              scrolledUnderElevation: 0,
              // 标题栏高度不变（仍是 kToolbarHeight）：下面「内置示例」那一行的位置跟以前一样
              title: const Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Text('灵犀', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
                Text('小程序引擎演示 · 下拉打开小程序面板', style: TextStyle(fontSize: 11, color: Colors.black38)),
              ]),
              actions: [
                Builder(
                  builder: (context) => IconButton(
                    tooltip: '小程序面板',
                    icon: const Icon(Icons.apps_rounded),
                    onPressed: () => PullDownDrawer.of(context)?.open(),
                  ),
                ),
                // 真机出问题、手边没 USB：日志传到 mini-store，开发机上 tools/mini-store/diag.sh 拉回来
                IconButton(
                  tooltip: '上传诊断日志',
                  icon: const Icon(Icons.bug_report_outlined),
                  onPressed: () => DiagUpload.ask(context),
                ),
              ],
            ),
            Expanded(
              // 本地的在最上面、紧贴标题栏：tools/android-jank.sh 等脚本点 (500,400) 打开「内置示例」，别把它挤下去
              child: ListView(controller: scroll, physics: physics, padding: const EdgeInsets.only(bottom: 24), children: [
                Material(
                  color: Colors.white,
                  child: Column(children: [
                    if (_status.isNotEmpty) ListTile(title: Text(_status)),
                    for (final (name, dir) in _apps) _localTile(name, dir),
                  ]),
                ),
                const SizedBox(height: 12),
                // 服务器上的小程序（tools/mini-store），点开即下载
                const StoreSection(),
                // 与原生示例入口页同一行（右上角的虫子图标是同一个功能，不显眼，这里再放一行）
                HomeCard(
                  padding: EdgeInsets.zero,
                  child: ListTile(
                    leading: const Icon(Icons.bug_report_outlined),
                    title: const Text('上传诊断日志'),
                    subtitle: const Text('把运行日志、机型与版本传到服务器，交给开发分析'),
                    onTap: () => DiagUpload.ask(context),
                  ),
                ),
              ]),
            ),
          ]),
        ),
      ),
    );
  }
}

/// 一个小程序页：导航栏、返回键、原生组件、宿主能力都接上
class MiniAppPage extends StatefulWidget {
  const MiniAppPage({
    super.key,
    required this.appDir,
    this.dumpTo,
    this.route,
    this.launchOptions,
    this.onReady,
    this.storeApp,
    this.onRelaunch,
  });

  final String appDir;
  final String? dumpTo;
  final String? route;

  /// 商店里的小程序（胶囊菜单的转发 / 收藏要用）；本地的（内置示例）为空
  final StoreApp? storeApp;

  /// 胶囊菜单「重新进入小程序」：由打开它的那一页重建（单实例，得先关后开）
  final VoidCallback? onRelaunch;

  /// 启动参数（外部链接 / 别的小程序打开时）：`App.onLaunch(options)`、`wx.getLaunchOptionsSync()` 读它
  final Map<String, Object?>? launchOptions;

  /// 还没销毁的小程序页个数。一个进程同一时刻只能有一个小程序实例：换小程序（深链、
  /// navigateToMiniProgram）时，新的要等旧的 dispose 完再建（[waitAllClosed]）
  static final live = ValueNotifier<int>(0);

  static Future<void> waitAllClosed() async {
    while (live.value > 0) {
      await Future<void>.delayed(const Duration(milliseconds: 30));
    }
  }

  /// 第一个页面出来了（引擎的第一条 `page` 事件）：商店的 loading 页据此收起
  final VoidCallback? onReady;

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
    launchOptions: widget.launchOptions,
    backend: _backend,
    // 内置示例里 media / perf 页有 <video>，宿主 tab 有 <map>。
    // video 在 Android 上走同层渲染（插件自带的 MediaPlayer 解码、引擎把帧画进页面，滑动时贴着页面走），
    // 不经过下面的 nativeViewBuilder；iOS 上仍由 HostVideo 盖在画面上
    nativeComponents: const ['video', 'map'],
    // map 也走同层渲染（Android）：下面 nativeViewBuilder 造的假地图离屏画成帧、引擎画进页面 ——
    // 滑动时贴着页面走。盖在画面上的话一滑就错开、一顿一顿（两条上屏路，见 doc/滑动顿挫避坑清单.md）
    // （对照用：`flutter build apk --dart-define=SAME_LAYER_MAP=false` 编一份盖在画面上的）
    sameLayerViews: const bool.fromEnvironment('SAME_LAYER_MAP', defaultValue: true) ? const ['map'] : const [],
    // 小程序侧的演示见内置示例的「宿主」tab。
    // - 选图 / 拍照 / 选视频 / 选文件 / 存相册 / 打开文档：插件自带（builtinHostApis 默认开），
    //   走系统自己的选择器，这里不用写；
    // - 其余「要宿主才能做」的（登录、支付、定位、扫码、通讯录……）：host_apis.dart，演示数据 +
    //   要用户点头的弹 Flutter 自己的界面。真实 App 在那里接自己的账号 / 支付 / 地图 SDK。
    // 注意：别点名 setClipboardData —— 点名会**替换**引擎自带的实现（弹「内容已复制」、
    // getClipboardData 读回），系统剪贴板由插件听 `clipboard` 事件写。
    hostApis: ExampleHostApis(() => mounted ? context : null).all,
  );

  @override
  void initState() {
    super.initState();
    MiniAppPage.live.value++;
    _c.logs.listen((l) => debugPrint('[小程序] $l'));
    // 例子里把引擎事件打进日志（logcat 里 grep「[事件]」），排查与自动化验收都靠它
    _c.events.listen((e) {
      // 滚动中的 nativeViews 每帧一条，不打（主线程上每帧多一行日志，Flutter 的帧就更容易晚，
      // 视频框跟页面就更容易错开）；停下来那条（速度 0）照打
      final scrolling = e['type'] == 'nativeViews' && (e['velocity'] as num? ?? 0) != 0;
      if (e['type'] != 'log' && !scrolling) debugPrint('[事件] ${e['type']} ${_brief(e)}');
      // 剪贴板（clipboard）与触感（vibrate）插件自己处理，这里只打日志
    });
    _c.activeBackend.addListener(() => debugPrint('[后端] 偏好 ${_backend.name} → ${_c.activeBackend.value}'));
    if (widget.dumpTo != null) _scheduleDump(widget.dumpTo!);
    if (widget.onReady != null) {
      void onPage() {
        if (_c.page.value == null) return;
        _c.page.removeListener(onPage);
        widget.onReady!();
      }

      _c.page.addListener(onPage);
    }
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

  /// 胶囊的「•••」：SDK 只留了口子（`onMenu`），展示什么由宿主定。示例照微信弹底部菜单
  /// （store/store_sheets.dart）：顶上小程序名（点了看关于）、转发 / 收藏、重新进入 / 诊断日志 / 关于。
  /// 「⊙」不用接：`onExit` 不给时默认关掉整个小程序页面，回到打开小程序的那一页
  Future<void> _showMenu() async {
    if (!mounted) return;
    final a = _c.about.value;
    final app = widget.storeApp;
    final store = StoreLinks.store;
    final fav = app != null && await store.isFavorite(app.id);
    if (!mounted) return;
    final name = app?.name ?? (a?.name.isNotEmpty == true ? a!.name : '内置示例');
    final top = [
      CapsuleMenuItem(Icons.share_outlined, '转发给朋友', () => copyMiniLink(context, app!, route: _c.page.value?.route),
          enabled: app != null, id: 'share', glyph: '↗'),
      CapsuleMenuItem(fav ? Icons.star_rounded : Icons.star_outline_rounded, fav ? '取消收藏' : '添加到\n我的小程序', () async {
        final now = await store.toggleFavorite(app!);
        _toast(now ? '已添加到「我的小程序」，首页下拉可见' : '已从「我的小程序」移除');
      }, enabled: app != null, active: fav, id: 'favorite', glyph: fav ? '★' : '☆'),
    ];
    final bottom = [
      CapsuleMenuItem(Icons.refresh_rounded, '重新进入\n小程序', () => widget.onRelaunch?.call(),
          enabled: widget.onRelaunch != null, id: 'relaunch', glyph: '↻'),
      CapsuleMenuItem(Icons.bug_report_outlined, '上传\n诊断日志', _uploadDiag, id: 'diag', glyph: '⚑'),
      CapsuleMenuItem(Icons.info_outline_rounded, '关于', _showAbout, id: 'about', glyph: 'ⓘ'),
    ];
    final iconUrl = app == null ? '' : store.iconUrlOf(app);
    final subtitle = app == null ? '本地小程序' : 'v${app.version}';
    // Android：原生窗口画（盖在平台视图上的 Flutter 弹层每帧 22~42ms，见 native_sheet.dart）
    if (NativeSheet.available) {
      try {
        final picked = await NativeSheet.capsule(
            id: app?.id ?? 'demo', name: name, iconUrl: iconUrl, subtitle: subtitle, top: top, bottom: bottom);
        if (!mounted || picked == null) return;
        if (picked == 'header') return _showAbout();
        [...top, ...bottom].where((it) => it.id == picked).firstOrNull?.onTap();
        return;
      } on MissingPluginException {
        // 通道不在（宿主没接）：退回 Flutter 画的那份
      }
    }
    if (!mounted) return;
    await showCapsuleMenu(context,
        id: app?.id ?? 'demo', name: name, iconUrl: iconUrl, subtitle: subtitle, onHeader: _showAbout, top: top, bottom: bottom);
  }

  /// 小程序页上的提示：Android 用系统 Toast（理由同上），别的平台用 SnackBar
  void _toast(String text) {
    if (NativeSheet.available) {
      NativeSheet.toast(text).catchError((_) {});
      return;
    }
    ScaffoldMessenger.maybeOf(context)
      ?..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(behavior: SnackBarBehavior.floating, duration: const Duration(milliseconds: 1600), content: Text(text)));
  }

  void _uploadDiag() {
    final a = _c.about.value;
    final b = _c.activeBackend.value;
    DiagUpload.ask(context, backend: b == null ? null : '$b', app: {
      'dir': widget.appDir,
      'appId': a?.appId ?? StoreLinks.currentAppId,
      'name': a?.name,
      'route': _c.page.value?.route,
    });
  }

  /// 关于框（信息来自引擎的 `app` 事件，见 `controller.about`）
  Future<void> _showAbout() async {
    if (!mounted) return;
    final a = _c.about.value;
    if (NativeSheet.available) {
      try {
        final i = await NativeSheet.alert(a?.dialogTitle ?? '关于小程序', a?.dialogMessage ?? '小程序还在加载', const ['上传诊断日志', '确定']);
        if (i == 0 && mounted) _uploadDiag();
        return;
      } on MissingPluginException {
        // 退回 Flutter 的对话框
      }
    }
    if (!mounted) return;
    showDialog<void>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(a?.dialogTitle ?? '关于小程序'),
        content: Text(a?.dialogMessage ?? '小程序还在加载'),
        actions: [
          // 出问题时就在小程序里：胶囊「•••」直接能传日志
          TextButton(
            onPressed: () {
              Navigator.pop(ctx);
              _uploadDiag();
            },
            child: const Text('上传诊断日志'),
          ),
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('确定')),
        ],
      ),
    );
  }

  @override
  void dispose() {
    _c.dispose();
    MiniAppPage.live.value--;
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MiniProgramScaffold(
      controller: _c,
      onMenu: _showMenu,
      nativeViewBuilder: (context, view, c) {
        switch (view.tag) {
          // 只有 iOS（或 sameLayerVideo: false）走到这里：video_player 出画面（系统硬解），
          // 事件按微信 <video> 的形状回传，见 host_video.dart。Android 默认同层渲染，用不到它
          case 'video':
            return HostVideo(view: view, controller: c);
          // 真实 App 在这里放 google_maps_flutter / 高德；示例画一张网格和一个标记
          case 'map':
            return FakeMap(view: view, controller: c);
        }
        return null;
      },
    );
  }
}
