//! 嵌入宿主用的 C ABI（Flutter 插件、原生 View、任何自己有窗口的宿主）。
//!
//! **只增不改**：`ffi_app.rs` / `ffi_backend.rs` 里的老函数签名与语义一个没动，
//! 不调这些新函数的宿主行为与从前逐字节一致（`tools/sdk-parity.sh` 守着）。
//!
//! | 函数 | 解决什么 |
//! |---|---|
//! | `mr_app_create_ex` | 系统信息 / 安全区 / 宿主 API / 原生组件要在 `app.js` 执行前就位 |
//! | `mr_app_resize` | 转屏、分屏、外层约束变化（从前尺寸创建后定死） |
//! | `mr_app_set_safe_area` | 刘海 / 底部横条：`wx.getSystemInfoSync` 与 `env(safe-area-inset-*)` 的真值 |
//! | `mr_app_pixels_into` | 带行跨度、RGBA / BGRA 直写平台缓冲（省一次整帧拷贝；Apple 只认 BGRA） |
//! | `mr_app_poll_event` | 引擎 → 宿主：页面、导航栏、软键盘、原生组件摆放、hostCall、日志 |
//! | `mr_app_host_reply` | 宿主实现的 `wx.*` 回结果 |
//! | `mr_app_dispatch_event` | 宿主推事件给 `wx.onXxx`（传感器、定位、蓝牙、网络、主题……） |
//! | `mr_app_native_view_event` | 宿主承载的原生组件（video / map…）回传事件 |
//! | `mr_video_frame_rgba` / `mr_video_frame_yuv` | 同层渲染的视频：宿主解出的帧交给引擎画进页面（任何线程） |
//!
//! `mr_app_pump_ex` 的返回位多了两位：`MR_FRAME_WANTS_MORE`（下一个 vsync 还要 pump）
//! 与 `MR_FRAME_EVENTS`（队列里有事件）。
//!
//! 线程约定与其它 `mr_app_*` 相同：**同一条线程**调用。
use crate::ffi_app::{cstr, MRApp};
use crate::host::engine_backend;
use crate::host::{EngineOptions, MiniEngine, PixelFormat, SafeAreaInsets};
use crate::compositor::HostEnv;
use std::ffi::{c_char, c_int, CString};

/// `mr_app_pump_ex` 的返回位：引擎还有活要干，下一个 vsync 再 pump 一次
pub const MR_FRAME_WANTS_MORE: c_int = engine_backend::FRAME_WANTS_MORE as c_int;
/// `mr_app_pump_ex` 的返回位：事件队列非空（`mr_app_poll_event` 取到 NULL 为止）
pub const MR_FRAME_EVENTS: c_int = engine_backend::FRAME_EVENTS as c_int;
/// `mr_app_pixels_into` 的格式：内存序 R G B A
pub const MR_PIXEL_RGBA8: c_int = 0;
/// `mr_app_pixels_into` 的格式：内存序 B G R A（iOS / macOS 的 32BGRA）
pub const MR_PIXEL_BGRA8: c_int = 1;

fn app_mut<'a>(app: *mut MRApp) -> Option<&'a mut MRApp> {
    // SAFETY: 非空时是 mr_app_create(_ex) 交出去、还没 destroy 的句柄（调用方的约定）
    unsafe { app.as_mut() }
}

/// 带选项创建。`options_json` 可为 NULL（等于 `mr_app_create`）。失败返回 NULL。
///
/// ```json
/// {
///   "systemInfo": {"platform": "android", "model": "Pixel 8", "language": "zh_CN"},
///   "safeArea": {"top": 24, "right": 0, "bottom": 16, "left": 0},
///   "hostApis": ["chooseImage", "getLocation", "setClipboardData"],
///   "nativeComponents": ["video", "map", "web-view"],
///   "menuButton": true
/// }
/// ```
///
/// `menuButton`：`navigationStyle: custom` 的页面上由引擎画右上角胶囊（位置 =
/// `wx.getMenuButtonBoundingClientRect()`），点击发 `menuButton` 事件。默认关。
#[no_mangle]
pub extern "C" fn mr_app_create_ex(
    app_dir: *const c_char,
    data_dir: *const c_char,
    width: u32,
    height: u32,
    dpr: f32,
    options_json: *const c_char,
) -> *mut MRApp {
    let Some(dir) = cstr(app_dir) else { return std::ptr::null_mut() };
    let data = cstr(data_dir);
    let opts = match EngineOptions::from_json(&cstr(options_json).unwrap_or_default()) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("❌ mr_app_create_ex 选项错误: {e}");
            return std::ptr::null_mut();
        }
    };
    let env = HostEnv::from_process();
    match MiniEngine::new_with_options(&dir, data.as_deref(), width, height, dpr, env, opts) {
        Ok(engine) => Box::into_raw(Box::new(MRApp::new(engine))),
        Err(e) => {
            eprintln!("❌ mr_app_create_ex 失败: {e}");
            std::ptr::null_mut()
        }
    }
}

/// 改尺寸（逻辑像素）与像素比。变了返回 1（并已出好新尺寸的一帧，宿主照常取像素），
/// 没变返回 0。
#[no_mangle]
pub extern "C" fn mr_app_resize(app: *mut MRApp, width: u32, height: u32, dpr: f32) -> c_int {
    app_mut(app).map(|a| a.engine.resize(width, height, dpr) as c_int).unwrap_or(0)
}

/// 安全区（逻辑像素）。变了返回 1。
#[no_mangle]
pub extern "C" fn mr_app_set_safe_area(app: *mut MRApp, top: f32, right: f32, bottom: f32, left: f32) -> c_int {
    let insets = SafeAreaInsets { top: top.max(0.0), right: right.max(0.0), bottom: bottom.max(0.0), left: left.max(0.0) };
    app_mut(app).map(|a| a.engine.set_safe_area(insets) as c_int).unwrap_or(0)
}

/// 把当前帧写进宿主缓冲。`stride` 是每行字节数（≥ 宽 × 4），`format` 是 `MR_PIXEL_*`。
///
/// 返回写到的最远字节数（`stride × (高 − 1) + 宽 × 4`）；参数不对（缓冲太短、stride 太小、
/// 格式不认识、还没出过帧）返回 0 且一个字节都不写。
#[no_mangle]
pub extern "C" fn mr_app_pixels_into(app: *mut MRApp, dst: *mut u8, len: usize, stride: u32, format: c_int) -> usize {
    let Some(a) = app_mut(app) else { return 0 };
    let Some(fmt) = PixelFormat::from_code(format) else {
        a.last_error = format!("mr_app_pixels_into: format={format} 不认识（MR_PIXEL_RGBA8/BGRA8）");
        return 0;
    };
    if dst.is_null() || len == 0 {
        return 0;
    }
    // SAFETY: 调用方保证 dst 指向至少 len 字节的可写内存
    let buf = unsafe { std::slice::from_raw_parts_mut(dst, len) };
    a.engine.write_pixels(buf, stride as usize, fmt)
}

/// 取一条事件（UTF-8 JSON），没有了返回 NULL。指针活到下一次对同一 app 调用 `mr_app_*`。
#[no_mangle]
pub extern "C" fn mr_app_poll_event(app: *mut MRApp) -> *const c_char {
    let Some(a) = app_mut(app) else { return std::ptr::null() };
    let Some(ev) = a.engine.poll_event() else { return std::ptr::null() };
    // JSON 里的 NUL 只可能来自页面数据，去掉而不是丢掉整条
    a.event_buf = CString::new(ev.replace('\0', "")).unwrap_or_default();
    a.event_buf.as_ptr()
}

/// 回 `hostCall` 事件：`ok` 非 0 走 success，否则 fail；`result_json` 是交给回调的 res
/// （对象；NULL 等于 `{}`）。id 不认识（重复回、页面已经换了）返回 0。
#[no_mangle]
pub extern "C" fn mr_app_host_reply(app: *mut MRApp, id: u32, ok: c_int, result_json: *const c_char) -> c_int {
    let Some(a) = app_mut(app) else { return 0 };
    let res = cstr(result_json).unwrap_or_else(|| "{}".into());
    a.engine.host_reply(id, ok != 0, &res) as c_int
}

/// 宿主承载的原生组件发生了事件：`key` 是 `nativeViews` 里的 `key`，`name` 是事件名
/// （`ended` / `timeupdate` / `markertap`…，派发给节点上的 `bind<name>`），
/// `detail_json` 是事件的 `detail`（NULL 等于 `{}`）。找到绑定返回 1。
#[no_mangle]
pub extern "C" fn mr_app_native_view_event(
    app: *mut MRApp,
    key: *const c_char,
    name: *const c_char,
    detail_json: *const c_char,
) -> c_int {
    let Some(a) = app_mut(app) else { return 0 };
    let (Some(key), Some(name)) = (cstr(key), cstr(name)) else { return 0 };
    let detail = cstr(detail_json)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    a.engine.dispatch_native_view_event(&key, &name, detail) as c_int
}

/// 同层渲染的 `<video>`（创建选项 `sameLayer` 含 `video`）：宿主解出的一帧，RGBA、`stride` 字节一行。
/// `key` 取自 `videos` 事件。**任何线程都可以调**（不碰引擎实例，帧存在进程级的表里）；
/// 调完让引擎线程出一帧（Android 插件：`requestFrame`），引擎只重画这个视频框。收下返回 1
#[no_mangle]
pub extern "C" fn mr_video_frame_rgba(key: *const c_char, width: u32, height: u32, stride: u32, data: *const u8) -> c_int {
    let Some(key) = cstr(key) else { return 0 };
    if data.is_null() || width == 0 || height == 0 {
        return 0;
    }
    let len = stride as usize * (height as usize - 1) + width as usize * 4;
    // SAFETY: 宿主保证 data 至少有 stride × (height − 1) + width × 4 个字节，调用期间有效
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    crate::renderer::components::host_feed::push_rgba(&key, width, height, stride as usize, bytes) as c_int
}

/// 同层渲染的通用组件（创建选项 `sameLayer` 里除 video 以外的标签，如 `map`）：宿主把控件画好的一帧，
/// RGBA、`stride` 字节一行，尺寸按 `sameLayerViews` 事件里的 `box`。`key` 取自那个事件。
/// **任何线程都可以调**（帧存在进程级的表里）；调完让引擎线程出一帧，引擎只重画这个框。
/// 内容变了才交（滚动不用交：画面在引擎画布里跟着走）。收下返回 1
#[no_mangle]
pub extern "C" fn mr_native_view_frame_rgba(key: *const c_char, width: u32, height: u32, stride: u32, data: *const u8) -> c_int {
    let Some(key) = cstr(key) else { return 0 };
    if data.is_null() || width == 0 || height == 0 {
        return 0;
    }
    let len = stride as usize * (height as usize - 1) + width as usize * 4;
    // SAFETY: 宿主保证 data 至少有 stride × (height − 1) + width × 4 个字节，调用期间有效
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    crate::renderer::components::host_view::push_rgba(&key, width, height, stride as usize, bytes) as c_int
}

/// 同上，YUV 4:2:0 三个平面（Android `ImageReader` 的 `YUV_420_888`：U / V 交错时 `uv_pixel_stride` 为 2，
/// `u` / `v` 指向各自的第一个样本）。转 RGBA 在调用线程上做。收下返回 1
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn mr_video_frame_yuv(
    key: *const c_char,
    width: u32,
    height: u32,
    y: *const u8,
    y_stride: u32,
    u: *const u8,
    v: *const u8,
    uv_stride: u32,
    uv_pixel_stride: u32,
) -> c_int {
    let Some(key) = cstr(key) else { return 0 };
    if y.is_null() || u.is_null() || v.is_null() || width == 0 || height == 0 {
        return 0;
    }
    let (w, h, ps) = (width as usize, height as usize, uv_pixel_stride.max(1) as usize);
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
    let y_len = y_stride as usize * (h - 1) + w;
    let c_len = uv_stride as usize * (ch - 1) + (cw - 1) * ps + 1;
    // SAFETY: 宿主保证三个平面按给的步长覆盖整帧（ImageReader 的 Plane 就是这样），调用期间有效
    let (yp, up, vp) = unsafe {
        (std::slice::from_raw_parts(y, y_len), std::slice::from_raw_parts(u, c_len), std::slice::from_raw_parts(v, c_len))
    };
    let f = crate::renderer::components::host_feed::Yuv420 {
        width,
        height,
        y: yp,
        y_stride: y_stride as usize,
        u: up,
        v: vp,
        uv_stride: uv_stride as usize,
        uv_pixel_stride: ps,
    };
    crate::renderer::components::host_feed::push_yuv(&key, &f) as c_int
}

/// 宿主推一个事件给逻辑层的 `wx.onXxx` 监听者：`name` 如 `accelerometerChange` / `locationChange` /
/// `bluetoothDeviceFound` / `networkStatusChange` / `themeChange` / `contextEvent`（`onXxx` 写法也认），
/// `payload_json` 是交给回调的 res（NULL 等于 `{}`）。返回收到它的监听者个数（0 = 没人听，可以停掉传感器）
#[no_mangle]
pub extern "C" fn mr_app_dispatch_event(app: *mut MRApp, name: *const c_char, payload_json: *const c_char) -> c_int {
    let Some(a) = app_mut(app) else { return 0 };
    let Some(name) = cstr(name) else { return 0 };
    let payload = cstr(payload_json).unwrap_or_else(|| "{}".into());
    a.engine.dispatch_event(&name, &payload) as c_int
}

/// 在逻辑层执行一段 JS，返回结果的字符串形式（UTF-8；抛异常时返回 NULL 并写 `last_error`）。
/// 指针活到下一次对同一 app 调用 `mr_app_*`。宿主注入深链参数、集成测试断言页面数据用。
#[no_mangle]
pub extern "C" fn mr_app_eval(app: *mut MRApp, code: *const c_char) -> *const c_char {
    let Some(a) = app_mut(app) else { return std::ptr::null() };
    let Some(code) = cstr(code) else { return std::ptr::null() };
    match a.engine.eval(&code) {
        Ok(v) => {
            a.eval_buf = CString::new(v.replace('\0', "")).unwrap_or_default();
            a.eval_buf.as_ptr()
        }
        Err(e) => {
            a.last_error = e;
            std::ptr::null()
        }
    }
}

#[cfg(test)]
#[path = "ffi_embed_tests.rs"]
mod tests;
