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
    'meta.title': 'mini-render — 用 Rust 从零写的小程序渲染引擎',
    'meta.desc': '自绘的小程序渲染引擎：不基于 WebView、不映射系统原生控件、不依赖 Skia。给它一份微信小程序源码和一块可写的像素缓冲，它把界面画出来、把交互跑起来。',

    /* ── 导航 ── */
    'nav.aria': '页面导航',
    'nav.pipeline': '渲染管线',
    'nav.gallery': '场景',
    'nav.perf': '实测',
    'nav.start': '快速开始',
    'nav.scope': '能力边界',
    'nav.docs': '文档',

    /* ── HERO ── */
    'hero.eyebrow': 'Rust · 自绘 · 无 WebView',
    'hero.h1': '小程序源码进，<br><span class="grad">像素出</span>。',
    'hero.lede': '一个用 Rust 从零实现的小程序渲染引擎。给它一份微信小程序源码（WXML / WXSS / JS / app.json）和一块可写的像素缓冲，它把界面画出来、把交互跑起来。中间<strong>没有浏览器内核、没有系统控件、没有第三方图形库</strong>。',
    'hero.not1': '不基于 WebView',
    'hero.not2': '不映射系统原生控件',
    'hero.not3': '不依赖 Skia',
    'hero.cta1': '30 秒跑起来',
    'hero.cta2': '看源码',
    'hero.foot': '对齐目标是微信 <strong>Skyline</strong> —— 同样原生渲染，同样没有 DOM。',
    'hero.shotsAria': '引擎渲染输出示例',
    'hero.alt1': '引擎渲染的资讯信息流页面',
    'hero.alt2': '引擎渲染的电商首页，含轮播、限时秒杀倒计时与推荐流',
    'hero.alt3': '引擎渲染的表单控件页面',
    'hero.cap1': '资讯信息流',
    'hero.cap2': '电商首页',
    'hero.cap3': '表单控件',

    /* ── 数字条 ── */
    'stat.1.k': '商城首页整帧',
    'stat.1.v': '750×1334 物理像素，纯 CPU',
    'stat.2.k': '与 Chrome 差异',
    'stat.2.v': '同源码 15 页逐像素比',
    'stat.3.k': '组件标签',
    'stat.3.v': '另有 39 个 wx.* API',
    'stat.4.k': '回归基线',
    'stat.4.v': '固定时钟下逐字节可复现',
    'stat.5.k': '单元测试',
    'stat.5.v': 'lib 483 + bin 9，警告 0',
    'stat.6.k': 'unsafe',
    'stat.6.v': '除 FFI 边界外全库没有',

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
    'why.r3.b': '<span class="tag near">思路最近</span> 但这里是 Rust + 自研光栅器，无 Skia',
    'why.r4.a': '打包或复用一个浏览器',
    'why.r4.b': '<span class="tag no">不打包</span> 浏览器，产物 34MB 级',
    'why.r5.k': '微信 Skyline',
    'why.r5.a': '原生渲染 + 自绘组件',
    'why.r5.b': '<span class="tag yes">对齐目标</span> 语义、手感、事件模型都按它对',
    'why.note': '代价也很实：CSS 覆盖面要自己一条一条补，性能靠 CPU 省着花。各端剩余差异只来自系统字体 —— 要彻底消掉就把字体一起打包（<code>assets/SourceHanSansSC-Regular.otf</code> 就是为这个准备的）。但先量一量：内置字体<strong>更费内存</strong>，不是更省。',

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
    'pipe.4.p': '命中的 CSS 加内联 <code>style</code> 合成计算样式。带继承语义（<code>color</code> / <code>font-size</code> / <code>line-height</code> …），简写与细项按固定档位排序落地。',
    'pipe.5.h': '布局',
    'pipe.5.p': 'taffy 求解 Flexbox 盒模型。<strong>文本节点挂自定义度量函数</strong>，换行与 min/max-content 由真实字形宽度决定；布局后有第二遍 <code>reflow</code> 收拾算完才知道的溢出。',
    'pipe.6.h': '绘制',
    'pipe.6.p': '自研 2D 光栅器：扫描线 + even-odd 填充、4× 超采样抗锯齿、圆角三次贝塞尔逼近、多字体回退、Apple <code>sbix</code> 彩色 emoji、两级图片过滤、渐变、掩膜化 <code>box-shadow</code>、裁剪栈。',
    'pipe.7.h': '动画',
    'pipe.7.p': '<code>@keyframes</code> / <code>transition</code> / <code>wx.createAnimation</code> 在<strong>绘制期</strong>按全局时钟求值。<code>transform</code> / <code>opacity</code> / 颜色只影响绘制不触发重排 → 动画帧成本≈静态帧。',
    'pipe.8.h': '分层合成',
    'pipe.8.p': '正常流 → tabBar → <code>position:fixed</code> 覆盖层，层叠顺序与浏览器一致（全屏遮罩能压暗 tabBar）。覆盖层用视口坐标、独立画布、按需重绘。',
    'pipe.9.h': '上屏',
    'pipe.9.p': '贴到窗口、交给宿主 View、或直接存 PNG。桌面走 softbuffer，移动端把 RGBA 交给宿主，无头模式 <code>save_png</code> —— 同一份代码，五个平台。',
    'flow.1': '小程序目录',
    'flow.2': '解析',
    'flow.3': '逻辑层',
    'flow.4': '布局',
    'flow.5': '光栅化',
    'flow.6': '合成',
    'flow.7': 'RGBA 缓冲',
    'pipe.note': '产物是一个普通的 Rust 库（<code>cdylib</code> / <code>staticlib</code> / <code>rlib</code>）。落地条件只有一条：<strong>给我一块可写的像素缓冲</strong> —— 所以 Android、iOS、Windows、macOS、Linux 跑同一份代码，没有屏幕的 CI 里也能直接出 PNG。',

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
    'perf.lede': '375×667 @2x（750×1334 物理像素），CPU 逐像素写出。这些数字是<strong>回归判据</strong>，不是宣传语。',
    'perf.b1': '商城首页整帧<small>轮播 + 每秒倒计时 + 骨架动画</small>',
    'perf.b2': '一次 <code>setData</code> 的重建<small>新旧渲染树并行比对</small>',
    'perf.b3': '首帧建树 + 样式<small>三条中文字体栈的应用</small>',
    'perf.b4': '拖动单帧<small>tea-app 首页，60 帧平均</small>',
    'perf.b5': '远程图片二次打开<small>磁盘缓存命中</small>',
    'perf.f1.h': '稳态帧稳得住',
    'perf.f1.p': '144Hz 屏（节拍 6.94ms）上商城首页的帧间隔从 <b>6.9~25ms 抖动</b> 收到 <b>6.7~7.2ms</b>；最慢一帧从 17.9ms 降到 <b>11.4ms</b>。',
    'perf.f2.h': '只重绘变化的那一块',
    'perf.f2.p': '首页秒杀倒计时的失效范围是 <b>45×33 像素</b>，不是整屏。拿不准就退回整帧 —— 少画一块留下脏像素，比多画一次严重得多。',
    'perf.f3.h': '滚动不重绘',
    'perf.f3.p': '页面画布是整页高的、用内容坐标，滚动只是取不同切片上屏，每帧 <b>2~3ms</b>。只有滚出已绘制条带那一刻才补画一次。',
    'perf.cons.h': '双端一致性',
    'perf.cons.p': '同一份源码：一边原生渲染，一边编译成 HTML 用 Chrome 截图，逐像素比。剩余差异集中在粗体字形与亚像素文本位置。',
    'perf.cons.r1k': '<code>sample-app</code> 商城<small>15 页</small>',
    'perf.cons.r1v': '<b>4.55%</b> 变化像素',
    'perf.cons.r2k': '<code>news-app</code> 资讯<small>6 页</small>',
    'perf.cons.r2v': '<b>6.33%</b> 变化像素',
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

    /* ── 关键取舍 ── */
    'trade.eyebrow': '关键取舍',
    'trade.h2': '为什么这么写',
    'trade.lede': '每一条背后都有踩过的坑，反面教材都记在踩坑记录里。',
    'trade.1.h': '文本度量驱动布局，不估算宽度',
    'trade.1.p': '度量上下文挂到 taffy 叶子上，真实字形参与布局，「定宽容器内换行」与「收缩容器被内容撑开」两种语义同时成立。',
    'trade.1.bad': '曾给每个文本盒 +4px「保险余量」，结果所有按内容定宽的徽标都比浏览器宽一圈。',
    'trade.2.h': '动画只重绘不重排',
    'trade.2.p': '<code>@keyframes</code> 里改 <code>width</code> / <code>height</code> 这类会引发重排的属性不生效，改 <code>transform</code> / <code>opacity</code> / 颜色都生效。换来的是动画帧和静态帧一样便宜。',
    'trade.3.h': '滚动手感按 iOS / 微信那套做',
    'trade.3.p': '橡皮筋衰减 <code>1 - 1/(x·0.55/d + 1)</code>；回弹是带初速度的<strong>临界阻尼弹簧</strong>，不是固定时长缓动；惯性撞边界不当场停死，把动量交给弹簧。',
    'trade.4.h': '手势归属不在按下时决定',
    'trade.4.p': '等第一次明显位移（4px）按主方向锁定，再按「到边界仍在推」交棒给外层。横向卡片列表里竖着划该滚页面，纵向列表里横着划谁也不动。',
    'trade.5.h': '宿主层沉进 lib，两端共用',
    'trade.5.p': '页面栈、覆盖层、触摸状态机、手势仲裁、picker 面板、像素合成都在 <code>src/host/</code>，只有帧调度各自实现。逐像素脚本 + 输入行为测试双重守着。',
    'trade.6.h': '不引入 GPU，不做元素级位图缓存',
    'trade.6.p': '定位就是「给我一块像素缓冲我就能画」。抗锯齿也没换成子采样 —— 实测与 Chrome 的差异反而从 4.7% 涨到 5.3%。',
    'trade.note': '全部八条与 13 类踩坑记录见 <a href="https://github.com/365code365/mini-program-core/blob/main/doc/%E8%B8%A9%E5%9D%91%E8%AE%B0%E5%BD%95.md">踩坑记录</a> 与 <a href="https://github.com/365code365/mini-program-core/blob/main/doc/%E6%9E%B6%E6%9E%84%E4%B8%8E%E5%AE%9E%E7%8E%B0%E5%8E%9F%E7%90%86.md">架构与实现原理</a>。',

    /* ── 快速开始 ── */
    'start.eyebrow': '上手',
    'start.h2': '30 秒跑起来',
    'start.lede': '桌面只要 Rust 工具链。集成进 App 只用<strong>一个 View</strong>。',
    'start.tabsAria': '快速开始方式',
    'start.tab1': '桌面',
    'start.tab2': '无头出图',
    'start.code1': '<code># 需要 Rust（https://rustup.rs）\ncargo build --release\n\n# 交互式选一个示例小程序\n./target/release/mini-launcher\n\n# 或直接指定\n./target/release/mini-app-window sample-app\n./target/release/mini-app-window sample/tea-app --route pages/index/index</code>',
    'start.code2': '<code># 没有窗口也能出图（CI / 回归用）\n./target/release/mini-app-window sample-app --route pages/index/index \\\n    --settle 0 --time 2 --snapshot target/shot\n\n# 动作参数按书写顺序执行，可重复\n#   --touch x,y   --swipe x1,y1,x2,y2   --drag px×帧数\n#   --type 文本    --key enter          --wheel\n#   --wait 秒      --frames N</code>',
    'start.code3': '<code>// 1) 编库 → jniLibs，再 publishToMavenLocal\n//    bash tools/build-mobile.sh android\n//    cd sdk/android &amp;&amp; ./gradlew publishToMavenLocal\n\nval mini = MiniProgramView(this)\nsetContentView(mini)\nmini.open(File(filesDir, "my-mini-app"))   // 解包后的小程序目录\n\noverride fun onBackPressed() {\n    if (!mini.goBack()) super.onBackPressed()   // 页面栈空了才退出\n}\noverride fun onResume() { super.onResume(); mini.onHostResume() }\noverride fun onPause()  { super.onPause();  mini.onHostPause() }</code>',
    'start.code4': '<code>// 1) 编库 → sdk/ios/MiniRender.xcframework（设备 + 模拟器）\n//    bash tools/build-mobile.sh ios\n\nlet mini = MiniProgramView(frame: view.bounds)\nview.addSubview(mini)\nmini.open(appDir: unpackedDir)\n\nif !mini.goBack() { navigationController?.popViewController(animated: true) }</code>',
    'start.inc.h': 'View 内部已经包好的',
    'start.inc.1': '专用渲染线程（引擎不是线程安全的）',
    'start.inc.2': '尺寸与 dpr 换算',
    'start.inc.3': '触摸三段，含历史点 / <code>coalescedTouches</code>，不抽稀',
    'start.inc.4': '按需出帧，静止不耗电',
    'start.inc.5': '沙盒目录、生命周期与内存告警',
    'start.you.h': '只有两件事必须你决定',
    'start.you.1': '<b>小程序包从哪来</b> —— 下载 / 校验 / 解包到沙盒。引擎按<strong>目录</strong>读，不解析 <code>.wxapkg</code>、不做签名校验。',
    'start.you.2': '<b>原生层组件谁承载</b> —— <code>&lt;web-view&gt;</code> / <code>&lt;map&gt;</code> 在微信里是原生组件层。引擎算位置与参数，控件由你用 <code>WKWebView</code> / <code>android.webkit.WebView</code> / ArkUI <code>Web()</code> 放上去。',

    /* ── 能力边界 ── */
    'scope.eyebrow': '能力边界',
    'scope.h2': '能做什么，明确不做什么',
    'scope.lede': '「明确不做」不是没排期，是取舍。',
    'scope.yes.h': '<span class="dot ok" aria-hidden="true"></span>已实现',
    'scope.yes.1': '<b>24 个组件标签</b>：容器 / 文本媒体 / 表单三类，含 <code>swiper</code>、<code>scroll-view</code>、<code>picker-view</code>',
    'scope.yes.2': '<b>事件</b>：六种绑定前缀与完整捕获-冒泡链；触摸序列按微信语义产出（slop 取消 tap、350ms longpress、被滚动接管补 <code>touchcancel</code>）',
    'scope.yes.3': '<b>模板</b>：<code>wx:if</code> / <code>wx:for</code> / <code>block</code>、表达式引擎、<code>class</code> / <code>style</code> 绑定、自定义组件（样式隔离 + 独立数据作用域）',
    'scope.yes.4': '<b>样式</b>：选择器与特异性层叠、继承、<code>@import</code>、CSS 变量、<code>rpx</code>、<code>calc()</code>、渐变、阴影、<code>transform</code> / <code>transition</code> / <code>@keyframes</code>',
    'scope.yes.5': '<b>39 个 <code>wx.*</code></b>：<code>request</code>（含 <code>abort</code>）、storage 全套且跨启动持久化、Toast / Loading / Modal、五个路由 API、设备信息全套',
    'scope.yes.6': '<b>生命周期</b>：三级共 28 个钩子（App 7 / Page 13 / Component 5 + <code>pageLifetimes</code> 3）',
    'scope.yes.7': '<b>交互</b>：惯性滚动与临界阻尼回弹、手势仲裁、侧滑返回、下拉刷新、picker 面板、局部重绘',
    'scope.yes.8': '<b>媒体</b>：Canvas 2D 完整上下文、<code>&lt;video&gt;</code>（MP4 解复用 + H.264 软解 + 音频）',
    'scope.no.h': '<span class="dot no" aria-hidden="true"></span>明确不做',
    'scope.no.r1k': 'GPU 渲染',
    'scope.no.r1v': '纯 CPU 才能在无 GPU 环境（CI、嵌入式）里出图',
    'scope.no.r2k': '解析 <code>.wxapkg</code> / 签名校验',
    'scope.no.r2v': '分包、灰度、完整性属于宿主的事，引擎按目录读',
    'scope.no.r3k': '把 <code>&lt;web-view&gt;</code> / <code>&lt;map&gt;</code> 编进引擎',
    'scope.no.r3v': '微信里它们是原生组件层，位置由引擎算、控件由宿主放',
    'scope.no.r4k': '远程字体 <code>wx.loadFontFace</code>',
    'scope.no.r4v': '字体来自系统。API 存在并回调 <code>success</code>，但不真的下载',
    'scope.no.r5k': '迁就偏离微信语义的写法',
    'scope.no.r5v': '对齐目标只有 Skyline 一个',
    'scope.miss.h': '还缺的 13 条',
    'scope.miss.p': '<code>wx:key</code> 的复用语义、<code>template</code> / <code>slot</code> / <code>wxs</code>、<code>scroll-view</code> 的滚动事件、<code>&lt;video&gt;</code> 的播放事件、<code>wx.login</code> 等云能力、移动端 SDK 的左边缘侧滑返回 —— 每条都写了影响。',
    'scope.fit.h': '适合',
    'scope.fit.1': '自家 App 想内嵌小程序容器，又不想背 WebView 的版本差异与首屏成本',
    'scope.fit.2': '要求渲染结果<strong>可控且可复现</strong>：同一套字体下各端输出逐像素相同',
    'scope.fit.3': '无屏环境批量出图：营销长图、服务端渲染卡片、UI 回归基线',
    'scope.fit.4': '想把一份小程序源码同时输出成 H5（内置编译器）',
    'scope.unfit.h': '不适合',
    'scope.unfit.1': '需要完整 Web 生态（任意 CSS、第三方 H5 SDK、<code>&lt;iframe&gt;</code>）—— 那就该用 WebView',
    'scope.unfit.2': '重度 3D / 大量滤镜 —— 纯 CPU 光栅撑不住',
    'scope.unfit.3': '期望零适配跑通任意线上小程序 —— 还有 13 条已知差距',
    'scope.unfit.4': '内存极紧的设备 —— 字体后端换完之前峰值偏高',
    'scope.mem.h': '最大的已知短板：内存',
    'scope.mem.p': '实测峰值 600MB~1GB，其中 <b>约 98% 是字体</b> —— fontdue 加载时把字体里每个字形的几何一次性展开（系统主中文字面约 2.9 万字形，实测 <b>+303MB</b>）。代价跟<strong>字形数量</strong>成正比，跟文件大小无关：内置的单字面 Source Han Sans（约 6.5 万字形）实测 <b>+405MB</b>，比系统字体更贵 —— 所以内置字体只该为了跨端一致而做，不是为了省内存。缓存已按字节封顶（图片 64MB / 字形 24MB，LRU），也有 <code>trim_memory()</code> 给宿主在内存告警时调用。真正压峰值只有换惰性字体后端这一条路，而它会改变每个字形的抗锯齿，需要单独一轮重定基线。',

    /* ── 技术栈 ── */
    'stack.eyebrow': '技术栈',
    'stack.h2': '依赖很少，而且都跟到稳定版',
    'stack.lede': '工程规模 <b>240 个 <code>.rs</code> / 约 5.3 万行</b>（含 29 个测试文件），单文件超 500 行就拆。除 FFI 边界外全库<strong>没有 <code>unsafe</code></strong>，也没有 <code>static mut</code>。',
    'dep.taffy': 'Flexbox / block 布局求解',
    'dep.rquickjs': 'QuickJS 绑定，逻辑层运行时',
    'dep.fontdue': '字形光栅化',
    'dep.image': 'PNG / JPEG / GIF 逐帧 / WebP 解码',
    'dep.ureq': '<code>wx.request</code> 与远程图片',
    'dep.symphonia': '音频解复用与解码',
    'dep.openh264': 'H.264 软解（可选）',
    'dep.winit': '桌面窗口与软件帧缓冲（可选）',
    'dep.rodio': '桌面音频播放（可选）',
    'dep.jni': 'Android JNI 入口',
    'stack.size.h': '产物体积<small>arm64 实测</small>',
    'stack.size.r1k': 'Android <code>.so</code> strip 后',
    'stack.size.r2k': 'iOS 静态库 <code>.a</code>',
    'stack.size.r2v': '184 MB<small>链进 App 后被死代码消除大幅缩减</small>',
    'stack.reg.h': '回归规模<small>每次改动都要全绿</small>',
    'stack.reg.r1k': '单元测试',
    'stack.reg.r1v': 'lib 483 + bin 9',
    'stack.reg.r2k': '场景画廊',
    'stack.reg.r2v': '65 张，逐字节可复现',
    'stack.reg.r3k': '局部重绘校验',
    'stack.reg.r3v': '5 个场景逐字节相同',
    'stack.reg.r4k': 'SDK 与桌面',
    'stack.reg.r4v': 'sample 15/15、news 6/6 逐像素相同',
    'stack.reg.r5k': '编译警告',
    'stack.todo.h': '最大的三件待办',
    'stack.todo.1': '<b>内存峰值</b> —— 换惰性字体后端',
    'stack.todo.2': '<b><code>setData</code> 仍整棵重建布局树</b> —— 要做增量树打补丁，基础设施已就位',
    'stack.todo.3': '<b>移动端左边缘侧滑返回</b> —— 要在页面被覆盖那刻留一张视口图',

    /* ── 文档 ── */
    'docs.eyebrow': '文档',
    'docs.h2': '写得比代码还细的那部分',
    'docs.lede': '下面这些文档都是中文的；<a href="https://github.com/365code365/mini-program-core/blob/main/README.zh-CN.md">README 也有中文版</a>。',
    'docs.1.h': '架构与实现原理',
    'docs.1.p': '一帧怎么画出来、分层与模块划分、帧成本怎么压下来、局部重绘、滚动手感与触控对齐、taffy 适配',
    'docs.2.h': '引擎测试说明',
    'docs.2.p': '能力矩阵、T1~T9 逐项测试清单与判据、内存分项实测、已知差距 13 条',
    'docs.3.h': '原生 App 集成指南',
    'docs.3.p': '极简集成、宿主九件事、C ABI、iOS / Android / 鸿蒙步骤、验收清单',
    'docs.4.h': '踩坑记录',
    'docs.4.p': '13 类「曾经错在哪」：现象 → 根因 → 现在的做法 → 回归判据',
    'docs.5.h': '场景画廊',
    'docs.5.p': '65 张真实渲染（同时是回归基线），以及素材怎么生成',
    'docs.6.h': '示例与调试',
    'docs.6.p': '示例小程序清单、浏览器实时预览、编译成 HTML、Rust / C / 移动端代码示例',
    'docs.7.h': '触控对齐测试报告',
    'docs.7.p': '滑动 / 点击 / 长按 / 手势与微信的逐项对照实测',
    'docs.8.h': 'SDK 说明',
    'docs.8.p': '三行接入，以及三条必须知道的约定',

    /* ── CTA / 页脚 ── */
    'cta.h2': '给我一块可写的像素缓冲，<br>剩下的交给引擎。',
    'cta.btn': '先跑一遍',
    'foot.1': '<b>mini-render</b> · MIT 许可',
    'foot.2': '内置图标矢量数据来自腾讯开源的 <a href="https://github.com/Tencent/weui">WeUI</a>（MIT）。',
    'foot.repo': '仓库',
    'foot.readme': 'README'
  };

  var DICT = { zh: ZH };
  var EN = {};                       /* 英文原文：初始化时从 DOM 抓，切回来直接还原 */
  var STORE = 'mr-lang';
  var current = 'en';

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

  function table(lang) { return lang === 'en' ? EN : DICT[lang] || EN; }

  function t(key) {
    var v = table(current)[key];
    return v == null ? (EN[key] == null ? '' : EN[key]) : v;
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
    document.documentElement.lang = lang === 'zh' ? 'zh-CN' : 'en';
    each('.lang-switch [data-lang]', function (b) {
      b.setAttribute('aria-pressed', String(b.getAttribute('data-lang') === lang));
    });
    /* 让 app.js 里那些自己造出来的按钮（复制）也跟着换 */
    document.dispatchEvent(new CustomEvent('mr:langchange', { detail: { lang: lang } }));
  }

  function pick() {
    var q = new URLSearchParams(location.search).get('lang');
    if (q === 'zh' || q === 'en') return q;
    try {
      var s = localStorage.getItem(STORE);
      if (s === 'zh' || s === 'en') return s;
    } catch (e) { /* 隐私模式下 localStorage 会抛，忽略 */ }
    return 'en';                     /* 默认英文 */
  }

  function init() {
    snapshotEnglish();
    each('.lang-switch [data-lang]', function (b) {
      b.addEventListener('click', function () {
        var lang = b.getAttribute('data-lang');
        if (lang === current) return;
        apply(lang);
        try { localStorage.setItem(STORE, lang); } catch (e) { /* 同上 */ }
        /* 让地址栏与分享出去的链接一致 */
        var url = new URL(location.href);
        if (lang === 'en') url.searchParams.delete('lang');
        else url.searchParams.set('lang', lang);
        history.replaceState(null, '', url.toString());
      });
    });
    var want = pick();
    if (want !== 'en') apply(want);
  }

  window.I18N = { t: t, apply: apply, lang: function () { return current; }, keys: Object.keys(ZH) };

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
  else init();
})();
