//! 画布上与图片有关的两条：`drawImage`（op 47）与 `createPattern` 之后设成填充 / 描边（op 48）。
//! 源是 `Image`（game/image，已解码的 RGBA）或另一块游戏画布（读它的后备缓冲）。
//!
//! JS 侧调用形状：`__game_2d(id, 47, 种类, 源 id, 平滑, 参数个数, …数字)`、`__game_2d(id, 48, 种类, 源 id, repetition, 是否描边)`；
//! 种类 1 = Image、2 = Canvas。

use super::GameCanvases;
use crate::game::image::{loader::Decoded, ImageStore};
use crate::renderer::components::ImageRects;
use crate::renderer::components::Canvas2DContext;
use crate::Canvas;
use rquickjs::Value;
use std::sync::{Arc, Mutex};

pub enum ImgSource {
    Image(Arc<Decoded>),
    /// 另一块画布：后备缓冲与 `canvas.width/height`
    Canvas(Arc<Mutex<Canvas>>),
    /// 画到自己身上：先拷一份（同一把锁不能锁两次）
    Copy { rgba: Vec<u8>, w: u32, h: u32 },
}

/// 解析 op 47 / 48 的源（在拿目标画布之前调：两块画布都在 `GameCanvases` 里）
pub fn source(cs: &mut GameCanvases, images: &mut ImageStore, kind: i32, sid: u32, dst: u32) -> Option<ImgSource> {
    match kind {
        1 => images.get(sid).map(ImgSource::Image),
        2 => {
            let slot = cs.get(sid)?;
            if slot.w == 0 || slot.h == 0 {
                return None;
            }
            let canvas = slot.ctx.get_canvas();
            if sid == dst {
                let c = canvas.lock().ok()?;
                return Some(ImgSource::Copy { rgba: crate::canvas::color_bytes(c.pixels()).to_vec(), w: c.width(), h: c.height() });
            }
            Some(ImgSource::Canvas(canvas))
        }
        _ => None,
    }
}

/// 拿到源像素（RGBA8、宽、高）跑 `f`
fn with_pixels<R>(src: &ImgSource, f: impl FnOnce(&[u8], u32, u32) -> R) -> Option<R> {
    match src {
        ImgSource::Image(d) => Some(f(&d.rgba, d.w, d.h)),
        ImgSource::Copy { rgba, w, h } => Some(f(rgba, *w, *h)),
        ImgSource::Canvas(c) => {
            let c = c.lock().ok()?;
            Some(f(crate::canvas::color_bytes(c.pixels()), c.width(), c.height()))
        }
    }
}

/// op 47：`a` 是 [种类, 源 id, 平滑, 参数个数, …]
pub fn draw_image(g: &mut Canvas2DContext, src: &ImgSource, a: &[Value]) {
    let n = |i: usize| a.get(i).and_then(|v| v.as_number()).unwrap_or(f64::NAN) as f32;
    let smooth = a.get(2).and_then(|v| v.as_bool()).unwrap_or(true);
    let argc = n(3) as i32;
    with_pixels(src, |rgba, iw, ih| {
        let (iwf, ihf) = (iw as f32, ih as f32);
        let r = match argc {
            3 => ImageRects { sx: 0.0, sy: 0.0, sw: iwf, sh: ihf, dx: n(4), dy: n(5), dw: iwf, dh: ihf },
            5 => ImageRects { sx: 0.0, sy: 0.0, sw: iwf, sh: ihf, dx: n(4), dy: n(5), dw: n(6), dh: n(7) },
            9 => ImageRects { sx: n(4), sy: n(5), sw: n(6), sh: n(7), dx: n(8), dy: n(9), dw: n(10), dh: n(11) },
            _ => return,
        };
        // HTML：任一参数不是有限数整条忽略
        if ![r.sx, r.sy, r.sw, r.sh, r.dx, r.dy, r.dw, r.dh].iter().all(|v| v.is_finite()) {
            return;
        }
        if let Some(r) = r.normalize(iw, ih) {
            let spans = match src {
                ImgSource::Image(d) => Some(d.spans()),
                _ => None,
            };
            g.draw_op(|g| g.draw_image_rgba(rgba, iw, ih, r, smooth, spans));
        }
    });
}

/// op 48：图案设成填充 / 描边。源没准备好（图没加载完）返回 false —— JS 外壳据此让 createPattern 返回 null
pub fn set_pattern(g: &mut Canvas2DContext, src: &ImgSource, repetition: &str, stroke: bool) -> bool {
    let (rx, ry) = match repetition {
        "" | "repeat" => (true, true),
        "repeat-x" => (true, false),
        "repeat-y" => (false, true),
        "no-repeat" => (false, false),
        _ => return false,
    };
    let rgba = match src {
        ImgSource::Image(d) => Some((d.rgba.clone(), d.w, d.h)),
        _ => with_pixels(src, |p, w, h| (Arc::new(p.to_vec()), w, h)),
    };
    match rgba {
        Some((rgba, w, h)) if w > 0 && h > 0 => {
            g.set_pattern_source(stroke, rgba, w, h, rx, ry);
            true
        }
        _ => false,
    }
}
