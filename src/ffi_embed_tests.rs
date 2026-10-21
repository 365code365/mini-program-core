//! `ffi_embed` 的 C ABI 用例（只用 sample-app，与其它引擎用例共用同一个小程序目录 ——
//! 小程序根目录是进程级的，换别的目录要放进独立进程的集成测试 `tests/embed_host.rs`）。
use super::*;
use crate::ffi_app::*;
use crate::ffi_backend::mr_app_pump_ex;
use std::ffi::{CStr, CString};

fn sample_app(slot: &str) -> Option<*mut MRApp> {
    let p = crate::app_dir::resolve("sample-app");
    if !p.join("app.json").exists() {
        return None; // 示例不在时跳过
    }
    let dir = CString::new(p.to_string_lossy().to_string()).ok()?;
    let data = crate::data_dir::subdir(&format!("ffi-embed-tests/{slot}"));
    std::fs::remove_dir_all(&data).ok();
    let data = CString::new(data.to_string_lossy().to_string()).ok()?;
    let opts = CString::new(r#"{"safeArea":{"top":0},"systemInfo":{"platform":"test"}}"#).ok()?;
    let app = mr_app_create_ex(dir.as_ptr(), data.as_ptr(), 375, 667, 2.0, opts.as_ptr());
    assert!(!app.is_null(), "创建实例失败");
    assert_eq!(mr_app_launch(app, std::ptr::null()), 1);
    Some(app)
}

fn poll_all(app: *mut MRApp) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    loop {
        let p = mr_app_poll_event(app);
        if p.is_null() {
            return out;
        }
        let s = unsafe { CStr::from_ptr(p) }.to_str().unwrap().to_string();
        out.push(serde_json::from_str(&s).expect("事件必须是合法 JSON"));
    }
}

#[test]
fn 空句柄与坏参数不崩() {
    let null = std::ptr::null_mut::<MRApp>();
    assert_eq!(mr_app_resize(null, 1, 1, 1.0), 0);
    assert_eq!(mr_app_set_safe_area(null, 1.0, 0.0, 0.0, 0.0), 0);
    assert_eq!(mr_app_pixels_into(null, std::ptr::null_mut(), 0, 0, 0), 0);
    assert!(mr_app_poll_event(null).is_null());
    assert_eq!(mr_app_host_reply(null, 1, 1, std::ptr::null()), 0);
    assert_eq!(mr_app_native_view_event(null, std::ptr::null(), std::ptr::null(), std::ptr::null()), 0);
    let bad = CString::new("{").unwrap();
    let dir = CString::new("/nonexistent").unwrap();
    assert!(mr_app_create_ex(dir.as_ptr(), std::ptr::null(), 10, 10, 1.0, bad.as_ptr()).is_null(), "坏选项要失败");
}

/// 判据：任意行跨度、两种格式写出来的像素，换回 RGBA 后与 `mr_app_pixels` 逐字节相同
#[test]
fn pixels_into_与_mr_app_pixels_逐字节一致() {
    let Some(app) = sample_app("pixels") else { return };
    assert_eq!(mr_app_pump(app, 1), 1);
    let (mut w, mut h) = (0u32, 0u32);
    mr_app_pixel_size(app, &mut w, &mut h);
    let (w, h) = (w as usize, h as usize);
    let mut tight = vec![0u8; w * h * 4];
    assert_eq!(mr_app_pixels(app, tight.as_mut_ptr(), tight.len()), tight.len());

    for (stride, format) in [(w * 4, MR_PIXEL_RGBA8), (w * 4 + 64, MR_PIXEL_RGBA8), (w * 4 + 36, MR_PIXEL_BGRA8)] {
        let mut buf = vec![0xAAu8; stride * h];
        let n = mr_app_pixels_into(app, buf.as_mut_ptr(), buf.len(), stride as u32, format);
        assert_eq!(n, stride * (h - 1) + w * 4, "stride={stride}");
        for y in 0..h {
            let row = &buf[y * stride..y * stride + w * 4];
            let want = &tight[y * w * 4..(y + 1) * w * 4];
            if format == MR_PIXEL_RGBA8 {
                assert_eq!(row, want, "第 {y} 行 stride={stride}");
            } else {
                for (px, wp) in row.chunks_exact(4).zip(want.chunks_exact(4)) {
                    assert_eq!([px[2], px[1], px[0], px[3]], [wp[0], wp[1], wp[2], wp[3]], "第 {y} 行 BGRA");
                }
            }
            // 行尾填充不能被写（宿主缓冲里那几十字节可能另有用途）
            let pad = &buf[y * stride + w * 4..(y + 1) * stride];
            assert!(pad.iter().all(|b| *b == 0xAA), "第 {y} 行的行尾填充被写了");
        }
    }
    // 缓冲不够 / stride 太小 / 格式不认识：返回 0，一个字节都不写
    let mut small = vec![0x55u8; w * 4];
    assert_eq!(mr_app_pixels_into(app, small.as_mut_ptr(), small.len(), (w * 4) as u32, MR_PIXEL_RGBA8), 0);
    assert!(small.iter().all(|b| *b == 0x55));
    let mut full = vec![0u8; w * h * 4];
    assert_eq!(mr_app_pixels_into(app, full.as_mut_ptr(), full.len(), (w * 4 - 4) as u32, MR_PIXEL_RGBA8), 0);
    assert_eq!(mr_app_pixels_into(app, full.as_mut_ptr(), full.len(), (w * 4) as u32, 9), 0);
    mr_app_destroy(app);
}

#[test]
fn 启动后有页面事件_改尺寸后像素尺寸跟着变() {
    let Some(app) = sample_app("resize") else { return };
    let flags = mr_app_pump_ex(app, 1);
    assert!(flags & MR_FRAME_EVENTS != 0, "启动后队列里应有 page 事件，flags={flags}");
    let evs = poll_all(app);
    let page = evs.iter().find(|e| e["type"] == "page").expect("要有 page 事件");
    assert_eq!(page["depth"], 1);
    assert!(page["route"].as_str().unwrap().starts_with("pages/"));
    assert!(page["window"].is_object());

    assert_eq!(mr_app_resize(app, 375, 667, 2.0), 0, "没变返回 0");
    assert_eq!(mr_app_resize(app, 414, 896, 3.0), 1);
    let (mut w, mut h) = (0u32, 0u32);
    mr_app_pixel_size(app, &mut w, &mut h);
    assert_eq!((w, h), (1242, 2688));
    let mut buf = vec![0u8; (w * h * 4) as usize];
    assert_eq!(mr_app_pixels(app, buf.as_mut_ptr(), buf.len()), buf.len(), "改完尺寸立刻就能取到新尺寸的帧");
    assert!(buf.chunks_exact(4).any(|p| p[0] != p[1] || p[1] != p[2] || p[0] < 200), "不能是一片空白");
    mr_app_destroy(app);
}
