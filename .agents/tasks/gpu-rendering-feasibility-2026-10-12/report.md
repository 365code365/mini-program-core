# 双渲染后端（CPU 参考 + GPU 可选）落地方案调查报告

> 只读调查，没有改任何源码，也没有加依赖。测量用的是本机 release 构建（Apple M3 Pro、11 核、macOS），跑完已执行
> `rm -rf target/mini-storage` 和 `bash tools/clean-target.sh`，`target/baseline_*` 都保留着。
> 依据 `.kiro/steering/10-skyline-target.md` 的最新条文（双渲染后端；CPU 是参考实现、永不删除；GPU 可用 feature 关掉；
> 失败回退 CPU；后端只管画；逐字节判据以 CPU 为准），本报告重点讲「怎么做」。

---

## 0. 结论（先看这一节）

**一句话**：GPU 能明显改善「滚动 / 合成 / 上屏 / 帧节奏」以及以后的「模糊类效果」，但现在测到的几个最慢帧大多是
**CPU 重新光栅化**和**布局重建**造成的，光把合成挪上 GPU 解决不了。所以路线是：先把 GPU 接成**合成器**
（内容仍由 CPU 光栅化，GPU 负责贴图、滚动偏移、变换、混合、上屏），再做**两后端共用的图层化**
（把 scroll-view 和动画子树做成可缓存的图层）。真正的「GPU 光栅化」放到最后，由数据决定做不做。

| 场景（本机实测或文档实测） | 现在 | 瓶颈 | GPU 合成器（P2/P3） | 图层化（P4，两后端都受益） | GPU 光栅（P5，可选） |
|---|---|---|---|---|---|
| 页面级滚动帧（sample-app 首页 `--drag 8x60`，settle 2） | 渲染 p50 0.52~0.81ms + 上屏 2~3ms（文档） | **上屏拷贝** | ✅ 上屏降到提交一次 draw，CPU ≈0 | — | — |
| 滚动跨出已绘制条带的那一帧 | **17.0~18.0ms @2x，24.1ms @3x** | CPU 重画整条带 | ❌ | ✅ 只补画新露出的条带（CPU 侧也能做） | 部分 |
| 嵌套 scroll-view 拖动（tea-app 分类页） | **p50 5.5ms @2x，p50 13.1ms @3x** | 每帧重画整块 scroll-view | ❌ | ✅ scroll-view 做成独立图层，滚动只改偏移 | 部分 |
| 首屏图片加载期整帧重绘（首页 settle 0） | **p50 19~25ms**，其中 image 每帧约 8ms | 图片重采样 + 整帧 | ❌ | 部分 | ✅ 图片采样是 GPU 的强项 |
| transform/opacity 动画（弹窗 popIn 等） | 每帧离屏重画子树 + 逐像素逆仿射（`compose.rs`） | CPU 采样 | ✅ 变换由着色器采样 | ✅ 子树内容缓存，只改矩阵 | — |
| 每秒一次 setData（首页倒计时） | 最慢一帧 11.4ms（文档：模板 1.0 + 建树样式 4.0 + 布局 0.3） | **布局/模板，不是光栅** | ❌ | ❌ | ❌（要做增量树打补丁） |
| `filter` / `backdrop-filter: blur()` | **没实现**（Skyline 支持单函数） | — | ✅ 在合成阶段做 | — | ✅ |
| 移动端 SDK 每一帧 | 合成 → u32 → RGBA8 → `mr_app_pixels` 拷贝 → `CGImage` / `copyPixelsFromBuffer`，整帧至少 4 次拷贝（**未在真机计时**） | **拷贝 + 没有损伤区** | ✅ 收益最大的地方 | ✅ | — |

**选型**：只引入 **wgpu**（锁死版本，当前稳定版 v30.0.1），在上面写自己的合成器，以后需要的话再写自己的图元渲染器。
不用 vello、skia-safe、femtovg。原因是它们的抗锯齿和文字光栅化跟 CPU 参考实现不一样，「GPU 与 CPU 同帧差异 ≤ 阈值」
这条判据会退化成几个百分点的噪声，没法再拿来判定（见第 3 节）。

**接缝放在合成层**，不放在渲染树层：`WxmlRenderer` 照旧往 `Canvas` 里画（布局、命中、事件、增量失效、动画时钟一行不动），
新增的 `Compositor` trait 接管现在 `host::render::present_to_buffer` 以及之后那串覆盖层绘制。这样「后端只管画」是结构上保证的，
不靠约定。合成器用 `textureLoad` 加整数混合公式复刻 CPU 的合成算术，**静态页可以做到 GPU 与 CPU 逐字节一致**，阈值只在
半透明覆盖层和变换图层上才需要。

**有两件事必须先做（跟 GPU 无关，但 GPU 要接在它们上面）**：
1. SDK（`host::engine::MiniEngine`）没有接桌面那套出帧闸门：没有 setData 损伤区，也不看 `has_active_animations()`、
   `swiper_needs_frame()`、`has_active_timers()`（`grep` 结果为 0）。从代码推断，**移动端的 CSS 动画和 swiper 滑动过渡
   不会持续出帧**（未在真机验证）。Android 宿主没有用 Choreographer 对齐 vsync，iOS 的 `CADisplayLink` 也没设
   `preferredFrameRateRange`。这些不修好的话，在高端机上换哪个后端都到不了 120Hz。
2. 桌面和 SDK 各自复制了一份合成调用序列（`frame_render.rs::present` 和 `engine.rs::compose_frame`）。要先把它们收成
   同一个 CPU 合成器（纯搬迁，用 md5 证明），GPU 才能一次接进两个宿主。

**工作量（估算，未验证）**：P0~P4 合计约 45~65 人日，外加真机矩阵的测试时间。P5 不封顶，按数据逐项立项。

---

## 1. 现状：像素是怎么来的、怎么上屏的

### 1.1 光栅化：纯 CPU、立即模式、没有显示列表

- `src/canvas/mod.rs`：`Canvas { pixels: Vec<Color>, clip_rect, translation, state_stack }`，`Color` 是 **RGBA8 直通 alpha**
  （没有预乘）。按职责分成 `pixels`（单点/行/掩膜混合）、`shapes`（矩形/圆/线）、`paths`（扫描线 + even-odd、4× 超采样）、
  `images`（两级过滤 + 缩放缓存）。没有 GPU 依赖，`Cargo.toml` 里跟图形有关的只有 `softbuffer`（桌面上屏）和 `fontdue`（字形）。
- **没有显示列表这一层**。渲染器边遍历边直接调 `Canvas` 方法：`&mut Canvas` 作为参数出现了 **71 处**（components 31、
  wxml_renderer 22、base 6……）。另外有十多处**直接读写像素**：`compose.rs::blit_transformed`（`src.pixels().to_vec()` 后逐像素
  逆仿射 + `set_pixel`）、`gradient.rs`（`fill_span` / `blend_pixels`）、`shadow.rs`（`blend_mask`）、`text.rs:570`（`blend_coverage`）、
  `box_paint.rs:217`、`components/canvas/*`、`scroll_cache.rs`（`blend_pixels`）、`draw.rs:204`（`blend_pixels_opacity`）。
- 文字（`src/text.rs::draw_text_weighted`）：fontdue 按 `(char, size*10, bold)` 光栅成覆盖率位图并缓存，落点逐列逐行 `round()`
  到整像素，再 `blend_coverage` 线性混合（不做 gamma）。彩色 emoji 走 sbix 位图 `draw_image`。
- 阴影（`components/shadow.rs`）：8 位覆盖率掩膜，三遍盒式模糊逼近高斯，按几何尺寸缓存，每帧只做一次 `blend_mask`。
- transform（`wxml_renderer/animation.rs::draw_with_transform` 和 `renderer/compose.rs`）：非纯平移时**每帧**新建离屏 `Canvas`，
  重画子树，再逐像素双线性逆仿射贴回。离屏内容**不跨帧缓存**。
- opacity < 1（`draw.rs::paint_opacity_group`）：同样每帧离屏重画，再 `blend_pixels_opacity` 贴回。
- scroll-view 离屏缓存（`ui/scroll_cache.rs`）**只在顶层路径生效**：`draw_interactive.rs:112-200` 的 `draw_with_interaction`
  会用缓存；子节点路径 `draw_child_with_interaction`（同文件 :384-460）是裁剪后**每帧直接把子元素重画进页面画布**。
  tea-app 这类 uni-app 产物是 `<scroll-view class="page">` 套在根 view 里，走的就是后者（本次 `MINI_SCROLL_LOG=1` 拖动
  60 帧，一行 `🎢` 缓存日志都没出现，可以佐证）。

### 1.2 合成：其实已经是「几张图层 + 一个合成器」

| 图层 | 画布 | 坐标 | 何时重画 | 合成方式（`host/render.rs::present_to_buffer`） |
|---|---|---|---|---|
| 页面内容 | `canvas`，整页高（首页 750×6870） | 内容坐标 | 数据变 / 动画损伤区 / 滚出已绘制条带 | 按 `scroll_offset` 取行切片，RGB 拷贝 |
| tabBar | `tabbar_canvas` | 视口底部 | 换页 / 整帧 | RGB 拷贝（忽略 alpha） |
| fixed 覆盖层 | `fixed_canvas`，视口大小 | 视口坐标 | `fixed_dirty` / 覆盖层自带动画 | 只合成 `opaque_row_span` 那几行，整数公式 `(c*a + d*(255-a))/255` |
| 下拉刷新指示器 | 临时 band `Canvas` | 视口顶部 | 每帧 | 逐像素整数混合（`render_pull_indicator`） |
| Toast / Loading / Modal | 直接画进 u32 缓冲 | 视口 | 每帧 | `ui_overlay::render_ui_overlay` |
| picker 面板 | 直接画进 u32 缓冲 | 视口 | 每帧 | `picker_sheet::render` |
| 侧滑返回 | 上一页视口截图 + 当前帧右移 | 视口 | 跟手 | `edge_back::compose`（`copy_within` + 逐行填充，仅桌面） |

顺序写死在两处：桌面 `src/bin/app_window/frame_render.rs::present()`，SDK `src/host/engine.rs::compose_frame()`。
**这张表就是 GPU 合成器的输入**，不需要从零发明图层树。

### 1.3 上屏与帧调度

- **桌面**：`softbuffer` 拿到 u32 缓冲，经 `present_to_buffer` 写入后 `buffer.present()`。softbuffer 不阻塞 vsync，帧节奏靠
  `winit_app.rs:255-290` 的「固定节拍累加 + sleep 到差 0.9ms + 自旋」。`frame_render.rs` 的注释记录了**偶发 30~50ms 的上屏停顿**，
  发生在 `buffer_mut()` / `present()` 内部（macOS 合成器那一侧）。
- **SDK**：`MiniEngine::pump → render → compose_frame` 先在 `self.compose: Vec<u32>` 上跑 `present_to_buffer` 和覆盖层，
  再整帧转成 `self.rgba`（RGBA8）。宿主侧：
  - iOS（`sdk/ios/Sources/MiniRender/MiniProgramView.swift:130-163`）：`CADisplayLink` → `mr_app_pixels` 拷进 Swift 数组 →
    `CGImage` → `imageLayer.contents`（Core Animation 还要再上传一次纹理）。没设 `preferredFrameRateRange`。
  - Android（`MiniProgramView.kt:139-167`）：`HandlerThread` 上 `nativePump` → `nativePixels` 拷进 `ByteArray` →
    `copyPixelsFromBuffer` → `postInvalidate` → `drawBitmap`（硬件加速画布再上传一次）。没用 Choreographer，
    `changed` 为真时就立刻 `post` 下一帧。
  - 按 3× 屏 1170×2532 估算，一帧大约 2.96M 像素、12MB，**每帧整帧拷贝至少 4 次**（compose→rgba→宿主数组→Bitmap/CGImage，
    还没算系统的纹理上传）。耗时**未在真机测**。
- `doc/原生App集成指南.md:202-220` 已经建议宿主改用 `ANativeWindow_lock` 或 Metal 纹理来少拷一次，但 SDK 还没这样做。

### 1.4 SDK 宿主与桌面宿主的行为差距（会直接影响 GPU 收益）

`engine.rs` 文件头自己写明「其余重绘都是整条带重画，没有接桌面那套动画/scroll-view 损伤区」。核对代码后确认：
`MiniEngine::pump` 只在 `needs_redraw` / 有焦点输入框 / 滚动时出帧，**没有任何地方读 `has_active_animations()`、
`swiper_needs_frame()`、`has_active_timers()`、`has_pending_network()`**。桌面那边的 `frame.rs::is_animating` 会读这些。
所以从代码推断，移动端的 `@keyframes` 动画、swiper 0.3s 滑动、骨架屏闪烁都不会连续出帧（**未在真机验证**）。
这一条同时也属于用户第 1 问（还有哪些没对齐）。

---

## 2. 瓶颈在哪：实测数据

### 2.1 本次无头实测（`mini-app-window --drag 8x60`，release，M3 Pro，单次运行）

`--drag` 计的是「输入 + 逻辑 + 渲染」，**不含上屏**（`headless_run.rs::run_drag_bench` 调的是 `pump_one_frame`，不 present）。
无头模式的预算按 60Hz（16.67ms）统计。

| 命令（都先 `rm -rf target/mini-storage`） | 平均 | p50 | p95 | 最慢 | 最慢帧构成 |
|---|---|---|---|---|---|
| `sample-app --route pages/index/index --settle 2 --drag 8x60` | 1.49 / 1.19ms | 0.81 / 0.52 | 1.83 / 1.21 | **17.98 / 17.00** | 页面 17.75ms |
| 同上 `--scale 3` | 1.96 | 0.97 | 3.67 | **24.13** | 页面 23.86ms |
| `sample-app ... --settle 0 --drag 8x60`（首屏图片/骨架期间） | 21.03 | 19.00 | 30.09 | 39.87 | 页面 39.49ms |
| 同上 `--scale 3` | 28.22 | 25.51 | 46.12 | 50.56 | 页面 25.98 + 覆盖层 23.93 |
| `sample/tea-app --route pages/category/category --settle 2 --drag 8x60`（三次） | 5.34 / 5.52 / 7.09 | **5.50 / 5.50 / 5.46** | 5.93 / 6.29 / 11.82 | 6.00 / 15.48 / 75.63 | 页面占 99% |
| 同上 `--scale 3`（单次） | 15.60 | **13.09** | 27.11 | 51.86 | 页面 51.67ms |
| `news-app --route pages/home/home --settle 2 --drag 8x60` | 1.23 | 0.10 | 1.10 | **39.24** | 页面 38.31ms |
| 同上 `--scale 3` | 2.06 | 0.09 | 23.67 | 39.43 | 页面 38.86ms |

再用 `MINI_DRAW_LOG=1` 归因：首页 settle 0 的 20 帧里，`image` 161.2ms / 524 次（约 8ms/帧）、`view` 76.6ms、
`背景:纯色` 38.7ms、`背景:圆角填充` 36.4ms、`text` 30.0ms。同时 `MINI_LAYOUT_LOG` 打出「退回整帧：结构变化」。

**怎么读这些数字**：
- 稳态页面滚动本身很便宜（p50 < 1ms），时间都花在跨条带的那一帧整带重画上（17~24ms，144Hz 下要丢 2~3 帧）。这是**光栅**瓶颈，
  但它是「一次画太多」造成的，不是「画得慢」：只补画新露出的条带就能消掉。
- 嵌套 scroll-view 拖动是**持续性**的光栅瓶颈：@2x 下 p50 5.5ms，压着 6.94ms 预算的边；@3x 下 13ms，在 120Hz 机上一定掉帧。
- 首屏加载期每帧整帧重绘，主要花在图片采样上。GPU 采样能大幅降低，但更根本的是别每帧都整帧重绘。
- 单次运行有明显的长尾噪声（tea-app 同一条命令最慢帧 6.0 / 15.5 / 75.6ms，推测是远程图到达或系统抢占），所以**验收必须多次取中位数**。

### 2.2 文档里已有的实测（README、`doc/架构与实现原理.md`）

- 首页整帧 6.1ms；稳态帧间隔 6.7~7.2ms；最慢一帧 / 帧间隔峰值 11.4 / 11.4ms，剩下那次超时是 setData 整棵重建布局树
  （模板 1.0 + 建树样式 4.0 + 布局 0.3ms），**跟光栅无关**。
- 滚动帧「只花上屏的 2~3ms」（750×1334，约 100 万像素的 RGB 转换和拷贝）。
- 带全屏遮罩弹层的状态下约 89 FPS（≈11ms/帧）。
- 字体首帧建树 955ms、内存峰值 600MB~1GB 主要在字体：**GPU 都帮不上**。

### 2.3 结论：GPU 能帮和帮不上的

- **能帮**：上屏和合成（桌面 2~3ms → ≈0，移动端收益更大）；vsync 对齐的帧节奏（swapchain `Fifo`，替代 sleep + 自旋，
  能否消掉 macOS 那 30~50ms 停顿**未验证**）；transform/opacity 动画的采样；侧滑返回的合成；图片缩放采样；`blur` 类滤镜。
- **帮不上**：setData 布局重建（11.4ms 尖峰的根因）、模板求值、JS、字体加载与内存、首帧建树。
- **只靠 GPU 合成帮不上，但图层化能帮**：跨条带重画、嵌套 scroll-view 滚动、动画子树每帧离屏重画。图层化 CPU 也能用，
  所以算两后端共享的收益。

---

## 3. 后端选型

| 维度 | **wgpu + 自研合成器 / 图元渲染器（推荐）** | vello 0.8 / vello_hybrid | skia-safe（Ganesh / Graphite） | femtovg |
|---|---|---|---|---|
| 桌面 | Metal / Vulkan / DX12 / GL | 经 wgpu | ✅ | GL / wgpu |
| iOS | Metal | 经 wgpu | ✅（预编译 aarch64-apple-ios / sim） | 经 wgpu |
| Android | Vulkan，GLES3 兜底 | 经典版要 compute shader；hybrid 走 GLES | 预编译只有 **aarch64 / x86_64，没有 armeabi-v7a**（本项目有出这个 ABI） | GLES3 / wgpu |
| 鸿蒙 | 未验证 | 未验证 | 没有预编译 | 未验证 |
| 无头快照 | 不需要 surface，渲染到纹理再读回 | ✅ | ✅ | 需要 GL 上下文 |
| 文字 / 中文字体 | **直接复用 fontdue 的覆盖率位图做字形图集**，字形与 CPU 一致 | 自己的 skrifa / Glifo 栈，字形 AA 不同，还要再加载一份字体（内存本来就是最大短板） | 自己的字体引擎，字形不同 | 自己的字体栈 |
| 与 CPU 的像素一致性 | 合成算术可复刻 → **静态页 0 差异**；以后的图元用同一套 SDF 线性斜坡（CPU 圆角就是这么算的），差异很小 | 解析式面积覆盖，与扫描线 4× 超采样不同，全屏都是小差异 | 不同 | 不同（nanovg 式 AA） |
| 体积 / 交叉编译 | 纯 Rust，cargo 交叉编译就行；增量体积未测（旧 issue 记录一个小程序 release 5~8MB） | 比 wgpu 多 shader 和 vello 本体 | C++ 构建，需要 LLVM / Python / Ninja，每个架构多几 MB | 小 |
| 维护 | 活跃，2026 年仍在发 v29、v30；**大约一季度一个破坏性大版本** | 1.0 之前；hybrid 自评「大致 beta」 | 活跃（跟 Chrome m155） | 活跃（Slint 在用） |

推荐 wgpu 的理由：
1. 它是唯一能一次性覆盖桌面、iOS、Android 和无头读回的**纯 Rust** 选项，交叉编译方式跟现在的 `tools/build-mobile.sh` 一致。
2. 它只提供 GPU 访问，不自带成像模型。抗锯齿、混合、字形都由我们决定，所以才能让 GPU 结果**贴着 CPU 参考实现**走。
   vello、skia、femtovg 都自带 AA 和字体栈，用了它们，「GPU 与 CPU 同帧差异 ≤ 阈值」只能把阈值放宽到几个百分点，这条判据就失去意义了。
3. Skyline 需要 GPU 的地方（合成、`transform`/`opacity`、滚动、`blur`）用合成器就能覆盖，暂时用不上完整的矢量引擎。

用法约定：Cargo 里写成 `wgpu = { version = "=30.0.1", optional = true, default-features = false, features = [<按平台选 metal/vulkan/gles>] }`
（具体 feature 名在 P2 动手时按 v30 文档核对，**未验证**），外加一个已知的小型 `block_on`（如 `pollster`，同样锁版本）
用来跑 `request_adapter` / `request_device`。升 wgpu 大版本单独立项，不和功能改动混在一起。

---

## 4. 对测试体系的冲击，以及怎么守住

### 4.1 三种做法对比

| 做法 | 收益 | 像素风险 | 结论 |
|---|---|---|---|
| A. GPU 只做合成（内容仍由 CPU 光栅化，作为纹理上传） | 上屏、滚动、变换、混合、vsync、模糊 | 能做到静态页 0 差异；只有半透明覆盖层（±1~2 LSB）和变换采样有微小差异 | **P2~P4 走这个** |
| B. GPU 全光栅（vello / skia 一类） | 光栅本身快 | 全屏都有 AA 差异，文字整体不同 | 不推荐 |
| C. 混合：A 加上逐项把图元挪上 GPU（矩形 / 圆角 SDF、渐变、图片、字形图集），复杂路径仍由 CPU 光栅成小纹理 | 按需拿到光栅收益 | 每挪一类图元，阈值重新标定一次 | **P5 按数据逐项做** |

### 4.2 判据怎么排

1. **CPU 判据一条不动**：`damage-check`、`sdk-parity`、gallery 65 张、重构 md5、`compare` 基线（sample 4.55%、news 6.33%）
   都按 CPU 跑。为防止误用：**无头模式（`--snapshot`、`--frames`、`--drag`、所有 examples）默认用 CPU**，只有显式传
   `--backend gpu` 才走 GPU；交互窗体默认 `auto`。各个 `tools/*.sh` 里再显式 `export MINI_BACKEND=cpu`，做双保险。
2. **新增 `tools/gpu-parity.sh`**：同一页、同一个 `--time` / `MINI_FAKE_NOW`，分别用 `--backend cpu` 和 `--backend gpu`
   出整帧（GPU 走渲染到纹理再读回），用 `pixdiff.py` 比较。脚本要报三个数：差异像素占比、包围盒、最大通道差。
   再给 `pixdiff.py` 加一个 `--tol N` 参数：通道差 ≤N 的像素算一致。
3. **阈值怎么定死**：P2 落地时在本机（M3 Pro）和至少一台 Windows 或 Linux 独显机上，把 sample 15 页、news 6 页、
   7 张交互快照全跑一遍：
   - 「无 UI 浮层、无变换图层」的帧：**差异像素必须是 0**（复刻整数公式就该做到，做不到说明合成器写错了）；
   - 带 Toast / Modal / picker / 下拉指示器的帧：`--tol 2` 下差异为 0，`--tol 0` 下的占比取实测最大值的 1.5 倍、向上取整到 0.01%；
   - P4 有变换图层之后，再按同样方法标定一次。
   - 所有阈值都写进 `doc/引擎测试说明.md`，包围盒必须落在对应图层的矩形里（落在外面直接判失败）。
4. **防空转**（同 `damage-check` 第一版的教训）：`gpu-parity.sh` 先 grep 日志里的 `后端 GPU（<adapter>）`，没有就失败，
   不能因为两边都悄悄跑了 CPU 而全绿。另外跑一次 `MINI_GPU_FAULT=tint`（着色器故意偏一个色阶），这时必须报不一致，
   证明比较链路是活的。
5. **没有 GPU 的 CI 机器**：打印「⚠️ GPU 未测：没有可用适配器（软件适配器不算）」，并用**单独的退出码**（例如 3），
   不能算通过。另外 `cargo build --no-default-features`（不带 gpu）也要编译通过，并跑全部 CPU 回归。
6. **流畅度按后端分开测**：`--drag` 和 `MINI_FPS` 都标明后端。GPU 后端加报「CPU 提交耗时」和「GPU 完成耗时」
   （用 `device.poll(Wait)` 或 timestamp query），并给出与 CPU 后端同场景的对比数字（steering 要求）。

### 4.3 为什么合成器能做到逐字节

CPU 合成只有三种算术：RGB 直接拷贝（页面、tabBar）、`(c*a + d*(255-a))/255` 整数截断混合（fixed 层、下拉指示器）、
覆盖层自己的图元绘制。GPU 端**不用硬件 blend**（它是浮点四舍五入，会差 1 LSB），改成**一趟 uber 着色器**：用 `textureLoad`
按整数坐标取各层像素（不经过采样器过滤），用 WGSL 的 `u32` 运算原样复刻上面的公式，写进非 sRGB 的输出格式
（`Rgba8Unorm` / `Bgra8Unorm`，避免伽马转换）。Toast / Modal / picker 这类「直接画进缓冲」的覆盖层，GPU 路径要改成先画进一张
透明 `Canvas` 再合成，CPU 路径**保持原样**。差异只会出现在这几块区域，所以第 4.2 节第 3 条要单独给阈值。

---

## 5. 架构方案

### 5.1 分层与接缝

```text
 逻辑层(QuickJS) → 模板/样式/布局(taffy) → FramePlan/损伤区(invalidate.rs) → 命中/事件/动画时钟
                                                     │            ← 以上与后端无关，一行不动
                                         WxmlRenderer 往 Canvas 画（CPU 光栅，内容的唯一来源）
                                                     │
                            ┌──────────── FrameLayers（本帧的图层描述 + 每层的脏矩形）────────────┐
                            │  page(内容坐标, scroll_px) · tabbar · fixed(rows) · pull · ui_overlay  │
                            │  picker · edge_back(offset, shot) · [P4] scroll-view / 动画子树图层     │
                            └───────────────────────────────┬────────────────────────────────────┘
                                         trait Compositor   │   ← 接缝 1（现在就做）
                       ┌────────────────────────────────────┴───────────────────────┐
              CpuCompositor（搬迁现有代码，逐字节不变）               GpuCompositor（feature "gpu"，wgpu）
              → u32 缓冲 → softbuffer / mr_app_pixels                → 纹理（按脏矩形局部上传）→ uber 合成 → swapchain
                                                                       或渲染到纹理 → 读回（无头 / 测试）
```

- **接缝 1：`src/compositor/`**（与 `src/canvas/` 平级的新目录，每个文件 < 500 行）
  - `mod.rs`：`BackendPref { Auto, Cpu, Gpu }`、`BackendKind`、`FrameLayers<'a>`、`trait Compositor { fn kind(); fn compose(&mut self, &FrameLayers) -> Result<(), CompositeError>; fn read_back(&mut self, &mut Vec<u8>) -> Result<…> }`
  - `cpu.rs`：把 `present_to_buffer` → `render_pull_indicator` → `render_ui_overlay` → `picker_sheet::render` → `edge_back::compose`
    这串调用按原顺序**原样搬过来**，桌面 `present()` 和 SDK `compose_frame()` 改为都调它。
  - `select.rs`：自动选择与机型分级（第 5.3 节）。
  - `gpu/`（`#[cfg(feature = "gpu")]`）：`device.rs`（实例、适配器、设备、丢失回调）、`surface.rs`（swapchain 配置与重建）、
    `tiles.rs`（页面图层分块纹理与脏块上传）、`pipeline.rs`、`shaders/compose.wgsl`、`readback.rs`、`fault.rs`（`MINI_GPU_FAULT` 注入）。
- **接缝 2（P5 才做，可选）**：把渲染器里的 `&mut Canvas` 换成 `&mut dyn PaintTarget`。trait 方法与 `Canvas` 现有公开方法一一对应
  （`fill_round_rect`、`draw_path`、`blend_coverage`、`blend_mask`、`draw_image_cached`、`save/restore/clip_rect` ……），
  `Canvas` 自己实现它。这一步先做纯机械替换，用 md5 证明等价，然后再加 GPU 录制端。
- **为什么不在 RenderNode 层分叉**（即让每个后端自己遍历渲染树）：绘制遍历里夹着大量非绘制的副作用，比如
  `push_event_bindings`、`register_interactive_element`、`fixed_layer.rs` 在绘制期对 fixed 子树重排、`animated_bounds` 收集、
  `image_state::probe_load` 派发 `bindload`、swiper 滑动位置。在这一层分叉等于把这些逻辑复制成两份。
  `doc/架构与实现原理.md` 记录过「状态落地三份实现、其中一份是退化版」和「SDK 指针层是简化版」两次分叉，这里会重演。

### 5.2 与布局缓存、增量失效的配合

- `FramePlan::Damage { rect, fixed_changed }`、动画损伤区、scroll-view 盒子这些**已经算好的脏矩形**，原样传给
  `FrameLayers.page_dirty`。GPU 端只把这块矩形 `queue.write_texture` 到对应的纹理分块，上传量和 CPU 的重画量一致。
- `FramePlan::Unchanged` 加滚动时，GPU 端**零上传**，只改 uniform 里的 `scroll_px`。这对应现在「滚动不重绘、只换切片」的语义，
  `viewport_inside_drawn_band` 不变。
- `fixed_changed == false` 时不重传 fixed 纹理（现在就复用画布）；`opaque_row_span` 继续用来裁出合成区间。
- 页面画布高度可能超过纹理上限（3× 屏下首页 ≈ 10305 行，超过常见的 8192；wgpu 降级限制是 2048）。所以页面图层**必须分块**：
  按 512 行一块、全宽，只让「视口 ± 裁剪余量」附近的块常驻显存，其余淘汰。显存占用因此有上界，不会是整页的两倍。

### 5.3 机型分级与自动选择

| 输入 | 来源 | 规则（Auto 模式） |
|---|---|---|
| 编译期 | `cfg(feature = "gpu")` | 没编进来就是 CPU |
| 适配器 | wgpu `AdapterInfo { backend, device_type, vendor, device, driver }` | `device_type` 必须是 Integrated 或 Discrete；`Cpu`（llvmpipe、SwiftShader、lavapipe）和 `Other` 一律拒绝；backend 只接受 Metal、Vulkan、Dx12，GLES 只在显式 `gpu` 或命中白名单时使用 |
| 平台 | 宿主传入（新 C ABI） | iOS：有 Metal 设备即可（驱动统一可靠），`physicalMemory < 2GB` 时走 CPU；Android：API ≥ 29、Vulkan ≥ 1.1、`isLowRamDevice() == false`、`totalMem ≥ 4GB`，不在黑名单里（`vendor:device:driver` 三元组，初始名单在 P3 真机测完后定，**不预先编造型号**） |
| 自检 | 首次启用时 | 用固定的测试帧（纯色块 + 半透明块 + 分块边界）跑一次 GPU 合成并读回，与 CPU 结果**逐字节**比较，不一致就禁用 GPU |
| 运行期 | 前 N 秒的帧统计 | GPU 帧 p95（CPU 提交 + GPU 完成）比同场景 CPU 估计还慢，或出现一次设备丢失，就降级 |
| 记忆 | `data_dir` 下 `gpu-verdict.json`（键：适配器四元组 + 引擎版本） | 自检失败、设备丢失、性能降级都会记下来，同一驱动下次启动直接走 CPU；驱动或引擎版本变了就重新评估 |

**显式指定**（steering 要求「可强制、可观测」，与诊断开关同等对待，写进 README）：
- 桌面：`MINI_BACKEND=cpu|gpu|auto` 和 `--backend cpu|gpu|auto`（命令行优先）。
- SDK：`mr_app_set_backend(app, 0=auto | 1=cpu | 2=gpu)`（在挂 surface 之前调用）；`mr_app_backend(app)` 返回实际在用的后端；
  `mr_app_backend_reason(app)` 返回选择或回退的原因字符串。Swift / Kotlin 各加一个 `backend` 属性。
- 强制 `gpu` 但用不了时**仍然回退 CPU**（不能黑屏），原因写进 reason 和日志。
- 观测：`MINI_FPS=1` 的每秒那行最前面加 `[后端 GPU · Metal · Apple M3 Pro]` 或 `[后端 CPU（GPU 不可用：<原因>）]`；
  新开关 `MINI_GPU_LOG=1` 打印初始化、自检、每帧上传字节数、回退事件。

### 5.4 回退机制

在 P2~P4，内容始终由 CPU 画进 `Canvas`，GPU 纹理只是镜像。所以**任何一帧都可以无损切回 CPU 合成**，
不需要重画。

| 触发 | 检测点 | 动作 |
|---|---|---|
| 初始化失败（没有适配器、`request_device` 报错、surface 创建失败） | `select.rs` | 记日志 `⚠️ GPU 初始化失败：<err>，回退 CPU`，从这一帧起就用 `CpuCompositor` |
| surface `Outdated` / `Suboptimal` / `Lost` | `get_current_texture()`（v29 起返回 `CurrentSurfaceTexture` 枚举） | 先重新配置 surface 一次，再失败就按设备丢失处理 |
| 设备丢失 / 未捕获的校验错误 | `device.set_device_lost_callback`、`on_uncaptured_error` | 丢掉全部 GPU 状态，下一帧走 CPU，写入 `gpu-verdict.json` |
| 自检不一致 | 首次自检 | 禁用 GPU 并记下来 |
| 运行期性能退化 | 帧统计 | 降级并记下来 |

宿主怎么配合：SDK 宿主保留**两条显示路径**，GPU 用 `CAMetalLayer` / `SurfaceView`，CPU 用现在的 `CGImage` / `Bitmap`。
`mr_app_pump` 新增返回值 `2` 表示「后端变了，请切换显示路径」。桌面端 wgpu 和 softbuffer 能不能在同一个 winit 窗口上来回切
（macOS 上两者都挂 `CALayer`），**未验证**，P2 要先做这个探针，切不了就退成「重建窗口 surface」。
回退路径必须能测：`MINI_GPU_FAULT=init|lost@<帧号>|mismatch|tint`，每种都要有对应的测试。

### 5.5 SDK 接口变更（只增不删）

```c
// 新增：GPU 上屏需要宿主给原生 surface；CPU 路径仍然用 mr_app_pixels
int  mr_app_set_backend(MRApp*, int pref);                 // 0 auto / 1 cpu / 2 gpu
int  mr_app_attach_surface(MRApp*, int kind, void* handle, uint32_t w, uint32_t h);
                                                           // kind: 1 = CAMetalLayer*，2 = ANativeWindow*
void mr_app_detach_surface(MRApp*);                        // 进后台 / surface 销毁时调用
int  mr_app_backend(MRApp*);                               // 实际在用：1 cpu / 2 gpu
const char* mr_app_backend_reason(MRApp*);
void mr_app_device_info(MRApp*, uint64_t total_mem, int low_ram, int os_api);  // 分级用
// mr_app_pump 返回 2 = 后端切换，宿主改显示路径；GPU 模式下 mr_app_pixels 改为调试用的读回（慢）
```

- iOS：`MiniProgramView` 里加一个 `CAMetalLayer` 子层，把指针交给 wgpu（wgpu 支持从 Core Animation layer 创建 surface，
  具体 API 名在 v30 下核对，**未验证**）。`CADisplayLink.preferredFrameRateRange` 设为 80~120（参考
  [WWDC21: Optimize for variable refresh rate displays](https://developer.apple.com/videos/play/wwdc2021/10147/)；
  iPhone 上还要在 Info.plist 开 `CADisableMinimumFrameDurationOnPhone`，这一条没能从 Apple 文档页抓到原文，**未验证**）。
- Android：GPU 模式用 `SurfaceView`，`surfaceCreated` 时在 JNI 里调 `ANativeWindow_fromSurface` 再交给引擎；出帧由
  `Choreographer.FrameCallback` 驱动，替代现在的连续 `post`。

### 5.6 Cargo feature

```toml
[features]
default = ["desktop", "audio", "h264"]   # P2 验收通过之前，gpu 不进 default
gpu = ["dep:wgpu", "dep:pollster"]
```

移动端构建：`--no-default-features --features bindgen,gpu`。CI 必须同时跑「不带 gpu」和「带 gpu」两种配置，保证关掉 GPU
后工程照常编译、照常通过全部回归（steering 硬性约束）。

---

## 6. 分阶段路线与验收数字

每个阶段都能单独验收；纯搬迁和改行为分开提交（steering 20）。下面「基线」都是第 2 节的本机实测；验收一律
**跑 5 次取中位数**，报最慢一帧和帧间隔峰值，不用平均值。

### P0　测量补齐与合成接缝（纯搬迁）· 约 4~6 人日
- P0a：`--drag` 补 p99 和「上屏」分段（无头模式下调用 CPU 合成并计时）；新增 `examples/sdk_bench.rs`，在 3× 尺寸下给
  `MiniEngine::pump` 计时，含 compose + RGBA 转换；`MINI_FPS` 和 `--drag` 输出加上后端标签（这时只有 CPU）；
  `--backend` / `MINI_BACKEND` 打通（选 gpu 时报「未编译」并回退）。
- P0b：新建 `src/compositor/{mod,cpu}.rs`，桌面 `present()` 和 SDK `compose_frame()` 都改调 `CpuCompositor`。
- **验收**：sample 15 页 + news 6 页整帧快照（`--time 2`）md5 全部不变；gallery 65 张不变（35 号 GIF 帧按已知噪声处理）；
  `interaction-check` 7 张不变；`sdk-parity` 15/15、6/6；`damage-check` 5/5；`cargo test --release` 全绿；
  并产出一份移动端尺寸（3×）的 CPU 基线数字表，作为 P3 的对照。

### P1　SDK 出帧闸门与帧节奏对齐（改行为，跟 GPU 无关但是前提）· 约 6~9 人日
- 先纯搬迁：把 `frame.rs::is_animating` / `render_frame_if_needed` / `scroll_area_damage` 以及 `frame_render.rs` 里的
  FramePlan、损伤区、fixed 复用逻辑沉到 `src/host/frame_plan.rs`，桌面改调它（md5 不变）。
- 再改行为：`MiniEngine::pump` 也用它；iOS 设 `preferredFrameRateRange`；Android 改用 Choreographer。
- **验收**：新增 `engine_input_tests` 用例，证明 SDK 下 CSS `infinite` 动画连续 10 帧 `pump` 都返回 1，且画面逐帧不同；
  swiper 换页过渡期间每帧都出帧；SDK 版 damage-check（增量 vs `MINI_NO_DAMAGE=1` 逐字节一致）通过；`sdk-parity` 仍 15/15、6/6。

### P2　桌面 GPU 合成器（wgpu）· 约 12~16 人日
- 实现 `gpu/` 各个文件：分块页面纹理、tabBar、fixed、覆盖层纹理、uber 合成着色器、swapchain（`Fifo`；GPU 模式下关掉
  sleep + 自旋节拍器）、无头读回、回退和故障注入、`tools/gpu-parity.sh`、`pixdiff.py --tol`。
- **验收**（M3 Pro，144Hz）：
  - 正确性：静态页 GPU 与 CPU **0 差异像素**（sample 15 + news 6）；覆盖层帧在 `--tol 2` 下为 0，`--tol 0` 的阈值实测后写进
    `doc/引擎测试说明.md`；`MINI_GPU_FAULT=tint` 必须报不一致；
  - 性能：`sample-app` 首页页面级拖动（settle 2）**含上屏**的 CPU 侧耗时 p95 ≤ 1.0ms（基线：渲染 p95 1.2~1.8ms，加上屏 2~3ms）；
    交互窗体 `MINI_FPS` 滚动期间帧间隔峰值 ≤ 7.2ms、最慢一帧 ≤ 6.9ms；
  - 回退：`MINI_GPU_FAULT=init` 和 `lost@30` 下不黑屏、不崩，回退后的快照与 CPU 基线逐字节一致，日志里能查到回退原因；
  - 工程：`cargo build --release --no-default-features` 和 `--features gpu` 都通过；记录 `--features gpu` 带来的 stripped
    体积增量，上限在 P2 开始时由用户拍板（建议 ≤ 8MB / arm64，**未验证**）。

### P3　移动端 GPU 上屏与机型分级 · 约 12~18 人日（含真机矩阵）
- 实现第 5.5 节的 C ABI、Swift 的 `CAMetalLayer` 路径、Kotlin 的 `SurfaceView` + Choreographer、`select.rs` 分级与自检、`gpu-verdict.json`。
- 真机矩阵：至少 1 台 120Hz iPhone、1 台 120Hz 高端 Android（Adreno）、1 台中端 Mali、1 台 API 24~28 的低端机。
  型号由用户定，本报告不预设。
- **验收**：
  - 高端机自动选中 GPU，低端机自动选中 CPU，reason 字符串说得清原因；
  - 高端机页面级滚动：帧间隔峰值 ≤ 8.5ms、最慢一帧 ≤ 8.3ms（120Hz）；每帧 CPU 侧耗时比 P0 的 3× CPU 基线至少降 70%（具体值以 P0 实测为准）；
  - 低端机走 CPU，数字不比 P1 差；
  - 强制 GPU 加故障注入下不黑屏，`mr_app_pump` 返回 2 后宿主切换成功；
  - `sdk-parity`（CPU）仍 15/15、6/6，SDK 的 GPU 读回对 CPU 同样过 `gpu-parity` 阈值。

### P4　图层化（两后端共享）· 约 12~18 人日
- 嵌套 scroll-view 做成独立图层：子路径也用 `ScrollViewCache`，滚动帧只改图层偏移。提升条件是「绘制顺序里后面没有与它
  重叠的兄弟节点」，不满足就不提升，退回现在的做法。CPU 合成时用同一份缓存做行拷贝（可以逐字节等价）。
- transform/opacity 动画子树：内容按「子树数据指纹 + 尺寸」缓存，动画帧只改矩阵和透明度；GPU 用着色器复刻
  `compose.rs::sample_bilinear` 的 alpha 加权双线性。可以顺带支持 `will-change: contents`（Skyline 用它声明绘制边界）作为显式提升提示。
- 跨条带滚动只补画新露出的条带（CPU 侧改动，两后端都受益）。
- SDK 接上侧滑返回（README TODO 第 3 条）。
- 新开关 `MINI_NO_LAYERS=1`：关掉图层化，用来做逐字节对照（与 `MINI_NO_DAMAGE` 同类）。
- **验收**：
  - CPU：`MINI_NO_LAYERS=1` 与默认在 sample / news / tea-app 抽样页逐字节一致；
  - tea-app 分类页拖动：p50 ≤ 1.5ms、最慢一帧 ≤ 6.9ms（@2x，基线 p50 5.5ms），@3x 下 p50 ≤ 3ms（基线 13.1ms），CPU 和 GPU 分别报数；
  - sample-app 首页跨条带帧：最慢一帧 ≤ 6.9ms（@2x，基线 17~18ms），@3x ≤ 8.3ms（基线 24.1ms）；
  - popIn 弹窗动画帧：CPU 后端 ≤ 3ms，GPU 后端 ≤ 1ms（P0 先测出基线）；
  - 变换图层的 GPU 与 CPU 差异阈值实测后写进文档。

### P5　GPU 专属效果与按需光栅（按数据逐项立项）
- `filter` / `backdrop-filter`（Skyline 支持单函数，不支持 drop-shadow 和 url，见
  [Skyline WXSS 支持](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/wxss.html)）：GPU 在合成阶段做；
  CPU 侧要**明确写下降级行为**（例如 `blur` 在 CPU 上按缓存掩膜做一次性盒式模糊，或者直接不支持并在 `MINI_FPS` / 日志里提示），
  steering 要求不能假装支持。
- 图片采样上 GPU（首屏加载期每帧约 8ms 的 image 开销）、字形图集（复用 fontdue 位图）、矩形 / 圆角 SDF：每项都先用
  `MINI_DRAW_LOG` 证明它是目标机上的瓶颈再做，做完单独标定阈值。
- 需要的话再做接缝 2（`PaintTarget`），先纯机械替换并用 md5 证明等价。

---

## 7. 主要风险

| 风险 | 影响 | 应对 |
|---|---|---|
| Android 驱动差异（wgpu v28 的发布说明里就修过「部分 Mali 驱动崩溃」） | 黑屏、崩溃、花屏 | 自检 + 黑名单 + 回退 + `gpu-verdict.json` 记忆；Auto 模式下 API < 29 一律走 CPU |
| 纹理尺寸上限（8192 / 降级 2048） | 长页面画不全 | 页面图层分块（第 5.2 节） |
| 显存加倍（CPU 画布 + GPU 镜像） | 内存本来就是最大短板（600MB~1GB） | 只让视口附近的块常驻；`trim_memory()` 同时释放 GPU 纹理 |
| wgpu 大约一季度一个破坏性大版本 | 升级成本 | 锁 `=30.0.1`，升级单独立项并重新标定阈值 |
| 伽马 / 颜色空间（sRGB surface 格式、Display P3） | 整屏偏色 | 只选非 sRGB 格式；实在只有 sRGB 的平台在着色器里做逆变换，并写进阈值说明 |
| 帧节奏语义变化（`Fifo` 会阻塞） | 现有节拍器和 vsync 打架 | GPU 模式关掉节拍器；`MINI_FPS` 分后端统计 |
| 桌面 softbuffer 与 wgpu 同窗切换 | 运行期回退可能要重建 surface | P2 第一周先做探针（**未验证**） |
| CI 没有 GPU | 「全绿但没测到」 | 单独退出码；考虑在 Linux CI 上装 lavapipe，只用于跑功能，不作为性能或阈值依据 |
| 合成线程模型 | Skyline 有独立渲染线程（[简介](https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/introduction.html)），本引擎的 JS、布局、光栅都在一个线程上 | GPU 不改变这一点；如果要「逻辑不阻塞渲染」，需要单独立项 |

---

## 8. 对照方案：不加 GPU，CPU 侧还能做什么

以下几项不需要任何新依赖，而且大都直接对应第 2 节的实测尖峰。其中 1~3 已经并入 P4（两后端共享）：

1. **增量条带**：滚出已绘制条带时只补画新露出的那一段。目标：17~24ms 的尖峰 → ≤ 6.9ms。
2. **嵌套 scroll-view 走缓存**：子节点路径也用 `ScrollViewCache`。目标：tea-app 分类页 p50 5.5ms → ~1ms（@2x）。
3. **动画子树内容缓存**：内容不变时不再每帧离屏重画，只重新做逆仿射贴图。
4. **setData 增量树打补丁**（README TODO 第 2 条）：首页每秒那次 11.4ms 是建树和样式造成的，GPU 帮不上，这是唯一的解法。
5. **首屏加载期别每帧整帧**：首页 settle 0 期间每帧都「退回整帧：结构变化」，可以把骨架 / 图片到位按区域失效，而不是整帧。
6. **SDK 少拷贝**：Android 用 `ANativeWindow_lock` 直接写宿主缓冲，iOS 用 `CVPixelBuffer` 或 IOSurface，至少省掉
   compose→rgba→宿主数组这两次整帧拷贝（集成指南里已有这条建议）。
7. **多线程光栅**（按条带并行）和 SIMD 混合：需要引入线程池，属于另一个题目。

**怎么取舍**：在桌面 144Hz 判据上，1~5 能拿到大部分收益。在移动端高刷屏上，上屏拷贝和帧节奏是 CPU 方案很难压下去的部分，
这正是 P2 / P3 GPU 合成器的价值所在。所以推荐的顺序是 P0 → P1 → P2 → P3 → P4，P4 的 CPU 部分可以与 P2 / P3 并行。

---

## 9. 顺带发现的「未对齐」项（属于用户第 1 问，供后续立项）

- SDK 不驱动 CSS 动画、swiper 过渡和定时器的连续出帧（第 1.4 节，从代码推断）。
- SDK 没有 setData 损伤区和侧滑返回（`engine.rs` 文件头，README TODO 第 3 条）。
- Android 宿主出帧没有对齐 vsync；iOS 没有请求 120Hz。
- Skyline 支持而本引擎没实现（在 `src/renderer`、`src/parser` 里 grep 结果为 0）：`filter`、`backdrop-filter`、CSS `mask-image`、
  `conic-gradient`、`text-shadow`、`will-change`。
- 嵌套 scroll-view 不走离屏缓存（第 1.1 节），性能上与 Skyline「滑动是纯合成」不对齐。

---

## 附录 A　复现命令

```bash
. "$HOME/.cargo/env" && cargo build --release --bin mini-app-window
rm -rf target/mini-storage
./target/release/mini-app-window sample-app --route pages/index/index --settle 2 [--scale 3] --drag 8x60 --snapshot target/_probe
./target/release/mini-app-window sample-app --route pages/index/index --settle 0 [--scale 3] --drag 8x60 --snapshot target/_probe
./target/release/mini-app-window sample/tea-app --route pages/category/category --settle 2 [--scale 3] --drag 8x60 --snapshot target/_probe
./target/release/mini-app-window news-app --route pages/home/home --settle 2 [--scale 3] --drag 8x60 --snapshot target/_probe
MINI_DRAW_LOG=1 MINI_LAYOUT_LOG=1 ./target/release/mini-app-window sample-app --route pages/index/index --settle 0 --drag 8x20 --snapshot target/_probe
MINI_SCROLL_LOG=1 ./target/release/mini-app-window sample/tea-app --route pages/category/category --settle 2 --drag 8x60 --snapshot target/_probe
rm -rf target/_probe target/mini-storage && bash tools/clean-target.sh
```

## 附录 B　外部资料（内容已改写，以原文为准）

- wgpu 发布说明（v30.0.1、v29 的 `CurrentSurfaceTexture`、v28 的 Mali 修复）：https://github.com/gfx-rs/wgpu/releases
- Linebender 2026 Q1（Vello 0.8、sparse strips 0.0.7、Hybrid 约为 beta、Glifo 字形）：https://linebender.org/blog/tmil-25/
- Vello 仓库：https://github.com/linebender/vello
- rust-skia（预编译平台列表、源码构建依赖）：https://github.com/rust-skia/rust-skia
- femtovg（GLES3 / wgpu 后端，Slint 在用）：https://github.com/femtovg/femtovg
- wgpu 体积的旧讨论（数字已过时，仅作量级参考）：https://github.com/gfx-rs/wgpu-rs/issues/841
- Skyline 简介（独立渲染线程）：https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/introduction.html
- Skyline WXSS 支持（filter、backdrop-filter、will-change、mask-image）：https://developers.weixin.qq.com/miniprogram/dev/framework/runtime/skyline/wxss.html
- WWDC21 可变刷新率：https://developer.apple.com/videos/play/wwdc2021/10147/

标注「未验证」的有：移动端各项耗时、wgpu 当前版本的体积增量、wgpu v30 的具体 feature 名和 CAMetalLayer API 名、
`CADisableMinimumFrameDurationOnPhone` 原文、鸿蒙支持、softbuffer 与 wgpu 同窗切换、GPU 能否消除 macOS 上屏停顿。
