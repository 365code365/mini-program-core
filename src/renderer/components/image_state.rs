//! `<image>` 的加载结论探测：`bindload` / `binderror` 的判据。
//!
//! 微信里图片一加载完就发 `bindload`，很多页面靠它驱动后续动作。tea-day 的
//! 启动页就是「背景图 load 之后才开始 5 秒倒计时」，引擎不发这个事件时只能等
//! 页面自己的 3 秒兜底定时器 —— 用户会先干看 3 秒不动的「5s」。
//!
//! 单独一个文件的原因很实际：`image.rs` 已经接近 500 行的上限，
//! 而这里要读它的两张缓存（静态图 / 动图），放一起会把那个文件顶过线。

use super::image::{get_anim_cache, get_image_cache, ImageData};

/// 一个 `<image>` 当前的加载结论
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageLoad {
    /// 还没有结论（本地图还没被绘制过、远程图还在下载或退避重试中）
    Pending,
    /// 已就绪，带**原始**像素尺寸（微信 `bindload` 的 `detail` 就是这两个字段）
    Ready { width: u32, height: u32 },
    /// 确定失败
    Failed,
}

/// 探测一个 src 的加载结论：**只查缓存**，不读盘、不发起下载。
///
/// 为什么必须「只查表」：这个函数是给事件派发用的，每帧对每个 `<image>` 都要问一次。
/// 顺手触发加载的话，视口外被裁掉、本来不该加载的图会被它拖下来。
pub fn probe_load(src: &str) -> ImageLoad {
    if src.trim().is_empty() {
        return ImageLoad::Pending;
    }
    // 动图：解码完就进这张表
    if let Ok(cache) = get_anim_cache().lock() {
        if let Some(anim) = cache.get(src) {
            return ImageLoad::Ready { width: anim.width(), height: anim.height() };
        }
    }
    // 静态图缓存：`Some(None)` 是「解码/读盘失败」的记录
    if let Ok(cache) = get_image_cache().lock() {
        match cache.get(src) {
            Some(Some(d)) => return ImageLoad::Ready { width: d.width, height: d.height },
            Some(None) => return ImageLoad::Failed,
            None => {}
        }
    }
    if src.starts_with("http://") || src.starts_with("https://") {
        return match super::image_net::peek::<ImageData>(src) {
            super::image_net::Peek::Ready(d) => {
                ImageLoad::Ready { width: d.width, height: d.height }
            }
            super::image_net::Peek::Failed => ImageLoad::Failed,
            super::image_net::Peek::Pending => ImageLoad::Pending,
        };
    }
    ImageLoad::Pending
}
