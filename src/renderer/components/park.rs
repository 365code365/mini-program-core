//! 组件的进程级状态表（按组件 id 存）：小程序挂起时整份带走、恢复时放回（host/engine_park.rs）。
//!
//! 这些表是进程级的 `static`（swiper 页码与动画、picker / picker-view 选中项、canvas 画布、进度条动画、
//! 播放器、同层视频与原生组件的帧表）。小程序 A 打开 B 时 A 留在内存里挂起（与微信一致），
//! B 在同一进程里建引擎：不把 A 的表拿走的话，B 里同 id 的组件会接着 A 的状态（两个小程序都有
//! `id="banner"` 的 swiper 很常见），B 退出后 A 又会看到 B 留下的。
//! 线程局部（thread_local）的状态不在这里：每个引擎在自己的线程上，天然分开。

/// 一个挂起的小程序的组件状态
pub(crate) struct Parked {
    swiper: super::swiper::SwiperStateManager,
    picker: super::picker::PickerStateManager,
    canvas: super::canvas::CanvasContextManager,
    picker_view: super::picker_view::Parked,
    progress: super::progress_anim::Parked,
    video: super::video::Parked,
    host_views: super::host_view::Parked,
}

/// 把当前这些表整份取走（表换成空的）
pub(crate) fn park() -> Parked {
    Parked {
        swiper: super::swiper::park(),
        picker: super::picker::park(),
        canvas: super::canvas::park(),
        picker_view: super::picker_view::park(),
        progress: super::progress_anim::park(),
        video: super::video::park(),
        host_views: super::host_view::park(),
    }
}

/// 放回（表里此刻的内容是别的小程序留下的，直接丢掉）
pub(crate) fn unpark(p: Parked) {
    super::swiper::unpark(p.swiper);
    super::picker::unpark(p.picker);
    super::canvas::unpark(p.canvas);
    super::picker_view::unpark(p.picker_view);
    super::progress_anim::unpark(p.progress);
    super::video::unpark(p.video);
    super::host_view::unpark(p.host_views);
}
