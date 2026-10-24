// 从外面打开小程序：外部 App / H5 的 URL Scheme，以及小程序里的 wx.navigateToMiniProgram。
//
// 链接的形状仿微信的 `weixin://dl/business/?appid=&path=&query=`：
//     minirender://open?appid=wx3b7d9f1a2c4e6b80&path=pages/detail/detail&query=id%3D3
// 也认 mini-store 的 H5 落地页地址（`http(s)://<商店>/open?appid=…`、`/a/<AppID>?path=…`），
// 二维码 / 短信里放网页地址、App 内扫到也能直接打开。
//
// 原生那一侧（Android MainActivity 的 intent-filter、iOS SceneDelegate 的 openURLContexts）
// 只负责把链接字符串经 `mini_render_example/links` 通道交过来：冷启动用 `initial` 取，运行中推 `link`。
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'mini_store.dart';
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
    if (link != null) open(link);
  }

  /// 打开一个小程序：先把栈上的小程序页收掉（一个进程同一时刻只能有一个小程序实例），
  /// 再推 loading 页（检查更新 → 下载 → 打开）
  static void open(StoreLink link, {StoreApp? hint}) {
    // 记进「最近使用」（深链只有 AppID 时等打开那一刻知道是谁了再记，见 MiniStoreLaunchPage）
    if (hint != null) store.touch(hint);
    final nav = navigatorKey.currentState;
    if (nav == null) return;
    nav.pushAndRemoveUntil(MiniStoreLaunchPage.route(store: store, link: link, hint: hint), (r) => r.isFirst);
  }

  /// 小程序里的 wx.navigateToMiniProgram：用户点了允许之后调。
  /// 与微信的差别：微信把目标小程序叠在上面、`navigateBackMiniProgram` 能回来；这里是**替换**（单实例）
  static void openFromMiniProgram(Map<String, Object?> args) {
    final appId = '${args['appId'] ?? ''}';
    final link = StoreLink.parse('minirender://open?${Uri(queryParameters: {
          'appid': appId,
          'path': '${args['path'] ?? ''}',
        }).query}');
    if (link == null) return;
    open(StoreLink(
      key: link.key,
      path: link.path,
      query: link.query,
      scene: 1037,
      referrer: {'appId': currentAppId ?? '', if (args['extraData'] != null) 'extraData': args['extraData']},
    ));
  }
}
