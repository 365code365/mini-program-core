# G8 WebGL 设计（2026-10-27）

用户 2026-10-27 要求「一次性把剩余的做完，不要再反复确认」：D-2 按 06 的推荐定为 **A（ANGLE）**，下面的取舍是在这个前提下按现有条件做的，
做不到 A 的平台写明退路。

## 1. 决策

| 编号 | 定为 | 依据 |
|---|---|---|
| D-2 | A：ANGLE 的 WebGL 兼容上下文（`EGL_ANGLE_create_context_webgl_compatibility`），校验与 GLSL 翻译交给 ANGLE | 探针（§6）：macOS 上 Metal 与 SwiftShader 两个后端都建得出、越界 `drawArrays` 得 `INVALID_OPERATION` |
| D-2 平台退路 | Android 若加载不到 ANGLE，退到系统 EGL/GLES（方案 B 的子集）：开 robust access 防越界读，校验只做内存安全那几条（§4） | 构建 Android 版 ANGLE 要 gn + 整套 Chromium 依赖（数 GB），本机放不下；系统 GLES 每台机器都有 |
| D-2b | ANGLE + SwiftShader（Vulkan）做软件兜底与逐字节基线 | 与合成后端无关：CPU 合成时 WebGL 照样可用 |
| 上屏 | 无头 / 桌面：pbuffer + 帧末读回主画布后备缓冲；Android：EGL 窗口 surface 直接建在 SurfaceView 上，`eglSwapBuffers`（§5） | 读回在真机上就是瓶颈（1270×2770 每帧 32ms），直接上屏后 60fps |
| WebGPU `wx.getGPU` | 返回 GPU 对象，`requestAdapter()` resolve `null`（WebGPU 标准里「没有可用适配器」的写法），游戏回退 WebGL | 映射到 wgpu 是另一个数周的项目；W 级，写进验收 |

## 2. 结构

```text
src/game/webgl/
  mod.rs        WebGl：画布 id → 上下文；当前上下文切换；帧末 present
  egl.rs        加载 libEGL（进程级一次），选后端，建 display / config / context / pbuffer
  ctx.rs        GlCtx：一个 WebGL 上下文（EGL 句柄、glow 函数表、属性、错误、WebGL 自有状态）
  natives.rs    __game_gl(id, op, ...) 分派；op 名字表由 Rust 交给 JS（一份来源）
  ops_*.rs      按领域分：状态 / 缓冲与绘制 / 纹理 / 着色器 / 帧缓冲 / WebGL2
  pixels.rs     texImage2D 的来源转换（FLIP_Y / PREMULTIPLY / 格式），读回与上屏换算
src/js/prelude/game-webgl.js   WebGLRenderingContext / WebGL2RenderingContext、对象包装、常量、扩展
```

- **对象句柄**：JS 包装对象的 `__id` 就是 GL 名字（不可写属性）。归属与已删除由 JS 判（别的上下文的对象、已删的对象 → `INVALID_OPERATION`），
  原生不再维护映射表。`FinalizationRegistry` 回收没人引用的对象。
- **错误**：自己生成的错误（JS 侧的归属检查、丢失上下文）与 `glGetError` 合并，`getError` 先报自己的。
- **WebGL 独有的 pixelStorei**（`UNPACK_FLIP_Y_WEBGL`、`UNPACK_PREMULTIPLY_ALPHA_WEBGL`、`UNPACK_COLORSPACE_CONVERSION_WEBGL`）不下发 GL，由 pixels.rs 处理。
- **扩展**：ANGLE 的 WebGL 上下文默认关着扩展，`getExtension` 时 `glRequestExtensionANGLE`；WebGL 名字到 GL 名字的映射表在 ctx.rs。

## 3. 上下文属性与默认帧缓冲

- `alpha / depth / stencil / antialias` 选 EGL config（`ALPHA_SIZE`、`DEPTH_SIZE`、`STENCIL_SIZE`、`SAMPLES=4`），选不到逐项放宽；`getContextAttributes` 报实际值。
- `premultipliedAlpha`（默认 true）：读回时按预乘还原成后备缓冲的直通 alpha，合成结果 = 预乘色压黑底。
- `preserveDrawingBuffer`（默认 false）：帧末读回之后标「待清」，下一次在默认帧缓冲上画 / 清 / 读之前按 WebGL 规则清一次（临时复位清屏状态再还原）。
- 画布改尺寸 → 重建 pbuffer（上下文与资源保留），视口不自动改（与浏览器一致）。

## 4. 不是 ANGLE 时（Android 系统 GLES）

- 上下文带 `EGL_CONTEXT_OPENGL_ROBUST_ACCESS_EXT`（驱动支持时）：越界读拿 0，不会读到别处内存。
- `drawArrays` / `drawElements`：按每个启用属性的缓冲尺寸、步长、偏移算可用顶点数；`drawElements` 扫索引（按缓冲 + 偏移 + 个数缓存最大值，`bufferData/SubData` 作废）。越界 → `INVALID_OPERATION`，不下发。
- 着色器不做 WebGL 的 GLSL 限制检查（驱动能编的都放过），写进验收。

## 5. 上屏（window.rs）

- 读回那一版实测（一加 PLY110）：1270×2770 每帧 32ms、427×931 每帧 8.6ms，都过不了 60fps；
- 现在：主画布是 WebGL、宿主给了 ANativeWindow、这块窗口没被 CPU 锁过 → 窗口 surface 当默认帧缓冲，帧末 swap；
- 覆盖层（Toast / Modal / 按钮）弹着：覆盖层黑底、白底各画一遍求出预乘 RGBA，辅助上下文先把游戏这一帧拷进纹理，
  再把覆盖层混合上去、swap（不碰游戏上下文的 GL 状态；游戏不动、覆盖层在动的帧用存下的那一帧）；
- 引擎内旋转、窗口刚接上的第一帧、窗口被 CPU 锁过：仍然读回 + CPU 合成，辅助上下文把整帧画上去。

## 6. 探针（examples/tmp_probe_angle，已删）

| 后端 | 结果 |
|---|---|
| ANGLE Metal（comfy-angle 0.1.1 的 macOS arm64 动态库，取自 Electron） | EGL 1.5，初始化 45ms，`OpenGL ES 3.0 (ANGLE 2.1.27037)`，WebGL 兼容扩展有，越界 drawArrays → 0x502 |
| ANGLE + SwiftShader（Chrome 自带的 `libvk_swiftshader.dylib`） | 初始化 1.0s，`OpenGL ES 3.1`，同上 |

库来源：`tools/fetch-angle.sh` 从 PyPI 拉 comfy-angle（BSD-3，sha256 写死），SwiftShader（Apache-2.0）从本机 Chrome 拷；目录 `third_party/angle/<平台>/`，不入库。
