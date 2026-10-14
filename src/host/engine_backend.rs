//! [`MiniEngine`] 的后端选择与带标志位的出帧（设计 §7.3、§8.2；v2 修订 R4、R5、R10）。
//!
//! 现在还没有 GPU 合成器，所以这里的一切最终都落到 CPU：
//! - [`pump_ex`](MiniEngine::pump_ex) 只会返回 `0` 或 [`FRAME_NEW`]。`PRESENTED`
//!   （引擎已画到 surface 上）与 `PENDING`（本次没上屏、下次补）都要有 surface 才可能出现；
//! - `BACKEND_CHANGED` 只在**引擎自己**引起实际后端变化时置位（R5：回退、`set_backend`
//!   之后重新选择）。宿主主动 attach / detach 不置位 —— 宿主自己知道。CPU 下实际后端
//!   永远是 CPU，所以它也不会出现，但判断照样按这个规矩写，GPU 接进来时不用再改语义。
//!
//! 老接口 `pump` 就是 `pump_ex & FRAME_NEW`，写 `== 1` 的 C 调用方照旧拿到 0 / 1。
use super::engine::MiniEngine;
use super::engine_stats;
use crate::compositor::select::{self, SelectInput};
use crate::compositor::{diag, pref, BackendKind, BackendPref, BackendStatus, DeviceInfo, HostEnv, HostKind, Platform, SurfaceKind};
use std::time::Instant;

/// 有新画面
pub const FRAME_NEW: u32 = 1;
/// 已由引擎画到挂载的 surface 上，宿主不要再取像素显示（要有 GPU 合成器才会出现）
pub const FRAME_PRESENTED: u32 = 2;
/// 实际后端与上一次 pump 不同（只在引擎自己引起变化时置位），宿主要立刻切换显示路径
pub const FRAME_BACKEND_CHANGED: u32 = 4;
/// 本次没上屏（surface 暂时取不到），下一次 pump 会强制重新合成（要有 GPU 合成器才会出现）
pub const FRAME_PENDING: u32 = 8;

/// SDK 的一次后端选择：偏好按「`MINI_BACKEND` > 宿主配置 > cpu」，再过不碰 GPU 的门槛
pub(crate) fn select_backend(
    env: &HostEnv,
    host_pref: Option<BackendPref>,
    device: &DeviceInfo,
    surface: Option<SurfaceKind>,
) -> BackendStatus {
    let r = pref::resolve(HostKind::Sdk, None, env, host_pref);
    select::decide(&SelectInput {
        pref: r.pref,
        source: r.source,
        platform: Platform::current(),
        device,
        gpu_compiled: select::GPU_COMPILED,
        has_surface: surface.is_some(),
    })
}

impl MiniEngine {
    /// 当前的后端结论（偏好、实际后端、原因）
    pub fn backend(&self) -> &BackendStatus {
        &self.backend
    }

    /// 宿主的后端偏好（`mr_app_set_backend`）。`MINI_BACKEND` 设了的话以它为准，
    /// 并打一行说明（诊断开关压过宿主配置，见 `compositor::pref`）。
    pub fn set_backend_pref(&mut self, pref: BackendPref) {
        self.host_pref = Some(pref);
        let r = pref::resolve(HostKind::Sdk, None, &self.env, self.host_pref);
        if let Some(host) = r.overrode_host {
            diag::emit(&diag::env_overrides_host(r.pref, host));
        }
        self.reselect_backend(true);
    }

    /// 机型信息（`mr_app_set_device_info`），机型分级要用。应在挂 surface 之前给。
    pub fn set_device_info(&mut self, info: DeviceInfo) {
        self.device_info = info;
        self.reselect_backend(true);
    }

    /// 宿主挂上了一块 surface。返回挂载后的实际后端。
    ///
    /// 只记种类、不收指针：现在没有 GPU 合成器，指针既不解引用也不保留
    /// （Android 也就不需要 `ANativeWindow_acquire`）。重复调用等于先卸再挂。
    pub fn attach_surface(&mut self, kind: SurfaceKind) -> BackendKind {
        self.surface = Some(kind);
        self.reselect_backend(false);
        self.backend.active
    }

    /// surface 要销毁了 / 进后台。之后是 CPU。
    pub fn detach_surface(&mut self) {
        self.surface = None;
        self.reselect_backend(false);
    }

    /// 重新选一次；结论变了打一行日志。`engine_caused` 为真且实际后端变了时，
    /// 下一次 `pump_ex` 带上 [`FRAME_BACKEND_CHANGED`]。
    fn reselect_backend(&mut self, engine_caused: bool) {
        let next = select_backend(&self.env, self.host_pref, &self.device_info, self.surface);
        if next == self.backend {
            return;
        }
        if engine_caused && next.active != self.backend.active {
            self.backend_changed = true;
        }
        diag::emit(&diag::selected(&next));
        self.backend = next;
    }

    /// 跑一帧，返回 `FRAME_*` 的按位或。`pump(t) == (pump_ex(t) & FRAME_NEW != 0)`。
    pub fn pump_ex(&mut self, now_ms: u64) -> u32 {
        // 不开 MINI_FPS 时连时钟都不取：pump 是每个 vsync 都要调的热路径
        let begin = self.stats.enabled().then(Instant::now);
        let produced = self.pump_frame(now_ms);
        let mut flags = if produced { FRAME_NEW } else { 0 };
        if std::mem::take(&mut self.backend_changed) {
            flags |= FRAME_BACKEND_CHANGED;
        }
        if let Some(begin) = begin {
            // CPU 后端没有 surface 可等：acquire 恒为 0，工作时间 = pump 时间
            let pump_ms = begin.elapsed().as_secs_f32() * 1000.0;
            if let Some(sum) = self.stats.record(begin, pump_ms, 0.0, produced) {
                diag::emit(&engine_stats::fps_line(&self.backend, &sum));
            }
        }
        flags
    }

    /// 跑一帧：逻辑层 → 事件 → 渲染 → 合成。返回是否产出了新画面。
    ///
    /// `now_ms` 用宿主的单调时钟（iOS `CACurrentMediaTime`、Android
    /// `System.nanoTime`）。传 0 表示让引擎自己取。
    pub fn pump(&mut self, now_ms: u64) -> bool {
        self.pump_ex(now_ms) & FRAME_NEW != 0
    }
}
