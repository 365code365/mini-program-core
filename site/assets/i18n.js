/* 落地页的中英切换。
 *
 * 页面**本身就是英文**（默认语言写在 HTML 里，没有 JS 也能读全文、也能被爬到），
 * 中文放在下面这张表里，切换时替换 innerHTML。为什么不做成两个 HTML 文件：
 * 那样每改一处版式都要改两遍，迟早会漂。这里结构只有一份，两种语言只差文案。
 *
 * 三条约定：
 *   1. 需要翻译的元素打 `data-i18n="键"`，属性用 `data-i18n-alt` / `data-i18n-aria`；
 *   2. 英文原文不写进表里 —— 初始化时从 DOM 里抓一份存着，切回来直接还原；
 *   3. 表里的键必须与 HTML 里的一一对应，多一个少一个都由 tools/site-i18n-check.py 报出来。
 *
 * 语言选择顺序：URL 的 `?lang=` → localStorage 里上次的选择 → 默认英文。
 * （不按浏览器语言自动切：默认英文是明确要求，自动切会让分享出去的链接看起来不一致。）
 */
(function () {
  'use strict';

  var ZH = {
    /* ── 通用 UI ── */
    'ui.skip': '跳到正文',
    'ui.langAria': '语言',
    'ui.copy': '复制',
    'ui.copied': '已复制',
    'ui.copyFail': '复制失败',
    'ui.copyAria': '复制这段代码',
    'theme.aria': '背景主题',
    'theme.dark': '深色',
    'theme.midnight': '午夜蓝',
    'theme.light': '浅色',
    'theme.auto': '跟随系统',
    'meta.title': 'mini-render — 用 Rust 从零写的小程序渲染引擎',
    'meta.desc': '对标微信 Skyline 的自绘小程序渲染引擎：官方 503 个 wx.* API 全在、Skyline 组件集补齐，不基于 WebView、不映射系统控件、不依赖 Skia。小程序源码进、像素出 —— 桌面、无头 CI、Android、iOS、Flutter 同一份代码。',

    /* ── 导航 ── */
    'nav.aria': '页面导航',
    'nav.skyline': 'Skyline',
    'nav.pipeline': '渲染管线',
    'nav.device': '真机',
    'nav.showcase': '演示',
    'nav.games': '小游戏',
    'nav.gallery': '场景',
    'nav.perf': '实测',
    'nav.start': '快速开始',
    'nav.host': "宿主接口",
    'nav.scope': '能力边界',
    'nav.docs': '文档',

    /* ── 全平台 ── */
    'nav.platforms': '全平台',
    'hero.harmony': '鸿蒙（C ABI）',
    'plat.eyebrow': '一份引擎，各端接入',
    'plat.h2': '手机、桌面、CI、网页 ——<br><span class="grad">同一份像素</span>',
    'plat.lede': '引擎只要一块可写的像素缓冲（或一块 GPU surface）和一个时钟。下面每个平台跑的是同一份 Rust 代码、同一套布局、同一个 <code>wx.*</code> 运行时，只有薄薄一层宿主不同。所有平台上 CPU 渲染都是逐字节的参考结果。',
    'plat.note': '状态只写我们自己验证过的：Android 在一加 90Hz 真机和模拟器上；iOS SDK 能编真机与模拟器，Metal 通路有测试覆盖；桌面窗口与无头快照每天在 macOS 上跑。Windows、Linux 用的是同一套 winit / wgpu 代码，但还没有 CI。鸿蒙有写进集成指南的 C ABI 接入路径（XComponent + <code>OH_NativeWindow</code>），打包好的 ArkTS SDK 还在做。',
    'plat.harmony.codeAria': 'ArkTS 接入示意',
    'plat.ios.st': 'SDK 可用',
    'plat.ios.how': 'Swift Package / CocoaPods · <code>MiniProgramView</code>（UIKit）',
    'plat.ios.gpu': 'CAMetalLayer → Metal · 失败回退 CPU',
    'plat.ios.alt': '引擎渲染的 Skyline 演示页',
    'plat.android.st': '真机验证 · 90Hz',
    'plat.android.how': 'AAR · Kotlin <code>MiniProgramView</code>（SurfaceView）',
    'plat.android.gpu': 'wgpu Vulkan / GLES · 失败回退 CPU',
    'plat.android.alt': '一加手机上的组件页',
    'plat.harmony.st': 'C ABI 接入 · SDK 制作中',
    'plat.harmony.how': 'C ABI <code>include/mini_render.h</code> + NAPI',
    'plat.harmony.gpu': 'XComponent 原生窗口 · CPU 直写',
    'plat.macos.st': '桌面窗口',
    'plat.macos.how': 'Rust crate · winit 窗口 · 或 C ABI',
    'plat.macos.gpu': 'wgpu Metal · 失败回退 CPU',
    'plat.macos.alt': '桌面窗口里的电商首页',
    'plat.windows.st': '同一份代码 · 未进 CI',
    'plat.windows.how': 'Rust crate · winit 窗口 · 或 C ABI（DLL）',
    'plat.windows.gpu': 'wgpu DX12 / Vulkan · 失败回退 CPU',
    'plat.windows.alt': '桌面窗口里的健康看板',
    'plat.linux.st': '桌面 + 无头 · 未进 CI',
    'plat.linux.how': 'Rust crate · winit 窗口 · 无头出 PNG',
    'plat.linux.gpu': 'wgpu Vulkan · CI 里走 CPU',
    'plat.flutter.st': '插件 · Android + iOS',
    'plat.flutter.how': '<code>mini_render</code> 插件 · Texture / PlatformView',
    'plat.flutter.gpu': '底下是同一个引擎库',
    'plat.flutter.alt': 'Flutter 示例里的网络信息流',
    'plat.web.st': '编译预览',
    'plat.web.how': '把小程序编译成静态 H5',
    'plat.web.gpu': '与原生渲染逐像素对比',
    'plat.web.alt': '编译成 H5 的新闻信息流',
    /* ── HERO ── */
    'hero.eyebrow': 'Rust · 自绘 · 对标 Skyline',
    'hero.h1': '小程序源码进，<br><span class="grad">像素出</span>。',
    'hero.lede': '一个用 Rust 从零实现的小程序渲染引擎。给它一份微信小程序源码（WXML / WXSS / JS / app.json）和一块可写的像素缓冲，它把界面画出来、把交互跑起来。中间<strong>没有浏览器内核、没有系统控件、没有第三方 2D 图形库</strong> —— 每个像素由 CPU 画，可选的 GPU 后端只做图层合成，结果与 CPU 逐字节相同。官方 <strong>503 个 <code>wx.*</code> API</strong> 都在，Skyline 自己的组件 —— <code>grid-view</code>、手势组件、<code>draggable-sheet</code>、worklet 动画 —— 也走同一条管线。',
    'hero.not1': '不基于 WebView',
    'hero.not2': '不映射系统原生控件',
    'hero.not3': '不依赖 Skia',
    'hero.cta1': '30 秒跑起来',
    'hero.cta2': '看真机表现',
    'hero.runs': '跑在',
    'hero.ci': '无头 CI',
    'hero.shotsAria': '引擎渲染输出示例',
    'hero.alt1': 'Flutter 示例的模拟 HTTP 信息流页面，一加真机截图',
    'hero.alt2': '引擎渲染的电商首页，含轮播、限时秒杀倒计时与推荐流',
    'hero.alt3': 'Flutter 示例的组件目录页，一加真机截图',
    'hero.cap1': '真机 · 信息流',
    'hero.cap2': '桌面 · 电商首页',
    'hero.cap3': '真机 · 组件目录',

    /* ── 数字条 ── */
    'stat.1.k': '商城首页整帧',
    'stat.1.v': '750×1334 物理像素，纯 CPU',
    'stat.2.k': '90Hz 真机屏上重复帧',
    'stat.2.v': '边滑边 setData 的列表，原来 11.2%',
    'stat.3.k': 'GPU 与 CPU',
    'stat.3.v': '个场景逐字节一致，含故障回退',
    'stat.4.k': '与 Chrome 差异',
    'stat.4.v': '同源码 15 页逐像素比',
    'stat.5.k': '自动化测试',
    'stat.5.v': '另有 65 张逐字节可复现的基线',
    'stat.6.k': '官方 wx.* API',
    'stat.6.v': '官方目录里的名字全在，另有 57 个组件标签',

    /* ── 为什么自绘 ── */
    'why.eyebrow': '为什么自绘',
    'why.h2': 'WebView 的痛点不在能不能渲染，<br>而在你<span class="grad">控制不了</span>',
    'why.lede': '内核版本随系统走、同一份 CSS 在不同 Android 上排版不同、首屏要等内核起来、滚动与手势的手感由内核决定、想插一层原生能力就要架桥。自绘把这些变成自己的代码：布局与绘制各端只有一份，帧调度、手势仲裁、内存上限全在手里。',
    'why.caption': '与常见跨端渲染方案的对比',
    'why.th1': '方案',
    'why.th2': '它的做法',
    'why.th3': '本引擎',
    'why.r1.k': '小程序 WebView 渲染层',
    'why.r1.a': 'WXML → DOM，交给浏览器内核排版绘制',
    'why.r1.b': '<span class="tag no">不用</span> DOM 与浏览器内核',
    'why.r2.a': 'JS 描述 → 映射成系统原生控件',
    'why.r2.b': '<span class="tag no">不映射</span> 系统控件，不受各端控件差异摆布',
    'why.r3.a': 'Dart + Skia 自绘',
    'why.r3.b': '<span class="tag near">思路最近</span> 但这里是 Rust + 自研光栅器，无 Skia —— 而且也能作为 Flutter 插件用',
    'why.r4.a': '打包或复用一个浏览器',
    'why.r4.b': '<span class="tag no">不打包</span> 浏览器，arm64 约 40MB（含 GPU 合成器）',
    'why.r5.k': '微信 Skyline',
    'why.r5.a': '原生渲染 + 自绘组件',
    'why.r5.b': '<span class="tag yes">对齐目标</span> 语义、手感、事件模型、API 面与组件集都按它对',
    'why.note': '代价也很实：CSS 覆盖面要自己一条一条补，性能靠 CPU 省着花。各端剩余差异只来自系统字体 —— 要彻底消掉就把字体一起打包（<code>assets/SourceHanSansSC-Regular.otf</code> 就是为这个准备的）。字形改成用到才展开之后，它只占约 <strong>2MB</strong>，与系统字体的 1.4MB 相近。',

    /* ── Skyline 对齐 ── */
    'sky.eyebrow': 'Skyline 对齐',
    'sky.h2': '官方 API 一个不缺，<br><span class="grad">Skyline 组件集补齐</span>',
    'sky.lede': '对照的是微信自己的 API 目录与组件目录，不是挑出来的清单。<strong>503</strong> 个 <code>wx.*</code> 运行时全都在；只有宿主能做的（蓝牙、支付、登录……）在宿主接管之前立刻 fail 并写明原因，<code>canIUse</code> 也如实回答。Skyline 专有的组件 —— <code>grid-view</code>、手势组件、<code>draggable-sheet</code>、worklet 动画 —— 与其它一切走同一套布局、绘制与局部重绘。',
    'sky.c1.k': '官方 wx.* 名字',
    'sky.c1.v': '运行时 516 个键；要宿主的在宿主接管前立刻 fail',
    'sky.c2.k': '组件标签',
    'sky.c2.v': '含 17 个内置组件与 8 个原生组件占位',
    'sky.c3.k': '滚动时地图与页面的错位',
    'sky.c3.v': 'Android 同层渲染，真机实测',
    'sky.g1': '容器与布局<small>12</small>',
    'sky.g2': '文本与媒体<small>8</small>',
    'sky.g3': '表单<small>16</small>',
    'sky.g4': '页面与导航<small>5</small>',
    'sky.g5': 'Skyline 手势<small>8</small>',
    'sky.g6': '原生组件<small>8</small>',
    'sky.leg.new': '2026 年 10 月新加',
    'sky.leg.part': '写明了降级',
    'sky.leg.same': '同层渲染（Android）',
    'sky.leg.ph': '宿主接管前画占位',
    'sky.alt1': '引擎渲染的演示页：grid-view 瀑布流、movable-view、手势组件、draggable-sheet、this.animate 与 worklet',
    'sky.alt2': '引擎渲染的演示页：page-container、root-portal、match-media、audio、editor、盖在原生组件占位上的 cover-view、span',
    'sky.alt3': '引擎渲染的演示页：EventChannel、createWorker、UDP 与 TCP、批量存储、WebAudio、MediaQueryObserver、旧版画布的裁剪与虚线、宿主能力的默认行为',
    'sky.cap1': 'Skyline 组件',
    'sky.cap2': '内置组件',
    'sky.cap3': '新补的 API',
    'sky.k1.h': '引擎自己做的',
    'sky.k1.1': '路由按 <code>app.json</code> 校验、<code>success</code> 在新页 <code>onShow</code> 之后、双向 <code>EventChannel</code>、路由事件、<code>enableAlertBeforeUnload</code>',
    'sky.k1.2': '<code>createTCPSocket</code> / <code>createUDPSocket</code>、<code>getLocalIPAddress</code>、在隔离模块作用域里跑的 <code>createWorker</code>',
    'sky.k1.3': 'WebAudio 节点图（<code>decodeAudioData</code> 原生解码）、<code>createMediaQueryObserver</code>、<code>createOffscreenCanvas</code>、<code>createBufferURL</code>',
    'sky.k1.4': '批量存储与 <code>getStorageInfo</code>、取自系统的 <code>getRandomValues</code>、解析 MP4 得到的 <code>getVideoInfo</code>、旧版画布的 <code>clip</code> / <code>setLineDash</code> / <code>measureText</code>',
    'sky.k2.h': '交给宿主的，只走一条通道',
    'sky.k2.1': '在 <code>hostApis</code>（Flutter、Android、iOS、C ABI）里点名，调用就交给你；不点名就立刻 fail，<code>errMsg</code> 写明宿主可接管',
    'sky.k2.2': '<code>chooseImage</code> / <code>chooseMedia</code> / <code>chooseVideo</code> / <code>chooseMessageFile</code> 与存相册随 Android SDK 和 Flutter 插件自带；桌面窗体开系统文件对话框',
    'sky.k2.3': '回包里 <code>file://</code> 开头的文件导入 <code>wxfile://tmp/</code>；<code>MapContext</code> 等上下文对象的每个方法带着 id 转给宿主；宿主事件经 <code>mr_app_dispatch_event</code> 送到 <code>wx.onXxx</code>',
    'sky.k3.h': '动画与触摸，按 Skyline 来',
    'sky.k3.1': '<code>this.animate</code> / <code>clearAnimation</code>，含滚动驱动的时间线',
    'sky.k3.2': '<code>wx.worklet</code>：共享值与派生值、<code>timing</code> / <code>spring</code> / <code>decay</code> / <code>sequence</code> / <code>repeat</code> / <code>delay</code>、<code>Easing</code>，以及写节点样式的 <code>applyAnimatedStyle</code>',
    'sky.k3.3': '手势组件经 <code>on-gesture-event</code> 报状态、位移与速度；<code>page-container</code> 弹着时，返回先关它',
    'sky.k3.4': '<code>touchmove</code> / <code>touchend</code> / <code>touchcancel</code> 锁定在收到 <code>touchstart</code> 的节点上，与微信、DOM 一致，手指移出去也一样',
    'sky.note': '降级都写明了，不藏：worklet 跑在逻辑线程；手势组件只认单指（scale 恒为 1、force-press 不触发）；<code>movable-view</code> 不能双指缩放；<code>editor</code> 是纯文本；Worker 与页面共用一个 QuickJS；<code>sticky-header</code> / <code>sticky-section</code> 不吸顶，<code>list-view</code> / <code>nested-scroll-*</code> 按普通容器排；WebAudio 只有声源出声，效果节点不改变声音；<code>wx.router</code> 的自定义转场不参与；<code>snapshot.takeSnapshot</code> 失败。每一条都列在 <a href="https://github.com/365code365/mini-program-core/blob/main/doc/%E5%BC%95%E6%93%8E%E6%B5%8B%E8%AF%95%E8%AF%B4%E6%98%8E.md">引擎测试说明</a> 里。',

    /* ── 渲染管线 ── */
    'pipe.eyebrow': '渲染管线',
    'pipe.h2': '一帧是怎么画出来的',
    'pipe.lede': '八步，每步都能单独打开诊断日志。逻辑层与渲染层之间只有<strong>数据快照</strong>这一条通道（<code>setData</code> → <code>Arc&lt;Value&gt;</code>）—— 没有 DOM、没有虚拟节点 diff、没有 GC 压力。',
    'pipe.1.h': '解析',
    'pipe.1.p': 'WXML → 节点树，WXSS → 样式表。手写解析器；CSS 选择器引擎在<strong>解析期就编译</strong>好，含标签 / 类 / #id / <code>*</code> / 属性 / 后代 / 子代与特异性排序。',
    'pipe.2.h': '逻辑层',
    'pipe.2.p': '跑 <code>App</code> / <code>Page</code> / <code>Component</code>，<code>setData</code> 产出数据快照。QuickJS 运行时：CommonJS 模块、<code>Promise</code> / <code>async</code>、定时器、微任务每帧 pump。',
    'pipe.3.h': '模板求值',
    'pipe.3.p': '<code>{{ }}</code> 表达式 + <code>wx:if</code> / <code>wx:for</code> 展开成渲染节点树。<code>class</code> / <code>style</code> 绑定到数组或对象时按 CSS 语义拼接。',
    'pipe.4.h': '样式计算',
    'pipe.4.p': '命中的 CSS 加内联 <code>style</code> 合成计算样式，带继承语义。选择器匹配不分配内存，级联<strong>只借用声明、不拷贝</strong> —— 真机上一次 <code>setData</code> 的整棵重建就是靠这个从约 7ms 降到约 3.5ms。',
    'pipe.5.h': '布局',
    'pipe.5.p': 'taffy 求解 Flexbox 盒模型。<strong>文本节点挂自定义度量函数</strong>，换行与 min/max-content 由真实字形宽度决定；断行行数按字体记住，没变的文字不会量第二遍。',
    'pipe.6.h': '绘制',
    'pipe.6.p': '自研 2D 光栅器：扫描线 + even-odd 填充、4× 超采样抗锯齿、圆角三次贝塞尔逼近、多字体回退、Apple <code>sbix</code> 彩色 emoji、两级图片过滤、渐变、掩膜化 <code>box-shadow</code>、裁剪栈。',
    'pipe.7.h': '动画',
    'pipe.7.p': '<code>@keyframes</code> / <code>transition</code> / <code>wx.createAnimation</code> 在<strong>绘制期</strong>按 vsync 时钟求值。<code>transform</code> / <code>opacity</code> / 颜色只影响绘制不触发重排 → 动画帧成本≈静态帧。',
    'pipe.8.h': '分层合成',
    'pipe.8.p': '正常流 → tabBar → <code>position:fixed</code> 覆盖层，层叠顺序与浏览器一致。<code>setData</code> 之后只重画变了的那块：列表在底部追加只重画从第一处变化往下那截，fixed 按钮出现只重画覆盖层。',
    'pipe.9.h': '上屏',
    'pipe.9.p': '贴到窗口、交给宿主 View、或直接存 PNG。桌面窗体与 Android 自检通过后走 GPU 合成（wgpu 把图层当纹理合成上屏），否则回退 CPU；iOS 原生视图把自己的 CAMetalLayer 交给同一个合成器（Metal）；无头模式与 Flutter 的 iOS 纹理走 CPU 合成 —— 同一份代码，五个平台外加 Flutter。',
    'flow.1': '小程序目录',
    'flow.2': '解析',
    'flow.3': '逻辑层',
    'flow.4': '布局',
    'flow.5': '光栅化',
    'flow.6': '合成',
    'flow.7': 'RGBA 缓冲',
    'pipe.note': '产物是一个普通的 Rust 库（<code>cdylib</code> / <code>staticlib</code> / <code>rlib</code>）。落地条件只有一条：<strong>给我一块可写的像素缓冲</strong> —— 所以 Android、iOS、Windows、macOS、Linux 跑同一份代码，没有屏幕的 CI 里也能直接出 PNG。',

    /* ── 小游戏 ── */
    'game.eyebrow': '新 · 小游戏',
    'game.h2': '不止小程序。<br><span class="grad gm-grad">完整的 3D 小游戏，同一个引擎。</span>',
    'game.lede': '给它一个 <code>game.json</code> + <code>game.js</code> 的包，引擎就切到小游戏模式：一整屏画布直接上屏、跟 vsync 走的 <code>requestAnimationFrame</code>、多点触控、低延迟音频、WebGL 1 / 2 与 <code>WXWebAssembly</code>。three.js、PixiJS、Phaser 不改一行就能跑。下面每一屏都是这个引擎画的。',
    'game.n.island': '萌宠小岛 · 3D 经营',
    'game.n.runner': '3D 跑酷',
    'game.n.bowling': '3D 保龄球',
    'game.n.pets': '萌宠小岛 · 孵蛋',
    'game.n.stack': '3D 叠方块',
    'game.n.flappy': '飞翔小鸟',
    'game.n.shop': '萌宠小岛 · 商店',
    'game.s1.k': 'WebGL 一致性',
    'game.s1.v': 'Khronos CTS 子集，适用的 175 页过 169 页',
    'game.s2.k': 'three.js 对 Chrome',
    'game.s2.u': '像素',
    'game.s2.v': '250 125 个像素里不同的只有这么多，同一个场景',
    'game.s3.k': '真机',
    'game.s3.v': 'three.js、PixiJS、Phaser：屏上重复帧 0',
    'game.s4.k': '30 分钟长稳',
    'game.s4.v': '真机 RSS 增长，不崩、不变慢',
    'game.s5.k': '触摸到游戏',
    'game.s5.v': '中位数，不攒批派发，多点触控',
    'game.s6.k': '整套 WebGL',
    'game.s6.v': 'arm64，跑在系统 GLES 驱动上',
    'game.f1.h': '直接上屏',
    'game.f1.p': 'WebGL 主画布在引擎线程上直接 swap 到 Android 的 <code>SurfaceView</code>。Toast、Modal 在 GPU 上叠在它上面，弹着 Toast 时转动的立方体仍有 57 fps。',
    'game.f2.h': '你的引擎，不用改',
    'game.f2.p': 'three.js、PixiJS、Phaser（WebGL 与 Canvas 两种渲染器）走它们平常的适配层加载。WebGL 2 有 VAO、实例化、uniform 块、3D 纹理；上下文丢失的报告与恢复和微信一样。',
    'game.f3.h': '手指与耳朵',
    'game.f3.p': '每根手指连同完整的历史采样，在下一次 <code>requestAnimationFrame</code> 之前派发。音频走低延迟游戏模式，输出延迟 28~32 ms，失去焦点、来电都处理好了。',
    'game.f4.h': 'WebAssembly 全平台',
    'game.f4.p': '<code>WXWebAssembly</code> 跑在纯 Rust 的解释器 wasmi 上，所以不许 JIT 的 iOS 也能用。内存零拷贝、i64 是 BigInt、陷入变成 <code>RuntimeError</code>；体积约 1.8 MB。',
    'game.f5.h': '一款完整的游戏',
    'game.f5.p': '萌宠小岛是完整的 3D 经营游戏：建造、升级、公示概率的宠物抽卡、每日任务、签到、离线收益，内购与激励视频走宿主 API，存档带版本迁移与校验和。',
    'game.f6.h': '几行就能接入',
    'game.f6.p': '把 <code>MiniGameView</code> 放进 Android 的 Activity 或 Flutter 的页面。它按 <code>game.json</code> 锁方向、退到后台自动暂停，第一帧真正上屏时告诉你，loading 层撤掉时不会黑一下。',
    'game.code': '<code>// Android\nval v = MiniGameView(activity, gameDir, dataDir, """{"appId":"wx…"}""")\nv.onFirstFrame = { loadingCover.hide() }       // 打开时不黑屏\nsetContentView(v)\n\n// Flutter\nMiniGameView(gameDir: dir, options: {\'appId\': id}, onFirstFrame: hideCover)</code>',
    'game.alt.island': '萌宠小岛：有麦田、作坊和村民的 3D 小岛经营',
    'game.alt.runner': '城市街道上的 3D 无尽跑酷',
    'game.alt.bowling': '3D 保龄球道，球瓶正在倒下',
    'game.alt.pets': '萌宠小岛：孵化宠物蛋',
    'game.alt.stack': '3D 叠方块高塔',
    'game.alt.flappy': '绿色水管之间的飞翔小鸟',
    'game.alt.shop': '萌宠小岛：游戏内商店里的钻石包',
    'game.alt.g2048': '2048 合并数字的棋盘',
    'game.enginesAria': '不用改就能跑的引擎',
    'game.fw.h': '横屏也行：<span class="grad gm-grad">拳魂</span>',
    'game.fw.p': '横屏 1 对 1 格斗：虚拟摇杆加四个键、搓招放必杀、能量槽超必杀、打击停顿与震屏、两个场景与电脑对手。引擎等屏幕转成横屏再建游戏，第一帧就是横屏比例。',
    'game.fw.a1': '拳魂：雨夜街头场景里两名格斗家交手',
    'game.fw.a2': '拳魂：画面压暗、打出招式名的超必杀',
    'game.fw.a3': '拳魂：另一位格斗家的超必杀把对手击倒',
    /* ── 真机 ── */
    'show.eyebrow': '演示',
    'show.h2': '十二个小程序，<br><span class="grad">一个引擎，一分钟</span>',
    'show.lede': '聊天、看板、火车票、股票、菜谱……这些复杂的示例小程序跑在 Android SDK 上，逐屏从设备截下来。每一屏都是本引擎画的，不是 WebView。',
    'show.aria': '演示视频',
    'show.play': '播放演示',
    'show.cap': '60 秒 · 1280×720 · Android 模拟器截图，配乐由脚本合成',
    'show.fallback': '这个浏览器在这里放不了视频，直接打开 ↗',
    'dev.eyebrow': '真机',
    'dev.h2': '90Hz 真机上的顺滑，<br><span class="grad">逐帧量出来</span>',
    'dev.lede': '同一个引擎跑在 Flutter 插件和 Android / iOS 原生 View 里。下面的数字来自一加 PLY110（90Hz、Adreno 810、Vulkan）上 Flutter 示例与原生 Android 示例（同一个演示小程序）的 release 包，判据是 SurfaceFlinger 给屏上每一帧打的时间戳，不是 App 自己的日志。',
    'dev.strip.h': '90Hz 下的一秒：滑一个每次滚动都 setData 的列表',
    'dev.strip.b': '优化前',
    'dev.strip.a': '现在',
    'dev.strip.ariaB': '优化前：11.2% 的帧屏上重复',
    'dev.strip.ariaA': '现在：1.5% 的帧屏上重复',
    'dev.strip.note': '每格是一个 vsync，红格是屏上还停在上一张的那一帧。红格的数量按实测比例（先快甩再慢拖），落在哪一格是示意。',
    'dev.m1.h': '边滑边 setData 的列表',
    'dev.m1.p': '页面在 <code>onPageScroll</code> 和 IntersectionObserver 回调里 <code>setData</code>。只慢拖是 6.6% → 0.4%。整页重画的帧：0。',
    'dev.m2.h': '进页面转场',
    'dev.m2.p': '每次 300ms 的进入 / 返回都是 29 帧、0 次重复；起步一步是中位一步的 0.89~1.00 倍，最大一步 1.08 倍。',
    'dev.m3.h': '回桌面再回来',
    'dev.m3.p': '新 Surface 到第一帧。合成器停放而不拆，窗口等引擎交出第一帧才上屏 —— 不闪黑。',
    'dev.m4.h': '冷启动',
    'dev.m4.p': '建引擎到 GPU 就绪：字体与 Vulkan 合成器在引擎创建之前就并行预热。',
    'dev.m5.h': '复杂页面超 8ms 的帧',
    'dev.m5.p': '渐变 + transform 页面慢拖。引擎线程只在交互或连续出帧时绑大核，停 800ms 就放开。',
    'dev.m6.h': 'setData 后的布局重建',
    'dev.m6.p': '最慢那几帧里的典型值。增量布局：没变的子树连同 taffy 节点与布局缓存原样留着，只重建变了的，列表增删也一样。整套测试每次增量后都与整棵重建逐节点比过：0 处不一致。',
    'dev.m7.h': '原生 Android View',
    'dev.m7.p': '同一个引擎库、同样的拖动：视图不再把每帧拷进 Bitmap 再在 <code>onDraw</code> 里画，改由引擎线程直接画进自己的 <code>SurfaceView</code>。首页慢拖：7.1% → 0.4%。',
    'dev.m8.h': '拖动时地图与页面的错位',
    'dev.m8.p': '盖在页面上的 <code>&lt;map&gt;</code> 即使逐帧推算，平均也错开 2.8px（最大 15px）。同层渲染把宿主画好的那一帧画进页面：24 张截图张张 0px。',
    'dev.m9.h': '拖过地图',
    'dev.m9.p': '页面慢拖经过同层渲染的地图时屏上重复帧的比例，两个示例都是。宿主只在地图内容变了时重画，从不按滚动帧重画。',
    'dev.m10.h': '每个字面的内存',
    'dev.m10.p': '中文系统字体从前一加载就把 29352 个字形轮廓全部展开。现在用到哪个字才展开，字体文件 mmap 进来；光栅化结果与上游 fontdue 逐位相同。',
    'dev.m11.h': '进后台后引擎的缓冲',
    'dev.m11.p': '真机按 Home 之后量的。页面画布只留画过的那段条带，合成缓冲放掉，侧滑返回截图无损压缩。Native Heap 74.5 → 57.7MB。',
    'dev.alt': 'Flutter 示例的 WXSS 页面，一加真机截图',
    'dev.cap': '真机截图',
    'dev.feat.h': '移动端宿主自带的',
    'dev.feat.1': 'Android 上引擎画进自己的 <code>SurfaceView</code>，系统合成器直接取，中间不经过 Flutter 的帧',
    'dev.feat.2': '启动自检逐字节一致才走 Vulkan GPU 合成；任何失败或设备丢失回退 CPU',
    'dev.feat.3': '引擎画的导航栏与胶囊按钮、300ms 路由转场、左边缘侧滑返回',
    'dev.feat.4': '拖动按触摸事件时间重采样；动画按 vsync 取样，不按画的那一刻',
    'dev.feat.5': 'Android 上 <code>&lt;video&gt;</code> 与 <code>&lt;map&gt;</code> 同层渲染：宿主解码或画出那一帧，引擎画进页面，于是它跟着页面滚，picker 面板、弹窗、tabBar 照样盖在上面，与微信一致（iOS 仍是盖在页面上）',
    'dev.feat.7': '<code>chooseImage</code> / <code>chooseMedia</code> / <code>chooseVideo</code> / <code>chooseMessageFile</code> 与存相册走系统选择器，Android SDK 与 Flutter 插件自带',
    'dev.feat.6': '静止不耗电：按需出帧；进后台放掉 GPU 纹理、图片缓存与合成缓冲，截图压起来',

    /* ── 双后端 ── */
    'be.eyebrow': '双后端',
    'be.h2': '两个后端，<span class="grad">同一张图</span>',
    'be.lede': 'CPU 光栅器画每一个像素，是唯一的像素参考 —— 没有 GPU 的 CI 和手机出的是同一张图。GPU 后端（wgpu：Metal / Vulkan / DX12 / GLES）只把页面、tabBar、fixed 三个图层当纹理合成上屏，用整数着色器复刻 CPU 的混合算术。',
    'be.t1': 'CPU：光栅 + 合成',
    'be.t2': 'GPU：三张纹理合成',
    'be.t3': '差值',
    'be.diff': '个字节不同',
    'be.p1': '<code>tools/gpu-parity.sh</code> 里 <b>34 / 34</b> 个场景逐字节一致，含运行中注入设备丢失',
    'be.p2': '启动自检与 CPU 逐字节一致才启用 GPU；任何失败当帧回退 CPU，并打一行原因',
    'be.p3': '桌面窗体、Android 与 iOS 原生视图自动选 GPU，无头与 Flutter 的 iOS 纹理走 CPU；<code>--backend</code> / <code>MINI_BACKEND</code> 可强制',
    'be.p4': '弹层（Toast / Modal / 操作菜单 / picker）只上传它盖住的那几行；滚动只是换一片页面纹理上屏',

    /* ── 场景画廊 ── */
    'gal.eyebrow': '场景画廊',
    'gal.h2': '这些都是引擎自己画的',
    'gal.lede': '纯 WXML + WXSS + 数据，375×667 @2x，无浏览器参与。它们同时是<strong>逐字节可复现的回归基线</strong> —— 任何渲染改动先看这 65 张有没有变。指到卡片上可以看整页。',
    'gal.1.alt': '电商首页：搜索栏、轮播图、金刚区、限时秒杀倒计时与商品推荐流',
    'gal.1.cap': '<b>电商首页</b>轮播 + 每秒倒计时 + 骨架动画',
    'gal.2.alt': '资讯信息流：顶部频道横滑与图文列表',
    'gal.2.cap': '<b>资讯信息流</b>频道横滑，纵向惯性滚动',
    'gal.3.alt': '商品详情页：主图、价格、规格与底部操作栏',
    'gal.3.cap': '<b>商品详情</b>固定底栏与滚动内容分层',
    'gal.4.alt': '直播带货页面：视频区、弹幕与商品卡',
    'gal.4.cap': '<b>直播带货</b>视频层 + 覆盖层叠加',
    'gal.5.alt': '健康数据看板：环形进度、柱状图与指标卡',
    'gal.5.cap': '<b>健康看板</b>渐变、环形进度、阴影',
    'gal.6.alt': '表单控件合集：输入框、开关、滑块、复选与单选',
    'gal.6.cap': '<b>表单控件</b>switch / slider / checkbox / radio',
    'gal.7.alt': 'picker 底部选择面板',
    'gal.7.cap': '<b>Picker 面板</b>底部弹层，宿主层实现',
    'gal.8.alt': '带底部 tabBar 的页面，图标与选中态',
    'gal.8.cap': '<b>tabBar</b>独立层，遮罩能压暗它',
    'gal.9.alt': '优惠券弹层，含锯齿票券边缘',
    'gal.9.cap': '<b>优惠券弹层</b>圆角挖洞与渐变票面',
    'gal.10.alt': '资讯正文页：长文排版与评论区',
    'gal.10.cap': '<b>资讯正文</b>字号即时生效，长文换行',
    'gal.11.alt': '签到页面：日历网格与连续签到奖励',
    'gal.11.cap': '<b>签到日历</b>网格布局与状态态样式',
    'gal.12.alt': 'WeUI 图标合集展示',
    'gal.12.cap': '<b>WeUI 图标</b>官方矢量数据，非近似描边',
    'gal.13.alt': '模态对话框与半透明遮罩',
    'gal.13.cap': '<b>模态对话框</b>showModal，按压态配对',
    'gal.14.alt': '输入事件页面：输入框、焦点与事件日志',
    'gal.14.cap': '<b>输入事件</b>focus / input / blur / confirm',
    'gal.15.alt': '个人中心页面：头像、数据统计与设置列表',
    'gal.15.cap': '<b>个人中心</b>收藏与已读跨启动持久化',
    'gal.note': '全部 65 张与素材生成方式见 <a href="https://github.com/365code365/mini-program-core/blob/main/doc/%E5%9C%BA%E6%99%AF%E7%94%BB%E5%BB%8A.md">场景画廊文档</a>。',

    /* ── 实测 ── */
    'perf.eyebrow': '实测数字',
    'perf.h2': '优化前 → 现在',
    'perf.lede': '桌面：375×667 @2x（750×1334 物理像素），CPU 逐像素写出；真机数字见上一节。这些数字是<strong>回归判据</strong>，不是宣传语。',
    'perf.b1': '商城首页整帧<small>轮播 + 每秒倒计时 + 骨架动画</small>',
    'perf.b2': '一次 <code>setData</code> 的重建<small>新旧渲染树并行比对</small>',
    'perf.b6': '<code>onReachBottom</code> 列表追加<small>90Hz 真机；只重画从第一处变化往下那几行</small>',
    'perf.b3': '首帧建树 + 样式<small>三条中文字体栈的应用</small>',
    'perf.b4': '拖动单帧<small>tea-app 首页，60 帧平均</small>',
    'perf.b5': '远程图片二次打开<small>磁盘缓存命中</small>',
    'perf.f1.h': '稳态帧稳得住',
    'perf.f1.p': '144Hz 屏（节拍 6.94ms）上商城首页的帧间隔从 <b>6.9~25ms 抖动</b> 收到 <b>6.7~7.2ms</b>；最慢一帧从 17.9ms 降到 <b>11.4ms</b>。',
    'perf.f2.h': '只重绘变化的那一块',
    'perf.f2.p': '首页秒杀倒计时的失效范围是 <b>45×33 像素</b>，不是整屏。列表变长只重画从第一处变化往下那截，fixed 按钮出现只重画覆盖层。拿不准就退回整帧 —— 留下脏像素比多画一次严重得多。',
    'perf.f3.h': '滚动不重绘',
    'perf.f3.p': '页面画布是整页高的、用内容坐标，滚动只是取不同切片上屏，每帧 <b>2~3ms</b>。只有要滑进来的那一截在露出之前先补画。',
    'perf.cons.h': '双端一致性',
    'perf.cons.p': '同一份源码：一边原生渲染，一边编译成 HTML 用 Chrome 截图，逐像素比。剩余差异集中在粗体字形与亚像素文本位置。',
    'perf.cons.r1k': '<code>sample-app</code> 商城<small>15 页</small>',
    'perf.cons.r1v': '<b>4.57%</b> 变化像素',
    'perf.cons.r2k': '<code>news-app</code> 资讯<small>6 页</small>',
    'perf.cons.r2v': '<b>6.36%</b> 变化像素',
    'perf.real.h': '跑得起来的真实工程',
    'perf.real.p': '不是 demo 级的自造样板。',
    'perf.real.r1k': '<code>tea-app</code><small>36 页</small>',
    'perf.real.r1v': '<b>uni-app</b> 编译到 mp-weixin 的产物：259KB Vue 3 运行时 + 60 个 CommonJS 模块',
    'perf.real.r2k': '<code>real-sample</code><small>8 页</small>',
    'perf.real.r2v': '微信官方 demo',
    'perf.real.r3k': '<code>sample-app</code><small>15 页</small>',
    'perf.real.r3v': '商城闭环：购物车 → 下单 → 订单 / 物流，跨页状态走 storage',
    'perf.real.r4k': '<code>news-app</code><small>6 页</small>',
    'perf.real.r4v': '资讯闭环：频道横滑、字号即时生效、评论、收藏持久化',
    'perf.real.r5k': '演示目录<small>30 页</small>',
    'perf.real.r5v': 'Flutter 示例与原生 Android 示例共用：含 Skyline 组件在内的组件目录、新补的 API、模拟 HTTP 信息流、同层渲染的视频与地图',

    /* ── 关键取舍 ── */
    'trade.eyebrow': '关键取舍',
    'trade.h2': '为什么这么写',
    'trade.lede': '每一条背后都有踩过的坑，反面教材都记在踩坑记录里。',
    'trade.1.h': '文本度量驱动布局，不估算宽度',
    'trade.1.p': '度量上下文挂到 taffy 叶子上，真实字形参与布局，「定宽容器内换行」与「收缩容器被内容撑开」两种语义同时成立。',
    'trade.1.bad': '曾给每个文本盒 <code>+4px</code>「保险余量」，结果所有按内容定宽的徽标都比浏览器宽一圈。',
    'trade.2.h': '动画只重绘不重排',
    'trade.2.p': '<code>@keyframes</code> 里改 <code>width</code> / <code>height</code> 这类会引发重排的属性不生效，改 <code>transform</code> / <code>opacity</code> / 颜色都生效。换来的是动画帧和静态帧一样便宜。',
    'trade.3.h': '滚动手感按 iOS / 微信那套做',
    'trade.3.p': '橡皮筋衰减 <code>1 - 1/(x·0.55/d + 1)</code>；回弹是带初速度的<strong>临界阻尼弹簧</strong>，不是固定时长缓动；惯性撞边界不当场停死，把动量交给弹簧。',
    'trade.4.h': '手势归属不在按下时决定',
    'trade.4.p': '等第一次明显位移（4px）按主方向锁定，再按「到边界仍在推」交棒给外层。横向卡片列表里竖着划该滚页面，纵向列表里横着划谁也不动。',
    'trade.5.h': '宿主层沉进 lib，所有宿主共用',
    'trade.5.p': '页面栈、路由转场、覆盖层、触摸状态机、手势仲裁、picker 面板、像素合成都在 <code>src/host/</code>，只有帧调度各自实现。逐像素脚本 + 输入行为测试双重守着。',
    'trade.6.h': 'CPU 画内容，GPU 只做合成',
    'trade.6.p': 'CPU 光栅器是唯一的像素参考，没有 GPU 的 CI、嵌入式照样出同一张图。GPU 后端（桌面窗体、Android 与 iOS 原生视图）只把页面 / tabBar / fixed 三个图层当纹理合成上屏，同一帧与 CPU <strong>逐字节一致</strong>，任何失败当帧回退 CPU。不用 vello / Skia：它们自带抗锯齿，一致性无从判定。',
    'trade.note': '全部八条和每一条背后「曾经错在哪」，见 <a href="https://github.com/365code365/mini-program-core/blob/main/doc/%E8%B8%A9%E5%9D%91%E8%AE%B0%E5%BD%95.md">踩坑记录</a> 与 <a href="https://github.com/365code365/mini-program-core/blob/main/doc/%E6%9E%B6%E6%9E%84%E4%B8%8E%E5%AE%9E%E7%8E%B0%E5%8E%9F%E7%90%86.md">架构与实现原理</a>。',

    /* ── 快速开始 ── */
    'start.eyebrow': '上手',
    'start.h2': '30 秒跑起来',
    'start.lede': '桌面只要 Rust 工具链。集成进 App 只用<strong>一个 View</strong> —— 或者一个 Flutter 组件。',
    'start.tabsAria': '快速开始方式',
    'start.tab1': '桌面',
    'start.tab2': '无头出图',
    'start.code1': '<code># 需要 Rust（https://rustup.rs）\ncargo build --release\n\n# 交互式选一个示例小程序\n./target/release/mini-launcher\n\n# 或直接指定\n./target/release/mini-app-window sample-app\n./target/release/mini-app-window sample/tea-app --route pages/index/index</code>',
    'start.code2': '<code># 没有窗口也能出图（CI / 回归用）\n./target/release/mini-app-window sample-app --route pages/index/index \\\n    --settle 0 --time 2 --snapshot target/shot\n\n# 动作参数按书写顺序执行，可重复\n#   --touch x,y   --swipe x1,y1,x2,y2   --drag px×帧数\n#   --type 文本    --key enter          --wheel\n#   --wait 秒      --frames N</code>',
    'start.code5': '<code>// pubspec.yaml：mini_render: { path: &lt;仓库&gt;/sdk/flutter }\n// 先编引擎库：bash tools/build-mobile.sh android   （或 ios）\n\nfinal c = MiniProgramController(\n  appDir: unpackedDir,                        // 解包后的小程序目录（含 app.json）\n  hostApis: {\'login\': (args) async =&gt; {\'code\': await myLogin()}},  // chooseImage 这些插件自带\n  nativeComponents: const [\'video\', \'map\'],   // 交给你自己的组件承载\n  sameLayerViews: const [\'map\'],              // Android：画进页面，跟着页面滚\n);\nNavigator.push(context, MaterialPageRoute(\n  builder: (_) =&gt; MiniProgramScaffold(        // 导航栏 + 返回键 + 胶囊 + 引擎画面\n    controller: c,\n    onMenu: () =&gt; showMyShareSheet(),          // 胶囊的「•••」\n    onExit: () =&gt; Navigator.pop(context),      // 胶囊的「⊙」\n  ),\n));</code>',
    'start.code3': '<code>// 1) 编库 → jniLibs，再 publishToMavenLocal\n//    bash tools/build-mobile.sh android\n//    cd sdk/android &amp;&amp; ./gradlew publishToMavenLocal\n\nval mini = MiniProgramView(this)\nmini.hostApis["login"] = MiniHostApi { call, reply -&gt; reply.success(JSONObject().put("code", myCode())) }\nmini.sameLayerViews["map"] = { MyMapView() }   // 一个 MiniSameLayerView，画进页面\nsetContentView(mini)\nmini.open(File(filesDir, "my-mini-app"))   // 解包后的小程序目录\n\noverride fun onBackPressed() {\n    if (!mini.goBack()) super.onBackPressed()   // 页面栈空了才退出\n}\n// 生命周期、铺满整屏、触感都由 View 自己跟着 Activity 做</code>',
    'start.code4': '<code>// 1) 编库 → sdk/ios/MiniRender.xcframework（设备 + 模拟器）\n//    bash tools/build-mobile.sh ios\n\nlet mini = MiniProgramView(frame: view.bounds)\nmini.hostApis["login"] = { call, reply in reply.success(["code": myCode()]) }\nview.addSubview(mini)\nmini.open(appDir: unpackedDir)              // 画进自己的 CAMetalLayer（GPU，失败回退 CPU）\n\nif !mini.goBack() { navigationController?.popViewController(animated: true) }</code>',
    'start.inc.h': 'View 内部已经包好的',
    'start.inc.1': '专用渲染线程（引擎不是线程安全的）',
    'start.inc.2': '尺寸、dpr 与安全区换算，Android 的系统栏',
    'start.inc.3': '触摸三段带时间戳，含历史点 / <code>coalescedTouches</code>，不抽稀',
    'start.inc.4': '按需出帧，静止不耗电',
    'start.inc.5': '沙盒目录、生命周期与内存告警',
    'start.inc.6': 'GPU surface 的挂载 / 卸载、预热与 CPU 回退',
    'start.you.h': '只有两件事必须你决定',
    'start.you.1': '<b>小程序包从哪来</b> —— 下载 / 校验 / 解包到沙盒。引擎按<strong>目录</strong>读，不解析 <code>.wxapkg</code>、不做签名校验。',
    'start.you.2': '<b>原生组件谁承载</b> —— <code>&lt;map&gt;</code>、<code>&lt;camera&gt;</code>、<code>&lt;web-view&gt;</code> 在微信里是原生组件层。把画好的帧交给引擎（<code>sameLayerViews</code>，Android），它就画进页面；或者让引擎报位置与参数，你用 <code>WKWebView</code> / <code>android.webkit.WebView</code> / ArkUI <code>Web()</code> 自己放上去。没人接管的标签画一块带标注的占位。',
    'host.eyebrow': "接入约定",
    'host.h2': "SDK 已经做了什么，剩下哪些要你写",
    'host.lede': "渲染、输入、生命周期和系统层的杂活都在 SDK 里。剩下的是只有你的 App 能做的事（账号、支付、定位、硬件），每一项都是 <code>hostApis</code> 里的一个名字。",
    'host.sdk.h': "SDK 自带：宿主不用写",
    'host.sdk.note': "格子里的开关在 <code>open</code> 之前都能关掉，交给你的 App 自己处理；事件照样会到 <code>onEvent</code> / <code>events</code>。",
    'host.you.h': "你要写的",
    'host.list.h': "交给宿主的 API",
    'host.list.p': "名字就是登记时写的那个。对象方法用带点的名字，如 <code>MapContext.getCenterLocation</code>，调用里带 <code>contextId</code>（组件 id）。类名后面的数字是方法个数。",
    'host.leg.demo': "示例 App 里有可运行的演示",
    'host.leg.builtin': "Android 与 Flutter SDK 自带",
    'host.r1': "选图、选视频、选文件、拍照、存相册、打开文档（下面标 ⧉ 的 7 个）",
    'host.r2': "<code>wx.setClipboardData</code> 与输入框复制 / 剪切写系统剪贴板",
    'host.r3': "<code>wx.vibrateShort</code> / <code>wx.vibrateLong</code> 的触感",
    'host.r4': "前后台与内存告警：App 与当前页的 <code>onShow</code> / <code>onHide</code>，后台停帧；同一状态报两次只派发一次",
    'host.r5': "画到状态栏下面、安全区、<code>env(safe-area-inset-*)</code>",
    'host.r6': "导航栏、胶囊按钮、页面转场、左边缘侧滑返回",
    'host.r7': "<code>input</code> / <code>textarea</code> 的软键盘与输入",
    'host.r8': "<code>&lt;video&gt;</code> 走系统播放器、画进页面；全屏时收起系统栏",
    'host.r9': "给 <code>wx.onXxx</code> 监听推事件",
    'host.you.1': "<b>只有你的 App 能做的 API</b>：把名字登记进 <code>hostApis</code>。调用带着参数到主线程，用 <code>reply.success(…)</code> / <code>reply.fail(errMsg)</code> 回（Flutter：返回一个 Map 或抛 <code>MiniHostApiError</code>）。用户取消时 <code>errMsg</code> 以 <code>:fail cancel</code> 结尾，与微信一致。",
    'host.you.2': "<b>没登记的名字</b>在 <code>wx</code> 上照样存在，下一轮事件循环就 fail，<code>errMsg</code> 写明可以用 <code>hostApis</code> 接管；<code>wx.canIUse</code> 报 <code>false</code>。不会卡住，也不会抛异常。",
    'host.you.3': "<b>只有你的 App 收得到的事件</b>：传感器、定位变化、蓝牙发现等 <code>wx.onXxx</code> 监听，要你调 <code>dispatchEvent(name, payload)</code> 才触发。返回值是监听者个数，没人听的传感器可以不开。",
    'host.you.4': "<b>返回键</b>：<code>if (!mini.goBack())</code> 就关掉小程序。minSdk 24 上 View 替你截不住 Activity 的返回。",
    'host.you.5': "<b>从示例改起</b>：<code>DemoHostApis.kt</code>（Android）和 <code>host_apis.dart</code>（Flutter）已经用演示数据和确认弹窗回了标 ✦ 的 23 个 API，把函数体换成你自己的账号、支付、地图 SDK 即可。",
    'host.g1': "账号与开放数据<small>42</small>",
    'host.g2': "支付<small>11</small>",
    'host.g3': "位置<small>8</small>",
    'host.g4': "媒体<small>20</small>",
    'host.g5': "跳转与转发<small>26</small>",
    'host.g6': "视频号<small>9</small>",
    'host.g7': "设备与系统<small>12</small>",
    'host.g8': "传感器<small>8</small>",
    'host.g9': "蓝牙与 BLE<small>20</small>",
    'host.g10': "Wi-Fi、iBeacon 与 HCE<small>13</small>",
    'host.g11': "卡包与交通卡<small>12</small>",
    'host.g12': "AI 与隐私<small>5</small>",
    'host.g13': "对象方法（按带点的名字登记）<small>20</small>",
    'host.g14': "用 <code>dispatchEvent</code> 推的事件<small>49</small>",

    /* ── 能力边界 ── */
    'scope.eyebrow': '能力边界',
    'scope.h2': '能做什么，明确不做什么',
    'scope.lede': '「明确不做」不是没排期，是取舍。',
    'scope.yes.h': '<span class="dot ok" aria-hidden="true"></span>已实现',
    'scope.yes.1': '<b>57 个组件标签</b>：渲染器画的 26 个（含 <code>swiper</code>、<code>scroll-view</code>、<code>picker-view</code>、对齐网格与瀑布流的 <code>grid-view</code>）；按官方行为实现的 <code>navigator</code> / <code>label</code> / <code>form</code>；页面用到才注入的 17 个内置 WXML 组件（<code>movable-view</code>、<code>page-container</code>、<code>draggable-sheet</code>、8 个手势组件……）；宿主接管前画占位的 8 个原生组件',
    'scope.yes.2': '<b>事件</b>：六种绑定前缀与完整捕获-冒泡链；触摸序列按微信语义产出（slop 取消 tap、350ms longpress、被滚动接管补 <code>touchcancel</code>）',
    'scope.yes.3': '<b>模板</b>：<code>wx:if</code> / <code>wx:for</code> / <code>wx:key</code> / <code>block</code>、表达式引擎、<code>template</code> / <code>import</code> / <code>include</code>、<code>wxs</code> 模块、自定义组件（每处一个实例、样式隔离、具名 / 默认 / 动态 <code>&lt;slot&gt;</code>、<code>virtualHost</code>、可冒泡的 <code>triggerEvent</code>、<code>wx://form-field</code> 系列 behavior）',
    'scope.yes.4': '<b>样式</b>：选择器与特异性层叠、继承、<code>@import</code>、CSS 变量、<code>rpx</code>、<code>calc()</code>、渐变、阴影、<code>transform</code> / <code>transition</code> / <code>@keyframes</code>、<code>@media</code>（尺寸、方向、深色模式）、<code>@font-face</code>',
    'scope.yes.5': '<b>官方 503 个 <code>wx.*</code> 全在</b>：TCP / UDP、Worker、WebAudio、<code>EventChannel</code>、路由事件、<code>request</code>、<code>connectSocket</code>、<code>downloadFile</code> / <code>uploadFile</code> 与 <code>FileSystemManager</code>、<code>InnerAudioContext</code>、跨启动持久化的 storage、Toast / Modal / ActionSheet、<code>previewImage</code>、路由、<code>createSelectorQuery</code>、<code>createIntersectionObserver</code>、<code>createVideoContext</code>、tabBar API、剪贴板、<code>canIUse</code>、网络状态、<code>loadFontFace</code>；硬件、支付、登录等经 <code>hostApis</code> 交给宿主，宿主接管前立刻 fail',
    'scope.yes.6': '<b>生命周期</b>：三级共 28 个钩子（App 7 / Page 13 / Component 5 + <code>pageLifetimes</code> 3）',
    'scope.yes.7': '<b>交互</b>：惯性滚动与临界阻尼回弹、手势仲裁、300ms 路由转场、左边缘侧滑返回、引擎画的导航栏与胶囊按钮、下拉刷新、picker 面板、操作菜单、图片预览、局部重绘',
    'scope.yes.12': '<b>Skyline 动画与手势</b>：带滚动时间线的 <code>this.animate</code> / <code>clearAnimation</code>、<code>wx.worklet</code> 与 <code>applyAnimatedStyle</code>、八个手势组件、带惯性与越界回弹的 <code>movable-view</code>、带吸附点的 <code>draggable-sheet</code>；触摸序列锁定在 <code>touchstart</code> 的节点上',
    'scope.yes.8': '<b>媒体</b>：Canvas 2D 上下文与 <code>&lt;canvas type="2d"&gt;</code>（渐变、图案、阴影、合成模式、像素读回）；<code>&lt;video&gt;</code>（MP4 解复用 + H.264 软解 + 音频、播放事件、<code>VideoContext</code>、全屏、封面、弹幕），或交给 App 自己的播放器承载',
    'scope.yes.9': '<b>双渲染后端</b>：CPU 光栅 + 可选的 GPU 合成（wgpu）。启动自检与 CPU 逐字节一致才启用，<code>--backend</code> / <code>MINI_BACKEND</code> 可强制，任何失败回退 CPU。已接进桌面窗体、无头、Android SDK 与 iOS 原生视图（CAMetalLayer）；Flutter 的 iOS 纹理走 CPU 合成',
    'scope.yes.10': '<b>组件行为</b>：<code>scroll-view</code> 的受控位置、事件、自定义下拉刷新、<code>reverse</code> 与 <code>padding</code>；<code>swiper</code> 的跟手拖动、衔接、前后边距与 Skyline 的 <code>direction</code> / <code>indicator-type</code> / <code>layout-type</code>；<code>form</code> 的 submit / reset；<code>picker</code> 的多列联动与 <code>header-text</code>；<code>picker-view</code> 内嵌滚轮',
    'scope.yes.11': '<b>宿主</b>：桌面窗体与启动器、给 CI 用的无头快照、C ABI、Android 与 iOS 原生 View（附原生 Android 示例工程），以及 Flutter 插件（<code>MiniProgramScaffold</code>、<code>hostApis</code>、经 <code>nativeViewBuilder</code> 承载的原生组件、同层渲染的 <code>sameLayerVideo</code> / <code>sameLayerViews</code>）',
    'scope.no.h': '<span class="dot no" aria-hidden="true"></span>明确不做',
    'scope.no.r1k': '用 GPU 画内容（vello / Skia 那种）',
    'scope.no.r1v': '已做的 GPU 后端只把 CPU 画好的图层合成上屏。内容一律由 CPU 画，CPU 才能当唯一的像素参考，无 GPU 环境（CI、嵌入式）也出同一张图。要在 GPU 上画内容就是一套与 CPU 并行的第二光栅器，这一轮没有做',
    'scope.no.r2k': '解析 <code>.wxapkg</code> / 签名校验',
    'scope.no.r2v': '<code>.wxapkg</code> 是微信编译后的私有格式，签名是微信的私钥体系；分包、灰度、完整性属于宿主的事，引擎按源码目录读',
    'scope.no.r3k': '把 <code>&lt;web-view&gt;</code> / <code>&lt;map&gt;</code> 编进引擎',
    'scope.no.r3v': 'Skyline 自己就不渲染 <code>web-view</code>（官方组件支持表：「暂不考虑」，该页面要切回 WebView 渲染）；<code>map</code> 在 Skyline 里是同层渲染的原生组件：地图由宿主画，引擎把那一帧画进页面（或报出位置，让宿主把控件盖上去）',
    'scope.no.r5k': '迁就偏离微信语义的写法',
    'scope.no.r5v': '对齐目标只有 Skyline 一个。这是原则，不是一个能「实现」的功能',
    'scope.miss.h': '还开着的',
    'scope.miss.p': '每条的影响都写在测试说明里：<code>wx:key</code> 参与增量失效与 WXS 响应事件；H5 导出的自定义组件；滚动条、Skyline 的吸顶分组与嵌套滚动；video 拖动进度与画中画；多指手势（<code>movable-view</code> 双指缩放与 scale 手势）；worklet 移出逻辑线程与 swiper 的 worklet 回调；<code>type="webgl"</code> 画布；直接写在 <code>&lt;view&gt;</code> 里的裸文本不换行；继承来的 <code>color</code> 顶掉 progress / slider / icon 的默认色；按元素作用域的 CSS 变量与 <code>grid-template</code>；textarea 的 <code>auto-height</code> 与输入法组字；iOS 上的同层渲染。',
    'scope.fit.h': '适合',
    'scope.fit.1': '自家 App（原生或 Flutter）想内嵌小程序容器，又不想背 WebView 的版本差异与首屏成本',
    'scope.fit.2': '要求渲染结果<strong>可控且可复现</strong>：同一套字体下各端输出逐像素相同',
    'scope.fit.3': '无屏环境批量出图：营销长图、服务端渲染卡片、UI 回归基线',
    'scope.fit.4': '想把一份小程序源码同时输出成 H5（内置编译器）',
    'scope.unfit.h': '不适合',
    'scope.unfit.1': '需要完整 Web 生态（任意 CSS、第三方 H5 SDK、<code>&lt;iframe&gt;</code>）—— 那就该用 WebView',
    'scope.unfit.2': '重度 3D / 大量滤镜 —— 内容光栅化在 CPU 上（GPU 只管合成），撑不住',
    'scope.unfit.3': '期望零适配跑通任意线上小程序 —— 还有一批已知差距',
    'scope.unfit.4': '内存预算极紧的设备 —— 字体已经不是问题，但解码后的图片（封顶 64MB）与约一屏高的 CPU 页面画布还是要算进去',
    'scope.mem.h': '内存：已经不是最大的短板',
    'scope.mem.p': '字体从前占峰值的约 98%：fontdue 加载时把每个字形的几何一次性展开。现在引擎带一份 vendored 的 fontdue，字形第一次画到才展开，字体文件 mmap 进来；度量与位图与上游逐位相同，基线一张没动。缓冲那一轮再把页面画布只留画过的条带，进后台时整屏缓冲放掉或压起来。',
    'scope.mem.p2': '缓存照样按字节封顶（LRU），进后台就放掉；<code>trim_memory()</code> 给内存告警时调用。手机：一加 PLY110，Flutter 示例。',
    'mem.caption': '内存：之前与现在',
    'mem.th1': '实测项',
    'mem.th2': '之前',
    'mem.th3': '现在',
    'mem.th4': '变化',
    'mem.r1': '系统主中文字面<small>约 2.9 万字形，加载后</small>',
    'mem.r2': '同一字体的粗体字面<small>第一次画粗体时</small>',
    'mem.r3': '内置 Source Han Sans<small>约 6.5 万字形，单字面</small>',
    'mem.r4': '加载系统主字面<small>耗时</small>',
    'mem.r5': '桌面内存报告<small>整个进程结束时</small>',
    'mem.r6': '手机 Native Heap<small>边滑边 setData 的列表之后</small>',
    'mem.r7': '手机总 PSS<small>同一轮</small>',
    'mem.r8': '进后台后引擎的缓冲<small>手机按 Home 之后</small>',
    'mem.r9': '后台 Native Heap<small>同一轮</small>',
    'mem.r10': '桌面连开 30 个页面<small>走完时进程 footprint</small>',

    /* ── 技术栈 ── */
    'stack.eyebrow': '技术栈',
    'stack.h2': '依赖很少，而且都跟到稳定版',
    'stack.lede': '<code>src/</code> 共 <b>410 个 <code>.rs</code> / 约 8.8 万行</b>（其中 49 个是测试模块），另有 33 个集成测试文件；单文件超 500 行就拆。除 FFI 边界外，<code>unsafe</code> 只有<strong>审过的 12 处</strong> —— 像素切片按字节看、用原生窗口指针建 surface、两个 Android 系统调用（logcat、CPU 亲和性）、一处清零分配的画布 —— 每处都带 SAFETY 注释；没有 <code>static mut</code>。',
    'dep.taffy': 'Flexbox / block 布局求解',
    'dep.rquickjs': 'QuickJS 绑定，逻辑层运行时',
    'dep.fontdue': '字形光栅化（vendored：字形用到才展开）',
    'dep.image': 'PNG / JPEG / GIF 逐帧 / WebP 解码',
    'dep.ureq': '<code>wx.request</code> 与远程图片',
    'dep.tungstenite': '<code>wx.connectSocket</code>：WebSocket 协议（TLS 走同一份 rustls）',
    'dep.rustls': '<code>wss://</code> 的 TLS 与根证书，与 ureq 共用',
    'dep.base64': '二进制过桥 / <code>unzip</code> 的 deflate / <code>wx.getRandomValues</code> 的系统随机数',
    'dep.symphonia': '音频解复用与解码',
    'dep.openh264': 'H.264 软解（可选）',
    'dep.winit': '桌面窗口与软件帧缓冲，也是 GPU 不可用时的上屏回退（可选）',
    'dep.wgpu': 'GPU 合成后端：Metal / Vulkan / DX12 / GLES（可选，锁死版本）',
    'dep.rodio': '桌面音频播放（可选）',
    'dep.arboard': '桌面剪贴板给 <code>wx.setClipboardData</code>，macOS / Windows 上的系统文件对话框给 <code>wx.chooseImage</code>（可选）',
    'dep.jni': 'Android JNI 入口',
    'stack.size.h': '产物体积<small>arm64 实测</small>',
    'stack.size.r1k': 'Android <code>.so</code> strip 后，含 GPU',
    'stack.size.r2k': '同上，不编 GPU 后端',
    'stack.reg.h': '回归规模<small>每次改动都要全绿</small>',
    'stack.reg.r1k': 'Rust 测试',
    'stack.reg.r2k': '场景画廊',
    'stack.reg.r2v': '65 张，逐字节可复现',
    'stack.reg.r3k': '局部重绘校验',
    'stack.reg.r3v': '桌面 5 个场景 + SDK 12 个用例，逐帧比',
    'stack.reg.r4k': 'SDK 与桌面',
    'stack.reg.r4v': 'sample 15/15、news 6/6 逐像素相同',
    'stack.reg.r7k': 'GPU 与 CPU',
    'stack.reg.r7v': '34/34 逐字节一致',
    'stack.reg.r8k': 'Flutter 插件测试',
    'stack.reg.r6k': '全量回归',
    'stack.reg.r6v': '<code>bash tools/regress-async.sh</code>，约 5 分钟，结果按时间戳落成批次',
    'stack.todo.h': '最大的三件待办',
    'stack.todo.1': '<b>iOS 的同层渲染</b> —— 那边 <code>&lt;map&gt;</code> 与 <code>&lt;video&gt;</code> 还是盖在页面上',
    'stack.todo.2': '<b>增量布局剩下的缺口</b> —— 带 transform / 动画的子树整段重建，内容变矮仍出整帧',
    'stack.todo.3': '<b>H5 导出的自定义组件</b> —— 与 Chrome 的双端对比目前只覆盖不用自定义组件的页面',

    /* ── 文档 ── */
    'docs.eyebrow': '文档',
    'docs.h2': '写得比代码还细的那部分',
    'docs.lede': '下面这些文档都是中文的；<a href="https://github.com/365code365/mini-program-core/blob/main/README.zh-CN.md">README 也有中文版</a>。',
    'docs.1.h': '架构与实现原理',
    'docs.1.p': '一帧怎么画出来、分层与模块划分、帧成本怎么压下来、局部重绘、滚动手感与触控对齐、taffy 适配',
    'docs.2.h': '引擎测试说明',
    'docs.2.p': '能力矩阵、T1~T31 逐项测试清单与判据（含每一轮真机实测与对照微信补齐 API / 组件那一轮）、内存分项实测、已知差距',
    'docs.3.h': '原生 App 集成指南',
    'docs.3.p': '极简集成、宿主要做的事、C ABI、GPU surface、iOS / Android / 鸿蒙 / Flutter 步骤、验收清单',
    'docs.4.h': '踩坑记录',
    'docs.4.p': '「曾经错在哪」：现象 → 根因 → 现在的做法 → 回归判据',
    'docs.5.h': '场景画廊',
    'docs.5.p': '65 张真实渲染（同时是回归基线），以及素材怎么生成',
    'docs.6.h': '示例与调试',
    'docs.6.p': '示例小程序清单、浏览器实时预览、编译成 HTML、Rust / C / 移动端代码示例',
    'docs.7.h': '触控对齐测试报告',
    'docs.7.p': '滑动 / 点击 / 长按 / 手势与微信的逐项对照实测',
    'docs.8.h': 'SDK 说明',
    'docs.8.p': '三行接入，以及三条必须知道的约定',
    'docs.9.h': 'Flutter 插件',
    'docs.9.p': '<code>MiniProgramController</code> / <code>MiniProgramScaffold</code>、宿主能力与自带的选择器、原生组件与同层渲染、导航栏与胶囊、Android 系统栏',
    'docs.10.h': '滑动顿挫避坑清单',
    'docs.10.p': '判据只认「屏上重复帧」；宿主上屏、引擎失效、量具三层踩过的坑；Flutter 与原生示例的当前数字，含同层地图的错位实测',

    /* ── CTA / 页脚 ── */
    'cta.h2': '给我一块可写的像素缓冲，<br>剩下的交给引擎。',
    'cta.btn': '先跑一遍',
    'foot.1': '<b>mini-render</b> · MIT 许可',
    'foot.2': '内置图标矢量数据来自腾讯开源的 <a href="https://github.com/Tencent/weui">WeUI</a>（MIT）。',
    'foot.repo': '仓库',
    'foot.readme': 'README'
  };

  /* 其余语言一种一个文件（assets/i18n/<代码>.js，tools/site-i18n-lang.py 生成），切过去时才加载 ——
     首屏不必把十几种语言都下下来。name 是语言菜单里显示的自称，html 是 <html lang>。 */
  var LANGS = [
    { code: 'en', name: 'English', html: 'en' },
    { code: 'zh', name: '简体中文', html: 'zh-CN' },
    { code: 'zh-tw', name: '繁體中文', html: 'zh-TW' },
    { code: 'ja', name: '日本語', html: 'ja' },
    { code: 'ko', name: '한국어', html: 'ko' },
    { code: 'hi', name: 'हिन्दी', html: 'hi' },
    { code: 'fr', name: 'Français', html: 'fr' },
    { code: 'de', name: 'Deutsch', html: 'de' },
    { code: 'es', name: 'Español', html: 'es' },
    { code: 'pt', name: 'Português', html: 'pt-BR' },
    { code: 'it', name: 'Italiano', html: 'it' },
    { code: 'ru', name: 'Русский', html: 'ru' },
    { code: 'tr', name: 'Türkçe', html: 'tr' },
    { code: 'vi', name: 'Tiếng Việt', html: 'vi' },
    { code: 'id', name: 'Bahasa Indonesia', html: 'id' },
    { code: 'th', name: 'ไทย', html: 'th' }
  ];
  function info(code) {
    for (var i = 0; i < LANGS.length; i++) if (LANGS[i].code === code) return LANGS[i];
    return null;
  }
  var DICT = { zh: ZH };
  var EN = {};                       /* 英文原文：初始化时从 DOM 抓，切回来直接还原 */
  var STORE = 'mr-lang';
  var current = 'en';
  /* 语言文件与 i18n.js 同一个版本号（Pages 发布时给 i18n.js 带 ?v=提交号，见 static.yml） */
  var me = document.currentScript && document.currentScript.src;
  var VER = me && me.indexOf('?') >= 0 ? me.slice(me.indexOf('?')) : '';
  var BASE = me ? me.replace(/i18n\.js(\?.*)?$/, '') : 'assets/';

  function each(sel, fn) {
    Array.prototype.forEach.call(document.querySelectorAll(sel), fn);
  }

  /* 把 DOM 里的英文原文存下来（含属性），这样英文不必在表里写第二遍 */
  function snapshotEnglish() {
    each('[data-i18n]', function (el) { EN[el.getAttribute('data-i18n')] = el.innerHTML; });
    each('[data-i18n-alt]', function (el) { EN[el.getAttribute('data-i18n-alt')] = el.getAttribute('alt') || ''; });
    each('[data-i18n-aria]', function (el) { EN[el.getAttribute('data-i18n-aria')] = el.getAttribute('aria-label') || ''; });
    EN['meta.title'] = document.title;
    var d = document.querySelector('meta[name="description"]');
    EN['meta.desc'] = d ? d.getAttribute('content') : '';
    EN['ui.copy'] = 'Copy';
    EN['ui.copied'] = 'Copied';
    EN['ui.copyFail'] = 'Copy failed';
    EN['ui.copyAria'] = 'Copy this snippet';
  }

  function table(lang) {
    if (lang === 'en') return EN;
    return DICT[lang] || (window.MR_I18N && window.MR_I18N[lang]) || EN;
  }

  function t(key) {
    var v = table(current)[key];
    return v == null ? (EN[key] == null ? '' : EN[key]) : v;
  }

  /* 语言文件还没下来时先下（一个 <script>：file:// 打开也能用，fetch 不行） */
  function load(lang, done) {
    if (lang === 'en' || DICT[lang] || (window.MR_I18N && window.MR_I18N[lang])) return done(true);
    var s = document.createElement('script');
    s.src = BASE + 'i18n/' + lang + '.js' + VER;
    s.onload = function () { done(!!(window.MR_I18N && window.MR_I18N[lang])); };
    s.onerror = function () { done(false); };
    document.head.appendChild(s);
  }

  function apply(lang) {
    var tb = table(lang);
    current = lang;
    each('[data-i18n]', function (el) {
      var v = tb[el.getAttribute('data-i18n')];
      if (v != null) el.innerHTML = v;
    });
    each('[data-i18n-alt]', function (el) {
      var v = tb[el.getAttribute('data-i18n-alt')];
      if (v != null) el.setAttribute('alt', v);
    });
    each('[data-i18n-aria]', function (el) {
      var v = tb[el.getAttribute('data-i18n-aria')];
      if (v != null) el.setAttribute('aria-label', v);
    });
    if (tb['meta.title']) document.title = tb['meta.title'];
    var d = document.querySelector('meta[name="description"]');
    if (d && tb['meta.desc']) d.setAttribute('content', tb['meta.desc']);
    var li = info(lang) || LANGS[0];
    document.documentElement.lang = li.html;
    each('.lang-switch [data-lang]', function (b) {
      b.setAttribute('aria-pressed', String(b.getAttribute('data-lang') === lang));
    });
    var cur = document.querySelector('.lang-cur-name');
    if (cur) cur.textContent = li.name;
    /* 让 app.js 里那些自己造出来的按钮（复制）也跟着换 */
    document.dispatchEvent(new CustomEvent('mr:langchange', { detail: { lang: lang } }));
  }

  function switchTo(lang) {
    load(lang, function (ok) {
      if (!ok) lang = 'en';
      apply(lang);
      try { localStorage.setItem(STORE, lang); } catch (e) { /* 隐私模式下 localStorage 会抛，忽略 */ }
      /* 让地址栏与分享出去的链接一致 */
      var url = new URL(location.href);
      if (lang === 'en') url.searchParams.delete('lang');
      else url.searchParams.set('lang', lang);
      history.replaceState(null, '', url.toString());
    });
  }

  function pick() {
    var q = new URLSearchParams(location.search).get('lang');
    if (q && info(q)) return q;
    try {
      var s = localStorage.getItem(STORE);
      if (s && info(s)) return s;
    } catch (e) { /* 同上 */ }
    return 'en';                     /* 默认英文 */
  }

  /* 语言菜单：按钮展开 / 收起，点外面、按 Esc 收起，上下键在选项间移动 */
  function menu() {
    var box = document.querySelector('.lang-switch');
    if (!box) return;
    var btn = box.querySelector('.lang-cur');
    var list = box.querySelector('.lang-menu');
    if (!btn || !list) return;
    var opts = function () { return Array.prototype.slice.call(list.querySelectorAll('[data-lang]')); };
    var open = function (on, focus) {
      list.hidden = !on;
      btn.setAttribute('aria-expanded', String(on));
      if (on && focus) {
        var o = opts();
        var sel = o.filter(function (b) { return b.getAttribute('aria-pressed') === 'true'; })[0] || o[0];
        if (sel) sel.focus();
      }
    };
    btn.addEventListener('click', function () { open(list.hidden, true); });
    document.addEventListener('click', function (ev) { if (!box.contains(ev.target)) open(false); });
    box.addEventListener('keydown', function (ev) {
      if (ev.key === 'Escape') { open(false); btn.focus(); return; }
      if (list.hidden || (ev.key !== 'ArrowDown' && ev.key !== 'ArrowUp')) return;
      ev.preventDefault();
      var o = opts();
      var i = o.indexOf(document.activeElement);
      var n = ev.key === 'ArrowDown' ? (i + 1) % o.length : (i - 1 + o.length) % o.length;
      o[n].focus();
    });
    opts().forEach(function (b) {
      b.addEventListener('click', function () {
        var lang = b.getAttribute('data-lang');
        open(false);
        btn.focus();
        if (lang !== current) switchTo(lang);
      });
    });
  }

  function init() {
    snapshotEnglish();
    menu();
    var want = pick();
    if (want !== 'en') load(want, function (ok) { if (ok) apply(want); });
  }

  window.I18N = { t: t, apply: apply, lang: function () { return current; }, keys: Object.keys(ZH), langs: LANGS };

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
  else init();
})();
