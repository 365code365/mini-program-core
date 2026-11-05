//! SVG 图片：第一次解码时用 resvg 栅格化成 RGBA，之后与位图走同一份缓存（`<image>` 的 IMAGE_CACHE、
//! canvas drawImage 的 image_src 缓存），每帧不再碰 SVG。
//!
//! 栅格化倍数 [`RASTER_SCALE`]：解码时不知道元素尺寸与 dpr（缓存键只有 src），按固有尺寸 × 2 画一张，
//! 页面上缩小走现有的 mip / 缩放缓存（2~3 倍屏上图标边缘清楚，1 倍屏是缩小，也清楚）。
//! 代价（已知差别，记在计划 WP-E）：`<image>` 的 bindload 报的是栅格尺寸（固有 × 2），
//! 裁剪类 mode（top / center …，「原图不缩放」）把 SVG 画成固有尺寸的两倍；canvas 那条路用固有尺寸（image_src.rs）。
//!
//! feature `svg` 关掉时 [`rasterize`] 恒为 None：.svg 的 `<image>` 走 binderror。
//! 不画 SVG 里的 `<text>` 与内嵌位图（resvg 的 text / raster-images 没开，见 Cargo.toml）。

/// 栅格倍数（相对 SVG 的固有尺寸）
pub const RASTER_SCALE: f32 = 2.0;
/// 栅格长边上限（与本地大图的 `LARGE_MAX_SIDE` 同一档）
const MAX_SIDE: f32 = 2048.0;

/// 是不是 SVG：扩展名（去掉 `?` / `#`）或内容开头（跳过 BOM 与空白后是 `<svg` / `<?xml … <svg`）
pub fn is_svg(src: &str, bytes: &[u8]) -> bool {
    let path = src.split(['?', '#']).next().unwrap_or(src);
    if path.len() >= 4 && path[path.len() - 4..].eq_ignore_ascii_case(".svg") {
        return true;
    }
    if src.starts_with("data:image/svg") {
        return true;
    }
    let head = &bytes[..bytes.len().min(512)];
    let head = head.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(head);
    let start = head.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(head.len());
    let head = &head[start..];
    if head.starts_with(b"<svg") {
        return true;
    }
    // `<?xml …?>` 开头的：前 512 字节里出现 `<svg` 才算（别的 XML 不是图）
    head.starts_with(b"<?xml") && head.windows(4).any(|w| w == b"<svg")
}

/// 栅格化结果：非预乘 RGBA、栅格宽高、SVG 的固有宽高（CSS px）
pub struct Raster {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub natural: (f32, f32),
}

/// 按固有尺寸 × [`RASTER_SCALE`]（长边夹到 2048）栅格化。解析失败 / 尺寸为 0 / feature 关掉返回 None
#[cfg(feature = "svg")]
pub fn rasterize(bytes: &[u8]) -> Option<Raster> {
    use resvg::{tiny_skia, usvg};
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default()).ok()?;
    let size = tree.size();
    let (nw, nh) = (size.width(), size.height());
    if !(nw > 0.0 && nh > 0.0 && nw.is_finite() && nh.is_finite()) {
        return None;
    }
    let s = RASTER_SCALE.min(MAX_SIDE / nw.max(nh));
    let (w, h) = (((nw * s).round() as u32).max(1), ((nh * s).round() as u32).max(1));
    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    let t = tiny_skia::Transform::from_scale(w as f32 / nw, h as f32 / nh);
    resvg::render(&tree, t, &mut pixmap.as_mut());
    // tiny-skia 是预乘 alpha，引擎的位图一律非预乘
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    Some(Raster { rgba, width: w, height: h, natural: (nw, nh) })
}

#[cfg(not(feature = "svg"))]
pub fn rasterize(_bytes: &[u8]) -> Option<Raster> {
    None
}

/// 本地 .svg 文件的固有尺寸（取整的 CSS px，排 widthFix / heightFix 用）：只解析、不栅格化。不是 SVG / 解析失败返回 None
pub fn natural_size_of_file(src: &str, path: &std::path::Path) -> Option<(u32, u32)> {
    let bytes = std::fs::read(path).ok()?;
    if !is_svg(src, &bytes) {
        return None;
    }
    natural_size(&bytes).map(|(w, h)| ((w.round() as u32).max(1), (h.round() as u32).max(1)))
}

#[cfg(feature = "svg")]
fn natural_size(bytes: &[u8]) -> Option<(f32, f32)> {
    let t = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).ok()?;
    Some((t.size().width(), t.size().height()))
}

#[cfg(not(feature = "svg"))]
fn natural_size(_bytes: &[u8]) -> Option<(f32, f32)> {
    None
}

/// 位图或 SVG 一律解成 (RGBA, 宽, 高, 固有宽高)：canvas 的图源用（image_src.rs）。
/// 位图的固有尺寸就是像素尺寸；解码结果与 `image::load_from_memory(..).to_rgba8()` 逐字节相同
pub fn decode_any(src: &str, bytes: &[u8]) -> Option<Raster> {
    if is_svg(src, bytes) {
        return rasterize(bytes);
    }
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some(Raster { rgba: img.into_raw(), width: w, height: h, natural: (w as f32, h as f32) })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10" viewBox="0 0 20 10">
        <rect x="0" y="0" width="10" height="10" fill="#ff0000"/></svg>"##;

    #[test]
    fn 认得出_svg() {
        assert!(is_svg("/a/b.SVG?x=1", b""));
        assert!(is_svg("x", b"\xEF\xBB\xBF  <svg xmlns='..'/>"));
        assert!(is_svg("x", b"<?xml version=\"1.0\"?>\n<svg/>"));
        assert!(!is_svg("a.png", b"\x89PNG"));
        assert!(!is_svg("a.xml", b"<?xml version=\"1.0\"?><plist/>"));
    }

    #[cfg(feature = "svg")]
    #[test]
    fn 按固有尺寸两倍栅格化_左半红右半透明() {
        let r = rasterize(SVG.as_bytes()).expect("应能解出");
        assert_eq!((r.width, r.height, r.natural), (40, 20, (20.0, 10.0)));
        let px = |x: u32, y: u32| &r.rgba[((y * r.width + x) * 4) as usize..][..4];
        assert_eq!(px(5, 10), &[255, 0, 0, 255]);
        assert_eq!(px(30, 10)[3], 0, "右半没画");
        assert!(rasterize(b"<svg").is_none(), "坏的 SVG 不出图");
    }
}
