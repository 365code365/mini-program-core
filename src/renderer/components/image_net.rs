//! 远程图片的**异步**加载。
//!
//! 之前 `<image src="https://…">` 是在绘制期同步 `ureq::get` 的 —— 渲染线程直接卡在
//! HTTP 往返上。实测 tea-app 的首屏背景图 276KB / 3.2s，也就是说那一帧卡 3.2 秒；
//! 长列表里每滚出一张新图就再卡一次，表现就是「很多页面滑动很卡」。
//!
//! 而且失败结果被**永久**缓存成 `None`：网络抖一下，那张图这辈子都不会再出现了
//! （表现就是「图片没展示」）。
//!
//! 现在的做法和浏览器一致：
//! 1. 绘制期只查表。没有就登记一个「在下载中」并**立刻返回 None**（这一帧画占位）；
//! 2. 后台线程下载 + 解码，完成后写回表并置脏；
//! 3. 宿主看到脏标记就重绘一帧，图片自然出现。
//!
//! 失败带**退避重试**，不再是一次失败终身失败。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// 单张远程图的状态
enum RemoteState<T> {
    /// 后台线程正在下载
    Loading,
    /// 已就绪
    Ready(Arc<T>),
    /// 失败，记下时刻与次数用于退避重试
    Failed { at: Instant, tries: u32 },
}

/// 首次失败后的重试间隔；每次失败翻倍，最多到 `MAX_BACKOFF`
const BASE_BACKOFF: Duration = Duration::from_secs(2);
const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// 超过这个次数就不再重试（避免对着 404 无限打）
const MAX_TRIES: u32 = 4;

type StateMap<T> = Mutex<HashMap<String, RemoteState<T>>>;

fn states<T: 'static + Send + Sync>() -> &'static StateMap<T> {
    // 每个 T 一张表。目前只有 ImageData 一种，用泛型是为了让解码结果类型留在调用方。
    static MAP: OnceLock<Box<dyn std::any::Any + Send + Sync>> = OnceLock::new();
    MAP.get_or_init(|| Box::new(Mutex::new(HashMap::<String, RemoteState<T>>::new())))
        .downcast_ref::<StateMap<T>>()
        .expect("远程图片状态表类型固定")
}

/// 有图片刚下载完，需要重绘一帧
static DIRTY: AtomicBool = AtomicBool::new(false);
/// 还在下载中的数量（宿主据此决定要不要继续出帧）
static PENDING: OnceLock<Mutex<u32>> = OnceLock::new();

fn pending() -> &'static Mutex<u32> {
    PENDING.get_or_init(|| Mutex::new(0))
}

pub fn log_enabled() -> bool {
    static L: OnceLock<bool> = OnceLock::new();
    *L.get_or_init(|| std::env::var("MINI_IMG_LOG").is_ok())
}

/// 取走「有新图到位」的重绘标记
pub fn take_dirty() -> bool {
    DIRTY.swap(false, Ordering::Relaxed)
}

/// 是否还有图片在下载
pub fn has_pending() -> bool {
    *pending().lock().unwrap() > 0
}

/// 查表拿一张远程图；没有就发起后台下载并返回 None（本帧画占位）。
///
/// `decode` 在**后台线程**里执行（解码大图同样很贵，不能放回渲染线程）。
pub fn get_or_fetch<T, F>(url: &str, decode: F) -> Option<Arc<T>>
where
    T: 'static + Send + Sync,
    F: FnOnce(&[u8]) -> Option<T> + Send + 'static,
{
    {
        let map = states::<T>().lock().unwrap();
        match map.get(url) {
            Some(RemoteState::Ready(v)) => return Some(v.clone()),
            Some(RemoteState::Loading) => return None,
            Some(RemoteState::Failed { at, tries }) => {
                if *tries >= MAX_TRIES {
                    return None;
                }
                let backoff = (BASE_BACKOFF * 2u32.pow(tries.saturating_sub(1))).min(MAX_BACKOFF);
                if at.elapsed() < backoff {
                    return None; // 还在退避窗口里，先别打
                }
            }
            None => {}
        }
    }
    spawn_fetch::<T, F>(url.to_string(), decode);
    None
}

fn spawn_fetch<T, F>(url: String, decode: F)
where
    T: 'static + Send + Sync,
    F: FnOnce(&[u8]) -> Option<T> + Send + 'static,
{
    // 记下已有的失败次数，失败时累加
    let tries = {
        let mut map = states::<T>().lock().unwrap();
        let tries = match map.get(&url) {
            Some(RemoteState::Failed { tries, .. }) => *tries,
            _ => 0,
        };
        map.insert(url.clone(), RemoteState::Loading);
        tries
    };
    *pending().lock().unwrap() += 1;
    if log_enabled() {
        eprintln!("🖼 → 下载 {}", url);
    }

    std::thread::spawn(move || {
        let started = Instant::now();
        let bytes = fetch_bytes(&url);
        let decoded = bytes.as_ref().and_then(|b| decode(b));
        let ms = started.elapsed().as_millis();
        {
            let mut map = states::<T>().lock().unwrap();
            match decoded {
                Some(v) => {
                    if log_enabled() {
                        eprintln!("🖼 ← {} ({} bytes, {}ms)", url, bytes.map(|b| b.len()).unwrap_or(0), ms);
                    }
                    map.insert(url.clone(), RemoteState::Ready(Arc::new(v)));
                }
                None => {
                    if log_enabled() {
                        eprintln!("🖼 ✗ {} 失败（第 {} 次, {}ms）", url, tries + 1, ms);
                    }
                    map.insert(url.clone(), RemoteState::Failed { at: Instant::now(), tries: tries + 1 });
                }
            }
        }
        DIRTY.store(true, Ordering::Relaxed);
        let mut n = pending().lock().unwrap();
        *n = n.saturating_sub(1);
    });
}

fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    // 先看磁盘缓存：远程图第二次打开应当秒出。
    // tea-app 的首屏背景是 276KB / 6s，而闪屏只停 5 秒 ——
    // 没有磁盘缓存的话那张图永远来不及出现（用户看到的就是"图片没展示"）。
    if let Some(bytes) = disk_get(url) {
        if log_enabled() {
            eprintln!("🖼 ⚡ 命中磁盘缓存 {}", url);
        }
        return Some(bytes);
    }
    use std::io::Read;
    let resp = ureq::get(url)
        .timeout(Duration::from_secs(15))
        .call()
        .ok()?;
    let mut buf = Vec::new();
    resp.into_reader()
        .take(20 * 1024 * 1024)
        .read_to_end(&mut buf)
        .ok()?;
    disk_put(url, &buf);
    Some(buf)
}

/// 磁盘缓存目录。放在 `target/` 下，**不污染 `sample/`**（示例是像素基线的输入）。
fn cache_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("mini-imgcache")
}

/// URL -> 文件名。用稳定哈希，避免 URL 里的 `/`、query 变成非法路径。
fn cache_key(url: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut h);
    // 带上扩展名便于人工排查。要先去掉 query/fragment 再取扩展名 ——
    // `a.png?v=1` 直接 rsplit('.') 会得到 `png?v=1`，退化成 bin。
    let path_only = url.split(['?', '#']).next().unwrap_or(url);
    let ext = path_only
        .rsplit('.')
        .next()
        .filter(|e| !e.is_empty() && e.len() <= 4 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("bin");
    format!("{:016x}.{}", h.finish(), ext)
}

fn disk_get(url: &str) -> Option<Vec<u8>> {
    let p = cache_dir().join(cache_key(url));
    let bytes = std::fs::read(p).ok()?;
    if bytes.is_empty() {
        return None;
    }
    Some(bytes)
}

fn disk_put(url: &str, bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    let dir = cache_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    // 先写临时文件再重命名：中途被打断不会留下半张图当成有效缓存
    let final_path = dir.join(cache_key(url));
    let tmp = dir.join(format!("{}.tmp", cache_key(url)));
    if std::fs::write(&tmp, bytes).is_ok() {
        let _ = std::fs::rename(&tmp, &final_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Dummy(usize);

    #[test]
    fn first_call_returns_none_and_marks_pending() {
        // 用一个必然失败的地址：重点是**立刻返回**，不阻塞调用方
        let started = Instant::now();
        let got = get_or_fetch::<Dummy, _>("https://no-such-host-abc.invalid/a.png", |b| {
            Some(Dummy(b.len()))
        });
        assert!(got.is_none(), "首次调用必须立刻返回 None（本帧画占位）");
        assert!(
            started.elapsed() < Duration::from_millis(200),
            "绘制期不允许被网络阻塞，实际耗时 {:?}",
            started.elapsed()
        );
        // 等后台线程收尾，避免影响其它用例
        for _ in 0..100 {
            if !has_pending() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn disk_cache_round_trips_and_keys_are_path_safe() {
        let url = "https://example.com/a/b/c.png?v=1&x=/../etc";
        let key = cache_key(url);
        assert!(!key.contains('/'), "缓存文件名不能带路径分隔符: {}", key);
        assert!(key.ends_with(".png"), "应保留扩展名便于排查: {}", key);
        // 同一 URL 稳定映射到同一文件
        assert_eq!(key, cache_key(url));
        assert_ne!(key, cache_key("https://example.com/a/b/c.png?v=2"));

        let data = b"\x89PNG fake bytes".to_vec();
        disk_put(url, &data);
        assert_eq!(disk_get(url).as_deref(), Some(data.as_slice()));
        let _ = std::fs::remove_file(cache_dir().join(cache_key(url)));
        // 空内容不该被当成有效缓存
        disk_put(url, &[]);
        assert!(disk_get(url).is_none());
    }

    #[test]
    fn failure_is_retried_not_cached_forever() {
        let url = "https://no-such-host-def.invalid/b.png";
        get_or_fetch::<Dummy, _>(url, |b| Some(Dummy(b.len())));
        for _ in 0..100 {
            if !has_pending() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        // 失败后应记为 Failed（带次数），而不是被当成「永远没有」
        let map = states::<Dummy>().lock().unwrap();
        match map.get(url) {
            Some(RemoteState::Failed { tries, .. }) => assert!(*tries >= 1),
            other => panic!("失败应记成 Failed 以便退避重试，实际: {}",
                            if other.is_none() { "缺失" } else { "其它状态" }),
        }
    }
}
