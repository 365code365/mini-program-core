//! canvas 的图源：`drawImage` / `createPattern` / `createImage` 的 src → 解码好的 RGBA（带缓存）。
//!
//! 从前 `draw_image` 每条指令都读盘 + 解码一次（动画每帧画同一张图就每帧解一次），且 3 / 5 参数没写尺寸时
//! 按 100×100 画、9 参数的源矩形整个丢掉。现在：
//! - **缓存**：按 src 记一份（本地文件带 (长度, 修改时间) 戳，文件被改写了重新解）；总量超过 [`CAP_BYTES`] 先逐出最早的；
//! - **src**：包内路径 / `wxfile://`（与 `<image>` 同一套解析）、相对进程目录（旧行为）、`data:` URI（base64 或百分号编码，
//!   `toDataURL` 的结果与 SVG 的 data URI 都认）、SVG（image/svg.rs 栅格化成固有尺寸 × 2）；远程图由 JS 的 createImage 先下载成本地文件；
//! - **drawImage 的参数**（HTML 标准）：3 参数按图的固有尺寸、5 参数缩放到目标矩形、9 参数源矩形 → 目标矩形；
//!   源矩形与目标尺寸都是**固有尺寸**的坐标（SVG 的栅格是两倍，这里换算）。
//!
//! 走哪条光栅路径（每条指令开销不变大：缓存命中是一次表查找 + 一次 stat）：
//! - 没有源矩形、矩阵只有平移 + 等比缩放、globalAlpha = 1、没有合成模式 / 阴影：沿用旧的 `Canvas::draw_image`
//!   （scaleToFill，有缩放缓存），像素与从前逐字节相同；
//! - 其余（9 参数、旋转 / 斜切 / 不等比缩放、半透明、合成模式）：`draw_image_rgba`（image2d.rs，仿射反算 + 双线性，小游戏同一条）。
use super::image2d::ImageRects;
use super::Canvas2DContext;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// 一张解码好的图源
pub struct ImgSrc {
    /// 非预乘 RGBA8，`w × h`
    pub rgba: Arc<Vec<u8>>,
    pub w: u32,
    pub h: u32,
    /// 固有尺寸（CSS px）：位图 = 像素尺寸；SVG = width / height（或 viewBox）
    pub natural: (f32, f32),
}

/// 缓存总量上限（解码后的字节）
const CAP_BYTES: usize = 64 << 20;

#[derive(Default)]
struct Cache {
    map: HashMap<String, (Option<(u64, std::time::SystemTime)>, Arc<ImgSrc>)>,
    order: VecDeque<String>,
    bytes: usize,
}

static CACHE: once_cell::sync::Lazy<Mutex<Cache>> = once_cell::sync::Lazy::new(|| Mutex::new(Cache::default()));

/// 本地文件路径与它的 (长度, 修改时间)
fn local_path(src: &str) -> Option<std::path::PathBuf> {
    crate::assets::resolve(src)
        .filter(|p| p.is_file())
        .or_else(|| Some(std::path::PathBuf::from(src)).filter(|p| p.is_file()))
        .or_else(|| Some(std::path::PathBuf::from(src.trim_start_matches('/'))).filter(|p| p.is_file()))
}

fn stamp(path: &std::path::Path) -> Option<(u64, std::time::SystemTime)> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.len(), m.modified().ok()?))
}

/// `data:[<mime>][;base64],<数据>`（`<image>` 的 data: 图源也用它，image/load.rs）
pub fn data_uri(src: &str) -> Option<Vec<u8>> {
    let rest = src.strip_prefix("data:")?;
    let (head, body) = rest.split_once(',')?;
    if head.ends_with(";base64") {
        return crate::wxfs::codec::unb64(body.trim());
    }
    // 百分号编码（`data:image/svg+xml,%3Csvg…` 常见写法）
    let b = body.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).ok()?, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    Some(out)
}

/// 取一张图源（命中缓存不解码）。读不到 / 解不了返回 None
pub fn source(src: &str) -> Option<Arc<ImgSrc>> {
    if src.is_empty() {
        return None;
    }
    let path = if src.starts_with("data:") { None } else { Some(local_path(src)?) };
    let st = path.as_deref().and_then(stamp);
    if let Ok(c) = CACHE.lock() {
        if let Some((s, img)) = c.map.get(src) {
            if *s == st {
                return Some(img.clone());
            }
        }
    }
    let bytes = match &path {
        Some(p) => std::fs::read(p).ok()?,
        None => data_uri(src)?,
    };
    let r = crate::renderer::components::image::svg::decode_any(src, &bytes)?;
    let img = Arc::new(ImgSrc { rgba: Arc::new(r.rgba), w: r.width, h: r.height, natural: r.natural });
    if let Ok(mut c) = CACHE.lock() {
        let size = img.rgba.len();
        if let Some((_, old)) = c.map.remove(src) {
            c.bytes -= old.rgba.len();
            c.order.retain(|k| k != src);
        }
        while c.bytes + size > CAP_BYTES {
            let Some(k) = c.order.pop_front() else { break };
            if let Some((_, old)) = c.map.remove(&k) {
                c.bytes -= old.rgba.len();
            }
        }
        if size <= CAP_BYTES {
            c.bytes += size;
            c.order.push_back(src.to_string());
            c.map.insert(src.to_string(), (st, img.clone()));
        }
    }
    Some(img)
}

/// `canvas.createImage()` 的加载：`{"width", "height"}`（固有尺寸）或 `{"err"}`（JS 侧发 onerror）。解出来的图留在缓存里
pub fn info_json(src: &str) -> String {
    match source(src) {
        Some(img) => serde_json::json!({ "width": img.natural.0, "height": img.natural.1 }).to_string(),
        None => serde_json::json!({ "err": format!("load image fail: {src}") }).to_string(),
    }
}

/// 一条 `drawImage` 指令：`{src, dx, dy[, dWidth, dHeight][, sx, sy, sWidth, sHeight]}`
pub(super) fn draw_image_cmd(ctx: &mut Canvas2DContext, cmd: &serde_json::Value) {
    let Some(src) = cmd.get("src").and_then(|v| v.as_str()) else { return };
    let Some(img) = source(src) else { return };
    let f = |k: &str| cmd.get(k).and_then(|v| v.as_f64()).map(|v| v as f32);
    let (nw, nh) = img.natural;
    let (dx, dy) = (f("dx").unwrap_or(0.0), f("dy").unwrap_or(0.0));
    let (dw, dh) = (f("dWidth").unwrap_or(nw), f("dHeight").unwrap_or(nh));
    let src_rect = cmd.get("sWidth").is_some().then(|| (f("sx").unwrap_or(0.0), f("sy").unwrap_or(0.0), f("sWidth").unwrap_or(nw), f("sHeight").unwrap_or(nh)));
    let m = ctx.transform;
    // 负宽高（HTML：规范化、不翻转）也交给 draw_image_rgba
    let simple = src_rect.is_none() && dw >= 0.0 && dh >= 0.0 && m[1] == 0.0 && m[2] == 0.0 && m[0] == m[3] && m[0] > 0.0 && ctx.global_alpha >= 1.0 && ctx.fx.trivial_for_image();
    if simple {
        return ctx.draw_image_scaled(&img, dx, dy, dw, dh);
    }
    // 固有尺寸坐标 → 栅格像素（SVG 栅格是固有尺寸的两倍；位图 k = 1）
    let (kx, ky) = (img.w as f32 / nw.max(1e-6), img.h as f32 / nh.max(1e-6));
    let (sx, sy, sw, sh) = src_rect.unwrap_or((0.0, 0.0, nw, nh));
    let r = ImageRects { sx: sx * kx, sy: sy * ky, sw: sw * kx, sh: sh * ky, dx, dy, dw, dh };
    if let Some(r) = r.normalize(img.w, img.h) {
        ctx.draw_image_rgba(&img.rgba, img.w, img.h, r, true, None);
    }
}

impl Canvas2DContext {
    /// 旧路径：目标矩形按平均缩放换到设备像素，`Canvas::draw_image` scaleToFill（带缩放缓存）
    fn draw_image_scaled(&mut self, img: &ImgSrc, dx: f32, dy: f32, dw: f32, dh: f32) {
        let (tw, th) = (dw * self.avg_scale(), dh * self.avg_scale());
        let (px, py) = self.tp(dx, dy);
        if let Ok(mut canvas) = self.canvas.lock() {
            canvas.draw_image(&img.rgba, img.w, img.h, px, py, tw, th, "scaleToFill", 0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_uri_两种编码() {
        assert_eq!(data_uri("data:text/plain,a%3Cb%3e").unwrap(), b"a<b>");
        assert_eq!(data_uri("data:text/plain;base64,aGk=").unwrap(), b"hi");
        assert!(data_uri("nope").is_none());
    }
}
