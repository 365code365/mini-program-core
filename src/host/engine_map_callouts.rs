//! 同层 `<map>` 的自定义气泡交给宿主（`sameLayerViews` 事件里每个地图的 `callouts`）：
//! 渲染器把 `slot="callout"` 里每个带 marker-id 的 cover-view 离屏画成 RGBA（`wxml_renderer::map_callout`），
//! 这里存成 PNG 放进 wxfile 的临时目录，交出 `{markerId, path, width, height}`（宽高是逻辑像素）。
//!
//! 文件名是像素内容的哈希：内容不变就是同一个路径，`sameLayerViews` 的签名不变、不重发，宿主的图标缓存也直接命中；
//! 已经存在的文件不再编码。只在 `sync_host_views` 重新数同层组件的时候（布局树整棵重建）走一遍，
//! 没有 slot="callout" 的地图只是一次树上查找。
use crate::renderer::WxmlRenderer;
use serde_json::{json, Value};
use std::hash::{Hash, Hasher};

pub(super) fn export(r: &mut WxmlRenderer, key: &str, dpr: f32) -> Value {
    let list = r.map_custom_callouts(key);
    if list.is_empty() {
        return Value::Array(Vec::new());
    }
    let dir = crate::wxfile::temp_dir();
    let dpr = dpr.max(0.1);
    let mut out = Vec::new();
    for (id, w, h, rgba) in list {
        let mut hs = std::collections::hash_map::DefaultHasher::new();
        (w, h).hash(&mut hs);
        rgba.hash(&mut hs);
        let path = dir.join(format!("map_callout_{:016x}.png", hs.finish()));
        if !path.is_file() {
            let Ok(png) = encode_png(w, h, &rgba) else { continue };
            if std::fs::write(&path, png).is_err() {
                continue;
            }
        }
        out.push(json!({
            "markerId": id,
            "path": path.to_string_lossy(),
            "width": w as f32 / dpr,
            "height": h as f32 / dpr,
        }));
    }
    Value::Array(out)
}

fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    use image::ImageEncoder as _;
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(rgba, w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    Ok(out)
}
