import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'capsule.dart';
import 'controller.dart';
import 'events.dart';
import 'view.dart';

/// 小程序的宿主外壳：导航栏（标题 / 颜色 / 返回键 / loading / 右上角胶囊）+ 系统返回键 + 画面。
///
/// 微信里导航栏是**原生外壳**，不是页面画的；引擎也不画它。这里按页面的窗口配置
/// （app.json `window` ← 页面 json ← `wx.setNavigationBarTitle/Color`）用 AppBar 画，
/// `navigationStyle: custom` 的页面不画（页面自己画，画面顶到状态栏下面）。
///
/// 胶囊：普通页面画在 AppBar 右侧（[MiniCapsule]）；custom 页面没有 AppBar，由引擎画在画面里
/// （位置就是 `wx.getMenuButtonBoundingClientRect()`）。两处点击都走 [MiniProgramController.menuButtonTaps]，
/// 这里接成 [onMenu] / [onExit]。
///
/// 系统返回键 / 侧滑返回：页面栈多于一页时交给小程序返回，只剩一页才放行给宿主。
class MiniProgramScaffold extends StatefulWidget {
  const MiniProgramScaffold({
    super.key,
    required this.controller,
    this.nativeViewBuilder,
    this.onExit,
    this.onMenu,
    this.actions,
  });

  final MiniProgramController controller;
  final MiniNativeViewBuilder? nativeViewBuilder;

  /// 返回宿主：点了胶囊的「⊙」，或页面栈只剩一页时按了返回。
  /// 不给时胶囊的「⊙」直接关掉这个页面（不管小程序里叠了几页，回到打开小程序的那一页；
  /// 小程序本身是根页面时退出 App），返回键照常交给系统
  final VoidCallback? onExit;

  /// 点了胶囊的「•••」：查看关于。展示什么由宿主决定（信息在 `controller.about`）；
  /// 不给就什么也不做（例子 `example/` 里弹了一个关于框）
  final VoidCallback? onMenu;

  /// 导航栏右侧。不给时是胶囊（[MiniProgramController.menuButton] 关掉时什么也没有）
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
  State<MiniProgramScaffold> createState() => _MiniProgramScaffoldState();
}

class _MiniProgramScaffoldState extends State<MiniProgramScaffold> {
  StreamSubscription<MiniMenuButtonAction>? _taps;

  @override
  void initState() {
    super.initState();
    _taps = widget.controller.menuButtonTaps.listen(_onMenuButton);
  }

  @override
  void didUpdateWidget(MiniProgramScaffold old) {
    super.didUpdateWidget(old);
    if (!identical(old.controller, widget.controller)) {
      _taps?.cancel();
      _taps = widget.controller.menuButtonTaps.listen(_onMenuButton);
    }
  }

  @override
  void dispose() {
    _taps?.cancel();
    super.dispose();
  }

  void _onMenuButton(MiniMenuButtonAction a) {
    if (!mounted) return;
    switch (a) {
      case MiniMenuButtonAction.menu:
        widget.onMenu?.call();
      case MiniMenuButtonAction.close:
        final exit = widget.onExit;
        if (exit != null) {
          exit();
          return;
        }
        // 「⊙」是返回宿主，不是退一页：不能用 maybePop —— 它要过下面的 PopScope，
        // 页面栈多于一页时被拦成 controller.back()，只退回小程序上一页（真机上踩过）。
        // pop() 不问 PopScope；小程序就是宿主的根页面时，返回宿主就是退出 App
        final nav = Navigator.of(context);
        if (nav.canPop()) {
          nav.pop();
        } else {
          SystemNavigator.pop();
        }
    }
  }

  @override
  Widget build(BuildContext context) {
    final controller = widget.controller;
    return ValueListenableBuilder<MiniPage?>(
      valueListenable: controller.page,
      builder: (context, page, _) {
        final canGoBack = (page?.depth ?? 0) > 1;
        final fg = page?.lightForeground == true ? Colors.white : Colors.black;
        final pageBg = MiniProgramScaffold._color(page?.window['backgroundColor'] as String?) ?? Colors.white;
        // Android 系统导航栏那一截（view.dart `_reservedBottom`）的底色：tabBar 页接 tabBar 的底色
        final navAreaBg = MiniProgramScaffold._color(page?.tabBarBackgroundColor) ?? pageBg;
        final actions = widget.actions ??
            (controller.menuButton
                ? [
                    MiniCapsule(
                      dark: page?.lightForeground == true,
                      onMenu: () => controller.handleMenuButton(MiniMenuButtonAction.menu),
                      onClose: () => controller.handleMenuButton(MiniMenuButtonAction.close),
                    ),
                    // 微信的胶囊距右 7
                    const SizedBox(width: 7),
                  ]
                : null);
        // 导航栏由引擎画（controller.navigationBar，默认）：这里不画 AppBar，画面一直铺满整屏，
        // 切页不改尺寸。旧做法（宿主画）：第一条 page 事件到之前也先占着导航栏 —— 多数页面有导航栏，
        // 先不占的话画面会先按整屏建引擎、导航栏一出来再缩一次（多一次整页重排）
        final bar = controller.navigationBar || (page != null && page.customNavigation)
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
                backgroundColor: MiniProgramScaffold._color(page?.backgroundColor) ?? Colors.white,
                foregroundColor: fg,
                elevation: 0,
                scrolledUnderElevation: 0,
                automaticallyImplyLeading: false,
                leading: canGoBack ? BackButton(onPressed: controller.back) : null,
                actions: actions,
              );
        return PopScope(
          // 只剩一页时：给了 onExit 就由它退出，否则放行给系统
          canPop: !canGoBack && widget.onExit == null,
          onPopInvokedWithResult: (didPop, _) {
            if (didPop) return;
            if (canGoBack) {
              controller.back();
            } else {
              widget.onExit?.call();
            }
          },
          child: AnnotatedRegion<SystemUiOverlayStyle>(
            // 引擎画导航栏时状态栏压在引擎画面上：图标颜色跟 navigationBarTextStyle 走（AppBar 本来替我们做了这件事）
            value: (page?.lightForeground == true ? SystemUiOverlayStyle.light : SystemUiOverlayStyle.dark).copyWith(
              statusBarColor: const Color(0x00000000),
              // Android 导航键压在留出来的那一截上：关掉系统的半透明灰罩（三键导航默认加），
              // 让它就是 navAreaBg 的颜色（微信安卓的导航键底也是跟着页面走的纯色），图标按底色深浅反色
              systemNavigationBarColor: const Color(0x00000000),
              systemNavigationBarContrastEnforced: false,
              systemNavigationBarIconBrightness:
                  navAreaBg.computeLuminance() > 0.5 ? Brightness.dark : Brightness.light,
            ),
            child: Scaffold(
              appBar: bar,
              // 键盘弹起时不缩画面（缩的话键盘动画每一帧都要重建引擎布局），
              // 由 MiniProgramView 把画面上移露出输入框
              resizeToAvoidBottomInset: false,
              body: MiniProgramView(
                controller: controller,
                nativeViewBuilder: widget.nativeViewBuilder,
                backgroundColor: pageBg,
                navigationBarAreaColor: navAreaBg,
              ),
            ),
          ),
        );
      },
    );
  }
}
