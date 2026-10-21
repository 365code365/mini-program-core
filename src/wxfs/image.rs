//! `wx.getImageInfo` / `wx.compressImage` 的原生侧：读尺寸与格式、按质量重新编码成 JPEG。
//!
//! 路径与 `<image src>` 同一套解析（`assets::resolve`：包内路径、`wxfile://`）；
//! 远程图片由 JS 侧先 `downloadFile` 再进来。

use serde_json::json;

fn type_name(f: ::image::ImageFormat) -> &'static str {
    match f {
        ::image::ImageFormat::Jpeg => "jpeg",
        ::image::ImageFormat::Png => "png",
        ::image::ImageFormat::Gif => "gif",
        ::image::ImageFormat::WebP => "webp",
        ::image::ImageFormat::Bmp => "bmp",
        ::image::ImageFormat::Tiff => "tiff",
        _ => "unknown",
    }
}

/// `{width, height, type}` 或 `{err}`
pub fn info(src: &str) -> String {
    let r = (|| -> Result<serde_json::Value, String> {
        let path = crate::assets::resolve(src).ok_or_else(|| format!("file not found: {src}"))?;
        let reader = ::image::ImageReader::open(&path).map_err(|e| e.to_string())?.with_guessed_format().map_err(|e| e.to_string())?;
        let fmt = reader.format().map(type_name).unwrap_or("unknown");
        let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
        Ok(json!({ "width": w, "height": h, "type": fmt }))
    })();
    r.unwrap_or_else(|e| json!({ "err": e })).to_string()
}

/// 重新编码：`quality` 0~100（微信默认 80）；`w` / `h` 给了就缩放（只给一个时按比例）。
/// 结果写到 `wxfile://tmp_…jpg`，返回 `{tempFilePath}` 或 `{err}`
pub fn compress(src: &str, quality: u8, w: u32, h: u32) -> String {
    let r = (|| -> Result<serde_json::Value, String> {
        let path = crate::assets::resolve(src).ok_or_else(|| format!("file not found: {src}"))?;
        let mut img = ::image::open(&path).map_err(|e| e.to_string())?;
        let (iw, ih) = (img.width().max(1), img.height().max(1));
        let (tw, th) = match (w, h) {
            (0, 0) => (iw, ih),
            (w, 0) => (w, ((w as f64) * ih as f64 / iw as f64).round().max(1.0) as u32),
            (0, h) => (((h as f64) * iw as f64 / ih as f64).round().max(1.0) as u32, h),
            (w, h) => (w, h),
        };
        if (tw, th) != (iw, ih) {
            img = img.resize_exact(tw, th, ::image::imageops::FilterType::Triangle);
        }
        let (uri, real) = crate::wxfile::new_temp("jpg");
        if let Some(d) = real.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let file = std::fs::File::create(&real).map_err(|e| e.to_string())?;
        let mut enc = ::image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), quality.clamp(1, 100));
        enc.encode_image(&img.to_rgb8()).map_err(|e| e.to_string())?;
        Ok(json!({ "tempFilePath": uri }))
    })();
    r.unwrap_or_else(|e| json!({ "err": e })).to_string()
}
