# 设计评审：双渲染后端（CPU 参考实现 + GPU 可选）

- 评审对象：`.agents/tasks/gpu-dual-backend-2026-10-12/design.md`（第一版）
- 对照：`.kiro/steering/10-skyline-target.md`「双渲染后端：CPU 与 GPU」、`.agents/tasks/gpu-rendering-feasibility-2026-10-12/report.md`
- 方法：只读。对照 `main@09b8fb8`（与 `feat/gpu-backend` 同一提交）的源码，以及 `~/.cargo/registry` 里的 wgpu 30.0.1 / wgpu-core / wgpu-hal / wgpu-types / softbuffer 0.4.8 / raw-window-handle 0.6.2 源码，逐条核对设计里的说法。没有跑构建和测试。
- **结论：CHANGES_REQUESTED**（HIGH 1 条，MEDIUM 15 条，NIT 7 条）

整体评价：骨架是对的。接缝放在合成层、整数着色器复刻 CPU 合成、直通帧、`mr_app_pump_ex` 只增不改、无头默认 CPU、防空转退出码，这些都站得住，wgpu API 名也都核对过了。没有发现后端越界（布局 / 命中 / 事件 / 增量失效 / 动画时钟漏进后端）的问题。主要问题是**验收链路测不到设计自己点名的「唯一风险」**：增量镜像。另外有一批状态机和判据口径没定死。

---

## 发现

### 1. [HIGH] 增量镜像（§4.3）在所有逐字节判据里都没被真正比对过，`gpu-parity.sh` 会「全绿但没测到」

**位置**：§4.3 末段、§7.2、§11 第 1~3 行、§13.2、§13.3 第 2~5 步、A1.4。

**问题**：§4.3 说「剩下唯一的风险是已在 `valid` 里的行被改写却没声明」，并称「`gpu-parity.sh` 用全部静态页、5 个增量重绘用例和交互用例逐字节守住这一点」。实际上守不住：

- 无头 `snapshot_all` 只在**最后**合成一次（`headless_run.rs:195-237`）。settle、`--eval`、`run_actions`、`--frames` 循环里只跑 `pump_one_frame()` / `render()`，从不合成。§7.2 让 GPU 在快照时才 `GpuCompositor::offscreen(w, h)`，§12.1 又规定「不带 `--bench` 时无头行为逐字节不变」。所以 GPU 合成器一辈子只见过一帧，`valid` 是空的，可见行全部从此刻的 CPU 画布补传，结果必然和 CPU 一样。`ops_since` 回放、`Rect` 局部上传、`valid ∪ visible` 合并、环形回绕，这些逻辑**一次都没被比对过**。
- `damage-check.sh` 的 5 个用例也是 `--eval` 加单次快照（`damage-check.sh:13-24`），同样只合成一次。
- `tab-click-check.sh` 只看快照路径名（`:28-30`），合成器根本不参与。所以 §11 拿「GPU 下 12/12」证明后端无关，是空转。
- §13.2 的 50 组随机 `FrameLayers` 没说要在**同一个**合成器上按序列跑；SDK 用例只比首帧和 `lost@2`。
- 真正连续出帧的只有 `--bench`（不读回）和窗口模式（不读回）。

这正是 steering 30 记录过的 `damage-check` 第一版空转，GPU 会在窗口里花屏，而判据全绿。

**修改**（四条都要）：

1. 无头 `--backend gpu`：`snapshot_all` 在 `setup_canvas` 之后建**一个**常驻 `GpuCompositor`（存进 `MiniAppWindow`），然后在 `pump_one_frame()` 末尾和无头路径里每一次 `render()` 之后，调 `gpu.compose_offscreen(&layers)`（不读回）。这样镜像回放的写入序列和窗口模式完全一样；快照时再读回。
2. 新增诊断开关 `MINI_GPU_VERIFY=1`（写进 README 与 §6.6）：每次 GPU 合成都读回，并与同一份 `FrameLayers` 的 `cpu::compose` 逐字节比较。不一致时打 `❌ GPU 逐帧校验不一致 帧 {k} @({x},{y}) CPU #rrggbb GPU #rrggbb` 并以退出码 5 退出；正常结束时打 `GPU 逐帧校验：{n} 帧一致`。
3. `gpu-parity.sh` 加一段「连续帧」检查，全部带 `MINI_GPU_VERIFY=1`：damage-check 的 5 个 `--eval` 用例，每个加 `--frames 30`；`sample-app pages/index/index --settle 2 --drag 8x120`（2× 和 3×）；`news-app pages/home/home --drag 8x120`；S1 夹具 `--drag 8x300`（要求日志里出现至少 3 次整带写入和至少 1 次 `镜像补传`，证明真的跨了条带、回绕了环）；再加 `interaction-check.sh`。防空转规则：每个运行的「校验帧数」≥ 实际出帧数 × 0.9，否则判「逐帧校验空转」失败。
4. `gpu/tests.rs` 加序列对拍：一个常驻合成器，固定种子跑 200 步随机操作（`write_band` / `write_rect` / `replace` / 改滚动 / `LayerCanvas::write` / 替换 `LayerCanvas`），**每一步**都和 `cpu::compose` 比较。SDK 侧在 `attach_offscreen_for_test` 下把 `src/tests/engine_input_tests.rs` 再跑一遍，同时开逐帧校验（steering 30 ③）。

### 2. [MEDIUM] 图层被整体替换或输出尺寸变化时，代数会撞车，surface 也没人重新 configure

**位置**：§4.1、§4.2、§5.4、§7.1。

**问题**：`setup_canvas` 会把三张画布整个换掉（`page_host.rs:147-152`）。它在 `resumed` 和 `WindowEvent::ScaleFactorChanged`（`winit_app.rs:58`，窗口在 Retina 和非 Retina 屏之间拖动时触发）里调用。设计里的 `gen` 是每个实例自己从 0 开始数的：

- 新的 `LayerCanvas::new()` 写一次就是 `gen = 1`，可能正好等于 GPU 记住的旧值，于是不重传。旧纹理尺寸不对，但 uniform 用的是新尺寸，GPU 会**悄悄**画错。
- 新的 `PageCanvas` 是 `gen 0 / seq 0`，GPU 记的是 `seen_seq = 57`。`ops_since(57)` 返回空迭代器还是 `None`，设计没说；返回空的话旧的 `valid` 会一直留着。
- 窗口尺寸变化时，§5.4 / §7.1 没写 GPU surface 要重新 configure。CPU 路径有 `surface.resize`（`frame_render.rs:258-261`）。macOS 的 Metal 尺寸不对时不会报 `Outdated`，只会把画面拉伸。

**修改**：

- `gen` 改成取自进程级 `static NEXT_GEN: AtomicU64`（`fetch_add(1, Relaxed)`）。`new()`、`write()`、`replace()` 每次都拿一个全局唯一的新值。GPU 侧按 `(gen, width, height)` 判断是否重传。
- `PageCanvas` 的游标改成 `(gen, seq)` 二元组：`ops_since(seen)` 在 `seen.gen != self.gen` 或 `seen.seq > self.seq` 时返回 `None`。
- `GpuCompositor::present` 在 acquire 之前比较 `(layers.width, layers.height)` 和已配置的尺寸，不同就 `surface.configure`，并重建直通纹理和离屏目标。
- 单测：两个 `LayerCanvas::new()` 的 `gen` 不相等；把 `PageCanvas` / `LayerCanvas` 换成不同尺寸后，序列对拍（第 1 条第 4 点）仍然逐字节一致。

### 3. [MEDIUM] `DirtyOp::Rect` 从浮点矩形到整数像素的取整规则没定

**位置**：§4.2、§4.3。

**问题**：损伤区是 `GeoRect`（f32，物理像素，`animation.rs:34`）。`clear_area` 向外取整（`pixels.rs:289-290`：`floor` / `ceil` 再夹到画布内），裁剪用的是浮点 `clip_rect`（`entry.rs:65`）。设计只写了「按矩形传」。实现方要是向内取整或四舍五入，就会漏传边上一列或一行，逐字节一致就破了。

**修改**：`write_rect` 先算出 `IRect { x0: floor(r.x), y0: floor(r.y), x1: ceil(r.right()), y1: ceil(r.bottom()) }`，每个分量都夹到 `[0, w] / [0, h]`，与 `clear_area` 同一个公式，抽成一个共用函数。`DirtyOp::Rect` 只存 `IRect`。单测断言：给定任意浮点矩形，`clear_area` 改到的像素集合 ⊆ `IRect`。

### 4. [MEDIUM] `Timeout` / `Occluded` 跳过的帧没人补，屏幕会停在旧画面；FFI 也没定义跳帧时的返回位

**位置**：§5.4、§9（acquire 那几行）、§8.2。

**问题**：设计写的是「本帧不上屏（屏上保留上一帧），返回 `Skipped`」。可如果被跳过的正好是某次变化的**最后**一帧，宿主已经清掉了 `needs_redraw`，之后就再也不会出帧，屏上一直是过期画面。这是 GPU 独有的问题：CPU 模式下 softbuffer 的 `present` 总会成功。桌面代码里也没有处理 `WindowEvent::Occluded`（全仓 grep 结果为 0）。SDK 这边，`Skipped` 时 `pump_ex` 返回什么没定义：返回 `NEW` 不带 `PRESENTED`，iOS 会把像素写进被隐藏的 `imageLayer`；返回 0，Android 的 `frame()` 只在有变化时续帧（`MiniProgramView.kt:166`），就会停住。

**修改**：

- 桌面：`Skipped` 时置 `present_pending = true` 并 `request_redraw()`。`about_to_wait` 把 `present_pending` 当成 `needs_redraw` 处理。新增 `WindowEvent::Occluded(false)` 分支，收到后 `request_redraw()`。
- SDK：新增返回位 `#define MR_FRAME_PENDING 8`，意思是「这次没东西可显示，下个 vsync 请再调一次 `pump_ex`」。下一次 `pump` 即使没有任何变化也要走 `compose_current()`。`NEW` / `PRESENTED` 都不置。Kotlin 改成 `if (flags != 0) post { frame() }`。
- 故障注入加 `timeout@N`（第 N 帧 acquire 当作 `Timeout`）。`gpu/tests.rs` 和 SDK 用例断言：第 N+1 帧会上屏，最终读回与 CPU 逐字节相同。

### 5. [MEDIUM] Android：`surfaceDestroyed → detach` 和「粘性回退就移除 `SurfaceView`」冲突，切一次后台 GPU 就永久没了

**位置**：§8.2（`detach` 之后是 CPU、`BACKEND_CHANGED`）、§8.4 `frame()` 那一条、§6.5「回退是粘性的」。

**问题**：进后台时 Android 会销毁 surface → `nativeDetachSurface` → 实际后端变成 CPU → 回前台后第一次 `pump_ex` 带 `BACKEND_CHANGED` → Kotlin 按 §8.4「回退是粘性的，切到 CPU 后直接移除 SurfaceView」把它删掉 → 之后不会再有 `surfaceCreated`，GPU 就回不来了。如果 `surfaceCreated` 先到，又会和移除操作抢。引擎和宿主都分不清「宿主主动卸载」和「引擎回退」。

**修改**：

- 规定 `BACKEND_CHANGED` **只**在引擎自己引起变化时置位：运行期回退，或者 `set_backend` 之后重新选择。宿主主动调的 `attach` / `detach` 不置位，因为宿主从返回值、从自己的调用里已经知道结果了。
- Kotlin：`surfaceDestroyed` 时显示 `BitmapLayer`，`SurfaceView` 留着；`surfaceCreated` 时重新 `nativeAttachSurface`；只有收到 `BACKEND_CHANGED` 且 `nativeBackend == CPU` 时才移除 `SurfaceView`。
- FFI 单测：`attach_offscreen_for_test` → `detach` → 下一次 `pump_ex` 不带 `BACKEND_CHANGED`；带 `lost@2` 时第 2 次带上。

### 6. [MEDIUM] `MINI_GPU_FAULT=lost@N` 调 `device.destroy()` 不会同步触发丢失回调；致命错误归类没定优先级

**位置**：§10 `MINI_GPU_FAULT` 一行、§5.7 最后一条、§9 设备丢失那几行、§6.4「设备丢失写缓存」。

**问题**：wgpu-core 30.0.1 的 `device_destroy`（`device/global.rs:1945-1967`）只做了 `valid.store(false)`，注释写明丢失回调要等后续 `maintain` / `poll` 发现队列空了才调。所以第 N 帧 `destroy()` 之后，`present` 开头检查 `lost` 标志时它还是 false，后面每个调用都会报 validation 错误，最后归进 `Fatal::Validation`（日志是 error、不写缓存），而不是设计期望的 `DeviceLost`。回退照样会发生，但走的分支和日志都不对，「同一键累计 2 次才拒绝」这条路径也就测不到。

**修改**：

- 注入时先 `device.destroy()`，紧接着 `device.poll(PollType::Poll)`，让回调当场触发。
- 每帧 `submit` 之后也做一次 `device.poll(PollType::Poll)`，让异步发生的丢失能被及时发现。
- 归类优先级写死：`lost` 标志 > `Validation` > `SurfaceLost`。`lost` 已置位时，随之而来的 uncaptured error 只记 `MINI_GPU_LOG`，不打 error。
- 用例断言 `lost@N` 时的原因串含「设备丢失」，而不是「校验错误」。

### 7. [MEDIUM] 启动自检：Auto 的性能结论来自单次采样，测试帧尺寸没定，直通帧可能用到随时间变化的覆盖层

**位置**：§6.3。

**问题**：

- 「取第 3 次」两边各只有一个计时样本。在边界机型上（移动端降频、调度抢占），同一台机器每次启动可能选出不同后端。A1.9 要求「本机自动选中 GPU」，也因此可能偶发失败。
- 测试帧的输出尺寸和画布宽度没写。如果用一个很小的测试帧，「GPU 主线程 < CPU 合成」比的就不是真实帧的成本。
- 「再加一帧直通」没说用哪种覆盖层。Loading 转圈按 `last_frame.elapsed()` 计算角度（`ui_overlay.rs:136`），picker 按 `anim_from.elapsed()` 计算滑入位置（`picker_sheet/mod.rs:93`）。CPU 参考帧和 GPU 输入要是分两次合成，就会因为时间不同而出现像素差，自检判成「像素不一致」并**写进判定缓存**，这台机器从此 Auto 只走 CPU。

**修改**：测试帧输出尺寸 = 宿主实际的 `(width, height)`，画布宽 = 宿主页面画布宽，画布高 2100。直通帧只放 `modal = Some(visible, pressed_button: None)`，不放 Toast / Loading / picker / 侧滑。计时先预热 2 次，再采 7 次取中位数。Auto 要求 `GPU 主线程中位数 ≤ 0.8 × CPU 中位数`（留 20% 滞回），并且 `GPU 完成中位数 < 4.0ms`。7 个样本都写进 `MINI_GPU_LOG`。

### 8. [MEDIUM] 真正上屏（swapchain）的正确性没有任何验收项

**位置**：§5.4、§17 第 2 行、A1.4、D2.x。

**问题**：A1.4 和自检都只比离屏 `Rgba8Unorm` 目标的读回。上屏走的是另一条路：`Bgra8Unorm` 的 surface、另一条管线，还有 CAMetalLayer 的色彩空间。§17 自己点出了「屏上偏色而离屏判据仍全绿」的风险，也给了「最大通道差 ≤ 2」的数，但这条**没进 A1.x**，移动端也没有对应项。steering 要求后端差异只能在像素层面，而且要用定死的阈值判。

**修改**：

- 加 **A1.10**：在 144Hz 显示器上，sample-app 15 页分别用 `--backend cpu` 和 `--backend gpu` 开窗口（`--route` 加 `--time 2`），`screencapture -l <窗口号> -o -x` 截图，`pixdiff.py --tol 2` 的差异占比必须是 0，并记下 `--tol 0` 时的最大通道差。不过的话，通过 `Surface::as_hal::<wgpu::hal::api::Metal>` 把层的 `colorspace` 设成与 softbuffer 一致，再测；判据不放宽。
- 加 **D2.6**：真机或模拟器上 GPU / CPU 各截一张（`xcrun simctl io booted screenshot`、`adb exec-out screencap -p`），同一条 `--tol 2` 判据。

### 9. [MEDIUM] iOS 静态库加了 wgpu-metal，但 Swift 包和 podspec 都没链 Metal / QuartzCore；A2.6 也测不出来

**位置**：§1「iOS 宿主…不加依赖」、§8.3、§15（没列 `Package.swift` / podspec / modulemap）、A2.6。

**问题**：`sdk/ios/Package.swift` 的 `linkerSettings` 只有 `c++` 和 `z`，`MiniRender.podspec` 也只有 `s.libraries = 'c++', 'z'`。静态库里一旦有 `MTLCreateSystemDefaultDevice` 和 `CAMetalLayer`，集成方的 App 链接时就会报 undefined symbol。A2.6 只 `xcodebuild` 了 library target，静态库 target 不做链接，所以测不出来。

**修改**：

- `sdk/ios/ffi-headers/module.modulemap` 加 `link framework "Metal"`、`link framework "QuartzCore"`、`link framework "CoreGraphics"`、`link framework "Foundation"`（`import MiniRenderFFI` 时自动链接）。`Package.swift` 加对应的 `.linkedFramework(...)`，podspec 加 `s.frameworks = 'Metal', 'QuartzCore', 'CoreGraphics'`。三个文件都写进 §15。
- 加 **A2.7**：用仓库里已有的 `examples/demo.c` 链一个可执行文件，命令是 `xcrun -sdk iphonesimulator clang -target arm64-apple-ios13.0-simulator examples/demo.c -Ltarget/aarch64-apple-ios-sim/release -lmini_render -lc++ -lz -framework Metal -framework QuartzCore -framework CoreGraphics -framework Foundation`，必须能链过。
- Android 侧：A2.2 再加一步 `llvm-readelf -d`，确认 `NEEDED` 相比改动前只多了 `libandroid.so` / `liblog.so`，没有 `libvulkan.so`（wgpu 是 dlopen 加载它的）。

### 10. [MEDIUM] SDK 的 pump 计时算不算 acquire 等待没定，D2.3 和 SDK 的 `MINI_FPS` 判据因此判不了

**位置**：§6.6 SDK `MINI_FPS` 那一条、D2.3。

**问题**：GPU 模式下 `compose_frame → present` 会在 acquire 里按 vsync 阻塞（§5.4），这段时间落在 `pump` 内部。桌面的 `work_ms` 明确不含 acquire 等待（§12.3），SDK 的「pump 最慢 / p99」却没说。如果算进去，GPU 的 pump p99 会接近一个刷新周期，D2.3「GPU 的 pump p99 ≤ 0.4 × CPU」就一定不过，而这只是测量口径造成的。

**修改**：`FrameStats` 分开记 `pump_ms`（总时长）、`acquire_ms`（包住 `get_current_texture` 的那段）和 `work_ms = pump_ms − acquire_ms`。每秒那行改成 `📊 [后端 …] N 帧/秒，工作 最慢 x p99 y ms，等 vsync p99 z ms，帧间隔峰值 w ms`。D2.3 改成比较 `work_ms` 的 p99 和最慢一帧，帧间隔峰值仍然要求 GPU ≤ CPU。

### 11. [MEDIUM] A1.5 的「第 120 帧」有两套计数口径，判不了

**位置**：§10（`lost@N` 的 N 从选定 GPU 那一帧起算）、§12.1（窗口基准先 settle 再 drive）、A1.5（「JSON 里第 120 帧起 `backend == "cpu"`」）。

**问题**：故障计数器把 settle 期间的帧也算进去，JSON 记了哪些帧却没定义。settle 0.5s × 144Hz ≈ 72 帧，所以注入可能落在 drive 的第 48 帧前后，也可能发生在 settle 期间。照字面执行，A1.5 会无缘无故失败或者无法判定。

**修改**：每帧 JSON 记录加三个字段：`phase: "settle" | "eval" | "drive"`、`gpu_frame: u32 | null`（就是故障计数器的值）、`presented: bool`，settle 帧也记。A1.5 改成：`gpu_frame < 120` 的记录全是 `gpu`；`gpu_frame == 120` 那条记录和之后所有记录都是 `cpu`，而且 `presented == true`；drive 阶段一共 300 条，全部 `presented == true`。

### 12. [MEDIUM] A1.6 的窗口判据没有「最慢一帧」，只测 S1，也没固定显示器刷新率

**位置**：A1.6 第 4 条、§12.1「节拍」。

**问题**：

- steering 规定判据用「最慢一帧和帧间隔峰值」。窗口 GPU 这项只有 `composite_interval_peak` 和合成帧 `work_ms` 的 **p99**。300 帧的 p99 能藏住 3 个长帧。
- 只测 S1，steering 给的范例（首页）对应 R1，却没有窗口判据。
- `--hz` 默认 144，但 GPU 窗口按显示器真实刷新率走 `Fifo`。如果在 120Hz 的内建 ProMotion 屏上跑，帧间隔最小也有 8.33ms，`≤ 7.2ms` 永远过不了；CPU 和 GPU 的对比也就失去意义。

**修改**：

- 窗口 S1 和 R1 都测，GPU 合成帧要满足：`composite_interval_peak ≤ 7.2ms`、合成帧 `work_ms` 最慢 ≤ 6.9ms、p99 ≤ 1.0ms。
- R1 的 GPU `composite_interval_peak` 不能高于 CPU。
- JSON 记 `display_hz`（从 winit 的 `refresh_rate_millihertz` 读）。`--hz` 默认就取这个值；如果 `|display_hz − 144| > 1`，窗口模式直接退出码 3，打「⚠️ 显示器不是 144Hz，窗口判据未测」。

### 13. [MEDIUM] P4 没有可判定的验收项

**位置**：§14 P4。

**问题**：评审要求每个阶段都有带数字的可判定验收。P4 只写了「按数据逐项立项」「做完单独标定阈值」，立项和验收都没有数字。

**修改**：给 P4 定一个模板，每个子项都要套用：

- **立项条件**：同一场景跑 5 次取中位数，`MINI_DRAW_LOG` 显示该图元占最慢帧 ≥ 30%，而且这一帧 > 6.94ms。
- **验收**：
  - 对应基准场景下，GPU 的最慢一帧比 P3 结束时至少降 30%，CPU 不退步（≤ +5%）；
  - 同帧差异 ≤「实测最大值 × 1.5，向上取整到 0.01%」（在本机加至少一台非 Apple 适配器上测），包围盒必须落在该图元的矩形内；
  - CPU 的降级行为写进 README 的取舍表，触发时打一行日志（例如 `blur` 半径 > 64px 时按 64 处理，并打一次 `⚠️ blur 半径 {r} 超出 CPU 上限 64，已截断`）。

### 14. [MEDIUM] 测试里的故障注入走进程级环境变量，和并行 `cargo test` 竞态

**位置**：§13.2「`MINI_GPU_FAULT=lost@2` 时……」、§10、`fault.rs`。

**问题**：`cargo test` 在同一个进程里多线程并行跑用例。一个用例设了 `MINI_GPU_FAULT`，别的 GPU 用例建合成器时也会读到。仓库里已经踩过同类坑：`data_dir.rs` 的测试注释写着全局状态竞态「40 轮里挂 5 次」。A1.1、A2.3「`cargo test` 全绿」会因此偶发失败。

**修改**：`fault.rs` 只负责解析。环境变量只在宿主入口（`window.rs`、`MiniEngine::new`）读一次，然后作为 `Option<FaultSpec>` 传给 `GpuCompositor::new(..)` 和 `attach_offscreen_for_test(fault: Option<FaultSpec>)`。测试一律显式传参，不调 `set_var`。

### 15. [MEDIUM] 判定缓存键里的「引擎版本」没定义；照最自然的写法，拒绝记录永远不会失效

**位置**：§6.4。

**问题**：`CARGO_PKG_VERSION` 一直是 `0.1.0`（`Cargo.toml:3`），仓库里也没有 `build.rs` 可以嵌入 git 哈希。要是拿它当引擎版本，一次因为我们自己着色器的 bug 被记下的「像素不一致」，修好之后还会一直生效：用户设备在同一驱动下永远只走 CPU，开发机上 A1.9 也会被一份旧缓存挡住。§6.4 说的「引擎版本一变就是新键」就落空了。

**修改**：

- 写死定义：`ENGINE_KEY = format!("{}+{:016x}+r{}", env!("CARGO_PKG_VERSION"), WGSL_HASH, SELFTEST_REV)`。其中 `const WGSL_HASH: u64 = fnv1a(include_str!("gpu/compose.wgsl").as_bytes())` 用 const fn 算；`const SELFTEST_REV: u32` 在 `mirror.rs` 或 `selftest.rs` 改动时手动加一，文件头写明这条规则。
- `cfg!(debug_assertions)` 的构建不写缓存。

### 16. [MEDIUM] 脚本要 grep 的日志串前后不一致，有的根本没定义

**位置**：§6.5、§6.6、§7.2、§13.3 第 6~7 步、A0.3、A1.5。

**问题**：

- §13.3 第 7 步要 grep `运行中回退`，但 §6.5 定义的回退行是 `⚠️ GPU 回退 CPU：<原因>`，里面没有「运行中」三个字。
- 第 6 步 grep 的 `合成模式 分层` / `合成模式 直通`，§6.6 只写了「每帧的合成模式（`分层` / `直通：Modal`）」，没给前缀。
- A0.3 期望 `渲染后端：CPU — 未编译 GPU 支持`，但 §6.5 只给了 GPU 那一行的格式。

脚本靠这些串判空转，串对不上，判据要么误报，要么空转。

**修改**：在 §6.6 加一张表，列出所有被脚本依赖的日志行，并在 `diag.rs` 里做成格式化函数，加单测：

| 用途 | 精确格式 |
|---|---|
| 选定 GPU | `🎨 渲染后端：GPU（{Metal\|Vulkan\|Dx12\|Gl} · {适配器名}）— {自动选择\|强制}` |
| 选定 CPU | `🎨 渲染后端：CPU — {原因}` |
| 启动期回退 | `⚠️ GPU 回退 CPU（启动）：{原因}` |
| 运行中回退 | `⚠️ GPU 回退 CPU（运行中，第 {n} 帧）：{原因}` |
| 合成模式（`MINI_GPU_LOG`） | `🧩 合成模式 分层 上传 {bytes}B` / `🧩 合成模式 直通：{Modal\|Toast\|Loading\|picker\|侧滑}` |
| 镜像补传 | `🧩 镜像补传 [{a},{b})` |

`gpu-parity.sh` 只 grep 这几个固定前缀。

### 17. [NIT] `rgba_for_host` 不一定是「屏上那一帧」

**位置**：§7.3、§16 第 10 条。

`rgba_stale` 时重新合成用的是**调用时**的时钟。SDK 传的是 `Instant::now()`（`engine.rs:586`），Loading 转圈和 picker 滑入都按墙上时钟画，所以这时的结果和 GPU 已经上屏的那一帧会有差别。**修改**：文档改成「画布内容一致；随时间变化的覆盖层（Loading、picker 动画）可能差一个相位」。A2.3 的用例本来就只比首帧，不受影响。

### 18. [NIT] iOS 的「内存 < 2 GiB 走 CPU」这条规则不会触发

**位置**：§6.2。

`Package.swift` 的最低版本是 `.iOS(.v13)`，而能装 iOS 13 的 iPhone 内存都不低于 2GB。也就是说 iOS 上实际没有分级，只剩自检的性能比较。**修改**：明确写「iOS 不按机型分级，由 §6.3 自检决定」，删掉这条规则；或者换成可判定的条件，例如宿主传入 `MTLDevice.supportsFamily(.apple4)`。

### 19. [NIT] 报告里的「Vulkan ≥ 1.1」门槛被去掉了，§16 没记

**位置**：§6.2、§16。

**修改**：在 §16 补一条：Android API ≥ 29 已经覆盖了 Vulkan 1.1 的要求（Android 10 要求 64 位设备支持 Vulkan 1.1），所以不再单独检查。如果要更严，就加 `adapter.get_downlevel_capabilities().is_webgpu_compliant()` 的检查。

### 20. [NIT] `mr_app_attach_surface` 在 iOS 主线程上同步建设备并跑自检，耗时没有上限也没有记录要求

**位置**：§8.2、§8.3。

**修改**：A2.3 / D2.1 记录 `attach` 的耗时（`MINI_GPU_LOG` 已经有分段）。D2.1 加一条：`attach` ≤ 150ms（真机实测后再定死）；超过就把自检挪到第一次 `pump_ex`，并在文档里写明。

### 21. [NIT] D2.4 的体积检查其实不需要真机，却排在 P2.3 之后

**位置**：§14 P2.3、D2.4。

`build-mobile.sh` 一旦默认带上 gpu，`release-mobile.yml` 发出去的包就会变大，而体积门槛要等真机阶段才查。**修改**：把 D2.4 挪成 A2.8（本机 `llvm-strip` 之后比较 `.so` / `.a` 的大小），并要求 A2.8 通过之后才能合入 P2.3。

### 22. [NIT] wgpu 的传递依赖只靠 Cargo.lock 锁定

**位置**：§1。

wgpu 30.0.1 对 `wgpu-core` / `wgpu-hal` / `wgpu-types` / `naga` 写的是 `version = "30.0.0"`（插入号范围），`=30.0.1` 只锁住了顶层。**修改**：§1 写明 `Cargo.lock` 已经入库（`git ls-files Cargo.lock` 能查到），合入时的锁定版本是 wgpu-core / wgpu-hal / wgpu-types / naga 30.0.1，`cargo update` 要单独提交。

### 23. [NIT] A1.6 里 `compose_p99 ≤ 0.30ms` 这个绝对值没写来源

**位置**：A1.6 第 1 条。

**修改**：注明它的估算依据（编码 + 提交 1 次绘制、零上传），或者改成只用相对判据 `≤ 0.5 × CPU`，在 P0 基线（A0.6）出来之后再补绝对值。

---

## 已核实的假设

- **后端越界：没有发现。** 合成器只拿到 `FrameLayers` 的只读引用；CPU 光栅、`plan_frame` / 损伤区（`frame_render.rs:73-110`）、命中和事件都不经过合成器。GPU 模式关掉的节拍器只做 sleep、自旋和 `request_redraw`（`winit_app.rs:255-290`），不推动动画时钟。动画时钟走墙上时间，或者由 `set_animation_time` 固定（`engine.rs:627`、`headless_run.rs:103-104`）。`capture_viewport` 继续只用 CPU。
- 合成序列确实有四处：`frame_render.rs:264`、`headless_run.rs:206`、`edge_back_host.rs:49`、`engine.rs:568`（另外三处的 `render_ui_overlay` / `picker_sheet::render` 调用也都核对过）。SDK 没有下拉指示器和侧滑返回这两层。
- `present_to_buffer` 的参数、整数混合公式、`has_tabbar` 但画布缺失时的行为，以及「只写 `min(tabbar_px, tabbar_h)` 行」，都与 §2.2、§5.2 一致（`render.rs:35-152`）。
- `Color` 是 `#[repr(C)]`、4 个 u8（`color.rs:5-9`）。`VIEWPORT_CULL_MARGIN_PX = 400.0`（`wxml_renderer/mod.rs:139`），2× 和 3× 屏都是 3 层的计算成立。
- 页面画布只在三处被写或被替换：`render_with_damage` 的 `clear_area` / `clear_band` 加重画、内容高度变化后重新分配（`frame_render.rs:119-155`），以及 `setup_canvas`（`page_host.rs:147-152`）。SDK 是 `render_page_band` 和重新分配（`engine.rs:444-456, 525-528`）。损伤区重画带 `canvas.clip_rect(rect)`（`entry.rs:62-66`）。
- 现有 C ABI 正好 20 个 `mr_app_*`；头文件对 `pump` 的承诺是「返回 1 表示有新画面」（`mini_render.h:142-143`），所以新增 `pump_ex` 而不改 `pump` 的理由成立。仓库没有 `build.rs`（`Cargo.toml` 里有一个没用上的 `cbindgen` 构建依赖），头文件由 `build-mobile.sh:106-110` 拷进 XCFramework。`release-mobile.yml` 调用的就是 `build-mobile.sh`。
- `engine.rs` 659 行、`ffi_app.rs` 338 行，必须先拆。
- `MINI_FAKE_NOW` 已经存在，各脚本里都是 `export MINI_FAKE_NOW="${MINI_FAKE_NOW:-2024-03-15}"`。`tab-click-check` 是 4+3+5 = 12 项；`damage-check` 5 个用例；sample 15 页、news 6 页、画廊 65 张（`doc/引擎测试说明.md:55-62`）。`data_dir::subdir` 存在。`clean-target.sh` 对 `baseline_*` 留白名单。
- 窗口 `with_resizable(false)`，但 `ScaleFactorChanged` 会重建画布（这是第 2 条的依据）。
- wgpu 30.0.1：feature 列表里有 `std` / `wgsl` / `metal` / `vulkan` / `gles` / `dx12`，`build.rs` 用 `cfg_aliases` 按目标门控（`metal` 只在 `target_vendor = "apple"` 生效，`dx12` 只在 Windows 生效）。`Instance::enabled_backend_features()` 是 const fn，没有后端时 `Instance::new` 会走到 `unreachable!`。`CurrentSurfaceTexture` 的 7 个变体与设计一致。`set_device_lost_callback`、`on_uncaptured_error`、`Device::destroy`、`Device::poll(PollType)`、`Surface::as_hal` 都存在。`SurfaceTargetUnsafe::CoreAnimationLayer(*mut c_void)` 存在，wgpu-hal 会断言它是 `CAMetalLayer`（`metal/surface.rs:47`）。`Limits::downlevel_defaults()` 的 2D 纹理上限是 2048，`downlevel_webgl2_defaults()` 也存在。`desired_maximum_frame_latency` 存在。
- `raw-window-handle` 0.6.2 已经在 `Cargo.lock` 里，`AndroidNdkWindowHandle::new(NonNull<c_void>)` 存在。softbuffer 0.4.8 在 macOS 上用 `addSublayer` 挂层（`cg.rs:173`）。
- Android 的 `minSdk = 24`；`goBack()` 里有可以照搬的同步等待写法；当前的 `frame()` 只在 `changed` 时续帧（这是第 4 条的依据）。

## 没有核实或核实为错的假设

- **错**：§4.3 / §11 说「`gpu-parity.sh` 用静态页、5 个增量用例和交互用例逐字节守住增量镜像」，以及「GPU 下 `tab-click-check` 12/12 证明后端无关」。实际上无头只合成最后一帧，`tab-click-check` 也不涉及合成器（第 1 条）。
- **错**：§10 说「`lost@N` 走真实的设备丢失回调」。`destroy()` 只把设备标记为无效，回调要等之后的 poll（第 6 条）。
- **错**：§1 / §8.3 说「iOS 宿主不加依赖」。Metal 和 QuartzCore 都没有链接（第 9 条）。
- **错**：§7.3 / §16-10 说 `rgba_for_host`「就是屏上那一帧」。随时间变化的覆盖层会差一个相位（第 17 条）。
- **错（无影响）**：§8.4 说 `ffi_jni.rs`「现约 230 行」，实际是 202 行。
- **不成立**：§6.2 的 iOS 2GiB 规则，在 iOS 13+ 上不会触发（第 18 条）。
- **未验证（设计已经自认）**：同一窗口从 wgpu 切到 softbuffer；CAMetalLayer 与 softbuffer 的色彩空间是否一致；`Fifo` 能否消除 macOS 上那 30~50ms 的上屏停顿；移动端耗时与体积增量；Android 驱动会不会导致进程级崩溃。
- **未验证（只看了代码和规范）**：`+0.25` 偏置在所有驱动上都能让 float→unorm 的转换落到同一个整数（按规范推导成立，但只有 A1.4、A1.10 和第 1 条的逐帧校验能实际证实）；`Rgba8Uint` 2D array 在 GLES 强制路径上的可用性；macOS 上 drop 掉 wgpu surface 之后，它的 CAMetalLayer 子层会不会盖在新建的 softbuffer 子层上面（设计放在 P1.1 探针里，可以接受）。

## 判定

HIGH 1 + MEDIUM 15 > 0，**CHANGES_REQUESTED**。
