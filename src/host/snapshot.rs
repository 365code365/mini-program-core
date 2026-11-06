//! `<snapshot>` 的 `takeSnapshot`（官方 api/skyline/Snapshot.takeSnapshot）：宿主一侧，SDK 与桌面共用。
//!
//! 逻辑层（query.js `__snapshotNode`）发 `__native_host_request('snapshot', {seq, cid, type, format})`，这里在下一次
//! 出帧前把节点子树离屏画出来（`WxmlRenderer::snapshot_subtree`），按 format 编成 PNG 或原样 RGBA：
//! - `type: 'file'`（默认）：写进 `wxfile://tmp/…`（png 用 .png、rgba 用 .rgba），回 `{tempFilePath}`；
//! - `type: 'arraybuffer'`：base64 交给逻辑层转 ArrayBuffer，回 `{data}`。
//!
//! 两种都另带 `width` / `height`（物理像素；官方没列，rgba 没有它们解不开）。页面还没布局时留到下一帧。
//! 没有请求的帧只是一次「队列里有没有这一种」的查找
use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;
use crate::ui::InteractionManager;
use base64::Engine as _;
use serde_json::{json, Value};

/// 答这一帧的截图请求。返回答了几条
pub fn tick(app: &mut MiniApp, r: Option<&mut WxmlRenderer>, im: &InteractionManager) -> usize {
    let reqs = app.take_host_requests("snapshot");
    if reqs.is_empty() {
        return 0;
    }
    let Some(r) = r.filter(|r| r.has_layout()) else {
        for q in reqs {
            app.push_host_request("snapshot", q);
        }
        return 0;
    };
    let n = reqs.len();
    for q in reqs {
        let seq = q.get("seq").and_then(Value::as_u64).unwrap_or(0);
        let res = take(r, im, &q);
        let (ok, payload) = match res {
            Ok(v) => (true, v),
            Err(e) => (false, json!({ "errMsg": format!("takeSnapshot:fail {e}") })),
        };
        if let Err(e) = app.eval(&format!("__snapshotDone({seq}, {ok}, {payload})")) {
            eprintln!("⚠️ takeSnapshot 回调失败：{e}");
        }
    }
    app.pump_jobs();
    n
}

fn take(r: &mut WxmlRenderer, im: &InteractionManager, q: &Value) -> Result<Value, String> {
    let cid = q.get("cid").and_then(Value::as_str).unwrap_or("");
    let rgba_out = q.get("format").and_then(Value::as_str) == Some("rgba");
    let file = q.get("type").and_then(Value::as_str) != Some("arraybuffer");
    let (w, h, rgba) = r.snapshot_subtree(cid, im)?;
    let bytes = if rgba_out { rgba } else { encode_png(w, h, rgba)? };
    if file {
        let (uri, path) = crate::wxfs::wxfile::new_temp(if rgba_out { "rgba" } else { "png" });
        crate::wxfs::wxfile::temp_dir(); // 临时目录不存在就建
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
        Ok(json!({ "tempFilePath": uri, "width": w, "height": h }))
    } else {
        Ok(json!({ "base64": base64::engine::general_purpose::STANDARD.encode(&bytes), "width": w, "height": h }))
    }
}

fn encode_png(w: u32, h: u32, rgba: Vec<u8>) -> Result<Vec<u8>, String> {
    use image::ImageEncoder as _;
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    Ok(out)
}
