import 'dart:math' as math;
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
    this.adjustPosition = true,
    this.cursorSpacing = 0,
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
      adjustPosition: j['adjustPosition'] != false,
      cursorSpacing: (j['cursorSpacing'] as num?)?.toDouble() ?? 0,
    );
  }

  /// 键盘弹起时把画面顶上去（`adjust-position`，默认 true）
  final bool adjustPosition;

  /// 输入框底边与键盘之间留多少（`cursor-spacing`，逻辑像素）
  final double cursorSpacing;

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

  /// 页面又滚了 [dy]（逻辑像素，正 = 往下）之后该摆在哪：[rect] 跟着挪，[clip] 里贴着控件自己边的
  /// 那几条边跟着挪、贴着遮挡物（导航栏、吸底栏、面板）的边留在原处，再裁进可见区 [area]；
  /// [holes] 是视口里不动的遮挡物，原样不动。`fixed` 的控件不随页面滚，原样返回。
  MiniNativeView shifted(double dy, Rect? area) {
    if (fixed || dy == 0) return this;
    final r = rect.translate(0, dy);
    var c = clip;
    if (!c.isEmpty) {
      bool near(double a, double b) => (a - b).abs() < 0.5;
      c = Rect.fromLTRB(
        c.left,
        near(c.top, rect.top) ? r.top : c.top,
        c.right,
        near(c.bottom, rect.bottom) ? r.bottom : c.bottom,
      ).intersect(r);
      if (area != null) c = c.intersect(area);
      if (c.width <= 0 || c.height <= 0) c = Rect.fromLTWH(r.left, r.top, 0, 0);
    }
    return MiniNativeView(key: key, tag: tag, rect: r, attrs: attrs, fixed: fixed, clip: c, holes: holes, mask: mask);
  }
}

/// `nativeViews` 带的这一帧滚动取样：宿主按**自己那一帧**的 vsync 推算页面此刻滚到了哪，
/// 而不是等事件到了才挪控件（引擎原理见引擎的 `src/host/native_motion.rs`）。
///
/// 引擎画面与宿主控件走两条上屏路：Android 上引擎画到自己的 SurfaceView（系统合成器直接取），
/// 控件的位置要经事件 → 主线程 → Dart → Flutter 自己的下一帧。只按事件挪，视频框比页面落后
/// 1~3 个 vsync、而且每帧落后几个不固定，看着就是「抖、不跟着页面走」。
@immutable
class MiniNativeMotion {
  const MiniNativeMotion({
    required this.frameTimeMs,
    required this.scroll,
    required this.velocity,
    required this.periodMs,
    required this.dpr,
    this.decay = 0,
    this.area,
  });

  /// 旧版引擎不带这些字段时返回 null（控件就按事件里的位置摆）
  static MiniNativeMotion? fromJson(Map<String, dynamic> j) {
    final t = j['frameTime'], s = j['scroll'], v = j['velocity'], p = j['period'];
    if (t is! num || s is! num || v is! num || p is! num || p <= 0) return null;
    final a = j['area'];
    double d(Map m, String k) => (m[k] as num?)?.toDouble() ?? 0;
    return MiniNativeMotion(
      frameTimeMs: t.toDouble(),
      scroll: s.toDouble(),
      velocity: v.toDouble(),
      periodMs: p.toDouble(),
      dpr: (j['dpr'] as num?)?.toDouble() ?? 1,
      decay: (j['decay'] as num?)?.toDouble() ?? 0,
      area: a is Map ? Rect.fromLTWH(d(a, 'x'), d(a, 'y'), d(a, 'width'), d(a, 'height')) : null,
    );
  }

  /// 这一帧取样用的 vsync 时刻（毫秒；Android 与 `SchedulerBinding.currentSystemFrameTimeStamp` 同一个单调钟）
  final double frameTimeMs;

  /// 页面滚动位置（逻辑像素，未取整；控件的 `y` 是按设备像素截断后的值算的）
  final double scroll;

  /// 滚动速度（逻辑像素 / 毫秒；正 = 内容往上走）
  final double velocity;

  /// vsync 周期（毫秒）
  final double periodMs;
  final double dpr;

  /// 速度的指数衰减率（每毫秒）。惯性时 > 0（`v(t) = v₀·e^(−decay·t)`，与引擎的滚动控制器同一条曲线），
  /// 拖动时 0（按匀速外推）
  final double decay;

  /// 正常流的可见区（视口逻辑坐标：导航栏以下、tabBar 以上），推算后的 `clip` 裁进这里
  final Rect? area;

  /// 推算最多往前推几个 vsync：再远就不是「事件晚到」而是引擎停了，屏上就是最后那一帧
  static const maxAheadFrames = 4;

  /// 在 vsync [vsyncMs] 上屏的那一帧（宿主比引擎晚上屏 [lagFrames] 个 vsync），控件相对事件里的位置
  /// 还要往下挪多少（逻辑像素）。速度为 0、时钟对不上时为 0。
  ///
  /// 按 vsync 栅格数帧（取整）：两边跟的是同一块屏的 vsync，宿主给引擎的时刻又是整数毫秒，
  /// 不取整的话每帧多推少推零点几毫秒，正是要消掉的那种抖。位置按设备像素截断（与引擎合成一致）。
  double shiftAt(double vsyncMs, double lagFrames) {
    if (velocity == 0) return 0;
    final frames = ((vsyncMs - frameTimeMs) / periodMs + lagFrames).roundToDouble();
    // 时钟对不上（不是同一个单调钟）或隔太久：不推
    if (frames < -2 || frames > maxAheadFrames + lagFrames.abs() + 2) return 0;
    final ahead = frames.clamp(-2.0, maxAheadFrames + lagFrames).toDouble();
    double snap(double s) => (s * dpr).truncateToDouble() / dpr;
    return snap(scroll) - snap(scroll + distance(ahead));
  }

  /// 往后 [frames] 个 vsync 页面再滚多少（逻辑像素）。惯性按引擎逐帧的离散衰减求和
  /// （每帧先衰减再走：`Σ v·dᶦ·P`，d = e^(−decay·P)），拖动按匀速。
  /// 惯性要是也按匀速推，四五帧下来冲过头好几个像素（衰减的时间常数才 200ms）
  double distance(double frames) {
    if (decay <= 0 || frames <= 0) return velocity * frames * periodMs;
    final d = math.exp(-decay * periodMs);
    return velocity * periodMs * d * (1 - math.pow(d, frames)) / (1 - d);
  }
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
  const MiniHostCall({required this.id, required this.api, required this.args, this.tempDir = '', this.files = const {}});

  factory MiniHostCall.fromJson(Map<String, dynamic> j) => MiniHostCall(
        id: (j['id'] as num?)?.toInt() ?? 0,
        api: j['api'] as String? ?? '',
        args: (j['args'] as Map?)?.cast<String, dynamic>() ?? const {},
        tempDir: j['tempDir'] as String? ?? '',
        files: {
          for (final e in ((j['files'] as Map?) ?? const {}).entries) '${e.key}': '${e.value}',
        },
      );

  final int id;
  final String api;
  final Map<String, dynamic> args;

  /// 宿主可以直接写的临时目录（小程序沙盒的 `wxfile://tmp/`）。选到的文件写进这里、回包给
  /// `file://<tempDir>/<名>`，引擎只换成虚拟路径不再拷贝；写在别处的 `file://` 引擎会拷一份进来
  final String tempDir;

  /// [args] 里出现的 `wxfile://` 虚拟路径 → 真实路径（`saveImageToPhotosAlbum({filePath})` 要读的文件）。
  /// 文件不存在的不列
  final Map<String, String> files;
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
