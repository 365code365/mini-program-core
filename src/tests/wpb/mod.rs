//! 组件对齐 WP-B（`.agents/tasks/component-gap-2026-11-03/plan.md`）：表单与输入。
//! 与 WP-C 同一套写法（`super::wpc` 的现场小程序 + `MiniEngine`），放在库内单测里，不新建 `tests/*.rs`。
pub(super) use super::wpc::{clear_log, engine, fixture, host_events, js, log, pump_until, rgb_at};
pub(super) use crate::host::{EngineOptions, MiniEngine};
mod controls;
mod input;
mod picker;

pub(super) const LOG_FN: &str = "function L(s){ getApp().log.push(s); }";

/// 点 (x, y) 一下
pub(super) fn tap(e: &mut MiniEngine, x: f32, y: f32, t: u64) {
    e.pointer_down(x, y);
    e.pump(t);
    e.pointer_up(x, y);
    e.pump(t + 16);
}

/// 出几帧（墙上时钟也走一点：setData 的回包、定时器）
pub(super) fn settle(e: &mut MiniEngine) {
    pump_until(e, 120, |_| false);
}

/// 选择器查询 `#id` 的盒子 `[left, top, width, height]`（同步等回调）
pub(super) fn rect_of(e: &mut MiniEngine, id: &str) -> [f64; 4] {
    js(e, &format!(
        "globalThis.__r = null; wx.createSelectorQuery().select('#{id}').boundingClientRect(function(r){{ globalThis.__r = r; }}).exec()"
    ));
    pump_until(e, 500, |e| js(e, "globalThis.__r ? 1 : 0") == "1");
    let v: serde_json::Value = serde_json::from_str(&js(e, "JSON.stringify(globalThis.__r)")).unwrap_or_default();
    let f = |k: &str| v[k].as_f64().unwrap_or(f64::NAN);
    [f("left"), f("top"), f("width"), f("height")]
}

/// 区域 [x0,x1)×[y0,y1) 里「接近 `rgb`」的像素数
pub(super) fn count_color(e: &mut MiniEngine, (x0, y0, x1, y1): (u32, u32, u32, u32), rgb: [u8; 3]) -> usize {
    let (w, _) = e.pixel_size();
    let px = e.rgba();
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * w + x) * 4) as usize;
            let d = (0..3).map(|k| (px[i + k] as i32 - rgb[k] as i32).abs()).max().unwrap_or(255);
            if d < 60 {
                n += 1;
            }
        }
    }
    n
}
