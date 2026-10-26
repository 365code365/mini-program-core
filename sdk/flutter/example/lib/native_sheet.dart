// 小程序页上的弹层（胶囊「•••」菜单、关于框）在 Android 上用原生窗口画（android/.../NativeSheet.kt）。
//
// 为什么：小程序画面是 hybrid composition 的平台视图，Flutter 在它上面画的每一帧都要经叠加层、与平台线程同步。
// 一加 PLY110 上量过 showModalBottomSheet 的入场动画：每帧 22~42ms（90Hz 下一帧该 11ms），250ms 只出七八帧，
// 看着就是「点了半天菜单才慢慢出来」。原生 Dialog 是单独的窗口，由系统合成，Flutter 一帧都不用画。
// 其它平台（iOS 走纹理，没有这个问题）与通道不在时退回 Flutter 自己画的那份（store/store_sheets.dart）。
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'store/store_sheets.dart' show CapsuleMenuItem;

class NativeSheet {
  static const _ch = MethodChannel('mini_render_example/sheet');

  static bool get available => !kIsWeb && Platform.isAndroid;

  /// 胶囊菜单：返回点中的那一项（`'header'` = 点了顶上的小程序名；null = 取消）。通道不在时抛 [MissingPluginException]
  static Future<String?> capsule({
    required String id,
    required String name,
    String iconUrl = '',
    String subtitle = '',
    required List<CapsuleMenuItem> top,
    required List<CapsuleMenuItem> bottom,
  }) {
    Map<String, Object?> item(CapsuleMenuItem it) =>
        {'id': it.id, 'glyph': it.glyph, 'label': it.label, 'enabled': it.enabled, 'active': it.active};
    return _ch.invokeMethod<String>('capsule', {
      'id': id,
      'name': name,
      'iconUrl': iconUrl,
      'subtitle': subtitle,
      'top': [for (final it in top) item(it)],
      'bottom': [for (final it in bottom) item(it)],
    });
  }

  /// 系统 Toast（短）
  static Future<void> toast(String text) => _ch.invokeMethod<void>('toast', {'text': text});

  /// 提示框：返回点了第几个按钮（null = 点外面收起）
  static Future<int?> alert(String title, String message, List<String> buttons) =>
      _ch.invokeMethod<int>('alert', {'title': title, 'message': message, 'buttons': buttons});
}
