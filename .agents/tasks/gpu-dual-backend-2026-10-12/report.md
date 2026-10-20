# CPU / GPU 双渲染后端：交付报告

开发在本地 worktree `.worktrees/gpu-backend`（分支 `feat/gpu-backend`，基于 `09b8fb8`）里做，已 rebase 并快进合并进 `main`；worktree 与分支已删除，代码就在 `src/compositor/`（CPU 与 GPU 同一个模块目录）。机器：Apple M3 Pro，macOS，内建 ProMotion 屏 120Hz。设计：`design.md` + `design-v2-addendum.md`（23 条评审意见全部接受）。

## 1. 最终架构与接缝

接缝放在**合成层**（`src/compositor/`）。布局、命中、事件、`setData` 增量失效、动画时钟、光栅化都不动。合成器只拿到 `FrameLayers` 的只读引用，所以「后端只管画」由类型保证，不靠约定。

- `cpu::compose`：参考实现，由原来四个宿主调用点各抄一份的合成序列收拢而来（纯搬迁，md5 证明等价）。
- `layer::{PageCanvas, LayerCanvas}`：写画布必须声明写了哪块，并留下只读的写入日志，GPU 据此只上传脏区。
- `gpu::GpuCompositor`（feature `gpu`）：
  - 页面画布按环形条带纹理镜像（只镜像视口 ± 余量，显存有上界）；tabBar 和 fixed 层按代数上传。
  - 整数着色器复刻 CPU 合成：`Rgba8Uint` 纹理 + `textureLoad`，不开硬件混合，输出 +0.25 偏置。
  - Toast / Modal / picker / 侧滑帧走直通：CPU 合成整帧后上传。
- 宿主接线：
  - 交互窗体：Fifo 交换链上屏，失败当帧回退到 softbuffer。
  - 无头：常驻离屏合成器，每帧都合成；快照取 GPU 读回。
  - SDK：C ABI / JNI 接口已备好，但实际后端仍是 CPU（见 §8）。

## 2. 选型与依赖

- `wgpu = "=30.0.1"`：`default-features = false`，只开 `std / wgsl / metal / vulkan / gles / dx12`，不开 webgpu。
- `pollster = "=1.0.1"`，用于同步等待 future。
- 传递依赖由入库的 `Cargo.lock` 锁定。加 wgpu 时，`wasm-bindgen / js-sys / web-sys` 被 cargo 从 0.2.126 / 0.3.103 小版本升到了 0.2.129 / 0.3.106，这几个都只在 wasm 目标上用到。
- 为什么不用 vello / skia / femtovg：它们自带抗锯齿和字体栈，「与 CPU 同帧差异 ≤ 阈值」这条判据会失去判定力。

## 3. 开关与自动选择、回退

| 开关 | 作用 |
|---|---|
| `--backend` / `MINI_BACKEND=auto\|cpu\|gpu` | 交互窗体默认 auto；无头与所有回归脚本默认 cpu；SDK 默认 cpu |
| `--backend-probe` | 真的建设备并跑自检。0 = GPU 可用，3 = 不可用，4 = 没编 gpu |
| `MINI_GPU_LOG` / `MINI_GPU_VERIFY` / `MINI_GPU_FAULT` | 详细日志 / 逐帧对拍（不一致退出码 5）/ 故障注入（`init`、`selftest`、`tint`、`lost@N`、`surface-lost@N`、`timeout@N`） |
| `MINI_FPS=1` | 每行带后端标签，例如 `📊 [后端 GPU · Metal · Apple M3 Pro]` |

**Auto 的判定**：
- 适配器门槛：软件适配器一律不算；不用 GLES；查黑名单；查判定缓存。
- 启动自检：用一张固定测试帧逐字节比对 GPU 与 CPU 输出；再比稳态成本，要求 GPU 主线程时间 ≤ 1ms，且 GPU 完成 < 4ms（原来是「≤ 0.8 × CPU 合成」，在窗口启动时把 M3 Pro 判掉了，已改，理由见 selftest.rs）。本机实测中位数：GPU 主线程 0.23ms，GPU 完成 0.97ms，CPU 0.70ms。

**回退**：初始化失败、自检不一致、设备丢失、surface 无法恢复、校验错误，一律当帧回退 CPU，并且粘性生效（本会话不再切回 GPU）。日志使用固定前缀（`diag.rs`）。

## 4. 各阶段验收结果

| 阶段 | 内容 | 结果 |
|---|---|---|
| P0 | 纯搬迁 ×5 + 后端选择模块 + C ABI / JNI + 基准 | 每一步快照 md5 21/21、damage 5/5、画廊无变化、tab 全过、sdk-parity 15/15 和 6/6、交互 7/7 |
| P3a-1 | 只有滚动偏移变化时复用布局树（像素逐字节不变） | 见 §5 A/B；同一套全部判据通过 |
| P1 | 桌面 GPU 合成器 | gpu-parity **34/34 逐字节一致**；窗口 `lost@120` / `surface-lost@120` 都在第 120 帧当帧回退，回退前全是 GPU 帧、回退后全是 CPU 帧，300 个 drive 帧全部上屏 |
| P2 | 移动端 GPU 上屏 | **未完成**（见 §8）。Android 三个 ABI 带 gpu feature 的 `cargo check` 通过；iOS 本机没有 SDK，未验证 |

## 5. 性能数字（基准 `examples/backend_bench`，5 次中位数，只统计 drive 段）

**CPU 侧优化：滚动不再重建布局**（同一时段 A/B，2 轮 × 3 次）

| 场景 | 改动前 最慢一帧 / p99 / p50 | 改动后 最慢一帧 / p99 / p50 |
|---|---|---|
| S1 300 卡片列表 2× | 46.7~80.9 / 40 / 0.21 ms | 2.0~3.7 / 1.5 / 0.22 ms |
| R1 sample 首页 2× | 28.4~29.9 / 26 / 17.9 | 13.1~13.3 / 11 / 3.4 |
| R1 3× | 59~78 / 40~47 / 24.5 | 23~27 / 20~21 / 6.6 |

**CPU 对比 GPU**（窗口模式，真实上屏；最慢一帧 / p99 / p50，单位 ms）

| 场景 | CPU | GPU |
|---|---|---|
| S1 列表滚动 | 8.80 / 6.95 / 3.06 | **3.42 / 2.45 / 0.37** |
| S6 嵌套 scroll-view | 4.76 / 3.55 / 3.27 | **1.27 / 1.20 / 1.00** |
| S5 Modal 弹着 | 8.00 / 7.64 / 3.47 | **2.99 / 2.93 / 1.69** |
| S3 阴影 + 大图 | 9.19 / 7.64 / 2.96 | 7.07 / 5.62 / 0.32 |
| R1 首页 | 17.80 / 13.61 / 6.10 | 13.24 / 12.11 / 3.90 |
| S2 大面积动画 | 50.76 / 40.10 / 19.49 | 20.81 / 18.00 / 15.53 |
| S4 整屏重绘 | 14.44 / 12.14 / 3.05 | 13.12 / 10.51 / 0.64 |

- GPU 快在上屏：主线程每帧只剩约 0.4ms 的编码和提交。
- 光栅主导的帧（动画子树离屏重画、跨条带整带重画）两边差不多，要靠 P3 图层化才能解决。
- 无头模式下 GPU 更慢（S1 2× 的 p50 是 1.09 对 0.22），这是口径造成的：GPU 每帧都 `poll(Wait)` 等完成，而 CPU 无头不上屏。无头的数字只用来看「GPU 完成」时间，p99 在 1.5~3.9ms。
- 完整表格（含无头 2× / 3×）在 `doc/引擎测试说明.md`「五之三」。

## 6. GPU / CPU 差异阈值与测法

阈值是 **0 个差异像素**（所有帧，分层与直通都一样），不需要按驱动标定百分比。测法是 `bash tools/gpu-parity.sh`，覆盖以下内容：

- 静态页 21 张：CPU 与 GPU 各出一张，逐字节比较。
- 增量重绘 5 个用例 × 32 帧、拖动和 Modal 5 个场景（93~394 帧）：在同一次运行里逐帧读回对拍。
- 交互路径 7/7；tabBar 点击全过。
- 防空转：分层和直通两种模式都要出现过；注入 tint 必须被检出。
- 回退 3/3。

单元测试另有：50 组随机图层配置对拍；200 步增量序列对拍；tint 检出；`lost@2` 必须报设备丢失。

## 7. 七项回归（最终提交，默认构建与 `--features gpu` 构建各一遍）

| 项 | 默认构建 | 带 gpu 构建 |
|---|---|---|
| build | 0 warning | 0 warning |
| cargo test | 567 过 | 575 过 |
| 快照 md5（对比 `baseline_cpu_before`） | 21/21 | 21/21 |
| damage-check | 5/5 | 5/5 |
| gallery | 无变化 | 无变化 |
| tab-click | 全过 | 全过 |
| sdk-parity | 15/15、6/6 | 15/15、6/6 |
| interaction-check | 7/7 | 7/7 |

**CPU 基线逐字节一致**：所有提交都与改动前 main 生成的 `target/baseline_cpu_before`（21 张整帧，外加画廊 65 张）md5 相同。

**已知噪声**：interaction-check 的 `picker_confirm` 偶发 0.251% 的差异，包围盒 x[617,698] y[465,548]，恰好是 showcase 页的 GIF 那一格（按墙上时钟取帧）。单独连跑 12 次都一致。

## 8. 拆分的文件（≤ 500 行）

- `host/engine.rs`：659 行拆成 `engine` / `engine_frame` / `engine_ui`（纯搬迁）。
- 新模块全部在 500 行以内：
  - `compositor/`：mod 253、layer 257、pref 321、select 300、verdict 301、diag 215、fault 130
  - `compositor/gpu/`：mod 406、mirror 232、present 213、device 177、pipeline 203、selftest 198、readback 69、tests 220
- 桌面宿主：`gpu_host` 296、`bench_run` 466、`bench_drive` 301、`pacer`、`backend_host`、`fps_report`。

## 9. 未完成与「未验证」

- **SDK 的 GPU 上屏（P2）没做**：`mr_app_attach_surface` 目前只记下 surface 种类，原因串是「SDK 的 GPU 上屏还没接入」。还要做的：
  - 引擎侧：`compose_frame` 的 GPU 分支，以及 `rgba_for_host`。
  - CAMetalLayer / ANativeWindow 的 surface。
  - JNI 的 `ANativeWindow_fromSurface`。
  - Swift 的 CAMetalLayer 路径、Kotlin 的 SurfaceView + Choreographer。
  - iOS 链接 Metal / QuartzCore。
- **真机矩阵**（D2.x）、Android 黑名单、内置 View 默认改成 `.auto`：需要设备。
- **P3a 第二项**（只补画新露出的条带）、嵌套 scroll-view 的离屏缓存、动画子树缓存：没做。R1 剩下的 11ms p99 就在这里。
- **未验证**：
  - 窗口截图色彩比对（A1.10）。surface 已显式设为 `SurfaceColorSpace::Srgb`，但还没截屏比对。
  - 144Hz 显示器上的窗口判据（本机是 120Hz，基准按 R12 报「未测」）。
  - iOS 交叉编译。
  - GPU 能否消除 macOS 上偶发的 30~50ms 上屏停顿。
- **设计偏离**：
  - R2（tea-app 分类页）在 375×667 下内容恰好一屏、拖不动，换成了夹具 S6。
  - R1 要先 eval 关掉新人券浮层，否则是对着遮罩空拖。
  - 跨两次运行比 PNG 的拖动类判据，改成在同一次运行里逐帧对拍（带墙上时钟的场景两次运行本来就不一样）。
