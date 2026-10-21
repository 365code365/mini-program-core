# Skyline 对齐差距盘点（2026-10-14）

> 只读调查：未改源码、未运行 cargo / 构建 / 截图工具。结论来自仓库文档、源码 grep 与微信官方文档。
> 读不到或无法用静态阅读确认的，标注「未验证」。外部资料以改写为主。

状态：骨架（逐节补充中）

## 0. 方法与口径

- 维度：①渲染结果 ②交互语义 ③流畅度 ④API 行为（判据见 `.kiro/steering/10-skyline-target.md`）
- 严重度：高 = 常见页面会直接看错/点错/掉帧，或移动端 SDK 与桌面行为分叉；中 = 特定组件/特性缺失，有降级；低 = 边角或纯诊断。
- 「GPU 双后端」范围：渲染后端、合成、上屏（`.agents/tasks/gpu-dual-backend-2026-10-12/`）。

## 1. C 类：项目自记 / 任务给出线索的已知问题（逐条核实）

每条格式：维度 · 当前行为 · Skyline 期望 · 证据 · 严重度 · 验收标准 · 是否属 GPU 双后端范围。

#### C-1 SDK 没接桌面的出帧闸门（CSS 动画 / swiper 过渡 / 定时器 / 网络）
- 维度：③流畅度、①渲染结果
- 当前：`MiniEngine::pump` 只在 `needs_redraw` / 有焦点输入 / 页面滚动时才出帧（`src/host/engine.rs:341-417`）。不读 `has_active_animations()`、`swiper_needs_frame()`、`has_active_timers()`、`has_pending_network()`；只调了 `advance_due_swipers()`（`engine.rs:377`），所以自动播放会「翻页」，但翻页的 300ms 滑动（`swiper.rs:17,134-146`）和 `@keyframes` 只画第一帧就停。桌面窗体的闸门在 `src/bin/app_window/frame.rs:26-43,173-180`、`winit_app.rs:211-220`。
- 宿主放大效应：iOS 每个 vsync 都调 pump（`sdk/ios/Sources/MiniRender/MiniProgramView.swift:120-134`），所以定时器、网络回包能被逻辑层处理，只是动画不出帧；**Android 只在 `pump` 返回 true 时续帧**（`sdk/android/.../MiniProgramView.kt:162`），pump 一返回 false 引擎线程就睡死，`setTimeout`/`setInterval`、`wx.request` 回包、swiper 自动播放全部停住，直到下一次触摸（静态阅读推断，未在真机验证）。
- 期望：Skyline 的 CSS Animation / Transition、swiper 自动播放按渲染线程帧连续推进，与逻辑线程是否空闲无关（[WXSS 支持](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/wxss.html)、[架构简介](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/introduction.html)）。
- 严重度：高（移动端是交付形态，桌面绿不代表 SDK 绿）
- 验收：在 `src/tests/engine_input_tests.rs` 同级加引擎测试：①含 `animation: spin 1s infinite` 的页面，连续 `pump(t0+16k)` 60 次，`pump` 返回 true ≥ 59 次，且第 1 帧与第 30 帧 RGBA 不同；②swiper 翻页后 300ms 内每次 pump 都出帧；③`setTimeout(..., 500)` 在不注入任何触摸的情况下按 16ms 步进 pump，≤ 520ms 内回调执行。Android 宿主改为「pump 返回 false 但引擎声明 `wants_frame` 时继续挂 vsync」，以 `MINI_FPS` 同等日志验证。
- GPU 范围：否（帧调度属宿主逻辑；但 GPU 上屏路径要复用同一闸门）

#### C-2 SDK 没有 setData 损伤区，每次重绘都是整条视口带
- 维度：③流畅度
- 当前：`render()` 每次都 `render_page_band` 整条「视口±余量」重画（`src/host/engine.rs:444-456,505-559`），模块头注释自认「没有接桌面那套动画/scroll-view 损伤区」（`engine.rs:14-17`）；集成指南也写「`mr_app_pump` 目前每帧整帧重绘」（`doc/原生App集成指南.md:377-379`）。桌面有 `renderer/wxml_renderer/invalidate.rs` 的增量失效。
- 期望：Skyline 的关键设计是只 patch 差异，`setData` 只触发受影响节点重排重绘（[架构简介](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/introduction.html)）。
- 严重度：高
- 验收：对 SDK 跑 `damage-check` 同款用例（增量 == 整帧逐字节）；并在 SDK 侧加 `MINI_FPS=1` 归因输出，sample-app 购物车页连续 `setData` 改数量 30 次，`需重绘` 整帧计数为 0、最慢一帧 ≤ 6.9ms（144Hz 机型，CPU 后端）。
- GPU 范围：部分（损伤区的「算」与后端无关；「上屏只传脏矩形」属上屏）

#### C-3 SDK 没有左边缘侧滑返回
- 维度：②交互语义
- 当前：`pointer_move` 传入的是一次性局部变量 `let mut edge_back = None;`（`src/host/engine_input.rs:127`），手势仲裁即便判定 `BeginEdgeBack`，状态也随栈帧丢弃；模块头注释自认未接（`engine.rs:14-17`）。桌面完整实现见 `src/host/edge_back.rs`。
- 期望：Skyline 默认路由支持 iOS 风格右滑返回，自定义路由也以手势驱动 `primaryAnimation`（[自定义路由](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/custom-route.html)）。
- 严重度：高（iOS 用户的主返回方式）
- 验收：`engine_input_tests.rs` 新增：栈深 2 时从 x=4 右划到 x=300 松手，`page_depth()` 由 2 变 1，且 `onUnload` 被调用一次；栈深 1 时同样手势 `page_depth()` 不变、页面不滚。像素侧：松手前一帧上屏图左侧出现上一页视口图（与桌面 `edge_back::compose` 同帧逐像素一致）。
- GPU 范围：部分（被覆盖页的视口快照与位移合成属合成/上屏）

#### C-4 Android 出帧没对齐 vsync，iOS 没设 preferredFrameRateRange
- 维度：③流畅度
- 当前：Android 用 `HandlerThread` + `engineHandler.post { frame() }` 自旋续帧（`sdk/android/.../MiniProgramView.kt:48-49,141-164`），没有 `Choreographer`；上屏走 `Bitmap.copyPixelsFromBuffer` + `postInvalidate`，与系统 vsync 相位无关 → 要么过度出帧耗电、要么撕帧/抖动。iOS `CADisplayLink` 未设 `preferredFrameRateRange`（`MiniProgramView.swift:120-125`，全文 grep 无该字段），ProMotion 机型默认被系统限在 60Hz；且 pump 跑在主线程（`l.add(to: .main ...)`），引擎耗时直接挤占 UIKit 主线程。另：`MiniEngine::mono_ms` 名为单调时钟，实现却是 `SystemTime::now()`（`engine.rs:651-658`），系统改时会让动画/长按计时跳变。
- 期望：Skyline 有独立渲染线程，帧节奏跟随系统 vsync（[架构简介](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/introduction.html)）。
- 严重度：高
- 验收：Android 改为 `Choreographer.postFrameCallback` 驱动，120Hz 设备上 `MINI_FPS=1` 稳态帧间隔峰值 ≤ 8.6ms、最慢一帧 ≤ 8.3ms；iOS 设 `preferredFrameRateRange(min:80,max:120,preferred:120)` 并在 Info.plist 开 `CADisableMinimumFrameDurationOnPhone`，ProMotion 机型滚动时 `CADisplayLink.targetTimestamp - timestamp` 实测 ≈ 8.3ms。`mono_ms` 换成 `Instant` 后加单测：手动回拨系统时间不影响长按 350ms 判定（可用注入时钟模拟）。
- GPU 范围：是（上屏/vsync 对齐是 GPU 双后端的 present 层要一起解决的事）

#### C-5 filter / backdrop-filter / mask-image / conic-gradient / text-shadow / will-change 全部缺失
- 维度：①渲染结果
- 当前：在 `src/renderer`、`src/parser` 里 grep 这六个名字零命中（唯一的 `mask-image` 字样是 `icon_data.rs:8,13` 的注释，指 WeUI 图标数据来源）。属性落进 `apply_property` 的 `_ => {}`（`src/renderer/components/base/style_apply/property.rs:493`）被静默丢弃。
- 期望：Skyline 支持 `filter`（单函数，不含 drop-shadow/url）、`backdrop-filter`（单函数）、`mask-image`（单张）、`conic-gradient()`、`text-shadow`、`will-change: auto|contents`（声明绘制边界）（[WXSS 支持](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/wxss.html)）。
- 严重度：中（毛玻璃导航栏、灰度置灰、文字描影在电商/资讯类常见）
- 验收：gallery 新增 6 张场景图（每个属性 1 张），与 Chrome 渲染的 H5 版本用 `pixdiff` 比，差异 ≤ 2% 且包围盒只落在该元素内；CPU 后端上 `backdrop-filter: blur(20px)` 覆盖 375×88 区域的单帧成本给出实测数字，超过 2ms 需在文档写降级策略。
- GPU 范围：是（blur/backdrop 是 GPU 合成最受益的部分；CPU 需给降级）

#### C-6 嵌套 scroll-view 不走离屏缓存
- 维度：③流畅度
- 当前：只有顶层绘制路径对 scroll-view 用 `scroll_cache`（`src/renderer/wxml_renderer/draw_interactive.rs:111-190`）；子节点路径的 scroll-view 每帧直接把可见子项重画到主画布（`draw_interactive.rs:384-460`），而真实页面里 scroll-view 几乎都是嵌套的。顶层缓存本身按整段内容高度分配画布（`draw_interactive.rs:145`），长列表会吃掉 `宽×内容高×4` 字节。
- 期望：Skyline 的 scroll-view 滚动在渲染线程合成，不依赖逻辑层（[基础组件](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/component.html)）。
- 严重度：中
- 验收：`MINI_SCROLL_LOG=1` 下拖动嵌套 scroll-view（如 sample-app `orders` 顶部 tab 条、news-app 列表），🎢 日志里「命中缓存」占比 ≥ 95%，最慢一帧 ≤ 6.9ms；离屏缓存总内存 ≤ 视口面积 × 3（改为分块/环形缓存）。
- GPU 范围：是（滚动合成层）

#### C-7 GIF 按墙上时钟取帧，且没有出帧闸门
- 维度：①渲染结果、③流畅度
- 当前：`AnimatedImage::current_index` 按 `started.elapsed()`（首次解码时刻，`Instant`）取帧（`src/renderer/components/image.rs:61-75,195`），不受 `--time`/`set_animation_time` 控制；画廊 `35_gif_frame4.png` 因此不确定（`doc/引擎测试说明.md:57`）。`is_animated()` 只在测试里被调用（`src/tests/gif_tests.rs`），宿主不会因为可见区有 GIF 而续帧 → 静止页面上的 GIF 不动，只有别的原因重绘时才跳一帧（静态阅读推断，未运行验证）。
- 期望：image 组件里的 GIF 持续播放（[image 组件](https://developers.weixin.qq.com/miniprogram/dev/component/image.html)）。
- 严重度：中
- 验收：①GIF 取帧改用渲染器动画时钟，`--time 2` 连跑两次画廊 `35_gif_frame4.png` md5 一致；②可见区有 GIF 时 `has_active_animations()` 为 true，静止页面上 1 秒内出帧数 ≥ GIF 帧数；GIF 滚出视口后不再续帧（`MINI_FPS=1` 归因里该来源为 0）。
- GPU 范围：否

#### C-8 远程图片在画布重建后停在占位图（G-1）
- 维度：①渲染结果
- 当前：项目自记：异步数据撑高页面导致画布重建后，远程图一直显示占位，滑动一下才出图；修复尝试已回滚（`doc/引擎测试说明.md` 第六节「渲染缺陷 G-1」）。
- 期望：图片加载完成即显示，触发 `bindload`（[image 组件](https://developers.weixin.qq.com/miniprogram/dev/component/image.html)）。
- 严重度：中
- 验收：`sample/tea-app pages/adoption/list` 以 `--settle 3 --time 2` 截图，图片区与 `--scroll 1 --scroll -1` 后的截图 `pixdiff` 差异为 0。
- GPU 范围：否

#### C-9 两条绘制路径行为不一致：无交互 `render()` 不做 swiper 分页（G-13）
- 维度：①渲染结果（测试可信度）
- 当前：只有 `render_with_interaction` 走 `draw_swiper_container`（`draw_interactive.rs:55-59,289-293`），画廊/离屏/compare 内置渲染不分页（`doc/引擎测试说明.md` G-13）。
- 期望：同一份源码一条渲染语义。
- 严重度：低（影响基线可信度，不影响用户）
- 验收：画廊 swiper 场景把 `current` 设为 1，两条路径输出逐字节一致。
- GPU 范围：否

#### C-10 多指手势与 `touches` 数组
- 维度：②交互语义
- 当前：`touches` 最多 1 个点，没有双指缩放/旋转（`doc/触控对齐测试报告.md` 第七节）。SDK FFI 只有单指 `nativePointer(phase,x,y)`（`MiniProgramView.kt:169-192`），Android 不读 `pointerCount`、不处理 `ACTION_POINTER_DOWN/UP`。
- 期望：`touches`/`changedTouches` 含全部触点；Skyline 的 `scale-gesture-handler` 依赖多指（[手势系统](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/gesture.html)）。
- 严重度：中
- 验收：FFI 增加 `pointer_id`；双指按下时 `bindtouchstart` 的 `e.touches.length === 2`，`identifier` 各不相同且为 number。
- GPU 范围：否

#### C-11 scroll-view 属性与事件大面积未读（G-4 / G-5）
- 维度：②交互语义、④API 行为
- 当前：src 下 grep `scroll-top|scroll-into-view|scroll-with-animation|bindscroll|scrolltolower|scrolltoupper|refresher|enable-back-to-top|show-scrollbar|scroll-y|enhanced|bounces|paging-enabled` 零命中；只读 `scroll-x`（`layout.rs:388`、`interactions.rs:176`、`draw_interactive.rs:393`）。
- 期望：Skyline 下 scroll-view「完全支持」并扩展大量新特性（`type="list|custom|nested"`、refresher、`scroll-into-view` 等）（[基础组件](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/component.html)、[scroll-view](https://developers.weixin.qq.com/miniprogram/dev/component/scroll-view.html)）。
- 严重度：高（内层列表触底加载、锚点跳转、下拉刷新是高频写法）
- 验收：探针页上：①`scroll-top="{{200}}"` 后首帧 scroll-view 偏移 = 200；②拖到底时 `bindscrolltolower` 恰好触发一次，`e.detail.direction === 'bottom'`；③`bindscroll` 的 `e.detail.scrollTop` 为 number 且与引擎偏移误差 ≤ 0.5px；④`scroll-into-view="item5"` 后 item5 顶边对齐视口顶边。
- GPU 范围：否

#### C-12 swiper：无手指拖动、无 `bindchange`、属性缺失（G-7）
- 维度：②交互语义、④API 行为
- 当前：`InteractionType` 没有 swiper（`src/ui/interaction/mod.rs:154-163`），swiper 只能靠自动播放翻页，不跟手；src 下 grep `circular`、`bindchange`/`source` 无 swiper 相关实现；翻页时长写死 300ms（`swiper.rs:17`，微信 `duration` 默认 500ms）；自动播放从末页回首页时按 `prev→current` 插值，会**倒着滑过全部页**（`swiper.rs:114-128,148-158`）。
- 期望：swiper「完全支持」，拖动跟手、`bindchange` 的 `e.detail = {current: number, source: 'autoplay'|'touch'}`（[基础组件](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/component.html)、[swiper](https://developers.weixin.qq.com/miniprogram/dev/component/swiper.html)）。
- 严重度：高（首页 banner 是最常见组件之一）
- 验收：①横划 swiper 120px 松手翻到下一页，`bindchange` 收到 `{current:1, source:'touch'}` 且 `typeof current === 'number'`；②`circular` 下末页→首页向前滑动一屏（中间帧偏移单调）；③不写 `duration` 时翻页动画 500ms（±1 帧）。
- GPU 范围：否

#### C-13 WXML 能力缺口（G-2）、WXSS `@media`/`@font-face`（G-3）、form/picker 事件（G-8）、canvas 2d（G-9）
- 维度：①④
- 当前：项目自记 `doc/引擎测试说明.md` 第六节 G-2/G-3/G-8/G-9（`wx:key`、`template is`、`import/include`、`slot`、`wxs` 不支持；无 `@media`、`@font-face`；`form` 不派发 submit/reset；`picker` 不派发 `columnchange`；canvas 只读 `canvas-id`）。本次未逐项复核源码（未验证是否已有局部修复）。
- 期望：Skyline 支持 Media queries（仅 DarkMode）与 `@font-face`（仅 ttf）（[WXSS 支持](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/wxss.html)）；form、canvas「完全支持」（[基础组件](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/component.html)）。
- 严重度：高（`slot`/`template`/`wxs` 缺失会让组件化项目结构直接缺块）
- 验收：每项各一个最小探针页 + 单测；`slot` 用例：组件 `<slot/>` 处渲染出调用方子节点，具名 slot 落位正确，截图与 H5 `pixdiff` ≤ 1%。
- GPU 范围：否

#### C-14 wx.* 缺口（G-10 / G-11）
- 维度：④API 行为
- 当前：项目自记未实现 `pageScrollTo`、`createSelectorQuery`、`createIntersectionObserver`、`nextTick`、`setNavigationBar*`、`setTabBar*`、`vibrate*`、`login` 等；`showActionSheet` 是直接回 `tapIndex: 0` 的空壳（`doc/引擎测试说明.md` G-10/G-11）。详见 §2.6 复核。
- 严重度：高（`createSelectorQuery` / `nextTick` / `pageScrollTo` 是组件库刚需）
- 验收：见 §2.6。
- GPU 范围：否

## 2. A 类：Skyline 支持、本引擎缺失或部分支持

### 2.1 组件
（待补）

### 2.2 WXSS
（待补）

### 2.3 手势系统
（待补）

### 2.4 worklet 动画
（待补）

### 2.5 路由与自定义转场
（待补）

### 2.6 wx.* API
（待补）

## 3. B 类：已实现但行为与 Skyline 不一致

（待补）

## 4. 前 10 项优先级

（待补）

## 附：参考链接

（待补）
