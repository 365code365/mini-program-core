# 14 2D 画布走 GPU（`MINI_2D`）

## 选哪个后端

| 取值（`MINI_2D` 压过创建选项 `canvas2d`） | 哪些画布走 GPU |
|---|---|
| `auto`（默认） | iOS / Android：**主画布**。离屏画布仍走 CPU（它们多半当 WebGL 纹理源或 drawImage 源，CPU 那份正好直接用）。桌面、无头（固定时钟）、引擎内旋转：全 CPU |
| `gpu` | 所有 2D 画布 |
| `cpu` | 全部 CPU（参考实现，永远可用） |

日志：`🎨 2D 画布后端 gpu（angle）：主画布`。回退写 `ℹ️ 2D 画布后端 cpu（GPU 建不了：…）` / `⚠️ GPU 2D 画布回退 CPU：…`。

## 做法（`src/game/canvas/gpu2d/`）

GPU 2D 不另写一份光栅器。每块 GPU 画布是一张 FBO 纹理（RGBA8、预乘），在单独的 EGL 上下文里画。这个上下文与 WebGL
用同一个 display：iOS / macOS 是 ANGLE-Metal，Android 是 ANGLE 或系统 GLES，无头是 SwiftShader。

| 调用 | GPU 上怎么画 |
|---|---|
| `fillRect`（纯色、轴对齐） | 每个矩形一个四边形，片元里按 CPU 抗锯齿矩形的算法算覆盖率（横向按重叠长度，纵向数 4 条子扫描线），连续的合成一个 draw call |
| `clearRect`（无裁剪） | 整数像素框写 0 |
| `drawImage`（无阴影） | 纹理四边形。图片按解码结果缓存成纹理；源是 CPU 画布时按内容代数（`Slot::gen`）缓存；源是 GPU 画布时直接采它的 FBO。源矩形内缩半个纹素夹住，精灵图集的相邻格子不会渗进来 |
| 路径 fill / stroke、fillText / strokeText、strokeRect、旋转的 fillRect、渐变 / 图案 / 阴影 | CPU 照原样画进一张透明临时层（裁剪到保守外接框外扩 2px），画到的那块当纹理传上去，按合成模式的混合函数叠 |
| `getImageData`、`toDataURL`、`putImageData`、当 WebGL 纹理源 / CPU 画布的 drawImage 源、multiply / screen 这类要读目标色的合成 | 读回 CPU 后备缓冲，在 CPU 上做 |

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
重新 make current。WebGL 读 GPU 2D 画布当纹理时，读回之后切回自己的上下文（`ops_texture.rs`）。

## 正确性判据（CPU 参考 vs GPU，`tests/game_cases/canvas_gpu.rs`）

同一份 JS 在两个后端各跑一次，比整帧输出：

| 场景 | 差 > 2 级的像素 | 最大通道差 | 上限 |
|---|---|---|---|
| rects（300 个小数坐标矩形、半透明、clearRect、负宽高） | 0 | 2 | 0.01% / 4 |
| paths（圆、描边、虚线贝塞尔、旋转矩形、圆角连接） | 0.14% | 6 | 0.2% / 8 |
| text（对齐、描边、旋转、maxWidth 压缩、基线） | 0 | 1 | 0.01% / 4 |
| fx（线性 / 径向渐变、阴影、7 种合成模式、离屏 source-in / copy） | 0 | 1 | 0.01% / 4 |
| images（缩放、半透明、旋转、不平滑、裁剪内画图与字） | 0.15% | 204 | 0.2% / 255 |
| mixed（GPU 与回退交替：getImageData、multiply、putImageData、toDataURL） | 0 | 1 | 0.01% / 4 |
| g-bench-sprites 30 帧 | 0 | 0 | 0.1% / 40 |

images 的最大差 204 只出现在旋转的 drawImage 轮廓上：CPU 的边缘有抗锯齿，GPU 的四边形边缘没有（取舍，见下）。
另外判：rects 300 个矩形 ≤ 6 个 draw call、只在取帧时读回 1 次；精灵稳态 0 个临时层、读回 ≤ 2 次；
WebGL `texImage2D(GPU 2D 画布)` 像素对。

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
加 swap。代价是每次调用的原生开销：drawImage 要算四角、查纹理、攒 78 个浮点数，比 CPU 的 32×32 小块拷贝贵。
2000 个精灵一帧多约 1ms。WebGL 主画布的游戏（萌宠小岛等）在 auto 下只有离屏 2D 画布，它们仍走 CPU，数字不变
（同一包背靠背：59.5 / 58.0fps，最长间隔 58ms）。

## 取舍与没做的

- 旋转 / 斜切的 drawImage 边缘不抗锯齿（GPU 四边形的硬边）。要做就把它改走临时层，代价是每次一块 CPU 光栅 + 上传。
- 渐变、图案还在 CPU 上算（临时层），整屏渐变底每帧要传一整张。可以改成着色器里算。
- 离屏 2D 画布当 WebGL 纹理时，WebGL 与 2D 不在同一个 share group，GPU 画布要先读回再传。auto 因此让离屏画布留在 CPU。
- `putImageData` 走 CPU，之后的第一笔 GPU 调用要整张传上去。每帧 putImageData 的游戏应当选 `cpu`。
