//! 整屏级的大图（铺满屏、四周还溢出去的背景图）：缩放结果只缓存画布里看得见的那块。
//!
//! `draw_image_cached` 对目标超过 4M 像素的图不缓存（整张缩好要几十 MB），每次都逐像素双线性重采样。
//! 云听音乐播放页的背景是一张 300×300 的封面放大成 1948×2710（`left/top: -200rpx`，比屏幕大一圈），
//! 于是每一次整页重画（进播放页那一帧、背景图下完那一帧、换歌）都要把 350 万个像素重采样一遍：
//! 桌面 sample 里占整帧绘制的四成，一加 PLY110 上 15~25ms。而画布里看得见的只有 1270×2371。
//!
//! 只在结果与现画**逐字节相同**的情况下走这里：
//! - 图的左上角不在画布里（`x ≤ 0`、`y ≤ 0`）：离屏与画布原点相同，传给 `draw_image` 的坐标一位不差，
//!   每个像素的算式与直接画完全一样；离屏的宽高取「画布」与「图的右下角 + 1」里小的那个，裁到的列行也一样；
//! - 不带圆角、源图整张不透明：缩出来的像素 alpha 只有 255（图里）和 0（图外），贴回时整段拷贝 / 跳过，
//!   没有「先混到透明底、再混到画布上」的两次舍入；
//! - 看得见的那块至少 [`CROP_MIN_PIXELS`]（补条带那种几十行的细条直接画：一次性的，不往缓存里塞）。
//!
//! 第一次（没缓存）照旧现画，缩好的那块在后台线程上做（`mini-img-crop`），之后的整页重画只是整段拷贝。
use super::*;

/// 自己一张表、自己的预算：放进缩放缓存（32MB）的话一块 12~16MB 能把别的图挤掉 ——
/// 播放页这一块一进来，歌单页的头图（1610×1610，10MB）就被逐出，返回时重缩 18~22ms（一加）
const CROP_BUDGET: usize = 18 << 20;

static CROP_CACHE: std::sync::OnceLock<std::sync::Mutex<ByteLru<std::sync::Arc<Vec<Color>>>>> = std::sync::OnceLock::new();

fn crop_cache() -> &'static std::sync::Mutex<ByteLru<std::sync::Arc<Vec<Color>>>> {
    CROP_CACHE.get_or_init(|| std::sync::Mutex::new(ByteLru::new(CROP_BUDGET)))
}

/// 放掉（`trim_memory`，与缩放缓存一起）
pub(super) fn clear_crops() {
    if let Ok(mut c) = crop_cache().lock() {
        c.clear();
    }
}

/// 占用字节
pub(super) fn crop_bytes() -> usize {
    crop_cache().lock().map(|g| g.bytes()).unwrap_or(0)
}

/// 看得见的那块至少这么大才缓存
const CROP_MIN_PIXELS: u64 = 1_000_000;
/// 最多这么大（与 `draw_image_cached` 的上限同一个量级：一张 16MB）
const CROP_MAX_PIXELS: u64 = 4_200_000;

impl Canvas {
    /// 画了返回 true；不满足条件（见模块注释）返回 false，调用方照旧 `draw_image`
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_image_crop_cached(
        &mut self,
        cache_key: &str,
        img_data: &[u8],
        img_w: u32,
        img_h: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        mode: &str,
        radius: f32,
    ) -> bool {
        if radius > 0.0 || x > 0.0 || y > 0.0 || self.translation != (0.0, 0.0) || cache_key.is_empty() {
            return false;
        }
        // 图的右 / 下边在画布里（或刚出去一点）时按图算，不按画布算：成组透明度的离屏按这一帧的裁剪区开，
        // 局部重画那一帧开 1270×2371、整页重画那一帧开 1270×2372，按画布算就是两块、谁也命中不了谁。
        // 离屏比画布大不要紧：贴回时按裁剪区（画布以内）贴，算式与裁到哪无关
        let fit = |edge: f32, canvas: u32| if edge.ceil() + 1.0 <= canvas as f32 + 8.0 { edge.ceil() + 1.0 } else { canvas as f32 };
        let (ow, oh) = (fit(x + w, self.width), fit(y + h, self.height));
        if ow < 1.0 || oh < 1.0 {
            return false;
        }
        let (ow, oh) = (ow as u32, oh as u32);
        let area = ow as u64 * oh as u64;
        if !(CROP_MIN_PIXELS..=CROP_MAX_PIXELS).contains(&area) {
            return false;
        }
        // 坐标与尺寸按位进键：差一点点的位置画出来就可能差一列
        let key = format!(
            "{cache_key}|crop|{}x{}|{:08x}|{:08x}|{:08x}|{:08x}|{mode}",
            ow,
            oh,
            x.to_bits(),
            y.to_bits(),
            w.to_bits(),
            h.to_bits()
        );
        let hit = crop_cache().lock().ok().and_then(|mut c| c.get(&key).cloned());
        let Some(pixels) = hit else {
            // 没缓存：这一次照旧现画（不比从前慢），缩好的那块交给后台线程做，下一次整页重画就是整段拷贝。
            // 在引擎线程上现做一张要多付一次分配 + 缺页 + 拷贝，比直接画还慢一倍（桌面 10ms 对 5ms）
            if img_data.chunks_exact(4).all(|p| p[3] == 255) {
                spawn_build(key, cache_key, img_data, (img_w, img_h), (x, y, w, h), (ow, oh), mode);
            }
            return false;
        };
        let bounds = self.draw_bounds();
        for row in 0..oh {
            let s = (row * ow) as usize;
            self.blend_row_opaque_fast(0, row as i32, &pixels[s..s + ow as usize], bounds);
        }
        true
    }
}

/// 正在后台缩的那几块（同一块只起一个线程）
static PENDING: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// 还有没缩完的（测试等它）
#[cfg(test)]
fn crop_builds_pending() -> bool {
    PENDING.lock().map(|p| !p.is_empty()).unwrap_or(false)
}

#[allow(clippy::too_many_arguments)]
fn spawn_build(key: String, name: &str, img: &[u8], (iw, ih): (u32, u32), (x, y, w, h): (f32, f32, f32, f32), (ow, oh): (u32, u32), mode: &str) {
    {
        let Ok(mut p) = PENDING.lock() else { return };
        if p.contains(&key) || p.len() >= 4 {
            return;
        }
        p.push(key.clone());
    }
    // 源图拷一份带过去（几百 KB）：调用方手里的是借用
    let (img, mode, name) = (img.to_vec(), mode.to_string(), name.to_string());
    let spawned = std::thread::Builder::new().name("mini-img-crop".into()).spawn(move || {
        let t = std::time::Instant::now();
        // 单线程慢慢缩：引擎线程这一帧正分四个线程现画同一张图，再抢核那一帧更慢（一加上 63 → 105ms）
        super::images::SERIAL.set(true);
        // Canvas::new 是清零的内存（全透明），不用再清一遍；画完把像素整块拿走，不再拷一份
        let mut off = Canvas::new(ow, oh);
        off.draw_image(&img, iw, ih, x, y, w, h, &mode, 0.0);
        let p = std::sync::Arc::new(std::mem::take(&mut off.pixels));
        if let Ok(mut c) = crop_cache().lock() {
            c.insert(key.clone(), p.clone(), p.len() * std::mem::size_of::<Color>());
        }
        if crate::renderer::components::image_net::log_enabled() {
            eprintln!("🖼 后台缩好 {name} 看得见的那块 {ow}×{oh}（{:.1}ms）", t.elapsed().as_secs_f64() * 1000.0);
        }
        if let Ok(mut p) = PENDING.lock() {
            p.retain(|k| *k != key);
        }
    });
    if spawned.is_err() {
        if let Ok(mut p) = PENDING.lock() {
            p.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 铺满、四周溢出的背景图：走缓存那条与逐像素现画逐字节相同（整帧、只画一条的裁剪区、第二次命中缓存）
    #[test]
    fn 溢出画布的大图只缓存看得见的那块_与现画逐字节相同() {
        let (iw, ih) = (30u32, 30u32);
        let img: Vec<u8> = (0..iw * ih).flat_map(|i| [(i * 7 % 256) as u8, (i * 13 % 256) as u8, (i * 3 % 256) as u8, 255]).collect();
        for (x, y, w, h) in [(-339.0f32, -339.0f32, 1948.0f32, 2710.0f32), (0.0, -10.25, 1300.0, 2000.5), (-0.5, 0.0, 1270.0, 900.0)] {
            for clip in [None, Some(crate::Rect::new(100.0, 700.0, 900.0, 65.0))] {
                for round in 0..3 {
                    let mut a = Canvas::new(1270, 1100);
                    let mut b = Canvas::new(1270, 1100);
                    a.clear(Color::WHITE);
                    b.clear(Color::WHITE);
                    if let Some(c) = clip {
                        a.save();
                        a.clip_rect(c);
                        b.save();
                        b.clip_rect(c);
                    }
                    // 第一次没缓存：不画（调用方现画），后台去缩；等它缩好之后都走缓存
                    let used = a.draw_image_crop_cached("t-crop", &img, iw, ih, x, y, w, h, "aspectFill", 0.0);
                    if !used {
                        assert_eq!(round, 0, "({x},{y}) 后台缩好之后该走缓存");
                        a.draw_image(&img, iw, ih, x, y, w, h, "aspectFill", 0.0);
                        while crop_builds_pending() {
                            std::thread::sleep(std::time::Duration::from_millis(2));
                        }
                    }
                    b.draw_image(&img, iw, ih, x, y, w, h, "aspectFill", 0.0);
                    assert!(a.pixels() == b.pixels(), "({x},{y},{w},{h}) 裁剪 {clip:?}：缓存那条与现画不一致");
                }
            }
        }
        // 左上角在画布里 / 带圆角 / 半透明的源图：不接
        let mut c = Canvas::new(1270, 1100);
        assert!(!c.draw_image_crop_cached("t-crop", &img, iw, ih, 3.0, -5.0, 1948.0, 2710.0, "aspectFill", 0.0));
        assert!(!c.draw_image_crop_cached("t-crop", &img, iw, ih, -3.0, -5.0, 1948.0, 2710.0, "aspectFill", 8.0));
        let mut semi = img.clone();
        semi[3] = 128;
        assert!(!c.draw_image_crop_cached("t-crop-semi", &semi, iw, ih, -3.0, -5.0, 1948.0, 2710.0, "aspectFill", 0.0));
    }
}
