# 双渲染后端（CPU 参考实现 + GPU 可选）技术设计

> 工作区：`/Users/shengye/pojects/mini/mini-program-core/.worktrees/gpu-backend`（分支 `feat/gpu-backend`，与 `main` 同为 `09b8fb8`）。
> 主要依据：`.agents/tasks/gpu-rendering-feasibility-2026-10-12/report.md`（下称「报告」）。与报告不一致的地方集中列在第 16 节，每条都写了理由。
> 需求来源：`.kiro/steering/10-skyline-target.md` 的「双渲染后端：CPU 与 GPU」。同时遵守 steering 00 / 20 / 30。
> 没有上一轮的 `design-review.json`，这是第一版。

---

## 0. 概述

引擎现在是纯 CPU 立即模式光栅：`WxmlRenderer` 把页面画进整页高的 `Canvas`（RGBA8 直通 alpha），宿主再把页面、tabBar、fixed 覆盖层、下拉指示器、Toast/Modal、picker 面板、侧滑返回这几层**按固定顺序**合成到一块 `u32` 缓冲，然后交给 softbuffer（桌面）或转成 RGBA 交给移动端宿主（SDK）。本设计在**合成层**切出后端接缝：内容仍然只由 CPU 光栅化（布局、命中、事件、`setData` 增量失效、动画时钟一行不动），新增的 GPU 合成器（wgpu）把这些图层当纹理镜像、只按脏区上传、用整数着色器复刻 CPU 的合成算术，再直接上屏（swapchain）。因为复刻的是同一套整数公式，**GPU 与 CPU 的同帧输出逐字节一致**，P1 的 GPU 判据阈值定为 0 个差异像素。

这条路线能拿到的收益是「上屏 / 合成 / 帧节奏」：桌面滚动帧的上屏拷贝（文档实测 2~3ms）、移动端每帧至少 4 次整帧拷贝（报告 §1.3）。它拿不到的是跨条带重画（17~24ms）、嵌套 scroll-view 每帧重画（p50 5.5ms@2x）、大面积动画的离屏重画——这些是 CPU 光栅瓶颈，放在 P3「图层化」（两后端共享）里解决，本设计给出 P3 的验收判据但**不在本工作流实现**。这一点要先说清楚，免得把「接上 GPU」误当成「所有卡顿都没了」。

**本工作流的实现范围**：P0（测量与接缝，CPU-only）、P1（桌面 GPU 合成器，本机 M3 Pro 可完整验收）、P2（移动端 GPU 上屏与机型分级；本机可做交叉编译与 SDK 路径的桌面验证，真机验收要用户提供设备）。P3、P4 只定判据，不实现。

---

## 1. 技术栈（批准后锁定）

| 项 | 选择 | 版本（精确锁定） |
|---|---|---|
| 语言 | Rust 2021 | 本机 rustc 1.97.1；wgpu 要求 ≥ 1.87 |
| GPU API 封装 | `wgpu` | `=30.0.1`，`default-features = false`，`features = ["std", "wgsl", "metal", "vulkan", "gles", "dx12"]` |
| 同步等待 future | `pollster` | `=1.0.1` |
| 原生窗口句柄（Android 构造 `AndroidNdkWindowHandle`） | `raw-window-handle` | `=0.6.2`（与 winit 0.30.13、softbuffer 0.4.8、wgpu 30.0.1 同一版本，Cargo.lock 里已有） |
| 着色器 | WGSL（经 naga 转 MSL / SPIR-V / GLSL ES） | 随 wgpu |
| CPU 上屏（不变） | `softbuffer` | 现有 0.4 |
| iOS 宿主 | Swift + UIKit + QuartzCore（`CAMetalLayer`） | 不加依赖 |
| Android 宿主 | Kotlin + `SurfaceView`；JNI 侧直接声明 `libandroid` 的 `ANativeWindow_*` 与 `liblog` 的 `__android_log_write` | 不加 crate（不引 `ndk`） |
| 工具脚本 | bash + python3（PIL，`tools/pixdiff.py` 已在用） | 不变 |
| 基准与夹具 | Rust example（用已有的 `image`、`serde_json`） | 不加依赖 |

**为什么是 wgpu**（同意报告 §3 的结论）：它是唯一同时覆盖 Metal / Vulkan / GLES / DX12、支持无 surface 离屏读回、又能用 cargo 原样交叉编译的纯 Rust 方案；它不自带成像模型，所以合成算术、抗锯齿、字形都由我们决定，GPU 才能贴着 CPU 参考实现。vello、skia-safe、femtovg 自带 AA 与字体栈，用了它们「同帧差异 ≤ 阈值」只能放宽到几个百分点，判据失去判定力；skia-safe 还没有 armeabi-v7a 预编译（本项目出这个 ABI）。

**feature 写法已核对**（报告标为「未验证」）：wgpu 30.0.1 的 `build.rs` 用 `cfg_aliases` 把后端特性按目标门控——`metal` 只在 `target_vendor = "apple"` 生效，`vulkan` / `gles` 只在 Windows/Linux/Android 生效（Apple 上要 `vulkan-portability` / `angle` 才开），`dx12` 只在 Windows 生效。所以**一条** `[dependencies]` 声明带上四个后端特性就够了，不需要按目标拆表。`Instance::new` 在「没有任何可用后端」时会 panic，这条只会在本项目不支持的目标上触发，第 9 节有对应处理。wgpu 30 的相关 API 也已对照源码确认：`SurfaceTargetUnsafe::CoreAnimationLayer(*mut c_void)`、`SurfaceTargetUnsafe::RawHandle { raw_display_handle, raw_window_handle }`、`Surface::get_current_texture() -> CurrentSurfaceTexture { Success, Suboptimal, Timeout, Occluded, Outdated, Lost, Validation }`、`Device::set_device_lost_callback`、`Device::on_uncaptured_error`、`Device::destroy`、`Device::poll(PollType)`。

```toml
[features]
default = ["desktop", "audio", "h264"]          # P1 验收通过后单独一次提交把 "gpu" 加进来（§14 A1.9）
gpu = ["dep:wgpu", "dep:pollster", "dep:raw-window-handle"]

[dependencies]
wgpu = { version = "=30.0.1", optional = true, default-features = false, features = ["std", "wgsl", "metal", "vulkan", "gles", "dx12"] }
pollster = { version = "=1.0.1", optional = true }
raw-window-handle = { version = "=0.6.2", optional = true }
```

升级 wgpu 大版本（大约一季度一次破坏性大版本）单独立项，不和功能改动混在一起，升级后重跑第 13 节的全部 GPU 判据。

---

## 2. 接缝放在哪一层：合成层

**决定**：接缝放在「图层合成」这一层，不放在显示列表 / 绘制命令层，也不放在渲染树遍历层。

- **没有显示列表可以接**。渲染器边遍历边直接调 `Canvas` 方法，`&mut Canvas` 作为参数出现约 71 处，另有十多处直接读写像素（`compose.rs::blit_transformed`、`gradient.rs`、`shadow.rs`、`text.rs::blend_coverage`、`scroll_cache.rs` 等，报告 §1.1）。要在命令层分叉，得先造一个显示列表并把这些直接像素操作全改掉，这是 P4 才考虑的「接缝 2」。
- **渲染树层分叉会复制副作用**。绘制遍历里夹着事件绑定登记、交互元素登记、fixed 子树重排、动画包围盒收集、`bindload` 派发、swiper 推进。让每个后端各自遍历，就是把这些逻辑写两份——`doc/架构与实现原理.md` 已记录过两次类似分叉（状态落地三份实现、SDK 指针层是简化版）。
- **合成层天然就是「只管画」**。宿主已经把一帧拆成固定的几层（报告 §1.2 的表），合成器只拿到这些画布的**只读引用**和覆盖层状态，拿不到 `MiniApp`、`InteractionManager`、`ScrollController`。所以「后端不碰布局 / 命中 / 事件 / `setData` / 动画时钟」是类型层面保证的，不靠约定。

```text
 逻辑层(QuickJS) → 模板/样式/布局(taffy) → FramePlan/损伤区(invalidate.rs) → 命中/事件/动画时钟   ← 与后端无关，不改
                                   │
                    WxmlRenderer 往 Canvas 画（CPU 光栅，内容的唯一来源）
                                   │   写画布必须经 PageCanvas / LayerCanvas（声明写了哪块）
                                   ▼
                FrameLayers<'a>（本帧各图层的只读引用 + 覆盖层状态）
                     ┌─────────────┴──────────────┐
          compositor::cpu::compose            compositor::gpu::GpuCompositor（feature "gpu"）
          → &mut [u32] → softbuffer / SDK RGBA   → 纹理镜像（按脏区上传）→ 整数合成着色器 → swapchain
                                                  或离屏目标 → 读回（无头 / 测试）
```

### 2.1 新目录 `src/compositor/`（与 `src/canvas/` 平级，每个文件 ≤ 500 行）

| 文件 | 职责 |
|---|---|
| `mod.rs` | `BackendPref { Auto, Cpu, Gpu }`、`BackendKind { Cpu, Gpu }`、`BackendStatus`、`FrameLayers<'a>` 及其子结构、`pub use` |
| `layer.rs` | `LayerCanvas`（写入即递增代数）、`PageCanvas`（写入必须声明条带/矩形，留只读的写入日志）、`DirtyOp`、`color_bytes()` 零拷贝转字节 |
| `cpu.rs` | `compose(buf, w, h, &FrameLayers)`：把现在散在四处的合成调用序列**原样**搬进来 |
| `pref.rs` | `MINI_BACKEND` / `--backend` / FFI 整数的解析；优先级规则 |
| `select.rs` | 纯函数的机型分级与适配器门槛（`platform_gate` / `adapter_gate`），无 GPU 依赖，可单测 |
| `verdict.rs` | `gpu-verdict.json` 的读写（serde） |
| `diag.rs` | 日志出口：桌面 / iOS 写 stderr，Android 写 `liblog`；`MINI_GPU_LOG` 开关 |
| `fault.rs` | `MINI_GPU_FAULT` 的解析（纯解析，注入点在 gpu 模块里） |
| `gpu/mod.rs` | `GpuCompositor`：对外的 `present` / `compose_offscreen` / `release_caches` |
| `gpu/device.rs` | Instance / Adapter / Device 创建、`AdapterFacts` 转换、丢失与未捕获错误钩子 |
| `gpu/surface.rs` | `NativeSurface`（winit 窗口 / CAMetalLayer / ANativeWindow / 离屏）、格式选择、configure、acquire 状态处理 |
| `gpu/mirror.rs` | 页面条带环形纹理、`MirrorRows` 记账、tabBar/fixed/pull/直通纹理的上传 |
| `gpu/pipeline.rs` | bind group 布局、uniform、渲染管线 |
| `gpu/compose.wgsl` | 合成着色器 |
| `gpu/readback.rs` | 离屏目标 → 256 字节对齐的缓冲 → 去填充 |
| `gpu/selftest.rs` | 启动自检：固定测试帧，GPU 与 `cpu::compose` 逐字节比对并计时 |
| `gpu/tests.rs` | `#[cfg(all(test, feature = "gpu"))]` 的随机图层配置对拍 |

不定义 `trait Compositor`（报告建议了一个），理由见第 16 节第 1 条：CPU 的输出目标在三个宿主里各不相同（softbuffer 借出的缓冲、SDK 的 `Vec<u32>`、快照的临时 `Vec`），GPU 的输出是 surface 或离屏纹理，统一成 trait 需要额外的「目标」抽象，而 CPU 路径并不需要它。改用一个自由函数加一个具体类型，宿主按 `BackendStatus.active` 分派。

### 2.2 `FrameLayers` 的形状

字段与 `present_to_buffer` 及其后几个覆盖层函数的参数**一一对应**，这样 `cpu::compose` 是纯搬迁，逐字节可证：

```rust
pub struct FrameLayers<'a> {
    pub width: u32, pub height: u32,          // 输出像素尺寸（窗口 / 引擎像素尺寸）
    pub scale: f32,                            // 设备像素比（覆盖层与指示器用）
    pub page: &'a PageCanvas,                  // 页面画布（内容坐标，整页高）
    pub scroll_px: i32,                        // (scroll * scale) as i32，与现在的截断方式一致
    pub page_bg: u32,                          // 0xRRGGBB
    pub has_tabbar: bool,                      // 与 tabbar 分开：present_to_buffer 在 has_tabbar 但画布缺失时仍会扣掉 tabBar 高度
    pub tabbar: Option<&'a LayerCanvas>,
    pub tabbar_px: u32,
    pub fixed: Option<&'a LayerCanvas>,
    pub fixed_rows: Option<(u32, u32)>,
    pub pull: Option<PullState>,               // { gap_logical, refreshing, progress, phase }；宿主只在 gap>0.5 || refreshing 时给 Some
    pub overlays: OverlayRefs<'a>,             // { last_frame: Instant, toast, loading, modal: &Option<_>, picker: Option<&PickerSheetState>, text: Option<&TextRenderer> }
    pub edge_back: Option<EdgeBackFrame<'a>>,  // { offset_px, shot: Option<&PageShot>, progress }
}
```

各宿主照现在的实参填：桌面交互窗体 `last_frame = self.last_frame`、指示器相位 `(started_at.elapsed()*1.1).fract()`；快照相位固定 `0.25`；SDK 的 `last_frame = Instant::now()`，`pull = None`、`edge_back = None`（SDK 现在就没有这两层）。

---

## 3. CPU 合成器：先做纯搬迁

报告说「两处」复制了合成序列，核对代码后实际是**四处**：

| 调用点 | 现在调用的序列 |
|---|---|
| `src/bin/app_window/frame_render.rs::present` | `present_to_buffer` → `render_pull_indicator` → `render_ui_overlay` → `picker_sheet::render` → `edge_back::compose` |
| `src/bin/app_window/headless_run.rs::snapshot_all` | 同上（指示器相位固定 0.25），再转 RGBA 存 PNG |
| `src/bin/app_window/edge_back_host.rs::capture_viewport` | 只有 `present_to_buffer`（给侧滑留下一页的视口图） |
| `src/host/engine.rs::compose_frame` | `present_to_buffer` → `render_ui_overlay` → `picker_sheet::render`，再转 RGBA |

`cpu::compose` 按上表第一行的顺序执行，`Option` 为 `None` 的层跳过；四个调用点改成「构造 `FrameLayers` → 调 `cpu::compose`」。`capture_viewport` 构造时把 `pull`、`overlays` 里的状态都给空、`edge_back = None`，结果与只调 `present_to_buffer` 相同。另外把 `render_pull_indicator` 拆成 `pull_indicator_band(...) -> Option<Canvas>`（画三点）和原来的混合循环两段，GPU 直接复用前一段，CPU 的调用结果不变。

`host::render::present_to_buffer`、`ui_overlay`、`picker_sheet::render`、`edge_back::compose` 这些函数本身**一行不改**，只是换了调用者。

---

## 4. 图层镜像与脏区

GPU 合成器只镜像 CPU 画布，不画内容。要做到「只传变化的部分」又不出错，关键是让「哪块被写过」由类型来记录，而不是在宿主各处手动打标记。

### 4.1 `LayerCanvas`（tabBar、fixed 覆盖层）

```rust
pub struct LayerCanvas { canvas: Canvas, gen: u64 }
impl LayerCanvas {
    pub fn new(c: Canvas) -> Self;
    pub fn read(&self) -> &Canvas;
    pub fn write(&mut self) -> &mut Canvas { self.gen += 1; &mut self.canvas }  // 拿可变引用即视为整层变了
    pub fn gen(&self) -> u64;
}
```

宿主字段 `tabbar_canvas` / `fixed_canvas` 换成 `LayerCanvas`（桌面是 `Option<LayerCanvas>`）。凡是要画它的地方都只能走 `write()`，忘了打标记在编译期就不成立。GPU 侧记住每层上次上传时的代数，不同就重传：tabBar 整层，fixed 只传 `fixed_rows` 这几行（`fixed_rows` 本来就在覆盖层重画的那一帧重算）。

### 4.2 `PageCanvas`（页面画布）

```rust
pub enum DirtyOp { Band(i32, i32), Rect(Rect) }
pub struct PageCanvas {
    canvas: Canvas,
    gen: u64,                     // 重新分配时 +1
    seq: u64,                     // 每次写入 +1
    log: VecDeque<(u64, DirtyOp)>,// 最近 32 次写入，满了丢最旧的
}
impl PageCanvas {
    pub fn read(&self) -> &Canvas;
    pub fn write_band(&mut self, y0: i32, y1: i32) -> &mut Canvas;   // 整宽条带：clear_band + 整帧重画
    pub fn write_rect(&mut self, r: Rect) -> &mut Canvas;            // 损伤区：clear_area + 带裁剪的局部重画
    pub fn replace(&mut self, c: Canvas);                           // 高度变了重新分配：gen += 1，log 清空
    pub fn gen(&self) -> u64;
    /// seen 之后的写入；log 已经丢掉了其中一部分时返回 None（调用方按「全部失效」处理）
    pub fn ops_since(&self, seen: u64) -> Option<impl Iterator<Item = &DirtyOp>>;
    pub fn seq(&self) -> u64;
}
```

写入记录是**只读可查**的日志，不是「取走即清空」的队列：`FrameLayers` 里只放 `&PageCanvas`，GPU 合成器自己记住看到过的 `seq`，不需要可变借用，也不需要内部可变性。日志有 32 条上限，CPU 模式下没人读也不会涨；GPU 落后超过 32 次写入（比如中间有一段 CPU 合成）时 `ops_since` 返回 `None`，GPU 把镜像整体作废，下一次合成按缺行补传（§4.3），结果仍然正确。

写入点（全部是现有的「清屏 + 画」位置）：桌面 `render_with_damage` 的 `clear_area`/`clear_band` 分支与画布重新分配后的重画；SDK `render_page_band` 与重新分配。

### 4.3 GPU 侧的页面镜像：环形条带纹理 + `MirrorRows`

页面画布可以上万行高（3× 屏首页约 10305 行），超过常见的 8192 纹理上限，降级上限只有 2048。但真正会上屏的只有「已绘制条带」那一段，所以 GPU 不镜像整页：

- 纹理是 `texture_2d_array<u32>`（`Rgba8Uint`），每层 1024 行、宽 = 画布宽，层数 `slices = ceil((视口像素高 + 2×VIEWPORT_CULL_MARGIN_PX + 2) / 1024)`。2× 屏 3 层，3× 屏 3 层（2801 行）。
- 画布第 r 行映射到环形下标 `k = r % (slices*1024)`，层 `k / 1024`、层内行 `k % 1024`。任何长度不超过环长的连续行区间都不会自相覆盖。
- `MirrorRows` 记一个区间 `valid = [a, b)`：这些画布行在 GPU 上是最新的。
  - 按 `ops_since(seen_seq)` 的顺序回放：`Band(y0, y1)` 夹到画布内上传整宽条带，`valid = [y0, y1)`（条带长度按构造不超过环长；万一超过，只传与本帧 `visible` 相交的那段，并打 warn）；`Rect(r)` 只上传与 `valid` 相交的部分，按**矩形**传（`write_texture` 的源步长设成画布行宽，不必传整行），`valid` 外的忽略。回放完 `seen_seq = seq()`。
  - `ops_since` 返回 `None`，或画布 `gen` 变了：`valid = None`；`gen` 变了还要重建纹理（尺寸可能变了）。
  - 上传源永远是**此刻**的 CPU 画布，所以两次合成之间画了好几次，也只按最新内容传一次。
  - **合成前**：本帧要采样的画布行 `visible = [max(0, scroll_px), min(canvas_h, scroll_px + content_h))`。先把 `visible \ valid`（最多两段）从 CPU 画布补传上去；然后如果 `valid ∪ visible` 是连续区间且长度不超过环长，`valid` 取这个并集，否则 `valid = visible`。前一种情况下并集里每一行要么本来有效、要么刚传过；后一种情况下 `visible` 本身不超过环长，补传不会覆盖到其中已有效的行。

最后一条让 GPU 输出的正确性**不依赖**「上屏行一定落在已绘制条带内」这条宿主不变量：就算哪天宿主漏了重画，GPU 采到的也是此刻 CPU 画布里的内容，和 CPU 上屏结果一样。补传时打一行 `MINI_GPU_LOG` 日志（`镜像补传 [a,b)`），方便发现宿主侧的问题。

剩下唯一的风险是「已在 `valid` 里的行被改写却没声明」。现有的写入只有两种：整帧重画（声明了条带；跨条带边缘的元素会写到条带外，但那些行随之落在新 `valid` 之外）和带裁剪的损伤区重画（`entry.rs` 里 `canvas.clip_rect(rect)`，写入不会出矩形）。`gpu-parity.sh`（§13.3）用全部静态页、5 个增量重绘用例和交互用例逐字节守住这一点。

---

## 5. GPU 合成器

### 5.1 纹理

| 纹理 | 格式 | 尺寸 | 何时上传 |
|---|---|---|---|
| 页面环形条带 | `Rgba8Uint` 2D array | 画布宽 × 1024 × slices | §4.3 |
| tabBar | `Rgba8Uint` | tabBar 画布尺寸 | 代数变了 |
| fixed | `Rgba8Uint` | 覆盖层画布尺寸 | 代数变了，只传 `fixed_rows` |
| 下拉指示器 | `Rgba8Uint` | 宽 × gap 行（按需扩容） | `pull` 为 `Some` 的每一帧 |
| 直通帧 | `Rgba8Uint` | 输出尺寸 | 直通帧（§5.3），惰性创建 |
| 离屏目标 | `Rgba8Unorm`（RENDER_ATTACHMENT \| COPY_SRC） | 输出尺寸 | 无头 / 自检 / 测试，惰性创建 |
| 1×1 占位 | `Rgba8Uint` | 1×1 | 某层不存在时绑它 |

源纹理一律用 **`Rgba8Uint`**：`textureLoad` 直接拿到 `vec4<u32>`，不经过 unorm→float 的转换，也不经过采样器。`Color` 是 `#[repr(C)]` 的 4×`u8`，用 `color_bytes(&[Color]) -> &[u8]` 零拷贝取字节（`layer.rs` 里加 `const _: () = assert!(size_of::<Color>() == 4 && align_of::<Color>() == 1);`）。直通帧的源是 CPU 合成出的 `u32`（0x00RRGGBB），小端内存里是 `B,G,R,0`，按字节原样上传，着色器里取 `(z, y, x)`。gpu 模块加 `#[cfg(not(target_endian = "little"))] compile_error!`。

### 5.2 着色器：怎么做到逐字节

一次绘制，一个全屏三角形，**不开硬件混合**（blend state 为 `None`，硬件混合是浮点舍入，会差 1 LSB）。片元着色器按整数像素坐标 `(x, y) = (u32(pos.x), u32(pos.y))` 依次复刻 `present_to_buffer` 与指示器混合：

1. 内容区（`y < content_h`，`content_h = height - (has_tabbar ? tabbar_px : 0)`）：`sy = y + scroll_px`，在画布内且 `x < copy_w` 取页面条带像素的 RGB（忽略 alpha，与 CPU 一致），否则取 `page_bg`。
2. tabBar 区：`has_tabbar && tabbar_on`，行 `< min(tabbar_px, tabbar_h)`、列 `< min(width, tabbar_w)` 时取 tabBar 的 RGB。这块区域里 CPU 不写的像素（只在 tabBar 画布比 `tabbar_px` 矮时出现），CPU 保留缓冲原值；快照与 SDK 的缓冲初值是 0，GPU 输出 0。
3. fixed：行在 `[row0, min(height, fixed_h, row1+1))`、列 `< min(width, fixed_w)`：`a == 0` 不动，`a == 255` 直接取，否则 `(c*a + d*(255-a)) / 255`（`u32` 截断除法，与 Rust 同）。
4. 下拉指示器：行 `< band_rows`，同一套整数混合。
5. 输出 `vec4f((vec3f(rgb) + 0.25) / 255.0, 1.0)` 写到**非 sRGB** 的 `*8Unorm` 视图。

第 5 步的 `+0.25`：Vulkan 规范允许 float→unorm 转换返回「最接近的两个整数之一」，实现可能截断而不是四舍五入。`(k + 0.25)/255 × 255 = k + 0.25`，四舍五入和截断都得到 k；`k = 255` 时超过 1.0，先被夹到 1.0，仍是 255。这样整条链路只剩整数运算和一次与舍入方式无关的转换。

uniform 字段：`width, height, content_h, scroll_px(i32), canvas_w, canvas_h, ring_rows, page_bg, has_tabbar, tabbar_on, tabbar_px, tabbar_w, tabbar_h, fixed_on, fixed_row0, fixed_row_end, fixed_w, pull_rows, mode(0 分层 / 1 直通), fault_tint`。

### 5.3 覆盖层：直通帧

Toast / Loading / Modal、picker 面板、侧滑返回是**直接画进 `u32` 缓冲**的，混合时会读目标像素，而且用的是浮点（例如 picker 遮罩的 `keep = 1 - mask_alpha`、侧滑的 `dim()`）。报告的做法是 GPU 路径里先把它们画进一张透明 `Canvas` 再合成，这会让 GPU 帧和 CPU 帧在这些区域出现小差异，还要维护一份「画进透明画布」的变体。

本设计改成**直通帧**：只要本帧 `toast`、`loading`、`modal` 任一为 `Some`，或 `picker` 为 `Some`，或 `edge_back` 为 `Some`，就由 `cpu::compose` 把整帧合成进一块暂存 `Vec<u32>`，整块上传到直通纹理，着色器 `mode = 1` 原样输出。判断故意从宽（`Some` 就算，不看 `visible`）：误判成直通永远是对的，误判成分层才会错。代价是覆盖层弹着的那些帧只省掉了上屏拷贝，省不掉合成；这些帧在使用中占比小，S5 场景（§12）专门量它，要求不比 CPU 慢 10% 以上。覆盖层改走 GPU 图层放进待办（§17），到时候另外标定阈值。

### 5.4 上屏（有 surface）

- **格式**：在 `surface.get_capabilities(&adapter).formats` 里依次找 `Bgra8Unorm`、`Rgba8Unorm`；都没有就找对应的 `*Srgb`，配置时加上非 sRGB 的 `view_formats`、往非 sRGB 视图里画（字节原样写入，不做伽马）；还不行就判定不支持，回退 CPU。
- `present_mode = Fifo`（规范保证都支持）；`alpha_mode` 有 `Opaque` 就用，否则取能力列表第一个；`desired_maximum_frame_latency = 2`；`usage = RENDER_ATTACHMENT`。
- **acquire 的结果怎么处理**：`Success` 照常；`Suboptimal` 本帧照用，下一帧前重新 configure；`Timeout` / `Occluded` 本帧不上屏（屏上保留上一帧），返回 `Skipped`；`Outdated` 重新 configure 再取一次，还不行按 `Lost` 处理；`Lost` 用同一个原生目标重建 surface 再取一次，还不行就是致命错误；`Validation` 是致命错误（说明我们代码有 bug）。致命错误怎么回退见 §6.5。
- **帧节奏**：GPU 模式下 `Fifo` 本身就在 acquire 时按 vsync 阻塞，所以桌面交互窗体在 GPU 模式下**关掉**「sleep 到差 0.9ms + 自旋」节拍器，只保留 `request_redraw`。acquire 的等待时间单独计为「等 vsync」，不算进单帧工作时间。

### 5.5 离屏（无头 / 自检 / 测试）

渲染到离屏目标 → `copy_texture_to_buffer`（`bytes_per_row` 按 256 对齐）→ `map_async` + `device.poll(PollType::Wait…)` → 去掉对齐填充 → RGBA。快照在 `--backend gpu` 时**必须**走这条读回，PNG 才代表 GPU 的输出。基准模式每帧也合成到离屏目标并 `poll(Wait)`，用来量「GPU 完成」时间，但不读回。

### 5.6 显存与回收

2× 屏常驻约：页面环 750×3072×4 ≈ 9.2MB，fixed 4MB，tabBar 不到 0.3MB；直通纹理和离屏目标按需再各 4MB。3× 屏约是它的 2.25 倍。`trim_memory()`（SDK 的内存告警入口）同时释放直通纹理、离屏目标和读回缓冲，下次用到再建。

### 5.7 设备与实例

每个引擎实例（桌面窗体 / 每个 `MRApp`）各建一套 Instance / Device，不做进程级共享：设备丢失时影响面限在一个实例里，代码也简单；多开小程序的场景少，每次多花的初始化时间记进 `MINI_GPU_LOG`。

- `InstanceDescriptor.backends`：Auto 下 Apple 只开 `METAL`，Android / Linux 只开 `VULKAN`，Windows 开 `DX12 | VULKAN`；强制 `gpu` 时 Android / Linux / Windows 再加 `GL`。
- `power_preference`：桌面 `HighPerformance`，移动端 `None`。
- `required_limits`：以 `Limits::downlevel_defaults()` 为底（强制走 GL 时以 `downlevel_webgl2_defaults()` 为底），`max_texture_dimension_2d = max(width, height, canvas_w, 1024)`，`max_texture_array_layers ≥ slices`，`required_features` 为空。
- 设备丢失回调写一个 `Arc<AtomicBool>`；未捕获错误写一个 `Arc<Mutex<Option<String>>>`。每次 `present` 先查这两个标志。

---

## 6. 后端选择：分级、自动、强制、回退

### 6.1 偏好从哪来

| 宿主 | 优先级（高 → 低） | 都没给时 |
|---|---|---|
| 桌面交互窗体、`--backend-probe` | `--backend` > `MINI_BACKEND` | `auto` |
| 桌面无头（`--snapshot` 及其上的所有动作、`--frames`、`--drag`、`--bench`） | `--backend` > `MINI_BACKEND` | `cpu` |
| SDK（C ABI / JNI / Swift / Kotlin） | `MINI_BACKEND` > `mr_app_set_backend` | `cpu` |

- 无头默认 `cpu`：所有逐字节判据都在无头下跑，判据以 CPU 为准（steering）。`tools/*.sh` 再显式写 `export MINI_BACKEND="${MINI_BACKEND:-cpu}"`（与 `MINI_FAKE_NOW` 同一写法），做双保险，`gpu-parity.sh` 也能借这个口子让同一套脚本跑 GPU。
- SDK 的 C ABI 默认 `cpu`：老的 C / JNI 调用方从不挂 surface，行为与现在完全一样。
- SDK 里环境变量**压过**宿主配置：`MINI_BACKEND` 是诊断开关，用途是在不改宿主代码的前提下强制某个后端排查问题（Android 用 `android.system.Os.setenv("MINI_BACKEND", "cpu", true)`，iOS 用 Xcode scheme 或 `setenv`）。生效时打一行 `MINI_BACKEND=cpu 覆盖宿主配置 gpu`。
- 桌面里命令行压过环境变量，与其它命令行参数的习惯一致。

### 6.2 机型分级（`select.rs`，纯函数）

先过平台门槛（`platform_gate`，不碰 GPU），再过适配器门槛（`adapter_gate`），二者都只吃数据结构（`DeviceInfo`、`AdapterFacts`），单测不需要 GPU。

| 规则 | Auto | 强制 `gpu` |
|---|---|---|
| 没编进 `gpu` feature | CPU：「未编译 GPU 支持（cargo feature gpu 未开）」 | 同左 |
| Android：`os_api < 29` 或未知（0） | CPU：「机型分级：Android API {n} < 29」 | 放行 |
| Android：`low_ram` | CPU：「机型分级：低内存设备」 | 放行 |
| Android：`total_mem < 3.5e9` 字节或未知 | CPU：「机型分级：内存 {x}GB < 3.5GB」 | 放行 |
| iOS：`total_mem` 已知且 `< 2 GiB` | CPU：「机型分级：内存 {x}GB < 2GB」 | 放行 |
| 适配器 `device_type == Cpu`（llvmpipe / SwiftShader / lavapipe） | CPU：「软件适配器」 | **同左**（软件适配器不算 GPU） |
| `device_type` 是 `Other` / `VirtualGpu` | CPU：「适配器类型 {t}」 | 放行 |
| 后端是 `Gl` | CPU：「Auto 不用 GLES」 | 放行 |
| 命中编译期黑名单 `DENY: &[DenyRule]`（`vendor`、`device`、`driver_info` 子串） | CPU：「黑名单：{规则}」 | 放行 |
| `max_texture_dimension_2d` 或数组层数不够 | CPU：「纹理上限 {n} 不足」 | 同左 |
| 命中判定缓存（§6.4） | CPU：「缓存判定：该驱动上次{原因}」 | 放行 |

- Android 用 3.5e9 字节而不是报告写的 4GB：`ActivityManager.MemoryInfo.totalMem` 报的是内核可用内存，标称 4GB 的机器一般只报 3.6~3.8GB，按 4GB 卡会把标称 4GB 的机器全部挡掉。
- 黑名单初始为空。条目只在 P2 真机矩阵测出问题后添加，不预先编型号（同意报告）。
- 桌面不需要 `DeviceInfo`；Auto 只接受 Metal / Vulkan / DX12 且类型为集显或独显的适配器。

### 6.3 启动自检（Auto 与强制都要过）

GPU 设备建好后、第一帧之前，用一张**固定测试帧**跑三次（`selftest.rs`，确定性 LCG 生成）：画布高 2100 行（跨环形纹理的层边界），滚动偏移让视口跨过画布底边（覆盖「画布外填底色」），带 tabBar，fixed 层同时有 a=0 / a=255 / 半透明像素，带一条下拉指示器，再加一帧直通。每次都与 `cpu::compose` 的结果**逐字节**比较：

- 不一致：回退 CPU，原因「自检像素不一致（首个差异 @x,y：CPU #rrggbb / GPU #rrggbb）」，写入判定缓存（§6.4）。
- 一致，且偏好是 Auto：取第 3 次（前两次含管线预热与首次上传，第 3 次没有新上传，量的正是滚动帧的稳态成本）做两项比较，任一不满足就回退：
  - GPU 占主线程的时间（编码 + 提交）`<` CPU 合成时间，否则原因「自检：GPU 主线程 {g}ms ≥ CPU {c}ms」；
  - GPU 完成时间（提交 → 完成）`< 4.0ms`（120Hz 一帧的一半，GPU 在下一帧前做得完），否则原因「自检：GPU 完成 {d}ms ≥ 4ms」。

  主线程时间是 GPU 真正替宿主省下的部分（实际运行时 GPU 与下一帧的逻辑、光栅是并行的），所以用它和 CPU 比；完成时间只做上限检查。CPU 这边只算合成、不算 softbuffer 上屏，这个比较对 GPU 是偏严的。
- 强制 `gpu` 不做性能比较，只要求像素一致。

报告建议的是「运行期前 N 秒帧统计比 CPU 估计慢就降级」。本设计换成上面的启动期比较，原因见第 16 节第 5 条。运行期只在设备丢失、校验错误、surface 无法恢复时降级。

### 6.4 判定缓存 `gpu-verdict.json`

- 路径：`data_dir::subdir("gpu").join("verdict.json")`。开发机上就是 `target/gpu/verdict.json`，`clean-target.sh` 会顺手清掉；移动端落在宿主给的沙盒目录。
- 键：`backend | vendor | device | name | driver | driver_info | 引擎版本`。驱动或引擎版本一变就是新键，重新评估。
- 记录：自检像素不一致，记一次就拒绝；设备丢失，同一个键累计 2 次才拒绝（单次可能是系统级 GPU 重置）。最多 32 条，超出丢最旧的。
- **注入故障时（`MINI_GPU_FAULT` 已设置）不写缓存**，否则一次回退测试会让之后的 Auto 运行全部走 CPU。
- 只有 Auto 读这个缓存；强制 `gpu` 无视缓存，但仍要过自检。

### 6.5 回退状态机

在 P1、P2，内容始终由 CPU 画在画布里，GPU 只是镜像，所以**任意一帧都可以无损切回 CPU 合成**，不需要重画。

```text
            ┌──────── 初始化 / 门槛 / 自检失败 ────────┐
 [选择] ────┤                                        ├──► [CPU]（本会话不再尝试，除非宿主再调 set_backend / attach）
            └── 通过 ──► [GPU] ── 致命错误（丢失 / 校验 / surface 无法恢复 / 注入）──┘
```

- 回退在**当前这一帧**生效：`present` 返回致命错误时，宿主立刻用 CPU 合成并显示同一帧，不留空帧、不黑屏。
- 每次选定或回退都打一行日志，不受 `MINI_GPU_LOG` 控制：`🎨 渲染后端：GPU（Metal · Apple M3 Pro）— 自动选择` / `⚠️ GPU 回退 CPU：<原因>`。原因同时写进 `BackendStatus.reason`（SDK 用 `mr_app_backend_reason` 取）。
- 设备丢失会写判定缓存；校验错误不写（那是我们代码的 bug，不是驱动不可靠），只打 error 级日志。
- 回退是粘性的：一个会话内回退后不再自动尝试 GPU，避免来回切换。

### 6.6 可观测

- 桌面 `MINI_FPS=1` 每秒那行最前面加后端标签：`📊 [后端 GPU · Metal · Apple M3 Pro] 144 帧/秒，帧间隔 …，最慢一帧 …（… 上屏 0.2[等 vsync 5.9]）`；CPU 时是 `[后端 CPU]`、`[后端 CPU（未编译 GPU 支持）]`、`[后端 CPU（回退：<原因>）]`。
- `--drag` 报告行也加同样的标签。
- SDK 新增 `MINI_FPS` 统计（`host/engine_stats.rs`）：`MiniEngine::new` 时读环境变量，每秒用 `diag` 打一行 `📊 [后端 …] N 帧/秒，pump 最慢 x ms p99 y ms，帧间隔峰值 z ms`。iOS 进 Xcode 控制台，Android 进 logcat（tag `mini-render`）。
- `MINI_GPU_LOG=1`：初始化各段耗时、适配器信息、选用的 surface 格式、自检结果与耗时、每帧的合成模式（`分层` / `直通：Modal`）与上传字节数、CPU 编码耗时、（无头）GPU 完成耗时、镜像补传、回退事件。
- `mini-app-window --backend-probe [--scale s]`：按交互窗体的规则做一次选择（离屏、不开窗），打印结论后退出。退出码 0 = GPU 可用，3 = 没有合格的适配器或被门槛挡掉，4 = 没编进 gpu feature。脚本据此区分「没测」和「测过了」。

---

## 7. 宿主集成

### 7.1 桌面交互窗体（`src/bin/window.rs` 及 `app_window/`）

- `MiniAppWindow` 加字段 `backend: BackendStatus`、`presenter: Presenter`（新文件 `app_window/present_backend.rs`）：

  ```rust
  enum Presenter {
      Soft { surface: softbuffer::Surface<Arc<Window>, Arc<Window>> },
      #[cfg(feature = "gpu")] Gpu(Box<GpuCompositor>),
      None,
  }
  ```
- `resumed()`：建窗口 → 解析偏好 → 不是 `cpu` 且编了 gpu 就尝试 `GpuCompositor::for_window(window.clone(), size, pref, …)`（含门槛与自检）→ 成功就不建 softbuffer；失败就照现在的代码建 softbuffer。
- `present()`：先构造 `FrameLayers`。`Gpu` 分支调 `gpu.present(&layers)`；返回致命错误时打回退日志、`drop` 掉 GPU、建 softbuffer surface，并在**同一帧**走 CPU 上屏。`Soft` 分支就是现在的代码，只是中间那串调用换成 `cpu::compose`。
- `RedrawRequested` 的节拍器：`Presenter::Gpu` 时跳过 sleep + 自旋（§5.4），其余不变；`about_to_wait` 不变。
- **同一窗口能否从 wgpu 切到 softbuffer**：两者在 macOS 上都往视图的根层挂子层（softbuffer 0.4.8 的 `cg.rs` 用 `addSublayer`），所以「先 drop wgpu surface、再建 softbuffer surface」大概率可行，但没验证。P1 第一项任务就是做这个探针（`MINI_GPU_FAULT=lost@120` 加窗口截图）。切不过去的话，退成「销毁并按同样属性重建窗口，再建 softbuffer」，两种做法都由同一个故障注入用例验收。
- softbuffer 的 surface 只在 CPU 模式或回退时才创建，不提前建：提前建会多挂一个子层，可能盖在 Metal 层上面。

### 7.2 桌面无头（`headless_run.rs`）

- `snapshot_all` 最后那段合成改成：`cpu` → `cpu::compose` 进 `Vec<u32>` 再转 RGBA（与现在一致）；`gpu` → `GpuCompositor::offscreen(w, h)` 的 `compose_offscreen(&layers)` 读回 RGBA。GPU 偏好但选择失败时用 CPU，并打回退日志（`gpu-parity.sh` 靠这行日志判空转）。
- 新增 `--bench <out.json>`、`--hz <f>`：见 §12。不带 `--bench` 时无头行为逐字节不变。

### 7.3 SDK（`src/host/engine*.rs`）

- `MiniEngine` 加 `backend: BackendStatus`（偏好默认 `Cpu`）、`device_info: DeviceInfo`、`surface: Option<NativeSurface>`、`#[cfg(feature = "gpu")] gpu: Option<Box<GpuCompositor>>`、`rgba_stale: bool`、`frame_flags: u32`、`stats: FrameStats`。
- `compose_frame`：GPU 生效时构造 `FrameLayers` 调 `gpu.present()`，跳过 `u32 → RGBA` 转换，置 `rgba_stale = true`、标志 `PRESENTED`；返回致命错误就回退并在同一次 `pump` 里走 CPU 合成，标志 `NEW | BACKEND_CHANGED`（不带 `PRESENTED`），宿主据此当场切到位图显示。
- `rgba_for_host(&mut self) -> &[u8]`：`rgba_stale` 时用 `cpu::compose` 把**当前画布**合成一遍再转 RGBA。因为 GPU 与 CPU 逐字节一致，结果就是屏上那一帧，不需要做 GPU 读回。`mr_app_pixels` 改调它；`rgba()` 保留（`examples/sdk_parity.rs` 在用），文档写明 GPU 模式下它可能是旧帧。
- `trim_memory()` 额外调 `gpu.release_caches()`。
- `#[doc(hidden)] pub fn attach_offscreen_for_test(&mut self)`（只在 `feature = "gpu"` 下有）：让桌面测试不开窗口也能走 SDK 的 GPU 路径。

`engine.rs` 现在 659 行，按 steering 20 要先拆再改（P0.2）：`engine.rs`（结构体、构造、路由），`engine_frame.rs`（`pump`、`render`、`render_page_band`、`compose_frame`、`compose_current`、`viewport_inside_drawn_band`、`caret_tick`），`engine_ui.rs`（`drain_ui_events` 与覆盖层查询）。

### 7.4 桌面窗体与 SDK 的分工小结

| | 桌面交互 | 桌面无头 | SDK |
|---|---|---|---|
| 默认偏好 | auto | cpu | cpu（C ABI）/ 内置 View 见 §8.5 |
| GPU 输出目标 | winit 窗口 surface | 离屏 + 读回 | CAMetalLayer / ANativeWindow；测试用离屏 |
| CPU 输出目标 | softbuffer | `Vec<u32>` | `Vec<u32>` → RGBA |
| 回退后 | 建 softbuffer（或重建窗口） | 本次快照用 CPU | 位图显示路径 |

---

## 8. 移动端：surface 怎么来、FFI 怎么扩（只增不改）

### 8.1 原则

- 现有 20 个 `mr_app_*` 的**签名和语义一个都不改**。尤其 `mr_app_pump` 继续只返回 0 / 1（报告提议让它返回 2，这会让写 `== 1` 的 C 调用方漏帧，见第 16 节第 3 条）。
- 新符号在**有没有 gpu feature 时都导出**，C ABI 的符号集合不随构建配置变化；没编 gpu 时它们返回「CPU」和对应原因。
- 不调新接口的宿主永远是 CPU，与现在逐字节一致（`sdk-parity` 守着）。
- 新函数放进新文件 `src/ffi_backend.rs`（`ffi_app.rs` 已有 338 行）；头文件 `include/mini_render.h` 是手写的（仓库没有 `build.rs`），同步加声明。

### 8.2 新增 C ABI

```c
// 后端偏好 / 实际后端
#define MR_BACKEND_AUTO 0
#define MR_BACKEND_CPU  1
#define MR_BACKEND_GPU  2
// surface 种类
#define MR_SURFACE_METAL_LAYER     1   /* CAMetalLayer*（iOS / macOS） */
#define MR_SURFACE_ANDROID_WINDOW  2   /* ANativeWindow* */
// mr_app_pump_ex 的返回位
#define MR_FRAME_NEW              1    /* 有新画面 */
#define MR_FRAME_PRESENTED        2    /* 已由引擎画到挂载的 surface 上，宿主不要再取像素显示 */
#define MR_FRAME_BACKEND_CHANGED  4    /* 实际后端与上一次 pump 不同，宿主要立刻切换显示路径 */

int  mr_app_set_backend(MRApp* app, int pref);           // 成功 1；pref 不在 0..=2 返回 0（写 last_error）
void mr_app_set_device_info(MRApp* app, uint64_t total_mem_bytes, int low_ram, uint32_t os_api);
                                                          // 机型分级用；挂 surface 之前调用。0 表示未知
int  mr_app_attach_surface(MRApp* app, int kind, void* native);
                                                          // 返回挂载后的实际后端：2 = GPU 生效；1 = 仍是 CPU（原因见 backend_reason）；
                                                          // 0 = 参数错误（app/native 为空、kind 不认识）
void mr_app_detach_surface(MRApp* app);                  // surface 将被销毁 / 进后台前调用；之后是 CPU
int  mr_app_backend(MRApp* app);                         // 实际后端：1 / 2；app 为空返回 0
const char* mr_app_backend_reason(MRApp* app);           // UTF-8；到下一次对同一 app 调用任何 mr_app_* 之前有效；app 为空返回 NULL
int  mr_app_pump_ex(MRApp* app, uint64_t now_ms);        // MR_FRAME_* 的按位或；mr_app_pump(app, t) == (mr_app_pump_ex(app, t) & MR_FRAME_NEW) ? 1 : 0
```

- `mr_app_attach_surface` **不带宽高**：引擎按自己的像素尺寸（`mr_app_pixel_size`）配置 swapchain，宿主负责把层或 `SurfaceView` 的缓冲设成同样大（iOS 设 `drawableSize`，Android 调 `holder.setFixedSize`）。这比报告的带宽高版本少一类「尺寸对不上」的失败。
- **所有权**：`CAMetalLayer*` 由调用方持有，必须活到 `mr_app_detach_surface` 或 `mr_app_destroy` 返回；`ANativeWindow*` 由引擎在挂载时自己 `ANativeWindow_acquire` 一次，卸载 / 替换 / 销毁时 `ANativeWindow_release`，调用方手里的引用照常自己释放。
- 重复调用 `attach` 等于先 `detach` 再 `attach`。
- `set_backend` 在已挂 surface 后调用：下一次 `pump_ex` 重新选择，结果变了就带上 `BACKEND_CHANGED`。
- 偏好是 GPU 或 Auto 但没挂 surface：实际后端 CPU，原因「未挂载 surface（宿主需调用 mr_app_attach_surface）」。
- GPU 模式下用老的 `mr_app_pump` + `mr_app_pixels` 也是对的（引擎按需用 CPU 再合成一遍），只是白做一遍合成。集成指南里写明：挂了 surface 就用 `pump_ex`。
- 线程约定不变：所有 `mr_app_*` 在同一线程调用。iOS 继续在主线程的 `CADisplayLink` 回调里调用；Android 在 `HandlerThread` 上调用。
- 后台：iOS 不允许后台提交 GPU 命令，宿主本来就在进后台时停掉 `pump`（现有约定），GPU 模式下这条变成**必须**，集成指南里加粗。

### 8.3 iOS（Metal）

`MiniProgramView.swift` 改动：

- 加一个 `CAMetalLayer` 子层（`frame = bounds`、`contentsScale = dpr`、`drawableSize = 像素尺寸`、`isOpaque = true`），初始隐藏。
- 引擎创建后：`mr_app_set_device_info(h, ProcessInfo.processInfo.physicalMemory, 0, 系统主版本号)` → `mr_app_set_backend(h, backend.rawValue)` → 若「有效偏好提示」不是 CPU，就 `mr_app_attach_surface(h, MR_SURFACE_METAL_LAYER, Unmanaged.passUnretained(metalLayer).toOpaque())`。返回 2 就显示 metal 层、隐藏 `imageLayer`；返回 1 就移除 metal 层。
- 「有效偏好提示」与引擎的优先级（§6.1）保持一致：用 C 的 `getenv("MINI_BACKEND")` 读**当前**环境（`ProcessInfo.environment` 可能是缓存值），能识别就用它，否则用 View 的 `backend`。这样即使 View 默认 `.cpu`，诊断时设 `MINI_BACKEND=gpu` 也能生效；而纯 CPU 时不会白建一个 surface。
- `tick()` 改调 `mr_app_pump_ex`：带 `BACKEND_CHANGED` 先按 `mr_app_backend` 切换两层的显隐；带 `NEW` 且不带 `PRESENTED` 时走现有的像素路径。
- `deinit` 在 `mr_app_destroy` 之前先 `mr_app_detach_surface`。
- 公开属性：`public enum RenderBackend: Int32 { case auto = 0, cpu = 1, gpu = 2 }`、`public var backend: RenderBackend`、`public private(set) var activeBackend: RenderBackend`、`public var backendReason: String`。

wgpu 侧：`instance.create_surface_unsafe(SurfaceTargetUnsafe::CoreAnimationLayer(ptr))`，再按 §5.4 configure。

### 8.4 Android（Vulkan，强制时可走 GLES）

- JNI（`ffi_jni.rs`，现约 230 行，加完不到 400 行）新增：`nativeSetBackend(h, pref): Boolean`、`nativeSetDeviceInfo(h, totalMem: Long, lowRam: Boolean, api: Int)`、`nativeAttachSurface(h, surface: Surface): Int`、`nativeDetachSurface(h)`、`nativeBackend(h): Int`、`nativeBackendReason(h): String`、`nativePumpEx(h, nowMs): Int`。老的 JNI 函数不动。
- `nativeAttachSurface`：`ANativeWindow_fromSurface(env, surface)`（+1）→ `mr_app_attach_surface(app, 2, win)`（引擎内部再 acquire 一次）→ `ANativeWindow_release(win)`（−1）。`ANativeWindow_*` 用 `#[link(name = "android")] extern "C"` 直接声明。没编 gpu 时不调 `fromSurface`，直接返回 1。
- wgpu 侧：`SurfaceTargetUnsafe::RawHandle { raw_display_handle: Some(RawDisplayHandle::Android(AndroidDisplayHandle::new())), raw_window_handle: RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(non_null)) }`。
- `MiniProgramView.kt`：父类从 `View` 改成 `FrameLayout`（构造函数与公开方法不变，集成方的用法照旧），子视图是原来的位图绘制（抽成内部类 `BitmapLayer : View`）和一个 `SurfaceView`。
  - 引擎创建后在渲染线程上调 `nativeSetDeviceInfo`（`ActivityManager.MemoryInfo.totalMem`、`isLowRamDevice`、`Build.VERSION.SDK_INT`）和 `nativeSetBackend`；若「有效偏好提示」不是 CPU，就切到 UI 线程添加 `SurfaceView` 并 `holder.setFixedSize(pw, ph)`。提示的取法同 iOS：用 `android.system.Os.getenv("MINI_BACKEND")` 读当前进程环境（`System.getenv` 在 JVM 启动时就缓存了，看不到 `Os.setenv` 的修改），能识别就用它，否则用 View 的 `backend`。纯 CPU 时不添加 `SurfaceView`，避免首帧前露出一块黑色的 surface 洞。
  - `surfaceCreated` → 投递到渲染线程 `nativeAttachSurface`，按返回值决定显示 `SurfaceView` 还是 `BitmapLayer`。
  - `surfaceDestroyed` → **同步**投递 `nativeDetachSurface` 并等它返回（沿用 `goBack()` 的等待写法）。Android 要求 `surfaceDestroyed` 返回后不能再碰这个 surface。
  - `frame()` 改调 `nativePumpEx`：`BACKEND_CHANGED` 时投递到 UI 线程切换显示（回退是粘性的，切到 CPU 后直接移除 `SurfaceView`）；`NEW` 且不带 `PRESENTED` 时走现有的 `Bitmap` 路径。
  - 公开属性：`enum class RenderBackend(val code: Int) { AUTO(0), CPU(1), GPU(2) }`、`var backend`、`val activeBackend`、`val backendReason`。
- 新的 Kotlin 配老的 `.so` 会在调用新方法时抛 `UnsatisfiedLinkError`。两者在 AAR 里一起发布，不单独升级，集成指南里写明。

### 8.5 内置 View 的默认偏好

C ABI 永远默认 CPU。`MiniProgramView`（Swift / Kotlin）的 `backend` 属性**先默认 `.cpu`**，P2 的真机验收（§14 D2.1~D2.3）通过后，单独一次提交改成 `.auto`。

这是有意的取舍。steering 要求「高端机默认可自动选择」，但 Android 上 GPU 驱动的问题有一部分是**进程级崩溃**，自检和回退接不住，而「不能崩」也是硬性约束。本工作流里没有真机，没法用证据证明 Auto 在真机上安全。桌面在 P1 就默认 Auto，因为本机能完整验收。

### 8.6 构建

`tools/build-mobile.sh` 的 `FLAGS` 改成 `--release --lib --no-default-features --features bindgen,gpu`；设 `MINI_MOBILE_GPU=0` 时去掉 `gpu`（用来验证「关掉 GPU 照样能编」）。`release-mobile.yml` 不改，它调用的就是这个脚本。鸿蒙不在范围内，继续只走 CPU；surface 种类 3 留给以后的 `OHNativeWindow`。

---

## 9. 错误处理

| 操作 | 失败条件 | 可恢复？ | 调用方拿到什么 | 日志 |
|---|---|---|---|---|
| 解析 `--backend` | 值不是 auto / cpu / gpu | 否 | 进程退出码 2，stderr 提示合法值 | error |
| 解析 `MINI_BACKEND` | 值无法识别 | 是 | 按该宿主的默认偏好处理 | warn 一次 |
| `Instance::new` | 本目标没有任何后端（只会在不支持的目标上发生） | 是 | 构造前先查 `Instance::enabled_backend_features()`，为空直接返回 CPU，原因「此平台没有可用的 GPU 后端」，不让它 panic | warn |
| `request_adapter` | 没有适配器 / 返回错误 | 是 | CPU，原因「没有可用的 GPU 适配器：<err>」 | warn |
| 门槛（§6.2） | 任一规则不满足 | 是 | CPU，原因为对应规则 | info |
| `request_device` | 返回错误 | 是 | CPU，原因「GPU 初始化失败：<err>」 | warn |
| 创建 surface | `CreateSurfaceError` | 是 | CPU；FFI `attach` 返回 1 | warn |
| 选 surface 格式 | 没有可用的非 sRGB 写法 | 是 | CPU，原因「surface 无非 sRGB 格式」 | warn |
| 自检 | 像素不一致 / Auto 下性能不占优 | 是 | CPU；像素不一致时写判定缓存 | warn |
| acquire：`Timeout` / `Occluded` | — | 是 | `Ok(Skipped)`，屏上保留上一帧 | `MINI_GPU_LOG` |
| acquire：`Suboptimal` / `Outdated` | 重新 configure 后仍失败 | 是 → 按 `Lost` | `Ok` 或继续往下走 | `MINI_GPU_LOG` |
| acquire：`Lost` | 重建 surface 后仍失败 | 否（对 GPU） | `Err(Fatal::SurfaceLost)`，宿主同帧回退 CPU | warn |
| acquire：`Validation` / 未捕获错误 | — | 否（对 GPU） | `Err(Fatal::Validation)`，同帧回退 | error，不写缓存 |
| 设备丢失回调 | — | 否（对 GPU） | 下一次 `present` 返回 `Err(Fatal::DeviceLost)`，同帧回退 | warn，写缓存 |
| 读回 `map_async` | 回调报错 | 否（对本次快照） | 无头快照改用 CPU 合成结果写 PNG，并打 `GPU 回退 CPU`（让 `gpu-parity.sh` 判失败） | error |
| 读写判定缓存 | 文件不存在 | 是 | 视为空 | 无 |
| | JSON 损坏 / 版本不对 | 是 | 视为空，下次写入时覆盖 | warn |
| | 写入失败 | 是 | 忽略 | warn |
| FFI 参数 | `app` / `native` 为空、`pref` 或 `kind` 越界 | 是 | 返回 0（`void` 函数直接返回），写 `last_error` | 无 |
| JNI `ANativeWindow_fromSurface` | 返回空 | 是 | 返回 1（CPU），原因「ANativeWindow_fromSurface 返回空」 | warn |
| 故障注入 `MINI_GPU_FAULT` | 无法解析 | 是 | 忽略注入 | warn |
| `--bench` 写 JSON | 目录建不出来 / 写失败 | 否 | 退出码 1 | error |
| 基准编排（example） | 二进制不存在、子进程超时（60s）、JSON 解析失败、守卫不过 | 否（对该场景） | 该场景标为失败，跑完其余场景后退出码 1；GPU 被要求但不可用时退出码 3 | error |

致命错误统一用 `GpuError { kind: Fatal{DeviceLost | Validation | SurfaceLost | Fault}, message: String }`；`present` 的成功值是 `PresentOutcome::{Presented(GpuFrameStats), Skipped(&'static str)}`。宿主只区分「成功 / 跳过 / 致命」三类。

---

## 10. 外部输入的校验规则

| 输入 | 必填？ | 类型与取值 | 失败时 |
|---|---|---|---|
| `--backend` | 可选 | 不区分大小写的 `auto` / `cpu` / `gpu` | 退出码 2 |
| `MINI_BACKEND` | 可选 | 同上；空串视为未设置 | warn，按默认处理 |
| `MINI_GPU_LOG` | 可选 | 只看是否设置 | — |
| `MINI_GPU_FAULT` | 可选 | `init`（`request_device` 之后当作失败）/ `selftest`（自检强制判不一致）/ `tint`（见下）/ `lost@N`（第 N 帧调用 `device.destroy()`，走真实的设备丢失回调）/ `surface-lost@N`（第 N 帧 acquire 当作 `Lost`，且重建 surface 也当作失败）。N 为 `u32` 帧号，从选定 GPU 那一帧起计 | warn，忽略 |
| `--backend-probe` | 可选 | 无值；可配 `--scale` | — |
| `--bench <path>` | 可选 | 路径，父目录不存在就创建 | 写失败退出码 1 |
| `--hz <f>` | 可选（只在 `--bench` 时生效，默认 144） | `f32`，范围 [30, 360] | 退出码 2 |
| `mr_app_set_backend` 的 `pref` | 必填 | 0 / 1 / 2 | 返回 0 |
| `mr_app_attach_surface` 的 `kind` | 必填 | 1 / 2（编译目标不支持的种类也返回 0：iOS 上传 2、Android 上传 1） | 返回 0 |
| `mr_app_attach_surface` 的 `native` | 必填 | 非空指针，所有权见 §8.2 | 返回 0 |
| `mr_app_set_device_info` | 可选 | `total_mem_bytes`、`os_api` 为 0 表示未知；`low_ram` 非 0 即真 | 不会失败；未知值按 §6.2 处理（Android 未知视为不达标） |

`tint` 只在**自检之后**的帧里生效（让 R 通道 +1，饱和），否则自检会先把 GPU 判掉，`gpu-parity.sh` 就测不到比较链路本身。

---

## 11. 不变量与归属

| 不变量 | 由谁保证 | 怎么验证 |
|---|---|---|
| 后端不影响布局、命中、事件、`setData`、动画时钟 | **类型**：合成器只拿 `FrameLayers` 的只读引用，拿不到 `MiniApp` / `InteractionManager` / `ScrollController`；CPU 光栅路径不改 | `gpu-parity.sh` 在 `MINI_BACKEND=gpu` 下跑 `tab-click-check`（12/12）与 `interaction-check`（全部一致） |
| 同一份 `FrameLayers`，GPU 输出 == `cpu::compose` 输出 | `compositor::gpu`（着色器复刻 §5.2，覆盖层走直通 §5.3） | 运行期：启动自检；离线：`gpu/tests.rs` 随机对拍 + `gpu-parity.sh` |
| GPU 采样到的页面行 == CPU 画布当前内容 | `PageCanvas`（可变访问必须声明写入范围）+ `MirrorRows`（缺行当场补传） | `gpu-parity.sh` 的 5 个增量用例与交互用例；`MINI_GPU_LOG` 的补传日志 |
| 关掉 gpu feature 时一切与改动前逐字节相同 | `#[cfg(feature = "gpu")]` 只包住 `compositor::gpu` 与宿主里的 GPU 分支；CPU 路径是纯搬迁 | 第 14 节 A0.1、A1.2 的 md5 比对 |
| 无头与回归脚本默认 CPU | `compositor::pref` 的默认值 + 脚本里的 `export MINI_BACKEND` | 单测 + 脚本输出里的后端标签 |
| C ABI 默认 CPU，老接口语义不变 | `ffi_backend.rs` 与 `MiniEngine::new` 的默认偏好 | FFI 单测（§13.1） |
| 任何 GPU 失败都不黑屏、不崩 | 宿主的「同帧回退」（§6.5、§7） | 故障注入用例（§13.3） |

---

## 12. 可复现的无头性能基准（新增）

现在只有 `mini-app-window --drag` 能量帧成本：只覆盖滚动，没有 p99，不含上屏，不分后端。新增一套基准，**所有性能结论都只引用它的输出**。

### 12.1 组成

- **测量端**：`mini-app-window` 新增 `--bench <out.json>` 与 `--hz <f>`，记录逻辑放在新文件 `src/bin/app_window/bench_run.rs`。
  - 无头（同时给了 `--snapshot`）：`--drag` 与 `--frames` 的每一帧在 `pump_one_frame` 之后**额外走一次当前后端的合成**（CPU：`cpu::compose` 进 `Vec<u32>`；GPU：离屏合成 + `poll(Wait)`，不读回），逐帧计时。
  - 窗口（没给 `--snapshot`）：正常开窗，由 `BenchDriver` 在每次 `RedrawRequested` 里推进脚本（先 settle，再执行 `--eval`，然后逐帧 drag 或空转），帧照常走真实上屏，跑完写 JSON 并退出事件循环。这是唯一能测「帧间隔峰值」与真实上屏（含 softbuffer 和 macOS 合成器那一侧）的方式。
  - 节拍：两种模式都按 `--hz`（默认 144）调度。CPU 用现在 `winit_app.rs` 里那套 sleep + 自旋（先纯搬迁成 `app_window/pacer.rs`，交互窗体和基准共用），GPU 窗口模式由 `Fifo` 调度。
- **编排端**：`examples/backend_bench/main.rs`（多文件 example：`scenes.rs` 生成夹具，`report.rs` 汇总）。

  ```bash
  cargo build --release --features gpu && cargo build --release --features gpu --examples
  cargo run --release --example backend_bench -- \
      [--runs 5] [--backends cpu,gpu] [--mode headless|windowed|both] [--scenes all|S1,S3,…] [--out target/_bench]
  ```

  另有 `--gen-only <dir>`：只生成夹具不跑（`gpu-parity.sh` 的回退用例要用 S1 夹具）。

  它会：把夹具小程序写到 `<out>/apps/`（**不进 `sample/`**，也不进仓库，跑完由 `clean-target.sh` 清掉）；用 `image` crate 生成确定性的 1500×1000 渐变加噪声 PNG；对每个「场景 × 后端 × 缩放 × 第几次」先 `rm -rf target/mini-storage`，再以 `MINI_FAKE_NOW=2024-03-15` 拉起子进程（60s 超时）；解析 JSON，跑守卫，按中位数汇总，写出 `<out>/report.json`、`<out>/report.md` 并打印表格。

### 12.2 场景

| 编号 | 内容 | 驱动 | 缩放 | 衡量什么 |
|---|---|---|---|---|
| S1 `scroll_list` | 300 行卡片（圆角 24rpx、两行文字、纯色圆头像） | `--settle 0.5 --drag 8x300` | 2×、3× | 页面级滚动：合成帧成本 + 跨条带重画 |
| S2 `anim_large` | 一个占视口约 70% 的块：渐变底 + 文字，`@keyframes` 无限循环 `rotate(0→360deg) scale(0.9↔1)` 与 `opacity 0.6↔1`，周期 2s | `--settle 0.5 --frames 300` | 2× | 大面积 transform / opacity 动画（损伤区 > 视口 30%，现在每帧整帧） |
| S3 `heavy_paint` | 10 张卡片：`box-shadow: 0 8rpx 32rpx rgba(0,0,0,.25)`、`border-radius: 32rpx`、`linear-gradient` 背景；顶部一张 1500×1000 生成图 `mode="aspectFill"` 撑满宽度 | `--settle 1 --drag 8x300` | 2×、3× | 阴影 / 圆角 / 大图的光栅与合成 |
| S4 `full_redraw` | S3 的内容，`setInterval` 每 16ms `setData` 切换根节点 class（两套背景色） | `--settle 0.5 --frames 300` | 2× | 整屏重绘（CPU 整帧光栅 + GPU 整带上传） |
| S5 `modal_overlay` | S1 的内容 + `--eval "wx.showModal({title:'t',content:'c'})"` | `--settle 0.5 --frames 300` | 2× | 覆盖层直通帧（GPU 不能比 CPU 慢） |
| R1 | `sample-app --route pages/index/index`（只读使用，不改源码） | `--settle 2 --drag 8x120` | 2×、3× | 与报告 §2.1 的基线可对照 |
| R2 | `sample/tea-app --route pages/category/category` | `--settle 2 --drag 8x120` | 2×、3× | 嵌套 scroll-view（报告 p50 5.5ms@2x） |

夹具用到的 WXML / WXSS 特性（插值 class、`setInterval`、`@keyframes`、`box-shadow`、`border-radius`、`linear-gradient`、`<image mode="aspectFill">`）引擎都已支持。夹具源码以字符串常量写在 `scenes.rs` 里，第一次跑时会给每个场景存一张首帧 PNG（`<out>/shots/`），人工确认画面对了再采数。

### 12.3 每帧记录与汇总口径

每帧：`work_ms`（帧开始 → 合成完成。CPU 无头是缓冲写完；CPU 窗口是 softbuffer 的 `buffer.present()` 返回；GPU 无头是 `submit` 加 `poll(Wait)` 返回；GPU 窗口是 `present()` 返回，**不含** acquire 等待）、`logic_ms`、`render_ms`（含页面 / 覆盖层 / tabBar 三段）、`compose_cpu_ms`（合成这一步占主线程的时间；GPU 是编码 + 提交，不含等待）、`gpu_done_ms`（仅 GPU 无头：提交 → 完成）、`vsync_wait_ms`（仅 GPU 窗口）、`interval_ms`（与上一帧开始的间隔）、`rerastered`（这一帧有没有重画页面画布）、`upload_bytes`、`passthrough`、`backend`。

每次运行的指标：

- `slowest_ms` = max(`work_ms`)
- `p99_ms`、`p50_ms`：最近秩法，`sorted[ceil(q·n) − 1]`
- `interval_peak_ms` = max(`interval_ms`)（从第 2 帧起算）
- `composite_interval_peak_ms`：同上，但排除「本帧或上一帧重画过」的帧。它衡量的是合成器本身的帧节奏；跨条带重画造成的尖峰属于光栅，归 P3 解决
- `over_budget` = `work_ms > 1000/hz` 的帧数
- `compose_p99_ms`、`gpu_done_p99_ms`
- 最慢那一帧的分段

跨 5 次运行：每个指标取中位数，`slowest_ms` 另报最小值与最大值（报告记录过单次长尾噪声，例如 tea-app 同一命令最慢帧 6.0 / 15.5 / 75.6ms）。

### 12.4 防空转守卫（不过即该场景失败）

- S1 / S3 / R1 / R2：实际滚动距离 ≥ 请求距离的 90%（或日志显示已经到底）。
- S2：`has_active_animations()` 为真的帧 ≥ 90%。
- S4：`rerastered` 的帧 ≥ 30%。
- S5：GPU 每帧都是 `passthrough`；CPU 每帧 `modal.visible`。
- GPU 运行：JSON 里的 `backend == "gpu"` 且每帧都是；否则判「GPU 未生效」失败，**不能悄悄跑成两遍 CPU**。
- GPU 被要求但 `--backend-probe` 返回 3 / 4：打印「⚠️ GPU 未测」，退出码 3，不能算通过。

### 12.5 输出

`report.md` 每个「场景 × 缩放 × 模式」一行：CPU 与 GPU 各自的 `slowest / p99 / p50 / interval_peak / composite_interval_peak / compose_p99`，GPU 再加 `gpu_done_p99`，最后一列写「GPU 相对 CPU」（p99 比值）。P0 只有 CPU 列，作为基线；P1 起补上 GPU 列。结果表格连同机器、刷新率、提交号一起写进 `doc/引擎测试说明.md`，steering 要求的「说清楚 GPU 快在哪个场景」就从这张表来。

---

## 13. 测试与判据

### 13.1 单元测试（`cargo test --release`，有无 gpu feature 都要跑）

- `compositor::pref`：命令行 / 环境变量 / FFI 整数的解析，§6.1 的优先级，各宿主的默认值。
- `compositor::select`：§6.2 每条规则一条用例（Android API 28 → CPU；`low_ram` → CPU；3.4e9 字节 → CPU；API 31 + 8GB + Vulkan 集显 → GPU；Auto 下 GLES → CPU、强制 → GPU；软件适配器在强制下也是 CPU；纹理上限不足 → CPU；黑名单命中 → CPU；未编译 → CPU）。
- `compositor::verdict`：读写往返；损坏文件被忽略；32 条上限；注入故障时不写。
- `compositor::fault`：语法解析。
- `layer.rs`：`PageCanvas` 写入日志的 32 条上限，`ops_since` 在落后过多时返回 `None`，`replace` 之后 `gen` 递增、日志清空；`LayerCanvas::write` 递增代数；`color_bytes` 的长度。
- `gpu/mirror.rs` 里纯算术的部分：环形下标映射；`MirrorRows` 在条带写入、矩形写入、缺行补传、超出环长时的区间结果。
- 直通判定函数：每种覆盖层状态各一条。
- FFI（`ffi_backend.rs`）：空指针不崩；默认 `mr_app_backend == 1`；首帧 `pump_ex == MR_FRAME_NEW`；CPU 模式下从不出现 `PRESENTED`；`set_backend(7) == 0`；`attach_surface(kind=9)` 与 `native == NULL` 都返回 0；不带 gpu 构建时 `attach` 返回 1，原因里含「未编译」；`mr_app_pump` 与 `pump_ex & 1` 一致。

### 13.2 GPU 对拍（`#[cfg(all(test, feature = "gpu"))]`，没有适配器时打印「GPU 未测」并跳过）

- `gpu/tests.rs`：固定种子生成 50 组 `FrameLayers`（滚动跨环形层边界、负滚动、滚出画布、画布比视口矮、画布比输出窄、tabBar 有 / 无、fixed 各种 alpha、指示器、直通），离屏合成后与 `cpu::compose` 逐字节比较。
- SDK 路径：`attach_offscreen_for_test` 之后，sample-app 15 页首帧 `pump_ex == NEW | PRESENTED`，且 `rgba_for_host()` 与 GPU 读回逐字节相同；`MINI_GPU_FAULT=lost@2` 时第 2 次 `pump_ex == NEW | BACKEND_CHANGED`，像素与纯 CPU 路径相同。

`cargo test` 在没有 GPU 的机器上跳过是允许的，**决定性的判据是 `gpu-parity.sh`**，它在没测到时不会返回 0。

### 13.3 `tools/gpu-parity.sh`（新增）

1. `--backend-probe`：退出码 4 →「❌ 未编译 gpu feature」退出 4；3 →「⚠️ GPU 未测：没有可用适配器（软件适配器不算）」退出 3。
2. 静态页：`sample-app`（`--settle 0 --time 2`，15 页）与 `sample/news-app`（6 页），`MINI_BACKEND=cpu` 和 `MINI_BACKEND=gpu MINI_GPU_LOG=1` 各跑一遍 `snapshot-all.sh`，逐页 `pixdiff.py`。
3. 增量重绘：`damage-check.sh` 的 5 个 `--eval` 用例分别用 cpu 和 gpu 出图并比较。
4. 交互：`MINI_BACKEND=cpu bash tools/interaction-check.sh target/_gp_ia_cpu`，再 `MINI_BACKEND=gpu bash tools/interaction-check.sh target/_gp_ia_gpu target/_gp_ia_cpu`（它自带「弹出 vs 没弹出 ≥ 20%」的空转守卫）。
5. 行为：`MINI_BACKEND=gpu bash tools/tab-click-check.sh` 必须 12/12。
6. 防空转：每个 GPU 运行的日志都要有 `渲染后端：GPU`；整轮下来 `合成模式 分层` 与 `合成模式 直通` 各至少出现一次；再用 `MINI_GPU_FAULT=tint` 跑一页，必须报不一致，否则「比较链路空转」失败。
7. 回退：在 S1 夹具（没有定时器，结果确定）上，`MINI_GPU_FAULT=init` 与 `MINI_GPU_FAULT=selftest` 的快照必须与 CPU 逐字节相同且日志有 `GPU 回退 CPU`；`--frames 60` 加 `MINI_GPU_FAULT=lost@30` 必须有 `运行中回退` 且最终快照与 CPU 相同。
8. 判定：上面所有比较的差异像素数都必须是 **0**（这就是 P1 的阈值，写进 `doc/引擎测试说明.md`）。结束时打印 `GPU 与 CPU 逐字节一致：N/N`，清理 `target/_gp_*` 与 `target/mini-storage`。

### 13.4 七项回归照跑

steering 30 的七项全部照跑（加上 `--features gpu` 构建）。脚本默认 CPU，所以结果与基线逐字节一致。建议在 steering 30 的清单里加第 ⑨ 项「编了 gpu 时跑 `bash tools/gpu-parity.sh`」——这是改用户规则文件，**需要用户同意**，本设计只在 `doc/引擎测试说明.md` 里加，不改 steering。

---

## 14. 分阶段路线与验收

通用约定：纯搬迁和改行为分开提交（steering 20）。「md5 一致」指在改动前的 `main`（`09b8fb8`）上先生成的基线：`target/baseline_main_sample`、`target/baseline_main_news`（`snapshot-all.sh … --settle 0 --time 2`）、`target/baseline_main_ia`（`interaction-check.sh`）、`target/baseline_main_fin2_*`（给 `sdk-parity` 用）。用 `baseline_` 前缀，`clean-target.sh` 才不会清掉。画廊的判据是 `git status --porcelain doc/gallery` 为空，只有 `35_gif_frame4.png` 按已知噪声处理。性能数字一律取基准跑 5 次的中位数，机器为本机 M3 Pro、144Hz。

### P0　测量与接缝（CPU-only）

| 步骤 | 内容 | 性质 |
|---|---|---|
| P0.1 | 在未改动的 `main` 上生成上面那些 `baseline_main_*` 基线 | 准备 |
| P0.2 | 拆 `engine.rs`（§7.3） | 纯搬迁 |
| P0.3 | `compositor::{mod, cpu}`，四个调用点改调 `cpu::compose`；拆出 `pull_indicator_band` | 纯搬迁 |
| P0.4 | `LayerCanvas` / `PageCanvas` 替换宿主里的画布字段，`FrameLayers` 的字段同步从 `&Canvas` 换成 `&PageCanvas` / `&LayerCanvas` | 纯重构（代数和脏区记账没有读者） |
| P0.5 | `pref`、`select`、`verdict`、`diag`、`fault` 模块；`--backend`、`MINI_BACKEND`、`--backend-probe`；`MINI_FPS` 与 `--drag` 的后端标签；SDK 的 `MINI_FPS` 统计；`ffi_backend.rs` 与 JNI 新函数（只有 CPU 行为）；头文件；各脚本的 `export MINI_BACKEND` | 改行为（只多了日志和新接口） |
| P0.6 | 搬出 `pacer.rs`（纯搬迁），再加 `bench_run.rs`、`examples/backend_bench`，跑出 CPU 基线 | 先纯搬迁，再新增 |

验收：

- **A0.1** P0.2、P0.3、P0.4、P0.6 的纯搬迁这几次提交，每一次都满足：sample 15 页与 news 6 页的 `rust.png` md5 与基线相同；画廊无变化；`sdk-parity` 15/15、6/6；`damage-check` 5/5；`interaction-check` 与 `baseline_main_ia` 全部一致；`tab-click-check` 12/12；`cargo test --release` 全绿。
- **A0.2** 本轮新增或改动的每个 `.rs` 文件 `wc -l ≤ 500`，例外要在文件头写明理由。
- **A0.3** `MINI_FPS=1` 的每一行以 `📊 [后端 CPU` 开头；`--drag` 报告带 `[后端 CPU]`；在没编 gpu 的构建上 `--backend gpu` 打 `渲染后端：CPU — 未编译 GPU 支持`、退出码 0、PNG md5 与基线相同；`--backend foo` 退出码 2；`--backend-probe` 退出码 4。
- **A0.4** §13.1 的 FFI 用例全部通过。
- **A0.5** `bash tools/build-mobile.sh check` 通过（iOS arm64 + Android 三个 ABI）。
- **A0.6** `backend_bench --backends cpu --runs 5 --mode both` 跑完 S1~S5、R1、R2（S1、S3、R1、R2 含 3×），守卫全过，CPU 基线表写进 `doc/引擎测试说明.md`。

### P1　桌面 GPU 合成器

| 步骤 | 内容 |
|---|---|
| P1.1 | 第一件事：同一窗口 wgpu → softbuffer 的切换探针（§7.1），结论写进 `doc/架构与实现原理.md` |
| P1.2 | 依赖与 feature；`gpu/` 各模块；离屏合成 + 读回；自检；判定缓存接线；故障注入；无头 `--backend gpu`；`gpu-parity.sh` |
| P1.3 | 窗口 surface 上屏、`Fifo`、GPU 模式关掉节拍器、同帧回退、`MINI_FPS` 的 GPU 分段 |
| P1.4 | 基准补 GPU 列；把阈值和对比表写进文档；README 中英文 |
| P1.5 | 单独一次提交：把 `gpu` 加进 `default` features（前提是 A1.1~A1.8 全过） |

验收：

- **A1.1** `cargo build --release`、`cargo build --release --features gpu`，以及两者的 `--examples`，都能编过；`cargo test --release` 与 `cargo test --release --features gpu` 都全绿。
- **A1.2** 不带 gpu 构建：七项回归全过，所有 PNG md5 与基线相同。
- **A1.3** 带 gpu 构建、脚本默认 CPU：同 A1.2。
- **A1.4** `bash tools/gpu-parity.sh` 退出码 0：静态页 21 页、增量用例 5 个、交互状态全部 **0 个差异像素**；分层和直通两种模式都覆盖到；`tint` 能被检出；GPU 下 `tab-click-check` 12/12。`doc/引擎测试说明.md` 写明 **P1 阈值：差异像素 0（所有帧，分层与直通）**，并列出测过的适配器。
- **A1.5** 回退：`init`、`selftest`、`lost@30`（无头）都满足 §13.3 第 7 条。窗口模式直接跑二进制（不经编排端，编排端的守卫会把回退判成失败）：`MINI_GPU_FAULT=lost@120 ./target/release/mini-app-window <S1 夹具> --backend gpu --hz 144 --drag 8x300 --bench target/_bench/fault.json`，要求进程退出码 0；JSON 里第 120 帧起 `backend == "cpu"`，之前都是 `"gpu"`；300 帧每一帧都上屏了（没有 `Skipped`、没有缺帧）；`surface-lost@120` 同样跑一遍；外加一次人工目视确认回退后画面正常（不黑、不冻结）。
- **A1.6** 性能（基准）：
  - 无头 S1 的合成帧（`rerastered == false`）：GPU `compose_p99 ≤ 0.30ms`，且 `≤ 0.5 ×` CPU 的 `compose_p99`（2×、3× 都要满足）。
  - 无头，所有场景都不退步：GPU `slowest ≤ CPU slowest × 1.10 + 0.5ms`，GPU `p99 ≤ CPU p99 × 1.10 + 0.3ms`（S5 直通也在内）。
  - 无头 3× 的 S1：GPU `gpu_done_p99 ≤ 2.0ms`。
  - 窗口 S1 2×：GPU `composite_interval_peak ≤ 7.2ms`，合成帧的 `work_ms` p99 `≤ 1.0ms`。CPU 的同项数字记录在案，不设门槛（softbuffer 的 30~50ms 偶发停顿会体现在这里，可以看出 GPU 是否消除了它，但这是报告列出的未验证项，不作为门槛）。
  - 对比表写进文档，逐场景说明 GPU 在哪里快、在哪里持平。预期：S1、S5、R1 的合成帧更快；S2、S3、S4、R2 由光栅主导，基本持平。
- **A1.7** 记录 `--features gpu` 带来的 `mini-app-window` stripped 体积增量。
- **A1.8** README.md 与 README.zh-CN.md 都写了 `MINI_BACKEND`、`--backend`、`--backend-probe`、`MINI_GPU_LOG`、`MINI_GPU_FAULT`、`MINI_FPS` 的后端标签、`cargo build --features gpu`；`doc/引擎测试说明.md` 的诊断开关表补上这些行。
- **A1.9**（P1.5 之后）默认构建的交互窗体在本机自动选中 GPU（`MINI_FPS` 显示 `[后端 GPU · Metal · …]`）；`cargo build --release --no-default-features --features desktop,audio,h264` 下 A1.2 仍然成立。

### P2　移动端 GPU 上屏与机型分级

| 步骤 | 内容 |
|---|---|
| P2.1 | SDK 里的 GPU 分支、`rgba_for_host`、`attach_offscreen_for_test`、`ffi_backend.rs` 的 GPU 行为、JNI 的 `ANativeWindow` 接线、`diag` 的 Android `liblog` 出口 |
| P2.2 | Swift 的 `CAMetalLayer` 路径；Kotlin 的 `FrameLayout` + `SurfaceView` 路径；集成指南 |
| P2.3 | `build-mobile.sh` 加 gpu feature |
| P2.4 | （有真机后）真机矩阵、黑名单、内置 View 默认值改成 `.auto` |

本机可以判定的验收：

- **A2.1** `bash tools/build-mobile.sh check`：带 gpu（默认）和 `MINI_MOBILE_GPU=0` 两种都通过（iOS arm64 + Android 三个 ABI）。
- **A2.2** 用 `nm -gU`（iOS 静态库）和 `llvm-nm -D`（Android `.so`）列出所有 `mr_app_*` 符号：两种构建的符号集合相同，并且是「改动前的全部 + §8.2 新增」。
- **A2.3** §13.2 的 SDK 路径用例在本机通过。
- **A2.4** `sdk-parity`（CPU）仍然 15/15、6/6。
- **A2.5** §13.1 里机型分级的每条规则都有用例并通过。
- **A2.6** iOS 的 Swift 源码能在本机编过：先 `bash tools/build-mobile.sh ios` 产出 XCFramework，再在 `sdk/ios` 下 `xcodebuild -scheme MiniRender -destination 'generic/platform=iOS Simulator' build`；本机没有 Xcode 就如实写「未编译验证」。Android 的 Kotlin 在本机有 Android SDK 时用 `sdk/android` 的 `./gradlew assembleRelease` 编过；没有 Android SDK 就如实写「未编译验证」。

要真机的验收（本工作流没有设备，由用户执行）：

- **D2.1** Auto：A14 及以上的 iPhone → GPU（原因里显示 Metal 适配器）；API ≥ 29、内存 ≥ 4GB、支持 Vulkan 的 Android 旗舰 → GPU；低端 Android（API < 29、低内存设备或内存 < 3.5e9 字节，至少其中一种）→ CPU，原因写明命中的规则。
- **D2.2** 强制 GPU 加 `MINI_GPU_FAULT=lost@60`（Android 用 `Os.setenv`）：回退那一下不黑屏（同一个 tick 内显示 CPU 位图），触摸照常响应，日志里有回退行。
- **D2.3** sample-app 首页连续滚动 10 秒，看每秒的 `MINI_FPS` 行：同一台设备上 GPU 的 `pump p99 ≤ 0.4 ×` CPU 的 `pump p99`，GPU 的帧间隔峰值不高于 CPU。
- **D2.4** 每个 ABI 的 stripped 体积增量 ≤ 8MB；超过就停下来问用户。
- **D2.5** D2.1~D2.3 都过了之后，单独一次提交把 `MiniProgramView` 的默认值改成 `.auto`。

### P3　图层化（两后端共享；本工作流不实现，判据先定）

内容：嵌套 scroll-view 做成独立图层（子路径也用 `ScrollViewCache`；只有「绘制顺序里后面没有和它重叠的兄弟节点」时才提升，否则维持现状）；transform / opacity 动画子树按「子树数据指纹 + 尺寸」缓存内容，动画帧只改矩阵，GPU 用着色器复刻 `compose.rs::sample_bilinear`；滚出条带时只补画新露出的那段；新开关 `MINI_NO_LAYERS=1`。

- **A3.1** CPU：`MINI_NO_LAYERS=1` 与默认在 sample、news、tea-app 的抽样页逐字节一致。
- **A3.2** R2 拖动 2×：两个后端都满足 p50 ≤ 1.5ms、slowest ≤ 6.9ms（报告基线 p50 5.5ms）；3× 下 p50 ≤ 3ms（基线 13.1ms）。
- **A3.3** S1、R1 2×：两个后端都满足 slowest ≤ 6.9ms（跨条带帧，基线 17~18ms），窗口 S1 的 `interval_peak ≤ 7.2ms`（全部帧）；3× 下 slowest ≤ 8.3ms（基线 24.1ms）。
- **A3.4** S2：CPU slowest ≤ 6.9ms，GPU slowest ≤ 2.0ms。
- **A3.5** 变换图层上 GPU 与 CPU 的同帧差异，按报告 §4.2 第 3 条的方法实测定阈值（差异占比取实测最大值的 1.5 倍、向上取整到 0.01%，包围盒必须落在变换图层矩形内），写进文档。

### P4　GPU 专属效果与按需光栅（按数据逐项立项）

`filter` / `backdrop-filter`（Skyline 只支持单个函数）、图片采样上 GPU、复用 fontdue 位图的字形图集、矩形 / 圆角 SDF。每一项立项前，都要先用基准加 `MINI_DRAW_LOG` 证明它是目标机上的瓶颈；做完单独标定阈值。CPU 做不到的效果要在那一项的设计里写明降级行为（例如 `blur` 在 CPU 上用现有三遍盒式模糊、限制最大半径，超出时打一次日志提示），不能假装支持。P1、P2 里没有任何 GPU 独有的效果，两个后端的输出完全一样，所以现在不需要降级说明。

---

## 15. 文件清单

**新增**

- `src/compositor/{mod, layer, cpu, pref, select, verdict, diag, fault}.rs`
- `src/compositor/gpu/{mod, device, surface, mirror, pipeline, readback, selftest, tests}.rs`、`src/compositor/gpu/compose.wgsl`
- `src/ffi_backend.rs`、`src/host/engine_frame.rs`、`src/host/engine_ui.rs`、`src/host/engine_stats.rs`
- `src/bin/app_window/{present_backend, bench_run, pacer}.rs`
- `examples/backend_bench/{main, scenes, report}.rs`
- `tools/gpu-parity.sh`

**修改**

- `Cargo.toml`（feature 与依赖，§1）
- `src/lib.rs`（`pub mod compositor; mod ffi_backend; pub use ffi_backend::*;`）
- `src/host/{engine.rs, render.rs, mod.rs}`
- `src/ffi_app.rs`（`mr_app_pixels` 改调 `rgba_for_host`；`MRApp` 加 `backend_reason: CString` 缓存）
- `src/ffi_jni.rs`
- `include/mini_render.h`
- `src/bin/window.rs`（字段、`--backend` / `--backend-probe` / `--bench` / `--hz`）
- `src/bin/app_window/{winit_app, frame_render, headless_run, edge_back_host, page_host, tabbar_host, frame, scroll_bench, mod}.rs`
- `sdk/ios/Sources/MiniRender/MiniProgramView.swift`
- `sdk/android/src/main/kotlin/dev/minirender/{MiniEngine, MiniProgramView}.kt`
- `tools/{snapshot-all, damage-check, sdk-parity, interaction-check, tab-click-check, build-mobile}.sh`
- `README.md`、`README.zh-CN.md`、`doc/引擎测试说明.md`、`doc/原生App集成指南.md`、`doc/架构与实现原理.md`（加一节「渲染后端」和探针结论）

**不动**：`sample/**`、`src/renderer/**` 的绘制路径、`src/canvas/**`、`host/ui_overlay.rs` 与 `host/picker_sheet/**` 的绘制、`src/bin/launcher.rs`（只是选择器，拉起的子进程会继承 `MINI_BACKEND`）、`src/bin/main.rs`、`examples/gallery.rs`、`examples/compare.rs`、`.kiro/steering/**`。

---

## 16. 与调查报告不一致的地方

1. **不定义 `trait Compositor`**（报告 §5.1）。CPU 输出写进宿主借出的缓冲，三个宿主各不相同；GPU 输出到 surface 或离屏纹理。统一成 trait 得再造一层「输出目标」抽象，CPU 路径用不上。改用自由函数 `cpu::compose` 加具体类型 `GpuCompositor`，宿主按 `BackendStatus` 分派，CPU 路径零额外开销。
2. **覆盖层走「直通帧」**（报告 §4.3 是画进透明画布再合成）。好处是不用维护第二份覆盖层绘制，而且 P1 所有帧都能做到 0 差异，判据最强。代价是覆盖层弹着时 GPU 只省了上屏；S5 场景专门盯着这个代价。
3. **`mr_app_pump` 保持 0 / 1，新增 `mr_app_pump_ex`**（报告是让 `pump` 返回 2）。头文件和文档对 `pump` 的承诺是「返回 1 表示有新画面」，写 `== 1` 的 C 调用方遇到 2 会漏帧，破坏 ABI 语义。
4. **报告的 P1（SDK 出帧闸门、Choreographer、`preferredFrameRateRange`）移出本工作流**。它属于对齐审计的范围（任务说明明确不实现那份报告里的条目），而且不是 GPU 正确性的前提：GPU 只合成 CPU 已经画好的东西，`Fifo` 本身也会按 vsync 节流。后果要说清楚：在它落地之前，移动端的 CSS 动画和 swiper 过渡在两个后端上都不会连续出帧，所以移动端的 GPU 收益只体现在滚动和合成上。基准放在桌面宿主上跑，也是因为 SDK 目前驱动不了动画场景。
5. **不做运行期的性能降级**（报告 §5.3 的「前 N 秒 GPU p95 比 CPU 估计慢就降级」）。会话内没有可比的 CPU 数据，要比就得同时跑一遍 CPU 合成，这恰好是想省掉的开销，而且容易来回切换。改成启动自检时比一次稳态合成成本（§6.3），结论确定，不会抖动。
6. **页面镜像用环形条带纹理加按需补传**（报告是 512 行一块、LRU 淘汰）。只镜像「视口 ± 余量」这一段，显存有固定上界；GPU 输出的正确性不再依赖宿主的「已绘制条带」不变量（§4.3）。
7. **合成序列是四处，不是两处**：还有 `snapshot_all` 和 `capture_viewport`。P0.3 四处一起收。
8. **wgpu 的 feature 已核对**：后端特性在 wgpu 内部按目标门控，一条依赖声明就够，不用按目标拆表（报告标为未验证）。
9. **`mr_app_attach_surface` 不带宽高**，由引擎按自己的像素尺寸配置，少一类尺寸不一致的失败。
10. **GPU 模式下 `mr_app_pixels` 用 CPU 再合成**，不做 GPU 读回：两者逐字节相同，CPU 再合成更便宜，也不用等 GPU。
11. **源纹理用 `Rgba8Uint`，输出加 0.25 偏置**（报告是从 unorm 纹理 `textureLoad` 再做 u32 运算）。不经过 unorm→float 的输入转换，输出转换不管实现是四舍五入还是截断都得到同一个值。
12. **Android 的内存门槛定为 3.5e9 字节**（报告写 4GB）：`totalMem` 报的是内核可用内存，标称 4GB 的机器一般报 3.6~3.8GB。
13. **SDK 里 `MINI_BACKEND` 压过宿主配置**：它是诊断开关，用途就是不改宿主代码也能强制某个后端（报告没有规定优先级）。
14. **内置 View 先默认 `.cpu`，真机验收后再改 `.auto`**（报告没有区分）。原因见 §8.5：没有真机证据时，用 Auto 冒 Android 驱动进程级崩溃的风险。
15. **`gpu` 在 P1（桌面）验收后进 `default`**（报告是 P2 之后）。桌面在本机能完整验收；移动端构建本来就用 `--no-default-features`、显式列 feature，不受 `default` 影响。

---

## 17. 风险与未验证项

| 风险 / 未验证项 | 影响 | 应对 |
|---|---|---|
| 同一窗口 wgpu → softbuffer 的切换 | 运行期回退可能要重建窗口 | P1.1 先做探针；两种做法共用同一个故障注入验收（A1.5） |
| macOS 上 CAMetalLayer 与 softbuffer 的 CGImage 色彩空间处理不同 | GPU 屏上偏色，而离屏读回判据仍然全绿 | P1.3 用 `screencapture -l <窗口号>` 分别截两个后端，`pixdiff` 最大通道差应 ≤ 2；超出就通过 `Surface::as_hal` 把层的 `colorspace` 设成和 softbuffer 一致，结论写进文档 |
| GPU 能否消除 macOS 上屏的 30~50ms 偶发停顿 | 影响窗口模式的峰值 | A1.6 记录 CPU 与 GPU 的 `interval_peak`，只陈述数据，不作门槛 |
| 移动端各项耗时、体积增量 | 收益与包体大小不确定 | D2.3、D2.4 按真机数据判定，体积超 8MB 就问用户 |
| Android 驱动导致进程级崩溃（自检和回退都接不住） | 违反「不能崩」 | 内置 View 默认 `.cpu`，等真机验收；Auto 只用 Vulkan 加 API ≥ 29；黑名单 |
| 覆盖层弹着时只省了上屏 | GPU 收益打折 | 待办：覆盖层图层化（Toast / Modal / picker 画进各自的透明层），另外标定阈值 |
| wgpu 大约一季度一个破坏性大版本 | 升级成本 | 版本锁死 `=30.0.1`，升级单独立项，重跑 §13 |
| 显存加倍（CPU 画布 + GPU 镜像） | 内存本来就是最大的短板（600MB~1GB） | 只镜像条带（§5.6）；`trim_memory` 释放 GPU 缓存 |
| 没有 GPU 的 CI | 「全绿但没测到」 | `gpu-parity.sh` 和基准都用退出码 3 表示没测；软件适配器一律不算 GPU |

---

## 18. 不在范围内

- 对齐审计报告里的条目，包括 SDK 的出帧闸门（动画 / swiper / 定时器连续出帧）、SDK 的 `setData` 损伤区与侧滑返回、Android Choreographer、iOS `preferredFrameRateRange` / `CADisableMinimumFrameDurationOnPhone`、`filter` / `backdrop-filter` / `mask-image` 等 WXSS 能力。
- P3、P4 的实现（判据已在 §14 给出）。
- GPU 光栅化（vello / skia 一类）与显示列表（接缝 2）。
- 鸿蒙的 GPU 路径（继续走 CPU）。
- 渲染线程与逻辑线程分离（Skyline 的独立渲染线程），GPU 不改变「JS、布局、光栅在同一线程」这一点。
- `mini-launcher`、`mini-app` 两个二进制。
- 修改 `sample/` 下的任何文件；修改 `.kiro/steering/**`（§13.4 的建议要等用户同意）。
