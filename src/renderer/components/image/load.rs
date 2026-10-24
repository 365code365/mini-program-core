//! 图片的加载与解码：本地读盘、远程异步下载、GIF 多帧、静态图解码。
//!
//! `image` 模块的一片：从 544 行的 image.rs 按职责拆出来（**纯搬迁**，函数体一行没改，
//! 只把可见性从私有放宽到 `pub(super)`），绘制与布局留在 `mod.rs`。
use super::*;

/// 尝试把字节解码为多帧动图；单帧或非 GIF 返回 None。
pub(super) fn decode_animated_gif(bytes: &[u8]) -> Option<AnimatedImage> {
    // 仅对 GIF magic 尝试多帧解码，避免无谓开销
    if bytes.len() < 6 || &bytes[..4] != b"GIF8" {
        return None;
    }
    use image::codecs::gif::GifDecoder;
    use image::AnimationDecoder;

    let decoder = GifDecoder::new(std::io::Cursor::new(bytes.to_vec())).ok()?;
    let frames = decoder.into_frames().collect_frames().ok()?;
    if frames.len() <= 1 {
        return None;
    }

    let mut out: Vec<(Arc<Vec<u8>>, u32)> = Vec::with_capacity(frames.len());
    let mut width = 0;
    let mut height = 0;
    let mut total_ms: u32 = 0;
    for frame in frames {
        let (numer, denom) = frame.delay().numer_denom_ms();
        // GIF 常见 0/1 延时表示「尽快」，浏览器按 100ms 处理，这里保持一致
        let delay = if denom == 0 { 100 } else { (numer / denom.max(1)).max(10) };
        let buffer = frame.into_buffer();
        width = buffer.width();
        height = buffer.height();
        total_ms = total_ms.saturating_add(delay);
        out.push((Arc::new(buffer.into_raw()), delay));
    }
    if out.is_empty() || width == 0 || height == 0 {
        return None;
    }
    Some(AnimatedImage {
        frames: out,
        width,
        height,
        total_ms: total_ms.max(1),
        started: std::time::Instant::now(),
    })
}

/// 加载图片（支持网络URL和本地文件；GIF 动图按时间取当前帧）
pub(super) fn load_image(src: &str) -> Option<Arc<ImageData>> {
    // 动图：命中后按经过时间取帧，实现循环播放
    {
        let cache = get_anim_cache();
        if let Ok(mut cache_guard) = cache.lock() {
            if let Some(anim) = cache_guard.get(src) {
                let index = anim.current_index();
                let (frame, _) = &anim.frames[index];
                return Some(Arc::new(ImageData {
                    data: frame.clone(), // Arc clone，非深拷贝
                    width: anim.width,
                    height: anim.height,
                    frame_index: index as u32,
                }));
            }
        }
    }

    // 检查缓存
    {
        let cache = get_image_cache();
        let mut cache_guard = cache.lock().ok()?;
        if let Some(cached) = cache_guard.get(src) {
            return cached.clone();
        }
    }

    // 远程图走**异步**路径：绘制期只查表，没有就后台下载，本帧先画占位。
    // 同步下载会把渲染线程钉在 HTTP 往返上（实测单张 276KB 的图 3.2s），
    // 长列表里每滚出一张新图就卡一次 —— 这是「滑动很卡」的直接来源。
    if src.starts_with("http://") || src.starts_with("https://") {
        let url = src.to_string();
        return crate::renderer::components::image_net::get_or_fetch::<ImageData, _>(src, move |bytes| {
            decode_with_animation(&url, bytes)
        });
    }

    // 本地大文件（选图 / 拍照的原图、下载的大图）：后台解码并缩到长边 [`LARGE_MAX_SIDE`]，本帧先画占位。
    // 小文件（包内图标、切图）照旧同步：读盘 + 解码很快，也避免首帧闪空、逐字节基线不变
    if let Some(path) = crate::assets::resolve(src) {
        if std::fs::metadata(&path).map(|m| m.len() > LARGE_FILE_BYTES).unwrap_or(false) {
            let key = src.to_string();
            return crate::renderer::components::image_net::get_or_load::<ImageData, _>(&format!("local:{src}"), move || {
                let bytes = std::fs::read(&path).ok()?;
                decode_with_animation(&key, &bytes).map(cap_side)
            });
        }
    }

    // 本地文件：读盘 + 解码很快，保持同步（也避免首帧闪空）。
    // 「很快」是桌面上的说法：一加上一张 1024² 的 jpg 也要十几毫秒，诊断开着时记进这一帧的〔慢：…〕
    let t = crate::renderer::frame_cost::start();
    let result = load_image_from_file(src);
    crate::renderer::frame_cost::record("解码", t, || match &result {
        Some(d) => format!("{} {}×{}", crate::renderer::frame_cost::short(src), d.width, d.height),
        None => format!("{} 失败", crate::renderer::frame_cost::short(src)),
    });

    // 存入缓存（Arc 共享，后续帧零拷贝取用）
    let result = result.map(Arc::new);
    {
        let cache = get_image_cache();
        if let Ok(mut cache_guard) = cache.lock() {
            let bytes = image_bytes(&result);
            cache_guard.insert(src.to_string(), result.clone(), bytes);
        }
    }

    result
}

/// 本地文件超过这么大就后台解码（包内的图几乎都在 300KB 以下）
const LARGE_FILE_BYTES: u64 = 768 * 1024;
/// 后台解码的大图缩到这个长边：页面上最大也就整屏宽（1080 物理像素），预览全屏 2× 也够。
/// 1200 万像素的原图解码后 48MB，三张就把图片缓存挤爆、每帧逐出重解 —— 选完图整页卡死的另一半原因
const LARGE_MAX_SIDE: u32 = 2048;

fn cap_side(img: ImageData) -> ImageData {
    let (w, h) = (img.width, img.height);
    if w.max(h) <= LARGE_MAX_SIDE || img.frame_index != 0 {
        return img;
    }
    let Some(buf) = image::RgbaImage::from_raw(w, h, (*img.data).clone()) else { return img };
    // thumbnail：面积平均，比 resize 快得多，缩小时质量足够
    let s = LARGE_MAX_SIDE as f32 / w.max(h) as f32;
    let small = image::imageops::thumbnail(&buf, ((w as f32 * s) as u32).max(1), ((h as f32 * s) as u32).max(1));
    let (nw, nh) = small.dimensions();
    ImageData { data: Arc::new(small.into_raw()), width: nw, height: nh, frame_index: 0 }
}

/// 从本地文件加载图片（按宿主登记的小程序根目录解析包内绝对路径）
pub(super) fn load_image_from_file(path: &str) -> Option<ImageData> {
    let resolved = crate::assets::resolve(path)?;
    let bytes = std::fs::read(&resolved).ok()?;
    decode_with_animation(path, &bytes)
}

/// 解码字节：若为多帧 GIF 则登记到动图缓存并返回首帧，否则按静态图解码。
pub(super) fn decode_with_animation(src: &str, bytes: &[u8]) -> Option<ImageData> {
    if let Some(anim) = decode_animated_gif(bytes) {
        let first = ImageData {
            data: anim.frames[0].0.clone(),
            width: anim.width,
            height: anim.height,
            frame_index: 0,
        };
        if let Ok(mut cache) = get_anim_cache().lock() {
            let bytes = anim_bytes(&anim);
            cache.insert(src.to_string(), Arc::new(anim), bytes);
        }
        return Some(first);
    }
    decode_image_bytes(bytes)
}

/// 解码图片字节数据
pub(super) fn decode_image_bytes(bytes: &[u8]) -> Option<ImageData> {
    use image::GenericImageView;
    
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();
    
    Some(ImageData {
        data: Arc::new(rgba.into_raw()),
        width,
        height,
        frame_index: 0,
    })
}
