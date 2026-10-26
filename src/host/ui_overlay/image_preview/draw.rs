//! 预览层画进输出缓冲：压暗页面、当前与相邻的两张（带开合 / 下拉的缩放位移）、关闭键与页码。
//! 从 mod.rs 搬出来（那个文件过了 500 行）。
use super::*;

/// 画进输出缓冲（0x00RRGGBB，行宽 `w`）。没有预览时什么也不写
pub fn render(buf: &mut [u32], w: u32, h: u32, sf: f32, text: Option<&TextRenderer>) {
    let Some(p) = with(|s| s.clone()) else { return };
    let look = FRAME_LOOK.with(|f| f.take()).unwrap_or_else(|| look_at(&p, crate::frame_clock::now()));
    LAST_SIZE.with(|s| s.set((w, h)));
    let (vw, keep) = (w as f32, ((1.0 - look.bg) * 256.0).round().clamp(0.0, 256.0) as u32);
    raster::darken(buf, keep);
    let k = (look.alpha * 256.0).round().clamp(0.0, 256.0) as u32;
    let mut missing = false;
    let mut paused_video = false;
    let mut fit = None;
    let view = p.zoom.at(crate::frame_clock::now());
    let lo = look.pos.floor() as isize;
    for i in [lo, lo + 1] {
        let Some(item) = (i >= 0).then(|| p.items.get(i as usize)).flatten() else { continue };
        let off = (i as f32 - look.pos) * vw;
        if off.abs() >= vw {
            continue;
        }
        // 当前那张（离正中最近）带着开合 / 下拉的缩放与位移，再乘上双指 / 双击的缩放平移（zoom.rs）
        let near = (i as f32 - look.pos).abs() < 0.5;
        let v = if near { view } else { zoom::View::FIT };
        let (scale, tx, ty) = if near { (look.scale * v.z, (look.tx + v.px) * sf, (look.ty + v.py) * sf) } else { (1.0, 0.0, 0.0) };
        let place = |fw: u32, fh: u32| {
            let (dw, dh) = (fw as f32 * scale, fh as f32 * scale);
            ((vw - dw) / 2.0 + off + tx, (h as f32 - dh) / 2.0 + ty, dw, dh)
        };
        let frame = (!item.video.is_empty() && video::active().as_deref() == Some(item.video.as_str()))
            .then(|| {
                video::note_box(w, h);
                video::frame(&item.video)
            })
            .flatten();
        if let Some(f) = &frame {
            let (data, fw0, fh0) = f.rgba();
            let (fw, fh) = raster::fit_size(fw0, fh0, w, h);
            raster::blit(buf, w, h, &raster::Src::Rgba(data, fw0, fh0), place(fw, fh), k);
        } else if let Some(f) = raster::fitted(&item.url, w, h) {
            if near {
                fit = Some((f.w as f32 / sf, f.h as f32 / sf));
            }
            // 放大着：缩好的那份只有屏幕大小，再放大就糊了 —— 从原图取（只算屏上看得见的那些像素）
            match crate::renderer::components::image_rgba(&item.url).filter(|_| v.zoomed()) {
                Some((data, iw, ih)) => raster::blit(buf, w, h, &raster::Src::Rgba(&data, iw, ih), place(f.w, f.h), k),
                None => raster::blit(buf, w, h, &raster::Src::Px(&f.px, f.w, f.h), place(f.w, f.h), k),
            }
        } else if !item.url.is_empty() {
            // 后台还在缩（或图还没解出来）：有原图就拿原图凑合，缩好了下一帧换上
            missing = true;
            if let Some((data, iw, ih)) = crate::renderer::components::image_rgba(&item.url) {
                let (fw, fh) = raster::fit_size(iw, ih, w, h);
                if near {
                    fit = Some((fw as f32 / sf, fh as f32 / sf));
                }
                raster::blit(buf, w, h, &raster::Src::Rgba(&data, iw, ih), place(fw, fh), k);
            }
        }
        if near && !item.video.is_empty() {
            paused_video = !video::playing(&item.video);
        }
    }
    let shown = look.pos.round().clamp(0.0, (p.items.len() - 1) as f32) as usize;
    let insets = (TOP.with(Cell::get), BOTTOM.with(Cell::get));
    chrome::draw(buf, (w, h, sf), text, insets, (shown, p.items.len()), paused_video, look.bg);
    with(|s| {
        if let Some(x) = s.as_mut() {
            x.dirty = false;
            x.missing = missing;
            // 拖动的换算（一屏多宽、下拉缩放按多高）按画面的逻辑尺寸；按下时宿主也会给一份
            x.vp = (w as f32 / sf, h as f32 / sf);
            if let Some(f) = fit {
                x.fit = f;
            }
        }
    });
}
