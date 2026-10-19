# 设计 v2 修订（回应 design-review.json 的 23 条意见 + 范围调整）

> 基础文档：`design.md`（v1）。本文件**覆盖** v1 中与之冲突的条文；没提到的条文照 v1 执行。
> 23 条意见全部接受，按评审给出的修法落地，下面逐条写成可执行的约定。实现与评审都以「v1 + 本修订」为准。

## 一、对 23 条意见的落地约定

### HIGH

**R1 增量镜像必须被逐帧比对（对应 #1）**
- 无头 `--backend gpu` 时，整个无头会话建**一个常驻** `GpuCompositor`（离屏），`pump_one_frame` / 每次 `render` 之后都调用 `compose_offscreen(&layers)`，不是只在最后快照那一帧才建、才合成。这样 `MirrorRows` 的增量回放在每一帧都真实发生。
- 新开关 `MINI_GPU_VERIFY=1`：每一帧 GPU 合成后读回，与同一份 `FrameLayers` 的 `cpu::compose` 结果逐字节比较；不一致打 `❌ GPU 校验不一致 帧#<n> @x,y CPU #rrggbb / GPU #rrggbb`，进程以**退出码 5** 结束。结束时打 `GPU 逐帧校验：N 帧全部一致`（N 为校验帧数）。
- `gpu-parity.sh` 加连续帧检查，全部在 `MINI_GPU_VERIFY=1` 下跑：damage-check 的 5 个 `--eval` 用例各 `--frames 30`；R1（sample 首页）与 news 首页 `--drag 8x120`；S1 夹具 `--drag 8x300`；`interaction-check` 全套。**防空转**：每个用例从日志解析 N，要求 N ≥ 该用例请求帧数的 90%，否则判失败。
- `gpu/tests.rs` 增加 200 步随机序列对拍：同一个 `GpuCompositor` 连续经历「整带写 / 矩形写 / 滚动 / replace / 跳过若干帧不合成」，每一步与 `cpu::compose` 逐字节比较。
- SDK：`attach_offscreen_for_test` 之后重跑 `src/tests/engine_input_tests.rs` 的全部用例（参数化一个后端维度），每次 pump 后校验。

### MEDIUM

**R2 代数与尺寸变化（#2）**：`gen` 取自进程级 `static NEXT_GEN: AtomicU64`（`fetch_add`），任何新建 / `replace` 都拿新值，不会撞车。GPU 侧游标为 `(gen, seq)`；`ops_since((gen, seq))` 在 gen 不匹配时返回 `None`。`present` / `compose_offscreen` 发现输出尺寸变化就重新 configure surface 并重建直通 / 离屏纹理。单测覆盖：replace 后旧游标拿到 None；两个 `PageCanvas` 的 gen 不相等。

**R3 浮点矩形取整（#3）**：从现有 `clear_area` 的取整代码里抽出共用函数 `fn pixel_rect(r: Rect, w: u32, h: u32) -> IRect`（floor 起点、ceil 终点、夹到画布内），`clear_area` 与 `PageCanvas::write_rect` 都调它；`DirtyOp::Rect` 只存 `IRect`。单测断言：`clear_area` 改动过的像素集合 ⊆ `pixel_rect` 的结果。抽函数这一步算纯搬迁（md5 证明）。

**R4 跳过的帧要补（#4）**：桌面 `present` 返回 `Skipped` 时置 `present_pending = true` 并 `request_redraw()`；处理 `WindowEvent::Occluded(false)` 时也置位并重绘。SDK 新增返回位 `MR_FRAME_PENDING = 8`：本次没上屏，下一次 `pump_ex` 强制 `compose_current`（即使没有新内容）。故障注入新增 `timeout@N`（第 N 帧 acquire 当作 `Timeout`），用例断言第 N+1 帧上屏、内容与 CPU 一致。

**R5 Android detach 与回退分开（#5）**：`MR_FRAME_BACKEND_CHANGED` **只在引擎自己引起变化时**置位（回退、`set_backend` 后重新选择）；宿主主动 `attach` / `detach` 不置位（宿主自己知道）。回退的粘性只针对「引擎判定 GPU 不可用」；`detach` 后再 `attach` 照常尝试 GPU。Kotlin：`surfaceDestroyed` 只把显示切到 `BitmapLayer`、不移除 `SurfaceView`；`surfaceCreated` 时重新 `nativeAttachSurface`。FFI 单测：attach → detach → attach，第二次 attach 仍返回 2（有 GPU 时）且没有 `BACKEND_CHANGED`。

**R6 设备丢失要同步可见（#6）**：`lost@N` 调 `device.destroy()` 后**立刻** `device.poll(PollType::Poll)`；每帧 `submit` 之后也 `poll(Poll)`，让丢失回调及时触发。致命错误归类优先级：`DeviceLost > Validation > SurfaceLost`（同一帧同时出现时取最高者）。用例断言原因串包含「设备丢失」。

**R7 自检口径（#7）**：测试帧尺寸 = 宿主实际输出尺寸（画布高仍取 `max(2100, 视口高 + 1024)` 以跨环形层边界）；直通帧只放 Modal（不放随时间变化的 Toast / Loading / 指示器相位）。预热 2 次，再采样 7 次取中位数；Auto 通过条件：GPU 主线程时间（编码+提交）**≤ 0.8 ×** CPU 合成时间，且 GPU 完成时间 **< 4.0ms**。7 个样本全部写进 `MINI_GPU_LOG`（选定行里给中位数）。

**R8 真实上屏的色彩（#8）**：新增验收 **A1.10**：`screencapture -l <窗口号>` 分别截 CPU 与 GPU 窗口（同页、`--time 2`、静止），`pixdiff.py --tol 2` 差异占比必须为 0；不过就通过 `Surface::as_hal` 设置 `CAMetalLayer.colorspace` 与 softbuffer 一致后重测，结论写进文档。本机若无屏幕录制权限导致截不到图，如实写「未验证」并给出手动复现命令。新增 **D2.6**（真机/模拟器截图，同一判据，用户执行）。

**R9 iOS 链接 Metal / QuartzCore（#9）**：`module.modulemap` 加 `link framework "Metal"`、`link framework "QuartzCore"`；`Package.swift` 加 `.linkedFramework("Metal")`、`.linkedFramework("QuartzCore")`；podspec 加 `s.frameworks = ...`。新增 **A2.7**：用 `examples/demo.c`（或等价最小 C 程序）链出 iOS 模拟器可执行文件，链接成功即过。**A2.2** 追加：Android `.so` 用 `readelf -d`（或 `llvm-readelf -d`）检查 `NEEDED` 里有 `libandroid.so`、`liblog.so`，且没有意外的新依赖。

**R10 SDK 计时口径（#10）**：`FrameStats` 分开记 `pump_ms`（整个 pump）、`acquire_ms`（等 surface，仅 GPU）、`work_ms = pump_ms - acquire_ms`。D2.3 比较 `work_ms` 的 p99 与最慢一帧；帧间隔峰值要求 GPU ≤ CPU。

**R11 帧号口径统一（#11）**：`--bench` 的每帧 JSON 记录增加 `phase`（`settle` / `eval` / `drive`）、`gpu_frame`（从选定 GPU 那一帧起计，与 `MINI_GPU_FAULT` 的 N 同一个计数器；CPU 时为 null）、`presented`（bool）。A1.5 改写为：「`gpu_frame == 120` 那一帧起 `backend == "cpu"`；之前全部 `"gpu"`；所有 `phase == "drive"` 的帧 `presented == true`」。

**R12 窗口性能判据（#12）**：A1.6 窗口部分改为 S1 与 R1 都测（2×）：GPU 合成帧（`rerastered == false`）最慢 ≤ 6.9ms、`work_ms` p99 ≤ 1.0ms、`composite_interval_peak ≤ 7.2ms`；R1 的 GPU `interval_peak` ≤ CPU 同项。JSON 记录 `display_hz`（从 winit monitor 的 refresh rate 取）；不是 144Hz 时编排端打印「⚠️ 显示器 {hz}Hz，不是 144Hz，窗口判据未测」并以退出码 3 结束该模式（不算通过）。

**R13 P4 立项与验收模板（#13）**：立项条件：基准里该图元在最慢帧的 `MINI_DRAW_LOG` 归因占比 ≥ 30%，且该帧 > 6.94ms。验收模板：该场景最慢一帧降 ≥ 30%；CPU 后端同场景不退步超过 +5%；GPU/CPU 差异阈值 = 实测最大差异占比 × 1.5（向上取整到 0.01%），包围盒必须落在该图元矩形内；CPU 降级行为写进 README 并在首次触发时打一次日志。

**R14 故障注入不走进程级环境变量（#14）**：`MINI_GPU_FAULT` / `MINI_GPU_VERIFY` / `MINI_BACKEND` 只在宿主入口（`window.rs` 的 main、`MiniEngine::new`）读一次，解析成 `Option<FaultSpec>` 等值，经构造参数传给 `GpuCompositor::new` / `attach_offscreen_for_test`。测试一律显式传参，**禁止**在测试里 `std::env::set_var`。

**R15 判定缓存键（#15）**：`ENGINE_KEY = format!("{}+{:016x}+{}", CARGO_PKG_VERSION, WGSL_HASH, SELFTEST_REV)`，其中 `WGSL_HASH` 是 `compose.wgsl` 源码（`include_str!`）的 `const fn` FNV-1a 64 位哈希，`SELFTEST_REV` 是 `selftest.rs` 里手写的常量（改自检逻辑时 +1）。`cfg!(debug_assertions)` 时不写缓存（只读也跳过）。

**R16 日志格式固定（#16）**：所有被脚本 grep 的日志行，在 `diag.rs` 里做成格式化函数并配单测，脚本只 grep 下表的**固定前缀**：

| 事件 | 固定前缀（脚本 grep 这个） | 完整示例 |
|---|---|---|
| 选定 GPU | `🎨 渲染后端：GPU` | `🎨 渲染后端：GPU（Metal · Apple M3 Pro）— 自动选择` |
| 选定 CPU | `🎨 渲染后端：CPU` | `🎨 渲染后端：CPU — 未编译 GPU 支持` |
| 启动期回退 | `⚠️ GPU 回退 CPU：` | `⚠️ GPU 回退 CPU：自检像素不一致（…）` |
| 运行中回退 | `⚠️ GPU 运行中回退 CPU：` | `⚠️ GPU 运行中回退 CPU：设备丢失（帧#120）` |
| 合成模式（`MINI_GPU_LOG`） | `🧩 合成模式 分层` / `🧩 合成模式 直通` | `🧩 合成模式 直通：Modal，上传 5.9MB` |
| 逐帧校验汇总 | `GPU 逐帧校验：` | `GPU 逐帧校验：300 帧全部一致` |
| 逐帧校验失败 | `❌ GPU 校验不一致` | 见 R1 |
| MINI_FPS 行 | `📊 [后端 ` | `📊 [后端 GPU · Metal · Apple M3 Pro] 144 帧/秒 …` |

### NIT

- **#17**：`rgba_for_host` 的文档改成「画布内容一致；随时间变化的覆盖层（Toast/Loading 动画相位）可能差一个相位」。
- **#18**：iOS 去掉「内存 < 2GiB 走 CPU」这条（iOS 13+ 设备不会触发），改为「iOS 由自检决定」，`select.rs` 不再有 iOS 内存规则。
- **#19**：§16 补一条：去掉报告里的 Vulkan ≥ 1.1 门槛，因为 Auto 已要求 Android API ≥ 29（Android 10 起要求 64 位设备支持 Vulkan 1.1）；另在 `adapter_gate` 里检查 downlevel 能力，`DownlevelFlags` 缺少我们用到的项时走 CPU。
- **#20**：记录 `attach` 耗时进 `MINI_GPU_LOG`；初定上限 150ms，超过就把自检推迟到第一次 `pump_ex`（真机实测后再定，文档写「未验证」）。
- **#21**：体积检查 D2.4 挪成 **A2.8**（本机测）：每个 ABI 的 stripped `.so` / 静态库体积增量 ≤ 8MB，作为 P2.3 合入前提；超了停下来报告，不自行放宽。
- **#22**：§1 补一句：`Cargo.lock` 已入库，wgpu 的传递依赖由它锁定；`cargo update` 只能单独提交。
- **#23**：A1.6 去掉 `compose_p99 ≤ 0.30ms` 这个绝对值，只保留相对判据（GPU `compose_p99 ≤ 0.5 ×` CPU）；P0 基线出来后若要补绝对值，写明来源。

## 二、范围调整：把 CPU 侧优化纳入本次（新增阶段 P3a）

v1 §18 把图层化（P3）整段移出了范围。但用户的原始要求是「大大提升渲染效率」并「优化代码」，可行性报告也实测出：最慢的几帧是 CPU 重新光栅化造成的，GPU 合成器解决不了。所以把 P3 里**不依赖 GPU、可逐字节证明**的两项纳入本次，作为 **P3a**，放在 P1 之后、P2 之前：

1. **跨条带只补画新露出的条带**：滚动出已绘制条带时，不再整带重画，而是把仍然有效的行在画布内平移 / 复用，只对新露出的那一段做带裁剪的重画（与 damage 路径同一套 `clip_rect` 机制）。`PageCanvas` 对这次写入声明 `Band`（新露出段）与必要的平移操作（新增 `DirtyOp::Shift` 或等价机制，让 GPU 镜像跟得上；GPU 侧最简单的正确做法是把平移当成整段失效）。
2. **嵌套 scroll-view 走离屏缓存**：子节点路径（`draw_child_with_interaction`）也用 `ScrollViewCache`，提升条件按 v1 P3：「绘制顺序里后面没有与它重叠的兄弟节点」；不满足就维持现状。

两项共用一个新开关 `MINI_NO_LAYERS=1`（关掉这两项优化，退回现有路径），写进 README 的诊断开关表。

**验收（P3a）**：
- **A3a.1 逐字节**：在 sample、news、tea-app 的抽样页（至少 sample 首页、news 首页、tea-app 分类页）上，`--drag 8x120` 结束时的快照、以及 `MINI_GPU_VERIFY` 风格的逐帧对拍（默认 vs `MINI_NO_LAYERS=1`，逐帧 RGBA 逐字节相同）。新增 `tools/layers-check.sh` 承担这项比较，带防空转守卫（日志里必须出现新路径被走到的标记，次数 ≥ 1）。如果某一项**无法**做到逐字节（例如 scroll-view 缓存的 blit 与直接绘制存在舍入差），必须给出 `pixdiff` 的差异占比与包围盒，并停下来在报告里说明，不能悄悄放宽。
- **A3a.2 性能**（基准 5 次中位数，CPU 后端）：S1 与 R1 2× 的最慢一帧 ≤ 6.9ms（基线约 17~18ms），3× ≤ 8.3ms（基线 24.1ms）；R2（tea-app 分类页）2× p50 ≤ 1.5ms（基线 5.5ms）、最慢 ≤ 6.9ms。达不到的如实报数字与归因，不改判据。
- **A3a.3**：七项回归全过，CPU 后端所有 PNG 与 `target/baseline_cpu_before` 逐字节一致。

v1 §14 的 P3 其余部分（动画子树内容缓存、GPU 变换图层）仍不在本次范围。

## 三、执行顺序

P0（测量与接缝，CPU-only）→ P1（桌面 GPU 合成器）→ P3a（CPU 侧优化）→ P2（移动端 GPU 代码 + 交叉编译，真机项标「未验证」）→ 收尾（报告、rebase、ff-only 合并）。

每个阶段结束都要：七项回归全过；CPU 后端与 `target/baseline_cpu_before` 逐字节一致；跑 `bash tools/clean-target.sh`（保留 `release` 与 `target/baseline_*`）；中文提交信息分阶段提交。

## 四、基线说明

`target/baseline_cpu_before/` 已在未改动的 main（`09b8fb8`）上生成：`fin2_sample/`、`fin2_news/`（`snapshot-all.sh … --settle 0 --time 2`）、`gallery/` 及 `manifest.md5`（86 条）。v1 §14 里提到的 `baseline_main_ia`（`interaction-check` 基线）还没生成，P0 的第一件事就是在**任何代码改动之前**补上：`bash tools/interaction-check.sh target/baseline_main_ia`。
