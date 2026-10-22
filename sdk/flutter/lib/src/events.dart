import 'dart:ui' show Rect;

import 'package:flutter/foundation.dart' show immutable;

/// 当前页面（引擎每次换页 / 栈深变化发一次）。
class MiniPage {
  const MiniPage({
    required this.route,
    required this.depth,
    required this.isTabBar,
    required this.window,
    this.tabBarBackgroundColor,
  });

  factory MiniPage.fromJson(Map<String, dynamic> j) => MiniPage(
        route: j['route'] as String? ?? '',
        depth: (j['depth'] as num?)?.toInt() ?? 0,
        isTabBar: j['isTabBar'] == true,
        window: (j['window'] as Map?)?.cast<String, dynamic>() ?? const {},
        tabBarBackgroundColor: j['tabBarBackgroundColor'] as String?,
      );

  /// `pages/index/index`
  final String route;

  /// 页面栈深度；`> 1` 时系统返回键应交给小程序
  final int depth;
  final bool isTabBar;

  /// tabBar 页的 tabBar 底色（`#rrggbb`，含 `setTabBarStyle` 改过的）；非 tabBar 页为 null
  final String? tabBarBackgroundColor;

  /// app.json 的 `window` ← 页面 json ← `wx.setNavigationBarTitle/Color` 合并后的窗口配置
  final Map<String, dynamic> window;

  String get title => window['navigationBarTitleText'] as String? ?? '';

  /// `navigationStyle: custom` 时页面自己画导航栏，宿主不该再画
  bool get customNavigation => window['navigationStyle'] == 'custom';

  /// 导航栏文字是白色（`navigationBarTextStyle: white`）
  bool get lightForeground => window['navigationBarTextStyle'] == 'white';
  String? get backgroundColor => window['navigationBarBackgroundColor'] as String?;
  bool get loading => window['navigationBarLoading'] == true;
}

/// 输入框聚焦：宿主据此弹对应类型的软键盘
class MiniInputFocus {
  const MiniInputFocus({
    required this.id,
    required this.value,
    this.cursor = -1,
    required this.inputType,
    required this.password,
    required this.maxLength,
    required this.confirmType,
    required this.confirmHold,
    required this.multiline,
    required this.rect,
  });

  factory MiniInputFocus.fromJson(Map<String, dynamic> j) {
    final r = (j['rect'] as Map?)?.cast<String, dynamic>() ?? const {};
    double d(String k) => (r[k] as num?)?.toDouble() ?? 0;
    final value = j['value'] as String? ?? '';
    final multiline = j['tag'] == 'textarea';
    return MiniInputFocus(
      id: j['id'] as String? ?? '',
      value: value,
      // 旧版引擎不带 cursor：当成在末尾（与从前的假设一致）
      cursor: (j['cursor'] as num?)?.toInt() ?? value.runes.length,
      inputType: j['inputType'] as String? ?? 'text',
      password: j['password'] == true,
      maxLength: (j['maxlength'] as num?)?.toInt() ?? 140,
      confirmType: j['confirmType'] as String? ?? (multiline ? 'return' : 'done'),
      confirmHold: j['confirmHold'] == true,
      multiline: multiline,
      rect: Rect.fromLTWH(d('x'), d('y'), d('width'), d('height')),
    );
  }

  final String id;
  final String value;

  /// 引擎里的光标位置（**Unicode 码点**下标，不是 UTF-16）：点在文字中间时就在那里
  final int cursor;

  /// `text` / `number` / `digit` / `idcard` / `safe-password` / `nickname`
  final String inputType;
  final bool password;
  final int maxLength;

  /// `done` / `send` / `search` / `next` / `go`；textarea 还有 `return`（默认：右下角是换行键）
  final String confirmType;
  final bool confirmHold;
  final bool multiline;

  /// 视口逻辑坐标
  final Rect rect;
}

/// 一个由宿主承载的原生组件（`video` / `map` / `web-view` …）在视口里的位置，
/// 以及它被引擎自己画的层（picker 面板、弹窗、Toast、fixed 元素、tabBar）挡住了多少。
///
/// 控件按 [rect] 摆（完整尺寸，被挡住时也不缩放），只在 [clip] 里、[holes] 之外画和收触摸，
/// 上面再叠一层不透明度为 [mask] 的黑色。[MiniProgramView] 已经照这个做了，自己摆控件时照抄。
class MiniNativeView {
  const MiniNativeView({
    required this.key,
    required this.tag,
    required this.rect,
    required this.attrs,
    required this.fixed,
    Rect? clip,
    this.holes = const [],
    this.mask = 0,
  }) : clip = clip ?? rect;

  factory MiniNativeView.fromJson(Map<String, dynamic> j) {
    Rect r(Map<String, dynamic> m) {
      double d(String k) => (m[k] as num?)?.toDouble() ?? 0;
      return Rect.fromLTWH(d('x'), d('y'), d('width'), d('height'));
    }

    final rect = r(j);
    final clip = j['clip'];
    return MiniNativeView(
      key: j['key'] as String? ?? '',
      tag: j['tag'] as String? ?? '',
      rect: rect,
      attrs: (j['attrs'] as Map?)?.map((k, v) => MapEntry('$k', '$v')) ?? const {},
      fixed: j['fixed'] == true,
      // 旧版引擎不报遮挡：当成整个露在外面
      clip: clip is Map ? r(clip.cast<String, dynamic>()) : rect,
      holes: [
        for (final h in (j['holes'] as List? ?? const []))
          if (h is Map) r(h.cast<String, dynamic>()),
      ],
      mask: (j['mask'] as num?)?.toDouble() ?? 0,
    );
  }

  /// 稳定键：`id` 属性，没有就是 `标签#序号`。回传事件用它（[MiniProgramController.sendNativeViewEvent]）
  final String key;
  final String tag;

  /// 视口逻辑坐标（已减滚动）
  final Rect rect;

  /// 节点属性（`src` / `autoplay` / `latitude` …，模板求值之后）
  final Map<String, String> attrs;
  final bool fixed;

  /// 还露在外面的部分（视口逻辑坐标）。为空 = 整个被挡住：这时控件**别销毁**
  /// （播放进度在里面），只是不画、不收触摸
  final Rect clip;

  /// [clip] 里还被引擎的层盖着的块（居中的弹窗盒子、Toast、fixed 元素），要挖掉
  final List<Rect> holes;

  /// 盖在控件上的半透明黑遮罩的不透明度（picker 面板、showModal）。`> 0` 时控件不收触摸：
  /// 遮罩吃掉触摸，点它是关面板
  final double mask;

  /// 整个被挡住了
  bool get hidden => clip.isEmpty;
}

/// 点了右上角胶囊的哪一半（引擎事件 `menuButton` 的 `action`）
enum MiniMenuButtonAction {
  /// 左半「•••」：查看关于（展示什么由宿主决定，信息见 [MiniProgramController.about]）
  menu,

  /// 右半「⊙」：返回宿主
  close;

  static MiniMenuButtonAction? parse(Object? s) => switch (s) {
        'menu' => menu,
        'close' => close,
        _ => null,
      };
}

/// 小程序的「关于」信息：引擎启动后第一帧的 `app` 事件带上来（字段见引擎的 `src/app_about.rs`）
@immutable
class MiniAbout {
  const MiniAbout({
    required this.name,
    required this.appId,
    required this.description,
    required this.entryPage,
    required this.pageCount,
    required this.engine,
  });

  factory MiniAbout.fromJson(Map<String, dynamic> j) => MiniAbout(
        name: j['name'] as String? ?? '',
        appId: j['appId'] as String? ?? '',
        description: j['description'] as String? ?? '',
        entryPage: j['entryPage'] as String? ?? '',
        pageCount: (j['pageCount'] as num?)?.toInt() ?? 0,
        engine: j['engine'] as String? ?? '',
      );

  /// `project.config.json` 的 `projectname`，没有就是 app.json 的导航栏标题 / 目录名
  final String name;

  /// 没设置（或开发者工具的游客 `touristappid`）时为空
  final String appId;

  /// 开发者工具写的默认描述不算
  final String description;
  final String entryPage;

  /// 主包 + 分包的页面数
  final int pageCount;

  /// `mini-render 0.1.0`
  final String engine;

  /// 演示用的弹框标题与正文（与桌面窗体、H5、原生 SDK 的同一份）
  String get dialogTitle => name.isEmpty ? '关于小程序' : '关于$name';

  String get dialogMessage => [
        if (description.isNotEmpty) description,
        'AppID：${appId.isEmpty ? '未设置' : appId}',
        '入口：$entryPage（共 $pageCount 页）',
        '引擎：$engine',
      ].join('\n');
}

/// 小程序调用了宿主实现的 `wx.*`
class MiniHostCall {
  const MiniHostCall({required this.id, required this.api, required this.args});

  factory MiniHostCall.fromJson(Map<String, dynamic> j) => MiniHostCall(
        id: (j['id'] as num?)?.toInt() ?? 0,
        api: j['api'] as String? ?? '',
        args: (j['args'] as Map?)?.cast<String, dynamic>() ?? const {},
      );

  final int id;
  final String api;
  final Map<String, dynamic> args;
}

/// 宿主实现的 `wx.*`：返回值是交给 `success` 回调的 res；抛 [MiniHostApiError] 走 `fail`
typedef MiniHostApi = Future<Map<String, dynamic>> Function(Map<String, dynamic> args);

/// 宿主 API 失败（走 `fail` 回调，`errMsg` 原样给小程序）
class MiniHostApiError implements Exception {
  const MiniHostApiError(this.errMsg, [this.extra = const {}]);
  final String errMsg;
  final Map<String, dynamic> extra;

  @override
  String toString() => 'MiniHostApiError($errMsg)';
}

/// 渲染后端偏好（[MiniProgramController.backend]）。
///
/// 只有 Android 有 GPU 上屏（引擎的 wgpu 合成器直接画进 SurfaceProducer 的 Surface）；
/// iOS 的外部纹理是 CVPixelBuffer，恒为 CPU。GPU 的输出与 CPU 逐字节相同（启动自检守着），
/// 选不上或运行中出错都会自动退回 CPU，见 [MiniProgramController.activeBackend]。
enum MiniBackend {
  /// 按机型分级与启动自检自动选（自检里 GPU 不比 CPU 快也会选 CPU）
  auto(0),

  /// 只用 CPU（默认，与原生 SDK 一致）
  cpu(1),

  /// 强制 GPU（仍要过启动自检；不过就退回 CPU）
  gpu(2);

  const MiniBackend(this.code);

  /// 引擎的 `MR_BACKEND_*`
  final int code;
}

/// 实际在用的后端与原因（例如「偏好 cpu（宿主配置）」「自动选择」「运行中回退：…」）
@immutable
class MiniBackendStatus {
  const MiniBackendStatus({required this.gpu, required this.reason});

  /// `pixelWidth` 那份纹理信息里带的 `backend`（1 CPU / 2 GPU）与 `backendReason`
  factory MiniBackendStatus.fromTextureInfo(Map<dynamic, dynamic> m) => MiniBackendStatus(
        gpu: (m['backend'] as num?)?.toInt() == MiniBackend.gpu.code,
        reason: '${m['backendReason'] ?? ''}',
      );

  final bool gpu;
  final String reason;

  @override
  bool operator ==(Object other) => other is MiniBackendStatus && other.gpu == gpu && other.reason == reason;

  @override
  int get hashCode => Object.hash(gpu, reason);

  @override
  String toString() => '${gpu ? 'GPU' : 'CPU'}（$reason）';
}
