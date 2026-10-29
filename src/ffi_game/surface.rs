//! 小游戏主画布直接上屏到宿主的原生图层（iOS：CAMetalLayer）。
//!
//! 与 Android 的 `nativeBlit`（ffi_jni_game.rs）是同一条路：主画布是 WebGL 时，EGL 窗口 surface 建在宿主给的
//! 原生窗口上，帧末 `eglSwapBuffers`，不读回、不经 CPU（game/webgl/window.rs）。iOS 的 ANGLE（Metal 后端）把
//! `CAMetalLayer*` 当 `EGLNativeWindowType`。读回那条路在一加上 1270×2770 每帧 32ms，iOS 同理过不了 60fps。
//!
//! ## 为什么尺寸由宿主设、引擎只核对
//! ANGLE 按图层的 `bounds × contentsScale` 定 drawable 尺寸，而这些属性该在主线程改；引擎线程上同步等主线程
//! 会与「主线程同步等引擎线程」（进后台、销毁）死锁。所以：引擎要的尺寸记在 [`Layer::want`]，宿主看到
//! [`mr_game_present_layer`] 返回 -1 就在主线程把图层改成那个像素尺寸，改完 [`mr_game_layer_size`] 报回来，
//! 下一帧再上屏。尺寸只在建画布 / 游戏改画布尺寸时变，平时一次比较。
//!
//! 主画布不是 WebGL（2D 游戏）、上下文丢了、`preserveDrawingBuffer` 不支持时返回 [`MR_GAME_PRESENT_CPU`]：
//! 宿主改用 `mr_game_pixels_into` 那条路（另一块图层），与 Android 的 `None` 同义。
use super::{guarded, MRGame};
#[cfg(feature = "webgl")]
use super::mr_game_with_engine;
use std::ffi::{c_int, c_void};

/// `mr_game_present_layer` 的返回值：不归图层管，宿主走 `mr_game_pixels_into`
pub const MR_GAME_PRESENT_CPU: c_int = 2;

#[derive(Default)]
pub(super) struct Layer {
    ptr: usize,
    /// 宿主报来的图层像素尺寸（已经生效的）
    size: (u32, u32),
    /// 引擎要的像素尺寸（-1 时宿主按它改）
    want: (u32, u32),
}

/// 挂上宿主的原生图层（iOS：`CAMetalLayer*`，宿主持有它直到 [`mr_game_detach_layer`] 返回）。
/// 换一块图层前不用先 detach。返回 1 = 挂上了，0 = 实例为空 / 失效 / 指针为空
#[no_mangle]
pub extern "C" fn mr_game_attach_layer(game: *mut MRGame, layer: *mut c_void) -> c_int {
    if layer.is_null() {
        return 0;
    }
    mr_game_detach_layer(game);
    let Some(g) = (unsafe { game.as_mut() }) else { return 0 };
    if g.poisoned {
        return 0;
    }
    g.layer = Layer { ptr: layer as usize, size: (0, 0), want: (0, 0) };
    1
}

/// 放手图层（同步：返回之后引擎不再碰它）。没挂过也可以调
#[no_mangle]
pub extern "C" fn mr_game_detach_layer(game: *mut MRGame) {
    let Some(g) = (unsafe { game.as_mut() }) else { return };
    if g.layer.ptr == 0 {
        return;
    }
    g.layer = Layer::default();
    // EGL 窗口 surface 先拆（它引用着这块图层）
    #[cfg(feature = "webgl")]
    guarded(game, (), |e| e.webgl_detach_window());
}

/// 宿主把图层的绘制尺寸设好了（像素：iOS 上 `bounds × contentsScale`）
#[no_mangle]
pub extern "C" fn mr_game_layer_size(game: *mut MRGame, width: u32, height: u32) {
    if let Some(g) = unsafe { game.as_mut() } {
        g.layer.size = (width, height);
    }
}

/// `mr_game_present_layer` 返回 -1 时：引擎要的图层像素尺寸
#[no_mangle]
pub extern "C" fn mr_game_layer_want(game: *mut MRGame, width: *mut u32, height: *mut u32) {
    let (w, h) = unsafe { game.as_ref() }.map(|g| g.layer.want).unwrap_or((0, 0));
    // SAFETY: 调用方给的输出指针，可为 NULL
    unsafe {
        if let Some(p) = width.as_mut() {
            *p = w;
        }
        if let Some(p) = height.as_mut() {
            *p = h;
        }
    }
}

/// 把当前帧交到图层上（`mr_game_pump_ex` 回报 `MR_FRAME_NEW`、或刚挂上图层时调）：
/// 1 = 已经 swap；0 = 这一帧没有新画面（图层保持上一帧）；-1 = 图层尺寸还不对（按 `mr_game_layer_want` 改好、
/// `mr_game_layer_size` 报回来再调）；-2 = 出错；[`MR_GAME_PRESENT_CPU`] = 不归图层管，走 `mr_game_pixels_into`
#[no_mangle]
pub extern "C" fn mr_game_present_layer(game: *mut MRGame) -> c_int {
    let Some(g) = (unsafe { game.as_mut() }) else { return 0 };
    if g.poisoned {
        return 0;
    }
    if g.layer.ptr == 0 {
        return MR_GAME_PRESENT_CPU;
    }
    #[cfg(feature = "webgl")]
    {
        let (ptr, size) = (g.layer.ptr, g.layer.size);
        let mut want = None;
        let mut geometry = |w: u32, h: u32| {
            want = Some((w, h));
            size == (w, h)
        };
        // 图层只给 EGL 用（CPU 帧画在宿主的另一块图层上），所以 cpu_locked 恒为 false
        let r = mr_game_with_engine(game, |e| e.webgl_present_window(ptr, false, &mut geometry));
        if let (Some(w), Some(g)) = (want, unsafe { game.as_mut() }) {
            g.layer.want = w;
        }
        return match r {
            Some(Some(v)) => v,
            Some(None) => MR_GAME_PRESENT_CPU,
            None => 0,
        };
    }
    #[cfg(not(feature = "webgl"))]
    {
        let _ = guarded(game, (), |_| ());
        MR_GAME_PRESENT_CPU
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 空句柄与没挂图层() {
        assert_eq!(mr_game_attach_layer(std::ptr::null_mut(), std::ptr::null_mut()), 0);
        mr_game_detach_layer(std::ptr::null_mut());
        mr_game_layer_size(std::ptr::null_mut(), 1, 1);
        let (mut w, mut h) = (7u32, 7u32);
        mr_game_layer_want(std::ptr::null_mut(), &mut w, &mut h);
        assert_eq!((w, h), (0, 0));
        assert_eq!(mr_game_present_layer(std::ptr::null_mut()), 0);
    }
}
