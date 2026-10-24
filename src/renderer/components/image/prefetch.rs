//! 图片预取：快要滑进屏幕的图，先在后台线程里解码、按框的尺寸降采样好，放进与绘制同一份缓存。
//!
//! 为什么：包内的图在绘制时**同步**解码（`load.rs`，桌面上很快、也不闪空），可一加 PLY110 上
//! 一张 1024² 的 jpg 解码 13~20ms、再面积降采样 8~15ms。下载来的「头条新闻」每露出一截新内容就有
//! 一两张新图第一次画，那一帧 35~60ms（`MINI_DAMAGE_LOG` 的〔慢：解码 3次 36.0ms · 降采样 3次 20.6ms〕）。
//!
//! 预取只是提前把**同一个函数**的结果放进**同一份缓存**：绘制路径一行没改，命中了就快，没命中照旧
//! 同步做 —— 像素逐字节不变。远程图 / 大文件本来就走后台（`image_net`），这里调 `load_image`
//! 只是替它们早一点开始下载 / 解码。
//!
//! 一条后台线程、一个去重的队列；队列满了丢最旧的（滑得快时，早排进去的多半已经滑过去了）。
use super::*;
use std::collections::{HashSet, VecDeque};
use std::sync::{Condvar, Mutex, OnceLock};

/// 队列最多排这么多张（多了丢最旧的）
const MAX_QUEUE: usize = 24;

struct Job {
    src: String,
    /// 框的设备像素尺寸与 mode（降采样按它）
    w: f32,
    h: f32,
    mode: String,
    /// 圆角与四分之一像素的亚像素偏移（缩放结果的键，见 `Canvas::image_scaled`）
    radius: f32,
    frac: (f32, f32),
}

#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    /// 排着的 + 正在做的（按 src + 尺寸去重）
    keys: HashSet<String>,
}

fn queue() -> &'static (Mutex<Queue>, Condvar) {
    static Q: OnceLock<(Mutex<Queue>, Condvar)> = OnceLock::new();
    Q.get_or_init(|| {
        std::thread::Builder::new()
            .name("mini-img-prefetch".into())
            .spawn(worker)
            .ok();
        (Mutex::new(Queue::default()), Condvar::new())
    })
}

fn key(src: &str, w: f32, h: f32, mode: &str) -> String {
    format!("{src}|{}x{}|{mode}", (w * 4.0) as i32, (h * 4.0) as i32)
}

/// 已经解好（静态图或动图在缓存里）
fn decoded(src: &str) -> bool {
    get_image_cache().lock().map(|c| c.peek(src).is_some()).unwrap_or(false)
        || get_anim_cache().lock().map(|c| c.peek(src).is_some()).unwrap_or(false)
}

/// 排一张（已经排着的不重复排）。几何是画它的时候那一份（设备像素，`WxmlRenderer::image_plans`）
pub fn request_plan(p: &crate::renderer::wxml_renderer::ImagePlan) {
    let frac = (p.x - p.x.floor(), p.y - p.y.floor());
    push(Job { src: p.src.clone(), w: p.w, h: p.h, mode: p.mode.clone(), radius: p.radius, frac });
}

/// 只要解码 + 降采样（不知道画在哪）。`w` / `h` 是框的设备像素尺寸
pub fn request(src: &str, w: f32, h: f32, mode: &str) {
    push(Job { src: src.to_string(), w, h, mode: mode.to_string(), radius: f32::NAN, frac: (0.0, 0.0) });
}

fn push(job: Job) {
    let (src, w, h, mode) = (job.src.as_str(), job.w, job.h, job.mode.as_str());
    if src.is_empty() || w < 1.0 || h < 1.0 || src.starts_with("data:") {
        return;
    }
    let k = key(src, w, h, mode);
    let (m, cv) = queue();
    let Ok(mut q) = m.lock() else { return };
    if q.keys.contains(&k) {
        return;
    }
    q.keys.insert(k);
    q.jobs.push_back(job);
    while q.jobs.len() > MAX_QUEUE {
        if let Some(old) = q.jobs.pop_front() {
            let ok = key(&old.src, old.w, old.h, &old.mode);
            q.keys.remove(&ok);
        }
    }
    cv.notify_one();
}

/// 这张图要不要排（给调用方先筛一遍：解好了、降采样也好了的就别进锁排队）
pub fn wanted(src: &str) -> bool {
    !src.is_empty() && !decoded(src)
}

fn worker() {
    loop {
        let job = {
            let (m, cv) = queue();
            let Ok(mut q) = m.lock() else { return };
            loop {
                if let Some(j) = q.jobs.pop_back() {
                    // 后进先出：最近一次排的离视口最近
                    break j;
                }
                q = match cv.wait(q) {
                    Ok(g) => g,
                    Err(_) => return,
                };
            }
        };
        run(&job);
        if let Ok(mut q) = queue().0.lock() {
            q.keys.remove(&key(&job.src, job.w, job.h, &job.mode));
        }
    }
}

fn run(job: &Job) {
    // 与绘制同一个入口：本地小图同步解码进缓存，远程图 / 大文件开始后台下载解码（这次拿不到就算了）
    let Some(img) = load_image(&job.src) else { return };
    if is_animated(&job.src) {
        return;
    }
    // 降采样的键与绘制时一样（`src#帧号`，见 ImageComponent::draw）
    let cache_key = format!("{}#{}", job.src, img.frame_index);
    let mip = crate::Canvas::image_mip(&cache_key, &img.data, img.width, img.height, job.w, job.h, &job.mode);
    if job.radius.is_nan() {
        return;
    }
    // 缩放结果：与 draw_image_cached 同样的输入（降采样过的那张 + 亚像素偏移），整屏级的大图它也不缓存
    let (data, iw, ih) = match &mip {
        Some(m) => (m.2.as_slice(), m.0, m.1),
        None => (img.data.as_slice(), img.width, img.height),
    };
    let (dw, dh) = (job.w.ceil() as u64 + 1, job.h.ceil() as u64 + 1);
    if dw * dh <= 4_000_000 {
        crate::Canvas::image_scaled(&cache_key, data, iw, ih, job.frac, job.w, job.h, &job.mode, job.radius);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 去重_队列满了丢最旧的() {
        let (m, _) = queue();
        // 不让后台线程抢走：直接看排队的结果（键里带尺寸，同一张图不同框算两张）
        let before = m.lock().unwrap().keys.len();
        let base = format!("/no/such/prefetch-test-{}.png", std::process::id());
        request(&base, 10.0, 10.0, "scaleToFill");
        request(&base, 10.0, 10.0, "scaleToFill");
        let after = m.lock().unwrap().keys.len();
        assert!(after <= before + 1, "同一张同一个框只排一次");
        request("", 10.0, 10.0, "scaleToFill");
        request(&base, 0.0, 10.0, "scaleToFill");
        assert!(m.lock().unwrap().keys.len() <= before + 1, "空地址 / 零尺寸不排");
    }
}
