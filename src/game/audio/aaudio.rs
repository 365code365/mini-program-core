//! Android 的音频输出：运行时 `dlopen("libaaudio.so")`（game-doc/06 D-4 = A 的 Android 落法）。
//!
//! 为什么不用 cpal：cpal 直接链接 `libaaudio`，而 AAudio 从 API 26 才有；SDK 的 minSdk 是 24 —— 带着 `DT_NEEDED libaaudio.so`
//! 的引擎库在 Android 7.x 上**整个加载失败**，小程序也跟着用不了。这里按需取符号：API 26+ 出声，24 / 25 返回 Err，
//! 上层落到静音的离线模式（时钟与事件照常，GT-AU9 同一条路）。顺带能设 cpal 不给的两样：
//! `PERFORMANCE_MODE_LOW_LATENCY`（快速通道，回调块小）与 `USAGE_GAME`（API 28+，有就设）。
//!
//! 回调里只调混音器（不分配、不加锁）；欠载次数取 `AAudioStream_getXRunCount`（GT-AU7）。
//! 设备断开（拔耳机、蓝牙断）时 AAudio 的错误回调把标志置上，引擎线程下一帧关掉旧流、用同一个混音器重开（最多 3 次）。

use super::mixer::Mixer;
use super::output::Stats;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

type Builder = c_void;
type Stream = c_void;
type DataCb = extern "C" fn(*mut Stream, *mut c_void, *mut c_void, i32) -> i32;
type ErrCb = extern "C" fn(*mut Stream, *mut c_void, i32);

extern "C" {
    fn dlopen(name: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(h: *mut c_void, name: *const c_char) -> *mut c_void;
}
const RTLD_NOW: c_int = 2;
const DIRECTION_OUTPUT: i32 = 0;
const FORMAT_PCM_FLOAT: i32 = 2;
const SHARING_SHARED: i32 = 1;
const PERF_LOW_LATENCY: i32 = 12;
const USAGE_GAME: i32 = 14;
const CALLBACK_CONTINUE: i32 = 0;

struct Api {
    create_builder: unsafe extern "C" fn(*mut *mut Builder) -> i32,
    set_direction: unsafe extern "C" fn(*mut Builder, i32),
    set_format: unsafe extern "C" fn(*mut Builder, i32),
    set_channels: unsafe extern "C" fn(*mut Builder, i32),
    set_sharing: unsafe extern "C" fn(*mut Builder, i32),
    set_perf: unsafe extern "C" fn(*mut Builder, i32),
    set_usage: Option<unsafe extern "C" fn(*mut Builder, i32)>,
    set_data_cb: unsafe extern "C" fn(*mut Builder, DataCb, *mut c_void),
    set_err_cb: unsafe extern "C" fn(*mut Builder, ErrCb, *mut c_void),
    open: unsafe extern "C" fn(*mut Builder, *mut *mut Stream) -> i32,
    delete_builder: unsafe extern "C" fn(*mut Builder) -> i32,
    start: unsafe extern "C" fn(*mut Stream) -> i32,
    pause: unsafe extern "C" fn(*mut Stream) -> i32,
    close: unsafe extern "C" fn(*mut Stream) -> i32,
    rate: unsafe extern "C" fn(*mut Stream) -> i32,
    channels: unsafe extern "C" fn(*mut Stream) -> i32,
    burst: unsafe extern "C" fn(*mut Stream) -> i32,
    set_buffer: unsafe extern "C" fn(*mut Stream, i32) -> i32,
    xruns: unsafe extern "C" fn(*mut Stream) -> i32,
    timestamp: unsafe extern "C" fn(*mut Stream, i32, *mut i64, *mut i64) -> i32,
    written: unsafe extern "C" fn(*mut Stream) -> i64,
}

fn api() -> Result<&'static Api, String> {
    static API: std::sync::OnceLock<Result<Api, String>> = std::sync::OnceLock::new();
    API.get_or_init(|| unsafe {
        let h = dlopen(c"libaaudio.so".as_ptr(), RTLD_NOW);
        if h.is_null() {
            return Err("没有 libaaudio.so（Android 8.0 / API 26 以下）".into());
        }
        let sym = |n: &CStr| -> Result<*mut c_void, String> {
            let p = dlsym(h, n.as_ptr());
            if p.is_null() { Err(format!("libaaudio 缺 {}", n.to_string_lossy())) } else { Ok(p) }
        };
        macro_rules! f {
            ($n:literal) => {
                std::mem::transmute::<*mut c_void, _>(sym($n)?)
            };
        }
        // SAFETY: 签名与 NDK aaudio/AAudio.h 一致；符号来自系统库，进程生命期内有效（不 dlclose）
        Ok(Api {
            create_builder: f!(c"AAudio_createStreamBuilder"),
            set_direction: f!(c"AAudioStreamBuilder_setDirection"),
            set_format: f!(c"AAudioStreamBuilder_setFormat"),
            set_channels: f!(c"AAudioStreamBuilder_setChannelCount"),
            set_sharing: f!(c"AAudioStreamBuilder_setSharingMode"),
            set_perf: f!(c"AAudioStreamBuilder_setPerformanceMode"),
            set_usage: sym(c"AAudioStreamBuilder_setUsage").ok().map(|p| std::mem::transmute::<*mut c_void, _>(p)),
            set_data_cb: f!(c"AAudioStreamBuilder_setDataCallback"),
            set_err_cb: f!(c"AAudioStreamBuilder_setErrorCallback"),
            open: f!(c"AAudioStreamBuilder_openStream"),
            delete_builder: f!(c"AAudioStreamBuilder_delete"),
            start: f!(c"AAudioStream_requestStart"),
            pause: f!(c"AAudioStream_requestPause"),
            close: f!(c"AAudioStream_close"),
            rate: f!(c"AAudioStream_getSampleRate"),
            channels: f!(c"AAudioStream_getChannelCount"),
            burst: f!(c"AAudioStream_getFramesPerBurst"),
            set_buffer: f!(c"AAudioStream_setBufferSizeInFrames"),
            xruns: f!(c"AAudioStream_getXRunCount"),
            timestamp: f!(c"AAudioStream_getTimestamp"),
            written: f!(c"AAudioStream_getFramesWritten"),
        })
    })
    .as_ref()
    .map_err(|e| e.clone())
}

/// 回调的用户数据：混音器归音频线程（回调）独占
struct CbState {
    mixer: Mixer,
    channels: usize,
    stats: Arc<Stats>,
    lost: Arc<AtomicBool>,
}

extern "C" fn on_data(_s: *mut Stream, user: *mut c_void, data: *mut c_void, frames: i32) -> i32 {
    // SAFETY: user 是 open 时交出去的 Box<CbState>，流关掉之前一直有效、只有回调线程碰它
    let st = unsafe { &mut *(user as *mut CbState) };
    let n = frames.max(0) as usize * st.channels;
    // SAFETY: AAudio 给的输出缓冲是 frames × channels 个 f32（格式在 open 时定为 PCM_FLOAT）
    let out = unsafe { std::slice::from_raw_parts_mut(data as *mut f32, n) };
    st.mixer.render(out);
    st.stats.block_frames.store(frames as u32, Ordering::Relaxed);
    st.stats.callbacks.fetch_add(1, Ordering::Relaxed);
    st.stats.frames.fetch_add(frames as u64, Ordering::Relaxed);
    CALLBACK_CONTINUE
}

extern "C" fn on_error(_s: *mut Stream, user: *mut c_void, _err: i32) {
    // 不能在错误回调里关流（AAudio 的规定）：置标志，引擎线程下一帧重开
    let st = unsafe { &*(user as *const CbState) };
    st.lost.store(true, Ordering::Relaxed);
}

pub struct Out {
    stream: *mut Stream,
    state: *mut CbState,
    pub stats: Arc<Stats>,
    lost: Arc<AtomicBool>,
    pub rate: u32,
    pub channels: usize,
    reopens: u32,
}

/// 先开一个不带回调的流问出设备的采样率 / 声道（立即关掉），再按它建混音器、开真正的流 —— 混音器只建一次
pub fn open<E>(make: &mut dyn FnMut(u32, usize) -> (Mixer, E)) -> Result<(Out, E, String), String> {
    let a = api()?;
    let (rate, channels) = unsafe {
        let (probe, rate, channels) = open_stream(a, None, 2)?;
        (a.close)(probe);
        (rate, channels.min(2))
    };
    let (mixer, ends) = make(rate, channels);
    let stats = Arc::new(Stats::default());
    let lost = Arc::new(AtomicBool::new(false));
    let state = Box::into_raw(Box::new(CbState { mixer, channels, stats: stats.clone(), lost: lost.clone() }));
    // SAFETY: state 由 Out 持有，流关掉之后才释放
    let (stream, _, _) = match unsafe { open_stream(a, Some(state), channels) } {
        Ok(x) => x,
        Err(e) => {
            drop(unsafe { Box::from_raw(state) });
            return Err(e);
        }
    };
    let out = Out { stream, state, stats, lost, rate, channels, reopens: 0 };
    let burst = unsafe { (a.burst)(stream) };
    if unsafe { (a.start)(stream) } != 0 {
        return Err("AAudioStream_requestStart 失败".into());
    }
    let note = format!("AAudio {rate} Hz × {channels} 声道，低延迟模式，每块 {burst} 帧{}", if a.set_usage.is_some() { "，USAGE_GAME" } else { "" });
    Ok((out, ends, note))
}

/// SAFETY: `state` 给了就必须在流关掉之前一直有效（回调线程独占它）
unsafe fn open_stream(a: &Api, state: Option<*mut CbState>, channels: usize) -> Result<(*mut Stream, u32, usize), String> {
    let mut b: *mut Builder = std::ptr::null_mut();
    if (a.create_builder)(&mut b) != 0 || b.is_null() {
        return Err("AAudio_createStreamBuilder 失败".into());
    }
    (a.set_direction)(b, DIRECTION_OUTPUT);
    (a.set_format)(b, FORMAT_PCM_FLOAT);
    (a.set_channels)(b, channels as i32);
    (a.set_sharing)(b, SHARING_SHARED);
    (a.set_perf)(b, PERF_LOW_LATENCY);
    if let Some(u) = a.set_usage {
        u(b, USAGE_GAME);
    }
    if let Some(st) = state {
        (a.set_data_cb)(b, on_data, st as *mut c_void);
        (a.set_err_cb)(b, on_error, st as *mut c_void);
    }
    let mut s: *mut Stream = std::ptr::null_mut();
    let r = (a.open)(b, &mut s);
    (a.delete_builder)(b);
    if r != 0 || s.is_null() {
        return Err(format!("AAudioStreamBuilder_openStream 失败：{r}"));
    }
    // 缓冲留两个 burst：低延迟与不欠载的折中（Google 的建议值）
    let burst = (a.burst)(s).max(1);
    (a.set_buffer)(s, burst * 2);
    Ok((s, (a.rate)(s).max(1) as u32, (a.channels)(s).max(1) as usize))
}

impl Out {
    /// 引擎线程每帧调：设备断开了就重开（同一个混音器），并更新延迟估计。返回日志
    pub fn poll(&mut self) -> Option<String> {
        let a = api().ok()?;
        if self.lost.swap(false, Ordering::Relaxed) {
            unsafe { (a.close)(self.stream) };
            self.stream = std::ptr::null_mut();
            if self.reopens >= 3 {
                return Some("🔊 音频设备断开且重开 3 次都失败，之后不再出声".into());
            }
            self.reopens += 1;
            // SAFETY: 旧流已关，回调不再碰 state；新流接着用同一个（同一个混音器、同一组队列）
            let r = unsafe { open_stream(a, Some(self.state), self.channels) };
            return Some(match r {
                Ok((s, rate, _)) => {
                    self.stream = s;
                    unsafe { (a.start)(s) };
                    if rate != self.rate {
                        format!("🔊 音频设备断开，已重开（采样率从 {} 变成 {rate}，音调会偏，待处理）", self.rate)
                    } else {
                        "🔊 音频设备断开，已重开".into()
                    }
                }
                Err(e) => format!("🔊 音频设备断开，重开失败：{e}"),
            });
        }
        if !self.stream.is_null() {
            let (mut pos, mut ns) = (0i64, 0i64);
            // 延迟 = 已写入但还没播出的帧 / 采样率 +「那一帧的播出时刻 → 此刻」的差
            if unsafe { (a.timestamp)(self.stream, 1, &mut pos, &mut ns) } == 0 {
                let written = unsafe { (a.written)(self.stream) };
                let now = monotonic_ns();
                let ahead_us = (written - pos).max(0) as f64 * 1e6 / self.rate as f64 - (now - ns) as f64 / 1e3;
                self.stats.latency_us.store(ahead_us.max(0.0) as u32, Ordering::Relaxed);
            }
        }
        None
    }

    /// 进后台：停掉回调（后台还每 4ms 回调一次输出静音是白耗电）；回前台重新开始
    pub fn set_paused(&mut self, paused: bool) {
        if let (Ok(a), false) = (api(), self.stream.is_null()) {
            unsafe {
                if paused { (a.pause)(self.stream) } else { (a.start)(self.stream) };
            }
        }
    }

    pub fn xruns(&self) -> i32 {
        api().ok().filter(|_| !self.stream.is_null()).map(|a| unsafe { (a.xruns)(self.stream) }).unwrap_or(0)
    }
}

impl Drop for Out {
    fn drop(&mut self) {
        if let Ok(a) = api() {
            if !self.stream.is_null() {
                // close 返回之后回调不会再来，state 可以放了
                unsafe { (a.close)(self.stream) };
            }
        }
        if !self.state.is_null() {
            drop(unsafe { Box::from_raw(self.state) });
        }
    }
}

fn monotonic_ns() -> i64 {
    #[repr(C)]
    struct Ts {
        s: i64,
        ns: i64,
    }
    extern "C" {
        fn clock_gettime(id: c_int, ts: *mut Ts) -> c_int;
    }
    let mut t = Ts { s: 0, ns: 0 };
    unsafe { clock_gettime(1, &mut t) };
    t.s * 1_000_000_000 + t.ns
}
