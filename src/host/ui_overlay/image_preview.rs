//! `wx.previewImage` 的全屏图片预览（官方 api/media/image/wx.previewImage.html）。
//!
//! 与 action sheet 同一类宿主浮层：画在合成那一步、GPU 后端整帧直通、状态在线程局部。
//! - `urls` 里的图依次排开，`current`（url 或下标，默认第一张）是打开时那一张；
//! - 左右拖动跟手，松手位移过 60px（或快甩）切到相邻那张，到头不再切；
//! - 轻点关闭；多于一张时底部显示「当前 / 总数」；
//! - 黑底、aspectFit，打开 / 关闭 0.2s 淡入淡出。远程图没到时先画黑底（下载中的图由图片层续帧）。
//! 取舍：不做双指缩放与长按菜单（`showmenu`）。

use super::draw::*;
use crate::runtime::MiniApp;
use crate::text::TextRenderer;
use crate::Color;
use serde_json::Value;
use std::cell::RefCell;
use std::time::Instant;

const FADE_SECS: f32 = 0.2;
/// 松手时位移超过它就换张（逻辑像素）
const SWITCH_DX: f32 = 60.0;
/// 位移小于它算轻点
const TAP_SLOP: f32 = 8.0;

#[derive(Clone, Debug)]
struct Preview {
    urls: Vec<String>,
    index: usize,
    /// 拖动：(按下的 x, 当前 x)
    drag: Option<(f32, f32)>,
    opened: Instant,
    closing: Option<Instant>,
    /// 当前这段淡入 / 淡出的终态已经画出来过（同 action_sheet 的同名字段）
    settled: bool,
}

thread_local! {
    static PREVIEW: RefCell<Option<Preview>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut Option<Preview>) -> R) -> R {
    PREVIEW.with(|s| f(&mut s.borrow_mut()))
}

pub fn visible() -> bool {
    with(|s| s.is_some())
}

pub fn animating() -> bool {
    with(|s| s.as_ref().map(|p| p.drag.is_some() || !p.settled).unwrap_or(false))
}

pub fn clear() {
    with(|s| *s = None);
}

/// 当前那张（测试 / 诊断用）
pub fn current() -> Option<(usize, String)> {
    with(|s| s.as_ref().map(|p| (p.index, p.urls[p.index].clone())))
}

/// 每帧：取走 `wx.previewImage` 的请求；关闭动画走完就丢掉。返回是否要重新合成
pub fn take_requests(app: &mut MiniApp) -> bool {
    let mut changed = false;
    for req in app.take_host_requests("previewImage") {
        let urls: Vec<String> = req
            .get("urls")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default();
        if urls.is_empty() {
            continue;
        }
        let index = match req.get("current") {
            Some(Value::Number(n)) => n.as_u64().unwrap_or(0) as usize,
            Some(Value::String(s)) => urls.iter().position(|u| u == s).unwrap_or(0),
            _ => 0,
        }
        .min(urls.len() - 1);
        with(|s| *s = Some(Preview { urls, index, drag: None, opened: Instant::now(), closing: None, settled: false }));
        changed = true;
    }
    let done = with(|s| s.as_ref().and_then(|p| p.closing).map(|t| t.elapsed().as_secs_f32() >= FADE_SECS).unwrap_or(false));
    if done {
        clear();
        changed = true;
    }
    changed || animating()
}

pub fn on_press(x: f32) -> bool {
    with(|s| {
        let Some(p) = s.as_mut() else { return false };
        if p.closing.is_none() {
            p.drag = Some((x, x));
        }
        true
    })
}

pub fn on_move(x: f32) -> bool {
    with(|s| {
        let Some(p) = s.as_mut() else { return false };
        if let Some(d) = p.drag.as_mut() {
            d.1 = x;
        }
        true
    })
}

pub fn on_release(x: f32) -> bool {
    with(|s| {
        let Some(p) = s.as_mut() else { return false };
        let Some((x0, _)) = p.drag.take() else { return true };
        let dx = x - x0;
        // 换了张 / 关闭：要再合成一次
        p.settled = false;
        if dx.abs() < TAP_SLOP {
            p.closing = Some(Instant::now());
        } else if dx <= -SWITCH_DX && p.index + 1 < p.urls.len() {
            p.index += 1;
        } else if dx >= SWITCH_DX && p.index > 0 {
            p.index -= 1;
        }
        true
    })
}

/// 画进输出缓冲（0x00RRGGBB，行宽 `w`）。没有预览时什么也不写
pub fn render(buf: &mut [u32], w: u32, h: u32, sf: f32, text: Option<&TextRenderer>) {
    let Some(p) = with(|s| s.clone()) else { return };
    let (t0, closing) = match p.closing {
        Some(t) => (t, true),
        None => (p.opened, false),
    };
    let t = (t0.elapsed().as_secs_f32() / FADE_SECS).clamp(0.0, 1.0);
    let alpha = if closing { 1.0 - t } else { t };
    if t >= 1.0 && p.drag.is_none() {
        with(|s| if let Some(x) = s.as_mut() { x.settled = true });
    }
    // 先在一张临时缓冲里画好整屏，再按 alpha 盖到页面上（淡入淡出）
    let mut layer = vec![0u32; buf.len()];
    let dx = p.drag.map(|(a, b)| (b - a) * sf).unwrap_or(0.0);
    let vw = w as f32;
    for (k, off) in [(p.index as isize - 1, -vw), (p.index as isize, 0.0), (p.index as isize + 1, vw)] {
        if k < 0 || k as usize >= p.urls.len() {
            continue;
        }
        if let Some((data, iw, ih)) = crate::renderer::components::image_rgba(&p.urls[k as usize]) {
            blit_fit(&mut layer, w, h, &data, iw, ih, (off + dx) as i32);
        }
    }
    if p.urls.len() > 1 {
        if let Some(tr) = text {
            let label = format!("{} / {}", p.index + 1, p.urls.len());
            let size = 14.0 * sf;
            let x = ((vw - tr.measure_text(&label, size)) / 2.0) as i32;
            draw_text_direct(&mut layer, w, h, tr, &label, x, (h as f32 - 48.0 * sf) as i32, size, Color::WHITE);
        }
    }
    for (dst, src) in buf.iter_mut().zip(layer.iter()) {
        if alpha >= 0.999 {
            *dst = *src & 0x00FF_FFFF;
            continue;
        }
        let mix = |a: u32, b: u32| ((a as f32) * (1.0 - alpha) + (b as f32) * alpha) as u32;
        let (r, g, b) = (mix((*dst >> 16) & 0xFF, (*src >> 16) & 0xFF), mix((*dst >> 8) & 0xFF, (*src >> 8) & 0xFF), mix(*dst & 0xFF, *src & 0xFF));
        *dst = (r << 16) | (g << 8) | b;
    }
}

/// RGBA 图按 aspectFit 最近邻缩放进缓冲中央，横向再挪 `shift` 像素（拖动 / 相邻那张）
fn blit_fit(buf: &mut [u32], w: u32, h: u32, data: &[u8], iw: u32, ih: u32, shift: i32) {
    if iw == 0 || ih == 0 || data.len() < (iw * ih * 4) as usize {
        return;
    }
    let scale = (w as f32 / iw as f32).min(h as f32 / ih as f32);
    let (dw, dh) = ((iw as f32 * scale) as i32, (ih as f32 * scale) as i32);
    let (ox, oy) = ((w as i32 - dw) / 2 + shift, (h as i32 - dh) / 2);
    for y in 0..dh {
        let ty = oy + y;
        if ty < 0 || ty >= h as i32 {
            continue;
        }
        let sy = ((y as f32 / scale) as u32).min(ih - 1);
        for x in 0..dw {
            let tx = ox + x;
            if tx < 0 || tx >= w as i32 {
                continue;
            }
            let sx = ((x as f32 / scale) as u32).min(iw - 1);
            let i = ((sy * iw + sx) * 4) as usize;
            let a = data[i + 3] as u32;
            // 透明像素压在黑底上
            let c = |v: u8| (v as u32 * a) / 255;
            buf[(ty as u32 * w + tx as u32) as usize] = (c(data[i]) << 16) | (c(data[i + 1]) << 8) | c(data[i + 2]);
        }
    }
}
