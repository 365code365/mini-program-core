// MiniStore（lib/store/mini_store.dart）+ StoreLink：本机起一个假的 mini-store，验
// 打开前查最新版、新版自动更新、按 AppID 找、离线用本地版、下架不糊弄、路径穿越、深链解析
import 'dart:convert';
import 'dart:io';

import 'package:archive/archive.dart';
import 'package:crypto/crypto.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mini_render_example/store/mini_store.dart';
import 'package:mini_render_example/store/store_links.dart';

List<int> _zip(Map<String, String> files) {
  final a = Archive();
  files.forEach((k, v) => a.addFile(ArchiveFile(k, utf8.encode(v).length, utf8.encode(v))));
  return ZipEncoder().encode(a)!;
}

/// TestWidgetsFlutterBinding 会把 HttpClient 换成一律回 400 的假货：这里要连本机真服务器，换回真的
class _RealHttp extends HttpOverrides {}

const _goodAppId = 'wx3b7d9f1a2c4e6b80';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  HttpOverrides.global = _RealHttp();
  late Directory files;
  late HttpServer server;
  late String url;
  var downloads = 0;
  // 服务器上的「当前版本」：测试里改它模拟上传了新版
  late Map<String, (int, List<int>)> packages;
  final appIds = {'good': _goodAppId, 'evil': 'wx0000000000000bad'};

  Map<String, Object> meta(String id) => {
        'id': id,
        'appId': appIds[id]!,
        'name': '名字$id',
        'version': packages[id]!.$1,
        'sha256': sha256.convert(packages[id]!.$2).toString(),
        'size': packages[id]!.$2.length,
        'packageUrl': '/api/apps/$id/package?v=${packages[id]!.$1}',
      };

  String? idOf(String key) => packages.containsKey(key) ? key : appIds.entries.where((e) => e.value == key).firstOrNull?.key;

  setUp(() async {
    files = await Directory.systemTemp.createTemp('ms_test');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger.setMockMethodCallHandler(
      const MethodChannel('dev.minirender/engine'),
      (call) async => call.method == 'paths' ? {'files': files.path, 'cache': files.path} : null,
    );
    packages = {
      'good': (1, _zip({'app.json': '{"pages":["pages/a/a"]}', 'pages/a/a.wxml': '<view>v1</view>'})),
      'evil': (1, _zip({'app.json': '{"pages":["a"]}', '../escape.txt': 'x'})),
    };
    downloads = 0;
    server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    url = 'http://127.0.0.1:${server.port}';
    server.listen((req) async {
      final p = req.uri.pathSegments;
      if (req.uri.path == '/api/apps') {
        req.response.write(jsonEncode({'apps': [meta('good'), meta('evil'), {'id': 'Bad/../x'}]}));
      } else if (p.length == 3 && p[0] == 'api' && idOf(p[2]) != null) {
        req.response.write(jsonEncode(meta(idOf(p[2])!)));
      } else if (p.length == 4 && p[3] == 'package' && idOf(p[2]) != null) {
        downloads++;
        req.response.add(packages[idOf(p[2])!]!.$2);
      } else {
        req.response.statusCode = 404;
        req.response.write('{"error":"没有这个小程序"}');
      }
      await req.response.close();
    });
  });

  tearDown(() async {
    await server.close(force: true);
    await files.delete(recursive: true);
  });

  test('本机记录：收藏 / 取消收藏、删最近使用不动收藏、打开时收藏跟着换成新版信息、每次改动都通知', () async {
    final store = MiniStore(baseUrl: url);
    final apps = (await store.list()).apps;
    final good = apps.firstWhere((a) => a.id == 'good');
    final evil = apps.firstWhere((a) => a.id == 'evil');
    var changes = 0;
    store.localChanges.addListener(() => changes++);

    await store.touch(good);
    await store.touch(evil);
    expect((await store.recent()).map((a) => a.id), ['evil', 'good']);

    expect(await store.toggleFavorite(good), isTrue);
    expect(await store.toggleFavorite(evil), isTrue);
    expect((await store.favorites()).map((a) => a.id), ['evil', 'good'], reason: '后收藏的在前');
    expect(await store.toggleFavorite(evil), isFalse);
    expect(await store.isFavorite('evil'), isFalse);

    await store.removeRecent('good');
    expect((await store.recent()).map((a) => a.id), ['evil']);
    expect(await store.isFavorite('good'), isTrue, reason: '删最近使用不该动收藏');

    final renamed = StoreApp.fromJson({...good.toJson(), 'name': '新名字', 'version': 9});
    await store.touch(renamed);
    expect((await store.favorites()).single.name, '新名字');
    expect(changes, 7);
  });

  test('列表：非法 id 被滤掉，带 AppID', () async {
    final l = await MiniStore(baseUrl: url).list();
    expect(l.offline, isFalse);
    expect(l.apps.map((a) => a.id), ['good', 'evil']);
    expect(l.apps.first.key, _goodAppId);
  });

  test('按 AppID 打开：首次下载，同版本不再下，服务器出新版就更新', () async {
    final store = MiniStore(baseUrl: url);
    final p1 = await store.prepare(_goodAppId);
    expect(p1.dir, '${files.path}/mini-apps/good');
    expect(p1.updated, isTrue);
    expect(File('${p1.dir}/pages/a/a.wxml').readAsStringSync(), '<view>v1</view>');
    expect(downloads, 1);

    final p2 = await store.prepare(_goodAppId);
    expect(p2.updated, isFalse);
    expect(downloads, 1, reason: 'sha256 没变不该再下');

    // 服务器上传了 v2：下次打开前查到不一样，自动下新包
    packages['good'] = (2, _zip({'app.json': '{"pages":["pages/a/a"]}', 'pages/a/a.wxml': '<view>v2</view>'}));
    final statuses = <String>[];
    final p3 = await store.prepare('good', onStatus: statuses.add);
    expect(p3.updated, isTrue);
    expect(p3.app.version, 2);
    expect(downloads, 2);
    expect(File('${p3.dir}/pages/a/a.wxml').readAsStringSync(), '<view>v2</view>');
    expect(statuses, ['检查更新…', '发现新版本 v2，更新中…', '解包中…']);
    expect((await store.local(_goodAppId))?.version, 2, reason: '本地记录按 AppID 也找得到');
  });

  test('服务器连不上：打开本地那一版；本地没有就报错', () async {
    final store = MiniStore(baseUrl: url);
    await store.prepare(_goodAppId);
    await server.close(force: true);
    final p = await store.prepare(_goodAppId);
    expect(p.stale, isNotNull);
    expect(p.dir, '${files.path}/mini-apps/good');
    await expectLater(store.prepare('evil'), throwsA(anything));
  });

  test('服务器说没有（404，已下架）：不拿本地旧版糊弄', () async {
    final store = MiniStore(baseUrl: url);
    await store.prepare('good');
    packages.remove('good');
    await expectLater(store.prepare('good'), throwsA(isA<StoreNotFound>()));
  });

  test('包里有 ../ 的路径：拒绝，且不留下半个目录', () async {
    final store = MiniStore(baseUrl: url);
    await expectLater(store.prepare('evil'), throwsA(isA<FormatException>()));
    expect(File('${files.path}/escape.txt').existsSync(), isFalse);
    expect(Directory('${files.path}/mini-apps/evil').existsSync(), isFalse);
  });

  test('服务器连不上时列表用上次缓存的', () async {
    await MiniStore(baseUrl: url).list();
    await server.close(force: true);
    final l = await MiniStore(baseUrl: url).list();
    expect(l.offline, isTrue);
    expect(l.apps.length, 2);
  });

  test('深链解析：scheme / H5 地址 / 短链，path 里带的 query 与 query 参数合并', () {
    final a = StoreLink.parse('minirender://open?appid=$_goodAppId&path=%2Fpages%2Fdetail%2Fdetail%3Fid%3D3&query=from%3Dh5')!;
    expect(a.key, _goodAppId);
    expect(a.path, 'pages/detail/detail');
    expect(a.query, {'id': '3', 'from': 'h5'});
    expect(a.route, 'pages/detail/detail?id=3&from=h5');
    expect(a.launchOptions['scene'], 1065);
    expect((a.launchOptions['query'] as Map)['id'], '3');

    final h5 = StoreLink.parse('http://64.83.22.217:9630/open?appid=$_goodAppId')!;
    expect((h5.key, h5.route), (_goodAppId, null));
    final short = StoreLink.parse('https://x.example/a/$_goodAppId?path=pages/a/a')!;
    expect((short.key, short.route), (_goodAppId, 'pages/a/a'));

    expect(StoreLink.parse('minirender://open'), isNull);
    expect(StoreLink.parse('https://x.example/other'), isNull);
    expect(StoreLink.parse('weixin://dl/business/?appid=x'), isNull);
  });
}
