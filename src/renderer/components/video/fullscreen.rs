//! 全屏播放：`VideoContext.requestFullScreen()` 之后画在所有页面层之上的那一层。
//!
//! 合成器（`compositor::cpu`）在页面、tabBar、fixed 层与 Toast/Modal 之间画它，GPU 后端
//! 遇到它整帧直通 CPU 合成（与 Modal 同一条路）。画面按视口 aspectFit，黑边。
//!
//! 取舍：`direction` 为横屏时微信会把画面转成横向占满屏幕；这里不转屏（宿主的方向锁
//! 不归引擎管），只在事件里如实报 `direction`，画面仍竖着 aspectFit。
//!
//! 点击：左上角 60×60 是「退出全屏」，其余位置切换播放 / 暂停。
use super::*;

/// 退出按钮的热区（逻辑像素）
const EXIT_HOT: f32 = 60.0;

/// 把全屏画面画进输出缓冲（0x00RRGGBB，行宽 `w`）。没有全屏时什么也不做
pub fn draw_fullscreen(buf: &mut [u32], w: u32, h: u32, sf: f32) {
    let Some((src, _)) = fullscreen_video() else { return };
    buf.iter_mut().for_each(|p| *p = 0);
    let playing = is_video_playing(&src);
    if let Some((data, fw, fh)) = get_video_frame(&src) {
        blit_fit(buf, w, h, &data, fw, fh);
    }
    let s = sf.max(1.0);
    // 左上角的「‹」（退出全屏）
    let (cx, cy, arm) = (22.0 * s, 30.0 * s, 9.0 * s);
    stroke(buf, w, h, (cx + arm * 0.5, cy - arm), (cx - arm * 0.5, cy), 2.0 * s);
    stroke(buf, w, h, (cx - arm * 0.5, cy), (cx + arm * 0.5, cy + arm), 2.0 * s);
    if !playing {
        // 暂停时中间一个播放三角
        let (mx, my, r) = (w as f32 / 2.0, h as f32 / 2.0, 18.0 * s);
        for py in (my - r) as i32..(my + r) as i32 {
            let t = 1.0 - ((py as f32 - my).abs() / r);
            let x1 = mx - r * 0.6 + t * r * 1.5;
            for px in (mx - r * 0.6) as i32..x1 as i32 {
                put(buf, w, h, px, py, 0xFFFFFF);
            }
        }
    }
}

/// 全屏时的一次点击（逻辑坐标、逻辑视口宽）。返回是否吃掉了这次点击（全屏时总是吃掉）
pub fn fullscreen_tap(x: f32, y: f32) -> bool {
    let Some((src, _)) = fullscreen_video() else { return false };
    if x < EXIT_HOT && y < EXIT_HOT {
        video_command(&src, &VideoCommand::ExitFullScreen);
    } else if is_video_playing(&src) {
        video_command(&src, &VideoCommand::Pause);
    } else {
        video_command(&src, &VideoCommand::Play);
    }
    true
}

fn put(buf: &mut [u32], w: u32, h: u32, x: i32, y: i32, c: u32) {
    if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
        buf[(y as u32 * w + x as u32) as usize] = c;
    }
}

/// 粗线段（逐点盖方块，够画一个箭头）
fn stroke(buf: &mut [u32], w: u32, h: u32, a: (f32, f32), b: (f32, f32), width: f32) {
    let steps = ((b.0 - a.0).abs().max((b.1 - a.1).abs()) as i32).max(1);
    let r = (width / 2.0).ceil() as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let (x, y) = ((a.0 + (b.0 - a.0) * t) as i32, (a.1 + (b.1 - a.1) * t) as i32);
        for dy in -r..=r {
            for dx in -r..=r {
                put(buf, w, h, x + dx, y + dy, 0xFFFFFF);
            }
        }
    }
}

/// RGBA 帧按 aspectFit 最近邻缩放进缓冲中央
fn blit_fit(buf: &mut [u32], w: u32, h: u32, data: &[u8], fw: u32, fh: u32) {
    if fw == 0 || fh == 0 || data.len() < (fw * fh * 4) as usize {
        return;
    }
    let scale = (w as f32 / fw as f32).min(h as f32 / fh as f32);
    let (dw, dh) = ((fw as f32 * scale) as u32, (fh as f32 * scale) as u32);
    let (ox, oy) = ((w - dw.min(w)) / 2, (h - dh.min(h)) / 2);
    for y in 0..dh.min(h) {
        let sy = ((y as f32 / scale) as u32).min(fh - 1);
        for x in 0..dw.min(w) {
            let sx = ((x as f32 / scale) as u32).min(fw - 1);
            let i = ((sy * fw + sx) * 4) as usize;
            let c = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | data[i + 2] as u32;
            buf[((oy + y) * w + ox + x) as usize] = c;
        }
    }
}
