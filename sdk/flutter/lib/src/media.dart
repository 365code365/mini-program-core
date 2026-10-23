import 'package:flutter/services.dart';

import 'events.dart';

/// 插件**自带**的宿主能力：选图、选视频、选文件、存相册、打开文档。
///
/// [MiniProgramController.builtinHostApis] 开着（默认）时这几个名字自动点名进 `hostApis`；
/// 宿主在 `hostApis` 里给了同名的实现，用宿主的（比如要接自己的相机页）。
///
/// 原生一半在插件里（Android `MediaPicker.kt` / `MediaImport.kt`，iOS `MediaPicker.swift`），走系统自己的界面：
///
/// | API | Android | iOS |
/// |---|---|---|
/// | `chooseImage` / `chooseMedia` / `chooseVideo`（相册） | 照片选择器（Android 13+ / 11+ 带更新），更老的系统 `ACTION_GET_CONTENT` | `PHPickerViewController`（iOS 14+），iOS 13 `UIImagePickerController` |
/// | 同上（相机） | 系统相机（`ACTION_IMAGE_CAPTURE` / `ACTION_VIDEO_CAPTURE`） | `UIImagePickerController` |
/// | `chooseMessageFile` | 系统文件选择器（`ACTION_OPEN_DOCUMENT`） | `UIDocumentPickerViewController` |
/// | `saveImageToPhotosAlbum` / `saveVideoToPhotosAlbum` | `MediaStore`（Android 10+ 不要权限；更老的要宿主声明 `WRITE_EXTERNAL_STORAGE`） | `PHPhotoLibrary`（要 `NSPhotoLibraryAddUsageDescription`） |
/// | `openDocument` | `ACTION_VIEW`（插件自带的 FileProvider） | `UIDocumentInteractionController` |
///
/// 文件直接写进引擎给的临时目录（[MiniHostCall.tempDir]），回包是 `file://` 路径，引擎换成
/// `wxfile://tmp/…` 交给小程序 —— `<image src>`、`uploadFile`、`saveFile`、`getFileSystemManager` 都认。
/// 照片按 EXIF 摆正后再交（引擎画图不读 EXIF 方向）；HEIC 等引擎解不了的格式转成 JPEG；
/// `sizeType` 只有 `compressed` 时缩到长边 1920、JPEG 质量 80。
///
/// iOS 宿主要在 Info.plist 里写用途说明，缺了的那一项走 fail 而不是闪退：相机 `NSCameraUsageDescription`
/// （录像另要 `NSMicrophoneUsageDescription`）、存相册 `NSPhotoLibraryAddUsageDescription`。
class MiniBuiltinHostApis {
  MiniBuiltinHostApis._();

  /// 插件自带的那几个 API 名
  static const names = [
    'chooseImage',
    'chooseMedia',
    'chooseVideo',
    'chooseMessageFile',
    'saveImageToPhotosAlbum',
    'saveVideoToPhotosAlbum',
    'openDocument',
  ];

  static const MethodChannel _channel = MethodChannel('dev.minirender/engine');

  /// 处理一次 hostCall（参数已由引擎按官方默认值补齐，见 `host-media.js`）。
  /// 失败抛 [MiniHostApiError]：用户取消是 `<api>:fail cancel`，没给权限是 `<api>:fail auth deny`
  static Future<Map<String, dynamic>> handle(MiniHostCall call, {MethodChannel? channel}) async {
    final ch = channel ?? _channel;
    final a = call.args;
    try {
      switch (call.api) {
        case 'chooseImage':
          final items = await _pickMedia(ch, call, kind: 'image', count: _int(a['count'], 9), compressed: _onlyCompressed(a));
          return {
            'tempFilePaths': [for (final f in items) f['path']],
            'tempFiles': [for (final f in items) {'path': f['path'], 'size': f['size'] ?? 0}],
          };
        case 'chooseMedia':
          final types = _strings(a['mediaType']);
          final img = types.contains('image') || types.contains('mix');
          final vid = types.contains('video') || types.contains('mix');
          final kind = img && vid ? 'mix' : (vid ? 'video' : 'image');
          final items = await _pickMedia(ch, call, kind: kind, count: _int(a['count'], 9), compressed: _onlyCompressed(a));
          return {
            'tempFiles': [
              for (final f in items)
                {
                  'tempFilePath': f['path'],
                  'size': f['size'] ?? 0,
                  'width': f['width'] ?? 0,
                  'height': f['height'] ?? 0,
                  'fileType': f['fileType'] ?? 'image',
                  if (f['fileType'] == 'video') 'duration': f['duration'] ?? 0,
                  if (f['fileType'] == 'video') 'thumbTempFilePath': f['thumb'] ?? '',
                },
            ],
          };
        case 'chooseVideo':
          final f = (await _pickMedia(ch, call, kind: 'video', count: 1, compressed: a['compressed'] != false)).first;
          return {
            'tempFilePath': f['path'],
            'duration': f['duration'] ?? 0,
            'size': f['size'] ?? 0,
            'width': f['width'] ?? 0,
            'height': f['height'] ?? 0,
            if ('${f['thumb'] ?? ''}'.isNotEmpty) 'thumbTempFilePath': f['thumb'],
          };
        case 'chooseMessageFile':
          final items = await ch.invokeListMethod<Map<dynamic, dynamic>>('pickFiles', {
            'dir': _dir(call),
            'count': _int(a['count'], 1),
            'type': '${a['type'] ?? 'all'}',
            'extension': _strings(a['extension']),
          });
          return {
            'tempFiles': [
              for (final f in items ?? const <Map<dynamic, dynamic>>[])
                {'path': f['path'], 'size': f['size'] ?? 0, 'name': f['name'] ?? '', 'type': f['fileType'] ?? 'file', 'time': f['time'] ?? 0},
            ],
          };
        case 'saveImageToPhotosAlbum':
        case 'saveVideoToPhotosAlbum':
          await ch.invokeMethod<void>('saveToAlbum', {
            'path': _localPath(call),
            'kind': call.api == 'saveVideoToPhotosAlbum' ? 'video' : 'image',
          });
          return {};
        case 'openDocument':
          await ch.invokeMethod<void>('openDocument', {
            'path': _localPath(call),
            'fileType': '${a['fileType'] ?? ''}',
            'showMenu': a['showMenu'] == true,
          });
          return {};
      }
    } on PlatformException catch (e) {
      throw MiniHostApiError('${call.api}:fail ${_reason(e)}');
    }
    throw MiniHostApiError('${call.api}:fail 插件没有内置这个 API');
  }

  static Future<List<Map<dynamic, dynamic>>> _pickMedia(
    MethodChannel ch,
    MiniHostCall call, {
    required String kind,
    required int count,
    required bool compressed,
  }) async {
    final a = call.args;
    // 引擎在移动端已经问过「拍照 / 从手机相册选择」，这里只剩一种来源
    final source = _strings(a['sourceType']).contains('album') ? 'album' : 'camera';
    final items = await ch.invokeListMethod<Map<dynamic, dynamic>>('pickMedia', {
      'dir': _dir(call),
      'kind': kind,
      'source': source,
      'count': count,
      'compressed': compressed,
      'maxDuration': _int(a['maxDuration'], 60),
      'camera': '${a['camera'] ?? 'back'}',
    });
    if (items == null || items.isEmpty) throw PlatformException(code: 'cancel');
    return items;
  }

  static String _dir(MiniHostCall call) {
    if (call.tempDir.isEmpty) throw PlatformException(code: 'io', message: '引擎没有给 tempDir（引擎库太旧）');
    return call.tempDir;
  }

  /// `filePath` 的真实路径：引擎在 [MiniHostCall.files] 里给了；本来就是本地绝对路径的原样用
  static String _localPath(MiniHostCall call) {
    final p = '${call.args['filePath'] ?? ''}';
    final real = call.files[p] ?? (p.startsWith('file://') ? p.substring(7) : (p.startsWith('/') ? p : null));
    if (real == null) throw PlatformException(code: 'not_found', message: 'file not found');
    return real;
  }

  static bool _onlyCompressed(Map<String, dynamic> a) {
    final s = _strings(a['sizeType']);
    return s.contains('compressed') && !s.contains('original');
  }

  static List<String> _strings(Object? v) => v is List ? [for (final x in v) '$x'] : (v is String ? [v] : const []);

  static int _int(Object? v, int dflt) => v is num ? v.toInt() : dflt;

  /// 原生侧的错误码 → 微信的 errMsg 尾巴
  static String _reason(PlatformException e) => switch (e.code) {
        'cancel' => 'cancel',
        'denied' => 'auth deny${e.message == null ? '' : '（${e.message}）'}',
        'not_found' => 'file not found',
        _ => e.message ?? e.code,
      };
}
