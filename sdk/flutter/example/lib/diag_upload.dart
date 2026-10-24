// 「上传诊断日志」：真机出问题、手边没有 USB / adb 时，把日志传到 mini-store（`POST /api/diag`），
// 开发机上 `bash tools/mini-store/diag.sh get latest` 拉回来分析。与原生示例 DiagUpload.kt 同一种 JSON。
//
// 日志两份：
// - Dart 侧环形缓冲（[DiagLog]）：main() 里把 debugPrint 接过来，引擎事件、小程序 console、商店流程都在这里。
//   iOS 上只有这一份；
// - Android 上另取本进程的 logcat（原生 MainActivity 的 `logcat` 方法）：引擎 Rust 侧（tag mini-render）、
//   插件、系统报错都在里面，比 Dart 那份全。
import 'dart:collection';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'store/mini_store.dart';

class DiagLog {
  static const _max = 6000;
  static final _lines = ListQueue<String>();

  /// main() 里调一次：之后每条 debugPrint 都留一份
  static void install() {
    final orig = debugPrint;
    debugPrint = (String? message, {int? wrapWidth}) {
      add(message ?? '');
      orig(message, wrapWidth: wrapWidth);
    };
    final onError = FlutterError.onError;
    FlutterError.onError = (details) {
      add('[FlutterError] ${details.exceptionAsString()}\n${details.stack}');
      onError?.call(details);
    };
  }

  static void add(String line) {
    _lines.add('${DateTime.now().toIso8601String().substring(11, 23)} $line');
    while (_lines.length > _max) {
      _lines.removeFirst();
    }
  }

  static String dump() => _lines.join('\n');
}

class DiagUpload {
  static const _host = MethodChannel('mini_render_example/links');

  /// 问一句说明，然后收集 + 上传，结果弹框给出诊断 id
  static Future<void> ask(BuildContext context, {Map<String, Object?>? app, String? backend}) async {
    final note = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('上传诊断日志'),
        content: Column(mainAxisSize: MainAxisSize.min, children: [
          const Text('会上传本 App 的运行日志、机型与系统版本，用于排查问题。'),
          TextField(controller: note, decoration: const InputDecoration(hintText: '发生了什么（可不填）')),
        ]),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
          TextButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('上传')),
        ],
      ),
    );
    if (ok != true || !context.mounted) return;
    final messenger = ScaffoldMessenger.maybeOf(context);
    messenger?.showSnackBar(const SnackBar(content: Text('正在收集日志…')));
    String title, msg;
    try {
      final id = await upload(note.text, app: app, backend: backend);
      title = '已上传';
      msg = '诊断 id：$id\n\n把这个 id 发给开发，或在开发机上跑\nbash tools/mini-store/diag.sh get $id';
    } catch (e) {
      title = '上传失败';
      msg = '$e';
    }
    debugPrint('[诊断] $title $msg');
    if (!context.mounted) return;
    await showDialog<void>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(title),
        content: SelectableText(msg),
        actions: [TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('好'))],
      ),
    );
  }

  static Future<String> upload(String note, {Map<String, Object?>? app, String? backend}) async {
    String logs = DiagLog.dump();
    Map<String, Object?> device = {};
    if (Platform.isAndroid) {
      try {
        final r = await _host.invokeMapMethod<String, Object?>('logcat');
        if (r != null) {
          device = Map.of(r)..remove('logcat');
          logs = '${r['logcat'] ?? ''}\n\n===== Dart 日志 =====\n$logs';
        }
      } catch (e) {
        logs = '（取 logcat 失败：$e）\n$logs';
      }
    }
    final body = {
      'host': 'flutter',
      'platform': Platform.operatingSystem,
      'device': device['device'] ?? Platform.localHostname,
      'system': device['system'] ?? Platform.operatingSystemVersion,
      'abi': device['abi'],
      'backend': backend,
      'app': app,
      'note': note,
      'clientTime': DateTime.now().millisecondsSinceEpoch,
      'logs': logs,
    };
    final client = HttpClient()..connectionTimeout = const Duration(seconds: 8);
    try {
      final req = await client.postUrl(Uri.parse('$kMiniStoreUrl/api/diag'));
      req.headers.contentType = ContentType.json;
      req.headers.set(HttpHeaders.contentEncodingHeader, 'gzip');
      final raw = gzip.encode(utf8.encode(jsonEncode(body)));
      req.contentLength = raw.length;
      req.add(raw);
      final res = await req.close().timeout(const Duration(seconds: 30));
      final text = await res.transform(utf8.decoder).join();
      final j = jsonDecode(text) as Map<String, dynamic>;
      if (res.statusCode != 200) throw HttpException('${j['error'] ?? 'HTTP ${res.statusCode}'}');
      return '${j['id']}';
    } finally {
      client.close(force: true);
    }
  }
}
