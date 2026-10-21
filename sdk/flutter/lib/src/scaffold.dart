import 'package:flutter/material.dart';

import 'controller.dart';
import 'events.dart';
import 'view.dart';

/// 小程序的宿主外壳：导航栏（标题 / 颜色 / 返回键 / loading）+ 系统返回键 + 画面。
///
/// 微信里导航栏是**原生外壳**，不是页面画的；引擎也不画它。这里按页面的窗口配置
/// （app.json `window` ← 页面 json ← `wx.setNavigationBarTitle/Color`）用 AppBar 画，
/// `navigationStyle: custom` 的页面不画（页面自己画，画面顶到状态栏下面）。
///
/// 系统返回键 / 侧滑返回：页面栈多于一页时交给小程序返回，只剩一页才放行给宿主。
class MiniProgramScaffold extends StatelessWidget {
  const MiniProgramScaffold({
    super.key,
    required this.controller,
    this.nativeViewBuilder,
    this.onExit,
    this.actions,
  });

  final MiniProgramController controller;
  final MiniNativeViewBuilder? nativeViewBuilder;

  /// 页面栈只剩一页时按了返回（默认 `Navigator.maybePop`）
  final VoidCallback? onExit;

  /// 导航栏右侧（对应微信的胶囊按钮位置）
  final List<Widget>? actions;

  static Color? _color(String? hex) {
    if (hex == null || !hex.startsWith('#')) return null;
    var h = hex.substring(1);
    if (h.length == 3) h = h.split('').map((c) => '$c$c').join();
    final v = int.tryParse(h, radix: 16);
    if (v == null) return null;
    return h.length == 8 ? Color(v) : Color(0xFF000000 | v);
  }

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<MiniPage?>(
      valueListenable: controller.page,
      builder: (context, page, _) {
        final canGoBack = (page?.depth ?? 0) > 1;
        final fg = page?.lightForeground == true ? Colors.white : Colors.black;
        // 第一条 page 事件到之前也先占着导航栏：多数页面有导航栏，先不占的话画面会先按
        // 整屏建引擎、导航栏一出来再缩一次（多一次整页重排）
        final bar = page != null && page.customNavigation
            ? null
            : AppBar(
                title: Row(mainAxisSize: MainAxisSize.min, children: [
                  if (page?.loading == true)
                    Padding(
                      padding: const EdgeInsets.only(right: 8),
                      child: SizedBox.square(dimension: 14, child: CircularProgressIndicator(strokeWidth: 2, color: fg)),
                    ),
                  Flexible(child: Text(page?.title ?? '', overflow: TextOverflow.ellipsis)),
                ]),
                centerTitle: true,
                backgroundColor: _color(page?.backgroundColor) ?? Colors.white,
                foregroundColor: fg,
                elevation: 0,
                scrolledUnderElevation: 0,
                automaticallyImplyLeading: false,
                leading: canGoBack ? BackButton(onPressed: controller.back) : null,
                actions: actions,
              );
        return PopScope(
          canPop: !canGoBack,
          onPopInvokedWithResult: (didPop, _) {
            if (!didPop) controller.back();
          },
          child: Scaffold(
            appBar: bar,
            // 键盘弹起时不缩画面（缩的话键盘动画每一帧都要重建引擎布局），
            // 由 MiniProgramView 把画面上移露出输入框
            resizeToAvoidBottomInset: false,
            body: MiniProgramView(
              controller: controller,
              nativeViewBuilder: nativeViewBuilder,
              backgroundColor: _color(page?.window['backgroundColor'] as String?) ?? Colors.white,
            ),
          ),
        );
      },
    );
  }
}
