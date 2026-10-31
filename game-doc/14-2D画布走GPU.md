# 14 2D 画布走 GPU（`MINI_2D`）

## 选哪个后端

| 取值（`MINI_2D` 压过创建选项 `canvas2d`） | 哪些画布走 GPU |
|---|---|
| `auto`（默认） | iOS / Android：**主画布**；**离屏画布**在有纹理共享组时也走（iOS 的 ANGLE 有，Android 系统驱动没有 → 离屏留在 CPU）。桌面、无头（固定时钟）、引擎内旋转：全 CPU |
| `gpu` | 所有 2D 画布 |
| `cpu` | 全部 CPU（参考实现，永远可用） |

日志：`🎨 2D 画布后端 gpu（angle，纹理共享组有）：auto，主画布与离屏画布`。开着 `MINI_GAME_SLOW_MS` 时每 2 秒一行
`🎨 GPU 2D 120 帧：draw call …（提交 …ms/帧、顶点 …MB/帧）、临时层 …MB、裁剪蒙版 …、读回 …、直拷 …`。回退写 `ℹ️ 2D 画布后端 cpu（GPU 建不了：…）` / `⚠️ GPU 2D 画布回退 CPU：…`。

## 做法（`src/game/canvas/gpu2d/`）

GPU 2D 不另写一份光栅器。每块 GPU 画布是一张 FBO 纹理（RGBA8、预乘），在单独的 EGL 上下文里画。这个上下文与 WebGL
用同一个 display：iOS / macOS 是 ANGLE-Metal，Android 是 ANGLE 或系统 GLES，无头是 SwiftShader。

| 调用 | GPU 上怎么画 |
|---|---|
| `fillRect`（纯色、轴对齐） | 每个矩形一个四边形，片元里按 CPU 抗锯齿矩形的算法算覆盖率（横向按重叠长度，纵向数 4 条子扫描线），连续的合成一个 draw call |
| `clearRect`（无裁剪） | 整数像素框写 0 |
| `fillRect`（线性 / 径向 / 锥形渐变，≤ 8 个色标，轴对齐） | 同一个四边形，颜色在片元里按 `DeviceSource::sample` 同一个式子算（种类 4）：整屏渐变底不再每帧在 CPU 上逐像素插值、传一整张 |
| `drawImage`（无阴影） | 纹理四边形。图片按解码结果缓存成纹理；源是 CPU 画布时按内容代数（`Slot::gen`）缓存；源是 GPU 画布时直接采它的 FBO。源矩形内缩半个纹素夹住，精灵图集的相邻格子不会渗进来。矩阵有旋转 / 斜切时四条边抗锯齿（见下） |
| 路径 fill / stroke、fillText / strokeText、strokeRect、旋转的 fillRect、图案 / 阴影 | CPU 照原样画进一张透明临时层（裁剪到保守外接框外扩 2px），画到的那块当纹理传上去，按合成模式的混合函数叠。源是渐变（无阴影）时 CPU 只画白色覆盖率，渐变在片元里乘（种类 5） |
| WebGL `texImage2D / texSubImage2D(GPU 2D 画布)`（RGBA8、0 级，有纹理共享组） | GPU 上直接拷进 WebGL 那张纹理（`share.rs`），按 FLIP_Y 翻、按 PREMULTIPLY 去不去预乘；画布内容代数没变就不拷 |
| `getImageData`、`toDataURL`、`putImageData`、CPU 画布的 drawImage 源、没有纹理共享组时的 WebGL 纹理源、multiply / screen 这类要读目标色的合成 | 读回 CPU 后备缓冲，在 CPU 上做 |

**旋转 drawImage 的边缘抗锯齿**（CPU 与 GPU 同一个式子）：像素中心到四条边的设备距离 d，覆盖率 = Π clamp(d + 0.5, 0, 1)。
GPU 把四边各外扩 1 像素，每个顶点带上到四条边的距离（仿射下线性插值是精确的）。90° 整数位置时内侧 d ≥ 0.5、外侧
d ≤ -0.5，结果与不做抗锯齿逐字节相同（单测 `旋转的边抗锯齿_九十度不变`）；轴对齐的缩放 / 平移不做，既有基线不变。

**纹理共享组**（`EGL_ANGLE_display_texture_share_group`）：2D 与 WebGL 的上下文都带这个属性建，纹理名在它们之间通用，
2D 的上下文把 WebGL 的纹理挂到自己的帧缓冲上画进去，然后 glFlush。同一线程、同一 display，ANGLE 是一条命令队列，
flush 之后 WebGL 看到的就是新内容。texSubImage2D 只在纹理 0 级尺寸已知、整块落在里面时走这里（越界要报
INVALID_VALUE，交给 CPU 那条）。随包的 ANGLE（iOS、macOS）有这个扩展；Android 用系统 GLES 驱动，没有，走读回。

临时层为什么等价：source-over 有结合律，所以「画进透明层再叠」等于「直接画在目标上」。其余能用混合函数表达的
Porter-Duff 模式，临时层按 source-over 画出「源 × 覆盖率」，GPU 再用该模式的系数叠上去，与 `fx_composite.rs`
的式子相同。裁剪：CPU 的做法是「拍快照，画完按蒙版插值」。对不清形状外像素的模式，这个结果对源是线性的，等于
「源 × 蒙版」，所以 GPU 采样裁剪蒙版乘上去。清形状外像素的模式（copy、source-in 等）带裁剪时走 CPU。

**哪边像素是新的**（`Auth`）：GPU 调用之前，如果 CPU 那边新，整张传上去；CPU 调用或读像素之前，如果 GPU 那边新，
整张读回。稳态只走 GPU 那几条调用时，两边不来回搬（测试判「读回次数」）。

**上屏**：主画布纹理铺满 EGL 窗口 surface，翻 y、压黑底（与 CPU 的 `to_rgba_opaque` 同一个约定）。覆盖层
（Toast / Modal，预乘）叠在上面，然后 swap。与 WebGL 直接上屏共用宿主约定（`webgl_present_window`，iOS 的
`mr_game_present_layer`、Android 的 `nativeBlit`），宿主不用改。

**回退**：

| 情况 | 结果 |
|---|---|
| 建不了上下文 / 编不过着色器 | 这个实例全 CPU |
| 窗口建不了 surface、被 CPU 锁过、引擎内旋转 | 主画布先读回，再降回 CPU |
| make current 失败（上下文丢了） | 所有 GPU 画布降回 CPU，没读回的内容丢失，日志写明 |

与 WebGL 共用 EGL：线程上记最后是谁 make current 的（`egl::owner`）。两边各自缓存「我是 current」，被对方切走后
重新 make current。WebGL 读 GPU 2D 画布当纹理时，拷完 / 读回之后切回自己的上下文（`ops_texture.rs`）。

**别为帧末收尾切上下文**：ANGLE 的 Metal 后端切走一个上下文时要等它的命令缓冲排进队列。第一版帧末无条件切到 2D
上下文 flush 一次，主画布是 WebGL 时每帧多切两次，那一下等待落进 WebGL 的 swap：iPhone 上 present 0.1 → 6ms，
发热后 10~14ms，farm001 掉到 50fps（40 秒 203 次掉帧）。现在帧末只在 2D 上下文正好是 current 时 flush，
用 2D 像素的每条路（上屏、拷给 WebGL、读回）都会先 flush，攒着的顶点不会丢。

## 正确性判据（CPU 参考 vs GPU，`tests/game_cases/canvas_gpu.rs`）

同一份 JS 在两个后端各跑一次，比整帧输出：

| 场景 | 差 > 2 级的像素 | 最大通道差 | 上限 |
|---|---|---|---|
| rects（300 个小数坐标矩形、半透明、clearRect、负宽高） | 0 | 2 | 0.01% / 4 |
| paths（圆、描边、虚线贝塞尔、旋转矩形、圆角连接） | 0.14% | 6 | 0.2% / 8 |
| text（对齐、描边、旋转、maxWidth 压缩、基线） | 0 | 1 | 0.01% / 4 |
| fx（线性 / 径向渐变、阴影、7 种合成模式、离屏 source-in / copy） | 0 | 1 | 0.01% / 4 |
| images（缩放、半透明、旋转、不平滑、裁剪内画图与字） | 0.28% | 8 | 0.4% / 12 |
| mixed（GPU 与回退交替：getImageData、multiply、putImageData、toDataURL） | 0 | 1 | 0.01% / 4 |
| grad（整屏线性底、半透明横向条、锥形、径向、渐变描边 / strokeRect / 文字、11 个色标、旋转） | 0 | 1 | 0.2% / 8 |
| g-bench-sprites 30 帧 | 0 | 0 | 0.1% / 40 |

images 的差都在旋转图片的轮廓与双线性插值上（两边都抗锯齿之后，最大差从 204 降到 8）。
另外判：rects 300 个矩形 ≤ 6 个 draw call、只在取帧时读回 1 次；精灵稳态 0 个临时层、读回 ≤ 2 次；grad 临时层 ≤ 7 块
（整屏底、横向条、径向方块不走临时层）；WebGL `texImage2D / texSubImage2D(GPU 2D 画布)` + FLIP_Y 像素对、
有纹理共享组时 3 次直拷 0 次读回、画布没变再传不拷。

## 真机数字（iPhone 16 Pro Max，2026-10-31，主画布按物理像素 1320×2868，`?dpr=1`）

| 场景 | 后端 | pump 中位 | present 中位 | 合计 | fps |
|---|---|---|---|---|---|
| g-bench-sprites n=500 | cpu | 2.2 | 5.7 | 7.9ms | 60 |
| | gpu | 3.5 | 0.4 | 3.9ms | 60 |
| g-bench-sprites n=2000 | cpu | 4.2 | 5.8 | 10.0ms | 60 |
| | gpu | 5.9 | 0.3 | 6.2ms | 60 |
| g-bench-rects n=2000（小数坐标 + 每帧一行字） | cpu | 6.5 | 6.3 | 12.8ms | 60 |
| | gpu | 7.2 | 0.2 | 7.4ms | 60 |
| g-bench-sprites n=500，逻辑尺寸主画布 | cpu | 2.9 | 2.9 | 5.8ms | 60 |
| | auto（主画布 GPU） | 3.1 | 0.3 | 3.4ms | 60 |

GPU 快在上屏这一步：CPU 每帧要把整张后备缓冲转成 BGRA 交给 CALayer，物理分辨率下约 6ms，GPU 只是一次 blit
加 swap。代价是每次调用的原生开销：drawImage 要算四角、查纹理、攒顶点，比 CPU 的 32×32 小块拷贝贵，
2000 个精灵一帧多约 1ms（顶点改成带索引的 4 个、17 个浮点数之后数字不变）。

**WebGL 主画布 + 离屏 2D 画布的游戏**（auto，离屏走 GPU、纹理共享组直拷；同一时段背靠背各 30~40 秒）：

| 游戏 | 后端 | fps 中位 / 最低 | 掉帧 | pump 中位 | present 中位 | 2D 画布整张上传（CPU） |
|---|---|---|---|---|---|---|
| farm001（Cocos，标签都画在 2D 画布上） | cpu | 60 / 60 | 0 | 16.6 | 0.1 | 187 次 |
| | auto | 60 / 60 | 0 | 6.1 | 0.1 | 0 |
| play-3d-island（UI 画布 1320×2868） | cpu | 59.2 / 58.5 | 21 | 15.6 | 0.4 | 12 次 |
| | auto | 59.0 / 58.5 | 19 | 9.2 | 0.4 | 0 |
| play-flappy | cpu → auto | 60 / 59.5 → 60 / 60 | 1 → 0 | 16.2 → 0.9 | | 26 → 0 |
| play-3d-runner | cpu → auto | 60 / 59 → 60 / 60 | 1 → 0 | 16.4 → 5.1 | | 9 → 0 |
| play-breakout / play-2048 | cpu → auto | 60 → 60 | 0 → 0 | 1.8 → 1.0 / 1.9 → 0.9 | | 9 → 0 / 46 → 0 |

pump 在 cpu 档的 16ms 里大半是 ANGLE 在第一个 GL 调用上等上一帧的 drawable（见 game-doc/13），所以 pump 降下来
不等于每帧真的省了这么多；硬数字是「2D 画布整张传给 WebGL」从每 30~40 秒几十上百次（萌宠小岛一次 11~14ms，
2D 画布 1448 / 2172 行）变成 0。萌宠小岛的掉帧都在自动游玩切场景的那几帧（慢帧 19 个，两档一样多），
稳态两档都贴 60。farm001 同一游戏不同时段 cpu 档的 pump 在 5.9~16.6 之间变（玩到的场景不同），按同一时段比。

## Android 真机（一加 PLY110，60Hz，1240×2772 物理分辨率，`tools/android-game-jank.sh`，屏上 0 次重复）

| 场景 | 后端 | 引擎工作中位（逻辑 + rAF / 上屏拷贝） | p99 |
|---|---|---|---|
| g-bench-sprites n=500 | cpu | 10.0（4.2 / 5.7） | 12.6 |
| | gpu | 6.4（4.0 / 0） | 8.0 |
| g-bench-sprites n=2000 | cpu | 12.0（7.3 / 4.7） | 13.8 |
| | gpu | 11.0（9.2 / 0） | 14.9 |
| g-bench-rects n=2000 | cpu | 11.8（7.4 / 4.4） | 13.4 |
| | gpu | 11.7（10.0 / 0） | 16.2 |

调用多的场景 gpu 档的「逻辑 + rAF」反而多 2~3ms，量过不是 2D 调用本身：慢帧归因里 2000 个 fillRect 的 2D 耗时
cpu 4.2ms、gpu 2.8ms，GPU 批次提交 0.25ms/帧。差在 CPU 频率：gpu 档时游戏线程所在的大核（cpu7）被调频压在
633MHz，cpu 档同一位置 2.3~2.7GHz（CPU 上屏的整帧拷贝把频率拉了上去，GPU 档 CPU 闲了）。整帧合计 gpu 档不慢，
500 个精灵快 36%。Android auto：主画布走 GPU；离屏画布因为系统驱动没有纹理共享组，留在 CPU。

## 取舍与没做的

- 图案（createPattern）填充仍在 CPU 上算（临时层）；超过 8 个色标的渐变、带阴影的渐变也走临时层。
- Android 系统驱动没有纹理共享组，离屏 2D 画布当 WebGL 纹理时要读回，所以 Android auto 不把离屏画布放到 GPU。
  Android 包里带 ANGLE（`libEGL_angle.so`）的话自动用上。
- `putImageData` 走 CPU，之后的第一笔 GPU 调用要整张传上去。每帧 putImageData 的游戏应当选 `cpu`。
