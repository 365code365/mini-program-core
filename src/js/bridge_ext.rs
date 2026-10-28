//! 补齐微信 API 时加的那批同步桥入口（与 `bridge_io.rs` 同一种写法：薄入口，活在各自模块里）：
//!
//! | 入口 | 给谁 |
//! |---|---|
//! | `__native_random(n)` | `wx.getRandomValues`、`UserCryptoManager.getRandomValues`（base64） |
//! | `__native_local_ip()` | `wx.getLocalIPAddress` |
//! | `__native_storage_keys()` / `__native_storage_size()` | `getStorageInfo(Sync)`：全部键（含上次会话存下、本次还没读过的）与字节数 |
//! | `__native_tcp_*` / `__native_udp_*` / `__native_net_close` | `wx.createTCPSocket` / `wx.createUDPSocket`（`crate::net_socket`） |
//! | `__native_video_info(src)` | `wx.getVideoInfo`（`crate::wxfs::video_info`） |
//! | `__native_canvas_offscreen(key, w, h)` | `wx.createOffscreenCanvas`：没有 `<canvas>` 元素也要有后备缓冲 |
//!
//! 契约测试（`native_bridge_contract_tests`）连同这个文件一起扫。
use super::runtime::JsRuntime;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(super) fn register(rt: &JsRuntime, storage: Arc<Mutex<HashMap<String, String>>>) -> Result<(), String> {
    rt.register_function("__native_random", move |args| {
        let n = args.first().and_then(|s| s.parse::<usize>().ok()).unwrap_or(0).min(65536);
        let mut buf = vec![0u8; n];
        match getrandom::getrandom(&mut buf) {
            Ok(()) => crate::wxfs::codec::b64(&buf),
            Err(_) => String::new(),
        }
    })?;
    rt.register_function("__native_local_ip", move |_| crate::net_socket::local_ip().unwrap_or_default())?;

    let s = storage.clone();
    rt.register_function("__native_storage_keys", move |_| {
        let map = s.lock().unwrap();
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        serde_json::to_string(&keys).unwrap_or_else(|_| "[]".into())
    })?;
    // 微信按「键 + 值」的 UTF-8 字节算占用
    let s = storage;
    rt.register_function("__native_storage_size", move |_| {
        s.lock().unwrap().iter().map(|(k, v)| k.len() + v.len()).sum::<usize>().to_string()
    })?;

    // ── TCP：(id, address, port, timeoutMs)；写入走 bin_stash.rs 的 __native_tcp_write_bin ──
    rt.register_function("__native_tcp_connect", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        crate::net_socket::tcp_connect(a(0).parse().unwrap_or(0), &a(1), a(2).parse().unwrap_or(0), a(3).parse().unwrap_or(2000))
            .err()
            .unwrap_or_default()
    })?;
    rt.register_function("__native_net_close", move |args| {
        crate::net_socket::close(args.first().and_then(|s| s.parse().ok()).unwrap_or(0)).err().unwrap_or_default()
    })?;
    // ── UDP：bind (id, port, ipv6) → 端口或 "err:…"；send (id, addr, port, b64, broadcast) ──
    rt.register_function("__native_udp_bind", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        match crate::net_socket::udp_bind(a(0).parse().unwrap_or(0), a(1).parse().unwrap_or(0), a(2) == "1") {
            Ok(p) => p.to_string(),
            Err(e) => format!("err:{e}"),
        }
    })?;
    rt.register_function("__native_udp_connect", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        crate::net_socket::udp_connect(a(0).parse().unwrap_or(0), &a(1), a(2).parse().unwrap_or(0)).err().unwrap_or_default()
    })?;
    rt.register_function("__native_udp_ttl", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        crate::net_socket::udp_set_ttl(a(0).parse().unwrap_or(0), a(1).parse().unwrap_or(64)).err().unwrap_or_default()
    })?;

    // createBufferURL：(base64) → wxfile://tmp/…（扩展名按内容认，`<image>` / `<video>` 照常解码）
    rt.register_function("__native_temp_write", move |args| {
        let bytes = crate::wxfs::codec::unb64(args.first().map(|s| s.as_str()).unwrap_or("")).unwrap_or_default();
        let (uri, real) = crate::wxfile::new_temp(sniff_ext(&bytes));
        if let Some(dir) = real.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        match std::fs::write(&real, &bytes) {
            Ok(()) => uri,
            Err(_) => String::new(),
        }
    })?;
    // revokeBufferURL：只删 wxfile://tmp 下的文件
    rt.register_function("__native_temp_remove", move |args| {
        let uri = args.first().map(|s| s.as_str()).unwrap_or("");
        if uri.starts_with("wxfile://tmp/") {
            if let Some(p) = crate::wxfile::to_path(uri) {
                std::fs::remove_file(p).ok();
            }
        }
        "undefined".to_string()
    })?;
    // WebAudioContext.decodeAudioData：(路径) → {sampleRate, channels, pcm: 交错 f32le 的 base64} 或 {errCode, errMsg}。
    // 解码与 InnerAudioContext 同一套（audio_ctx::decode：WAV 自己解析，mp3 / AAC 交给 symphonia）
    rt.register_function("__native_audio_decode", move |args| {
        match crate::audio_ctx::decode::load(args.first().map(|s| s.as_str()).unwrap_or("")) {
            Ok(p) => {
                let bytes: Vec<u8> = p.samples.iter().flat_map(|s| s.to_le_bytes()).collect();
                serde_json::json!({ "sampleRate": p.rate, "channels": p.channels, "pcm": crate::wxfs::codec::b64(&bytes) }).to_string()
            }
            Err((code, msg)) => serde_json::json!({ "errCode": code, "errMsg": msg }).to_string(),
        }
    })?;
    rt.register_function("__native_video_info", move |args| {
        crate::wxfs::video_info::info_json(args.first().map(|s| s.as_str()).unwrap_or(""))
    })?;
    // (key, 逻辑宽, 逻辑高)：建 / 对齐后备缓冲（离屏画布按 1 倍像素比，与 <canvas type="2d"> 的 width/height 语义一致）
    rt.register_function("__native_canvas_offscreen", move |args| {
        let a = |i: usize| args.get(i).cloned().unwrap_or_default();
        let (w, h) = (a(1).parse::<f64>().unwrap_or(300.0).max(1.0) as u32, a(2).parse::<f64>().unwrap_or(150.0).max(1.0) as u32);
        if let Ok(mut m) = crate::renderer::components::CANVAS_MANAGER.lock() {
            m.sync_backing(&a(0), w.min(4096), h.min(4096), 1.0);
        }
        "undefined".to_string()
    })?;
    Ok(())
}

/// 按文件头认常见格式（`createBufferURL` 给出的路径要带对扩展名，播放器 / 解码器有的认扩展名）
fn sniff_ext(b: &[u8]) -> &'static str {
    match b {
        [0x89, b'P', b'N', b'G', ..] => "png",
        [0xff, 0xd8, 0xff, ..] => "jpg",
        [b'G', b'I', b'F', b'8', ..] => "gif",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "webp",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'A', b'V', b'E', ..] => "wav",
        [_, _, _, _, b'f', b't', b'y', b'p', ..] => "mp4",
        [b'I', b'D', b'3', ..] | [0xff, 0xfb, ..] => "mp3",
        [b'B', b'M', ..] => "bmp",
        [b'%', b'P', b'D', b'F', ..] => "pdf",
        _ => "bin",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sniffs_common_formats() {
        assert_eq!(super::sniff_ext(&[0x89, b'P', b'N', b'G', 0, 0]), "png");
        assert_eq!(super::sniff_ext(&[0xff, 0xd8, 0xff, 0xe0]), "jpg");
        assert_eq!(super::sniff_ext(b"\0\0\0\x18ftypmp42"), "mp4");
        assert_eq!(super::sniff_ext(b"hello"), "bin");
    }
}
