// 从外面打开小程序：外部 App / H5 的 URL Scheme，以及小程序里的 wx.navigateToMiniProgram。
//
// 链接的形状仿微信的 `weixin://dl/business/?appid=&path=&query=`：
//     minirender://open?appid=wx3b7d9f1a2c4e6b80&path=pages/detail/detail&query=id%3D3
// 也认 mini-store 的 H5 落地页地址（`http(s)://<商店>/open?appid=…`、`/a/<AppID>?path=…`），
// 二维码 / 短信里放网页地址、App 内扫到也能直接打开。
//
// 原生那一侧（Android MainActivity 的 intent-filter、iOS SceneDelegate 的 openURLContexts）
// 只负责把链接字符串经 `mini_render_example/links` 通道交过来：冷启动用 `initial` 取，运行中推 `link`。
import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mini_render/mini_render.dart' show MiniProgramController;

import 'mini_store.dart';
import '../main.dart' show MiniAppPage;
import 'store_pages.dart';

/// 解析好的一次「打开小程序」
class StoreLink {
  const StoreLink({required this.key, this.path = '', this.query = const {}, this.scene = 1065, this.referrer});

  /// AppID（或 id）
  final String key;

  /// 页面路径（不带前导 `/`）；空 = 首页
  final String path;
  final Map<String, String> query;

  /// 微信的场景值：1065 URL Scheme（H5 / 外部 App）、1037 小程序打开小程序
  final int scene;

  /// 从别的小程序跳来时：`{appId, extraData}`
  final Map<String, Object?>? referrer;

  /// 引擎的起始页（`pages/x/x?a=1`）；空 = 首页
  String? get route {
    if (path.isEmpty) return null;
    return query.isEmpty ? path : '$path?${Uri(queryParameters: query).query}';
  }

  /// 交给引擎的 launchOptions（`wx.getLaunchOptionsSync()` / `App.onLaunch(options)`）
  Map<String, Object?> get launchOptions => {
        'path': path,
        'query': query,
        'scene': scene,
        if (referrer != null) 'referrerInfo': referrer,
      };

  /// `minirender://open?...` / `http(s)://…/open?appid=…` / `…/a/<AppID>`；认不出返回 null
  static StoreLink? parse(String url) {
    final u = Uri.tryParse(url.trim());
    if (u == null) return null;
    String? key;
    if (u.scheme == 'minirender' && (u.host == 'open' || u.path == 'open' || u.path == '/open')) {
      key = u.queryParameters['appid'] ?? u.queryParameters['id'];
    } else if (u.scheme == 'http' || u.scheme == 'https') {
      final seg = u.pathSegments;
      if (seg.length == 1 && seg[0] == 'open') key = u.queryParameters['appid'];
      if (seg.length == 2 && seg[0] == 'a') key = seg[1];
    }
    if (key == null || key.isEmpty) return null;
    var path = (u.queryParameters['path'] ?? '').replaceFirst(RegExp(r'^/+'), '');
    final query = <String, String>{};
    // path 里可以直接带 ?a=1（微信也允许），query 参数另给的再合进去
    final qi = path.indexOf('?');
    if (qi >= 0) {
      query.addAll(Uri.splitQueryString(path.substring(qi + 1)));
      path = path.substring(0, qi);
    }
    final q = u.queryParameters['query'];
    if (q != null && q.isNotEmpty) query.addAll(Uri.splitQueryString(q));
    return StoreLink(key: key, path: path, query: query);
  }
}

class StoreLinks {
  static final navigatorKey = GlobalKey<NavigatorState>();
  static const _channel = MethodChannel('mini_render_example/links');
  static final store = MiniStore();
  static bool _inited = false;

  /// 当前打开着的小程序的 AppID（navigateToMiniProgram 的 referrerInfo.appId）
  static String? currentAppId;

  /// 当前打开着的那个（[open] 记下）
  static StoreLink? _current;

  /// 小程序打开小程序的来路：A → B → C 时是 [A, B]。返回 / 关闭 C 先回 B、再回 A，最后才回 App 首页（与微信一致）。
  ///
  /// [parked] 不为空：A 没关，挂起着留在内存里（[MiniProgramController.park]，它的页面还在路由栈里、被 B 盖着），
  /// 回去是把 B 那一页退掉、A [MiniProgramController.unpark] —— 页面栈、滚动位置、列表数据、JS 状态原样，没有 loading。
  /// 挂不上（还没建好、平台没实现）才退回老办法：截一张 [shot]、关掉 A，回去时按 [link] 重开、先盖着那张图
  static final List<({StoreLink link, String? shot, MiniProgramController? parked, String? appId})> _back = [];

  /// 当前开着的小程序页的控制器（[MiniAppPage] 起来时登记、销毁时撤）：跳走时挂起它
  static MiniProgramController? current;

  /// 挂着的小程序页被关掉了（App 自己的入口又打开了别的，整个路由栈收掉）：来路里的那一条作废
  static void forgetParked(MiniProgramController c) => _back.removeWhere((b) => identical(b.parked, c));

  /// 有没有可回去的上一个小程序（`navigateBackMiniProgram` 成不成功看它）
  static bool get canGoBack => _back.isNotEmpty;

  /// 入口页起来之后调一次：接住冷启动带进来的链接，之后的链接由原生推过来
  static Future<void> init() async {
    if (_inited) return;
    _inited = true;
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'link') _handle('${call.arguments}');
      return null;
    });
    try {
      final initial = await _channel.invokeMethod<String>('initial');
      if (initial != null && initial.isNotEmpty) _handle(initial);
    } on MissingPluginException {
      // 宿主没接通道（例如桌面上跑）：只是没有深链
    }
  }

  static void _handle(String url) {
    final link = StoreLink.parse(url);
    debugPrint('[商店] 外部链接 $url → ${link == null ? '认不出' : '${link.key} ${link.route ?? '首页'}'}');
    if (link != null) {
      // 外部链接是一次新的进入：之前的来路作废
      _clearBack();
      open(link);
    }
  }

  /// 打开一个小程序：先把栈上的小程序页收掉（一个进程同一时刻只能有一个小程序实例），
  /// 再推 loading 页（检查更新 → 下载 → 打开）。[resume]：回到上一个小程序（见 [backToReferrer]）
  static void open(StoreLink link, {StoreApp? hint, MiniResume? resume}) {
    // 记进「最近使用」（深链只有 AppID 时等打开那一刻知道是谁了再记，见 MiniStoreLaunchPage）
    if (hint != null) store.touch(hint);
    final nav = navigatorKey.currentState;
    if (nav == null) return;
    _current = link;
    nav.pushAndRemoveUntil(
        MiniStoreLaunchPage.route(store: store, link: link, hint: hint, resume: resume), (r) => r.isFirst);
  }

  /// 从列表 / 搜索 / 最近使用这类 App 自己的入口打开：来路清空
  static void openFresh(StoreLink link, {StoreApp? hint}) {
    _clearBack();
    open(link, hint: hint);
  }

  /// 来路作废。挂着的那些不用在这里关：接下来的 pushAndRemoveUntil 把它们的页面收掉，页面 dispose 时关
  static void _clearBack() {
    for (final b in _back) {
      if (b.shot != null) File(b.shot!).delete().ignore();
    }
    _back.clear();
  }

  /// 回到打开当前小程序的那个小程序（返回键、侧滑、胶囊「⊙」、`navigateBackMiniProgram`、`exitMiniProgram`）。
  /// 没有来路返回 false（调用方照常回 App 首页）。
  ///
  /// A 挂着（通常如此）：退掉 B 那一页，等 B 销毁完，A 原样恢复（`onShow` 收到 scene 1038 + referrerInfo.extraData）——
  /// 与微信一样停在离开时的那一页、那个滚动位置，不重排、不闪。A 没挂上的老办法见 [_resume]
  static bool backToReferrer({Object? extraData}) {
    if (_back.isEmpty) return false;
    final from = currentAppId ?? '';
    final a = _back.removeLast();
    final referrer = <String, Object?>{'appId': from, if (extraData != null) 'extraData': extraData};
    final parked = a.parked;
    if (parked != null) {
      unawaited(_unpark(a.link, parked, a.appId, referrer));
      return true;
    }
    unawaited(_resume(
      StoreLink(
        key: a.link.key,
        path: a.link.path,
        query: a.link.query,
        // 1038：从另一个小程序返回（微信的场景值）
        scene: 1038,
        referrer: referrer,
      ),
      a.shot,
    ));
    return true;
  }

  /// 退掉 B 那一页（它上面没别的了：B 打开过的 C 已经退回来过），等 B 的控制器销毁完，再恢复 A。
  /// 顺序要紧：A 的引擎要等 B 换过的进程级状态（包根、storage 目录、组件状态表）收回来之后才能跑
  static Future<void> _unpark(StoreLink link, MiniProgramController a, String? appId, Map<String, Object?> referrer) async {
    final sw = Stopwatch()..start();
    final nav = navigatorKey.currentState;
    if (nav == null) return;
    // B 那一页（MiniStoreLaunchPage）退掉；A 的页面就在它下面
    nav.pop();
    await MiniAppPage.waitAllClosed();
    _current = link;
    currentAppId = appId;
    current = a;
    final page = a.page.value;
    final ok = await a.unpark(enterOptions: {
      'path': page?.route ?? link.path,
      'query': const <String, String>{},
      'scene': 1038,
      'referrerInfo': referrer,
    });
    debugPrint('[商店] 回到 ${link.key}（挂起的，原样恢复）${ok ? 'ok' : '没恢复上'} ${sw.elapsedMilliseconds}ms');
  }

  /// 先把截图解码进图片缓存再换页：不然换过去的头几帧图还没解出来，露一下白底
  static Future<void> _resume(StoreLink link, String? shot) async {
    final ctx = navigatorKey.currentContext;
    ImageProvider? cover;
    if (shot != null && ctx != null && File(shot).existsSync()) {
      final p = FileImage(File(shot), scale: View.of(ctx).devicePixelRatio);
      try {
        await precacheImage(p, ctx).timeout(const Duration(milliseconds: 300));
        cover = p;
      } catch (_) {
        // 解不出来 / 太慢：不盖图，照样不走 loading
      }
    }
    open(link, resume: MiniResume(cover: cover, file: shot));
  }

  /// 离开当前小程序之前截一张它的画面（JPEG，几十毫秒）；截不到给 null（照样不盖 loading，只是没有图）
  static Future<String?> _shoot() async {
    final c = current;
    if (c == null) return null;
    try {
      final cache = (await MiniProgramController.paths())['cache'] ?? '';
      if (cache.isEmpty) return null;
      // 每张一个新名字：同一路径的 FileImage 会命中图片缓存里的旧图
      final path = '$cache/referrer_${DateTime.now().microsecondsSinceEpoch}.jpg';
      final sw = Stopwatch()..start();
      final ok = await c.captureFrame(path).timeout(const Duration(seconds: 1), onTimeout: () => false);
      debugPrint('[商店] 离开前截帧 ${ok ? path : '失败'} ${sw.elapsedMilliseconds}ms');
      return ok ? path : null;
    } catch (e) {
      debugPrint('[商店] 离开前截帧失败：$e');
      return null;
    }
  }

  /// 小程序里的 wx.navigateToMiniProgram：用户点了允许之后调。
  /// 与微信一致：A 不关、挂起留在内存里（[MiniProgramController.park]），B 的页面推在 A 的上面；
  /// 返回时退掉 B、A 原样回来（[backToReferrer]）。挂不上时才退回「截一张、关掉 A、回来重开」
  static Future<void> openFromMiniProgram(Map<String, Object?> args) async {
    final appId = '${args['appId'] ?? ''}';
    final parsed = StoreLink.parse('minirender://open?${Uri(queryParameters: {
          'appid': appId,
          'path': '${args['path'] ?? ''}',
        }).query}');
    if (parsed == null) return;
    final link = StoreLink(
      key: parsed.key,
      path: parsed.path,
      query: parsed.query,
      scene: 1037,
      referrer: {'appId': currentAppId ?? '', if (args['extraData'] != null) 'extraData': args['extraData']},
    );
    final cur = _current;
    final c = current;
    final sw = Stopwatch()..start();
    if (cur != null && c != null && await c.park()) {
      debugPrint('[商店] 挂起 ${cur.key}（打开 ${link.key}）${sw.elapsedMilliseconds}ms');
      _back.add((link: cur, shot: null, parked: c, appId: currentAppId));
      final nav = navigatorKey.currentState;
      if (nav == null) return;
      _current = link;
      unawaited(nav.push(MiniStoreLaunchPage.route(store: store, link: link)));
      return;
    }
    // 挂不上：记下来路与 A 此刻的画面（要在 A 销毁之前截），关掉 A，回来时重开
    if (cur != null) _back.add((link: cur, shot: await _shoot(), parked: null, appId: currentAppId));
    open(link);
  }
}
