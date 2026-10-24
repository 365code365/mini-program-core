// 小程序商店客户端：从 mini-store 服务（tools/mini-store/server.py）拉列表、下载包、解到本地目录。
//
// 引擎按「目录」打开小程序（含 app.json），所以这里做的就是宿主那一步「检查更新 → 下载 → 校验 → 解包」：
// - **每次打开前都问一次服务器最新版**（[prepare]，`GET /api/apps/<AppID>`）：sha256 与本地一致就直接打开，
//   不一致就先下新包。列表里的版本信息可能是几分钟前拉的，不能拿来判断；
// - 解到 `files/mini-apps/<id>/`：目录名就是 id —— 引擎的 storage 按目录名分文件（`mini-storage/<id>.json`），
//   不同小程序互不串数据；旁边 `<id>.json` 记着本地这一版的信息（AppID、版本、sha256）；
// - 服务器连不上：本地有就打开本地那版（离线可用），没有才报错；
// - 外部 App / H5 / 别的小程序按 **AppID** 找（[prepare] 的 key 可以是 AppID 也可以是 id）。
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:archive/archive.dart';
import 'package:crypto/crypto.dart';
import 'package:mini_render/mini_render.dart';

/// 服务地址：`flutter build apk --dart-define=MINI_STORE=http://host:port` 换
const kMiniStoreUrl = String.fromEnvironment('MINI_STORE', defaultValue: 'http://64.83.22.217:9630');

/// 与服务器同一条规则：id 要当目录名用
final _idRe = RegExp(r'^[a-z0-9][a-z0-9._-]{0,63}$');

/// 微信 AppID 的形状（服务器分配的也是这个形状）
final appIdRe = RegExp(r'^wx[0-9a-f]{16}$');

class StoreApp {
  StoreApp({
    required this.id,
    required this.appId,
    required this.name,
    required this.description,
    required this.version,
    required this.sha256,
    required this.size,
    required this.packageUrl,
  });

  factory StoreApp.fromJson(Map<String, dynamic> j) => StoreApp(
        id: j['id'] as String,
        appId: (j['appId'] as String?) ?? '',
        name: (j['name'] as String?) ?? j['id'] as String,
        description: (j['description'] as String?) ?? '',
        version: (j['version'] as num?)?.toInt() ?? 0,
        sha256: (j['sha256'] as String?) ?? '',
        size: (j['size'] as num?)?.toInt() ?? 0,
        packageUrl: (j['packageUrl'] as String?) ?? '/api/apps/${j['id']}/package',
      );

  Map<String, Object> toJson() => {
        'id': id,
        'appId': appId,
        'name': name,
        'description': description,
        'version': version,
        'sha256': sha256,
        'size': size,
        'packageUrl': packageUrl,
      };

  final String id;
  final String appId;
  final String name;
  final String description;
  final int version;
  final String sha256;
  final int size;
  final String packageUrl;

  /// 深链 / 跳转用的 key：有 AppID 用 AppID
  String get key => appId.isNotEmpty ? appId : id;
}

/// 一次列表的结果：[offline] = 服务器连不上，用的是上次缓存的列表
class StoreList {
  StoreList(this.apps, {this.offline = false, this.error});

  final List<StoreApp> apps;
  final bool offline;
  final String? error;
}

/// [prepare] 的结果：[stale] 不为 null 表示没能拿到最新版，打开的是本地旧版（值是原因）
class PreparedApp {
  PreparedApp(this.dir, this.app, {this.updated = false, this.stale});

  final String dir;
  final StoreApp app;

  /// 这次下了新包（首次安装或升级）
  final bool updated;
  final Object? stale;
}

class MiniStore {
  MiniStore({this.baseUrl = kMiniStoreUrl});

  final String baseUrl;
  String? _root;

  Future<String> _rootDir() async {
    if (_root != null) return _root!;
    final files = (await MiniProgramController.paths())['files']!;
    final dir = Directory('$files/mini-apps');
    await dir.create(recursive: true);
    return _root = dir.path;
  }

  HttpClient _client() => HttpClient()
    ..connectionTimeout = const Duration(seconds: 6)
    ..idleTimeout = const Duration(seconds: 5);

  Future<String> _getText(String url, Duration timeout) async {
    final client = _client();
    try {
      final req = await client.getUrl(Uri.parse(url));
      final res = await req.close().timeout(timeout);
      final body = await res.transform(utf8.decoder).join();
      if (res.statusCode == 404) throw StoreNotFound(body);
      if (res.statusCode != 200) throw HttpException('HTTP ${res.statusCode}');
      return body;
    } finally {
      client.close(force: true);
    }
  }

  /// 拉列表；连不上就退回上次缓存的那份
  Future<StoreList> list() async {
    final cache = File('${await _rootDir()}/index.json');
    try {
      final body = await _getText('$baseUrl/api/apps', const Duration(seconds: 10));
      final apps = _parse(body);
      await cache.writeAsString(body);
      return StoreList(apps);
    } catch (e) {
      final apps = cache.existsSync() ? _parse(await cache.readAsString()) : <StoreApp>[];
      return StoreList(apps, offline: true, error: '$e');
    }
  }

  static List<StoreApp> _parse(String body) {
    final list = (jsonDecode(body) as Map<String, dynamic>)['apps'] as List;
    return [
      for (final j in list.cast<Map<String, dynamic>>())
        if (_idRe.hasMatch(j['id'] as String? ?? '')) StoreApp.fromJson(j),
    ];
  }

  /// 服务器上这个小程序的最新版（key = AppID 或 id）
  Future<StoreApp> latest(String key) async {
    if (!appIdRe.hasMatch(key) && !_idRe.hasMatch(key)) throw StoreNotFound('非法的 AppID：$key');
    final body = await _getText('$baseUrl/api/apps/$key', const Duration(seconds: 6));
    final app = StoreApp.fromJson(jsonDecode(body) as Map<String, dynamic>);
    if (!_idRe.hasMatch(app.id)) throw const FormatException('服务器给的 id 不合法');
    return app;
  }

  /// 本地已装的那一版（key = AppID 或 id）；没装返回 null
  Future<StoreApp?> local(String key) async {
    final root = await _rootDir();
    final dir = Directory(root);
    for (final f in dir.listSync().whereType<File>().where((f) => f.path.endsWith('.json'))) {
      if (f.path.endsWith('/index.json')) continue;
      try {
        final app = StoreApp.fromJson(jsonDecode(f.readAsStringSync()) as Map<String, dynamic>);
        if ((app.id == key || app.appId == key) && File('$root/${app.id}/app.json').existsSync()) return app;
      } catch (_) {}
    }
    return null;
  }

  /// 打开前的准备：问服务器最新版 → 本地不是这一版就下载解包 → 返回目录。
  /// [onStatus] 给 loading 页显示的阶段文字；[onProgress] 给下载进度（已收 / 总字节）
  Future<PreparedApp> prepare(
    String key, {
    void Function(String status)? onStatus,
    void Function(int received, int total)? onProgress,
  }) async {
    final root = await _rootDir();
    onStatus?.call('检查更新…');
    final StoreApp app;
    try {
      app = await latest(key);
    } on StoreNotFound {
      rethrow; // 服务器明确说没有（已下架 / AppID 不对）：不拿本地旧版糊弄
    } catch (e) {
      final have = await local(key);
      if (have == null) rethrow;
      return PreparedApp('$root/${have.id}', have, stale: e);
    }
    final installed = await local(app.id);
    if (installed != null && installed.sha256 == app.sha256) return PreparedApp('$root/${app.id}', app);
    onStatus?.call(installed == null ? '下载中…' : '发现新版本 v${app.version}，更新中…');
    try {
      final bytes = await _download(app, onProgress);
      onStatus?.call('解包中…');
      await _unpack(bytes, root, app);
      return PreparedApp('$root/${app.id}', app, updated: true);
    } catch (e) {
      if (installed == null) rethrow;
      return PreparedApp('$root/${installed.id}', installed, stale: e);
    }
  }

  Future<Uint8List> _download(StoreApp app, void Function(int, int)? onProgress) async {
    final client = _client();
    try {
      final req = await client.getUrl(Uri.parse(baseUrl).resolve(app.packageUrl));
      final res = await req.close().timeout(const Duration(seconds: 15));
      if (res.statusCode != 200) throw HttpException('下载失败：HTTP ${res.statusCode}');
      final total = res.contentLength > 0 ? res.contentLength : app.size;
      final out = BytesBuilder(copy: false);
      // 单块之间 20 秒没动静就算断了（整包不设上限：几 MB 的包在慢网上也要能下完）
      await for (final chunk in res.timeout(const Duration(seconds: 20))) {
        out.add(chunk);
        onProgress?.call(out.length, total);
      }
      final bytes = out.takeBytes();
      final digest = sha256.convert(bytes).toString();
      if (app.sha256.isNotEmpty && digest != app.sha256) {
        throw const FormatException('包校验失败（sha256 与服务器给的不一致）');
      }
      return bytes;
    } finally {
      client.close(force: true);
    }
  }

  /// 先解到临时目录、整个换上去：解一半断掉也不会留下半个小程序
  static Future<void> _unpack(Uint8List bytes, String root, StoreApp app) async {
    final tmp = Directory('$root/.${app.id}.tmp');
    if (tmp.existsSync()) await tmp.delete(recursive: true);
    await tmp.create(recursive: true);
    final base = tmp.absolute.path;
    final archive = ZipDecoder().decodeBytes(bytes);
    for (final f in archive.files) {
      if (!f.isFile) continue;
      final name = f.name.replaceAll('\\', '/');
      final parts = name.split('/');
      // 服务器已经挡过一次；客户端照样再挡（路径穿越 = 写到沙盒别处）
      if (name.startsWith('/') || parts.contains('..') || parts.contains('')) {
        throw FormatException('包里有非法路径：$name');
      }
      final out = File('$base/$name');
      await out.parent.create(recursive: true);
      await out.writeAsBytes(f.content as List<int>, flush: false);
    }
    if (!File('$base/app.json').existsSync()) throw const FormatException('包里没有 app.json');
    final dir = Directory('$root/${app.id}');
    if (dir.existsSync()) await dir.delete(recursive: true);
    await tmp.rename(dir.path);
    await File('$root/${app.id}.json').writeAsString(jsonEncode(app.toJson()));
  }
}

/// 服务器明确回了 404：没有这个小程序
class StoreNotFound implements Exception {
  const StoreNotFound(this.body);

  final String body;

  @override
  String toString() {
    try {
      return '${(jsonDecode(body) as Map)['error']}';
    } catch (_) {
      return body.isEmpty ? '没有这个小程序' : body;
    }
  }
}
