//! 原生桥的 I/O 一组：WebSocket、文件系统、音频、图片信息。
//!
//! 从 `bridge.rs` 拆出来（那边已近 500 行）。全都是**同步返回**的薄入口：
//! 真正的活在各自模块里（`websocket` 起线程、`wxfs` 直接读写、`audio_ctx` 改状态），
//! 返回值是空串（成功）/ 错误串，或一段 JSON。
//!
//! 契约测试（`native_bridge_contract_tests`）把这个文件与 `bridge.rs` 一起扫，
//! 新的 `__native_*` 写在这里同样算注册过。

use super::{BridgeEvent, JsRuntime};
use std::sync::{Arc, Mutex};

type Queue = Arc<Mutex<Vec<BridgeEvent>>>;

/// 读像素 / 导出之前：把这块画布还排在桥上的绘制指令先执行掉（按顺序），读到的才是「刚画完」的样子。
/// 别的事件原样留在队列里
fn flush_canvas(queue: &Queue, key: &str) {
    let mine: Vec<String> = {
        let mut q = queue.lock().unwrap();
        let (m, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *q)
            .into_iter()
            .partition(|e| matches!(e, BridgeEvent::CanvasDraw { canvas_id, .. } if canvas_id == key));
        *q = rest;
        m.into_iter()
            .filter_map(|e| match e {
                BridgeEvent::CanvasDraw { commands, .. } => Some(commands),
                _ => None,
            })
            .collect()
    };
    for c in mine {
        crate::renderer::components::execute_canvas_draw(key, &c);
    }
}

fn pre_of(v: &serde_json::Value) -> (f32, f32) {
    let a = v.get("pre").and_then(|p| p.as_array()).cloned().or_else(|| v.as_array().cloned()).unwrap_or_default();
    let g = |i: usize| a.get(i).and_then(|x| x.as_f64()).unwrap_or(1.0) as f32;
    (g(0), g(1))
}

pub(super) fn register(rt: &JsRuntime, queue: Queue) -> Result<(), String> {
    // ── WebSocket：(id, url, headersJson, protocolsJson, timeoutMs) → "" 或错误串 ──
    rt.register_function("__native_socket_connect", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        match crate::websocket::connect(a(0).parse().unwrap_or(0), &a(1), &a(2), &a(3), a(4).parse().unwrap_or(0)) {
            Ok(()) => String::new(),
            Err(e) => e,
        }
    })?;
    // (id, 't' | 'b', 文本或 base64)
    rt.register_function("__native_socket_send", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        let id = a(0).parse().unwrap_or(0);
        let r = if a(1) == "b" { crate::websocket::send(id, None, Some(a(2))) } else { crate::websocket::send(id, Some(a(2)), None) };
        r.err().unwrap_or_default()
    })?;
    // (id, code, reason)
    rt.register_function("__native_socket_close", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        crate::websocket::close(a(0).parse().unwrap_or(0), a(1).parse().unwrap_or(1000), &a(2)).err().unwrap_or_default()
    })?;
    // ── 文件系统：(op, argsJson) → {ok, …} ──
    rt.register_function("__native_fs", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        crate::wxfs::call(&a(0), &a(1))
    })?;
    // ── InnerAudioContext：(op, id, argsJson) → JSON ──
    rt.register_function("__native_audio", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        crate::audio_ctx::call(&a(0), a(1).parse().unwrap_or(0), &a(2))
    })?;
    // ── 图片：getImageInfo (src) / compressImage (src, quality, w, h) ──
    rt.register_function("__native_image_info", move |args| {
        crate::wxfs::image::info(args.first().map(|s| s.as_str()).unwrap_or(""))
    })?;
    // ── Canvas 2D：measureText (size, text) → 宽；读像素 / 导出（先执行排着的指令）──
    rt.register_function("__native_measure_text", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        crate::renderer::components::canvas_measure_text(a(0).parse().unwrap_or(10.0), &a(1)).to_string()
    })?;
    // (key, x, y, w, h, preJson) → base64 RGBA
    let q = queue.clone();
    rt.register_function("__native_canvas_read", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        let key = a(0);
        flush_canvas(&q, &key);
        let pre = pre_of(&serde_json::from_str(&a(5)).unwrap_or(serde_json::Value::Null));
        let (w, h) = (a(3).parse::<f64>().unwrap_or(0.0).max(0.0) as u32, a(4).parse::<f64>().unwrap_or(0.0).max(0.0) as u32);
        let bytes = crate::renderer::components::CANVAS_MANAGER
            .lock()
            .ok()
            .and_then(|m| m.get_existing_context(&key).map(|c| c.read_rgba(a(1).parse().unwrap_or(0.0), a(2).parse().unwrap_or(0.0), w, h, pre)))
            .unwrap_or_else(|| vec![0; (w * h * 4) as usize]);
        crate::wxfs::codec::b64(&bytes)
    })?;
    // (key, 'png' | 'jpeg', quality, json) → data URL
    let q = queue.clone();
    rt.register_function("__native_canvas_encode", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        let key = a(0);
        flush_canvas(&q, &key);
        let jpeg = a(1) == "jpeg";
        let pre = pre_of(&serde_json::from_str(&a(3)).unwrap_or(serde_json::Value::Null));
        let r = crate::renderer::components::CANVAS_MANAGER
            .lock()
            .ok()
            .and_then(|m| m.get_existing_context(&key).map(|c| c.encode(jpeg, a(2).parse().unwrap_or(0.92), None, (0, 0), pre)));
        match r {
            Some(Ok(bytes)) => format!("data:image/{};base64,{}", if jpeg { "jpeg" } else { "png" }, crate::wxfs::codec::b64(&bytes)),
            _ => "data:,".to_string(),
        }
    })?;
    // canvasToTempFilePath：(key, json{x,y,width,height,destWidth,destHeight,fileType,quality,pre}) → {tempFilePath} | {err}
    let q = queue;
    rt.register_function("__native_canvas_save", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        let key = a(0);
        flush_canvas(&q, &key);
        let o: serde_json::Value = serde_json::from_str(&a(1)).unwrap_or(serde_json::Value::Null);
        let n = |k: &str| o.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let jpeg = o.get("fileType").and_then(|v| v.as_str()) == Some("jpg");
        let region = (n("width") > 0.0 && n("height") > 0.0).then(|| (n("x") as f32, n("y") as f32, n("width") as u32, n("height") as u32));
        let r = crate::renderer::components::CANVAS_MANAGER.lock().ok().and_then(|m| {
            m.get_existing_context(&key)
                .map(|c| c.encode(jpeg, n("quality") as f32, region, (n("destWidth") as u32, n("destHeight") as u32), pre_of(&o)))
        });
        let out = match r {
            Some(Ok(bytes)) => {
                let (uri, real) = crate::wxfile::new_temp(if jpeg { "jpg" } else { "png" });
                if let Some(d) = real.parent() {
                    std::fs::create_dir_all(d).ok();
                }
                match std::fs::write(&real, bytes) {
                    Ok(()) => serde_json::json!({ "tempFilePath": uri }),
                    Err(e) => serde_json::json!({ "err": e.to_string() }),
                }
            }
            Some(Err(e)) => serde_json::json!({ "err": e }),
            None => serde_json::json!({ "err": "canvas not found" }),
        };
        out.to_string()
    })?;
    rt.register_function("__native_compress_image", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        crate::wxfs::image::compress(&a(0), a(1).parse::<f64>().unwrap_or(80.0) as u8, a(2).parse().unwrap_or(0), a(3).parse().unwrap_or(0))
    })?;
    Ok(())
}
