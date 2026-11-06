//! 填充规则：扫描线默认按 even-odd 配对交点（引擎自己的 UI 绘制一直是这样，逐字节基线以它为准）；
//! `<canvas>` 的 `fill()` / `clip()` 按 HTML 默认的 **nonzero**（`fill('evenodd')` 才是 even-odd）。
//!
//! 为什么用线程局部开关而不是给 `Paint` 加字段：`Paint` 在 ffi/canvas.rs 里按结构体字面量构造，加字段要动一串
//! 不相干的代码；规则只有 canvas 那几处会改，开关包住那一次 `draw_path` 即可（[`scoped`]）。
//!
//! 为什么 nonzero 与 even-odd 在简单图形上逐字节相同：每条子扫描线先按 x 排好交点、**按同一 x 成组**累计环绕数；
//! 环绕数的绝对值从没超过 1 时（不自交、不重叠的图形，含共边的相邻矩形），每组交点都在「里 / 外」之间切换一次，
//! 与 even-odd 两两配对完全一样 —— 这时原样交还排好序的交点，后面的覆盖率计算一个字节不变。
//! 只有真的绕了两圈以上（自交的五角星、同向的两个重叠圆）才改用 nonzero 的边界。

use std::cell::Cell;

thread_local! {
    static NONZERO: Cell<bool> = const { Cell::new(false) };
}

/// 在 `f` 里按 nonzero（`true`）或 even-odd（`false`）填充，结束后恢复原值
pub fn scoped<R>(nonzero: bool, f: impl FnOnce() -> R) -> R {
    let prev = NONZERO.with(|c| c.replace(nonzero));
    let r = f();
    NONZERO.with(|c| c.set(prev));
    r
}

/// 当前是不是 nonzero（扫描线每次填充读一次）
pub(super) fn nonzero() -> bool {
    NONZERO.with(|c| c.get())
}

/// 一条扫描线上的交点（x, 边的方向 ±1）→ 填进 `out` 的区间端点（两两一对）。
/// 环绕数一直在 [-1, 1] 里时交出全部交点（= even-odd），否则只交出「环绕数 0 ↔ 非 0」的切换点
pub(super) fn nonzero_spans(crossings: &mut [(f32, i8)], out: &mut Vec<f32>) {
    crossings.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut w = 0i32;
    let mut simple = true;
    let mut i = 0;
    while i < crossings.len() {
        let x = crossings[i].0;
        while i < crossings.len() && crossings[i].0 == x {
            w += crossings[i].1 as i32;
            i += 1;
        }
        if w.abs() > 1 {
            simple = false;
            break;
        }
    }
    if simple {
        out.extend(crossings.iter().map(|c| c.0));
        return;
    }
    let mut w = 0i32;
    let mut i = 0;
    while i < crossings.len() {
        let x = crossings[i].0;
        let inside = w != 0;
        while i < crossings.len() && crossings[i].0 == x {
            w += crossings[i].1 as i32;
            i += 1;
        }
        if inside != (w != 0) {
            out.push(x);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 简单图形交出全部交点_自交才合并() {
        let mut out = Vec::new();
        // 一个矩形：+1 进、-1 出
        nonzero_spans(&mut [(10.0, -1), (2.0, 1)], &mut out);
        assert_eq!(out, vec![2.0, 10.0]);
        // 共边的两个矩形（2..5、5..9）：同一 x 上一出一进，算一组，仍是 even-odd 的四个点
        out.clear();
        nonzero_spans(&mut [(2.0, 1), (5.0, -1), (5.0, 1), (9.0, -1)], &mut out);
        assert_eq!(out, vec![2.0, 5.0, 5.0, 9.0]);
        // 同向重叠的两段（2..8 与 4..10）：环绕数到 2，nonzero 是一整段 2..10
        out.clear();
        nonzero_spans(&mut [(2.0, 1), (4.0, 1), (8.0, -1), (10.0, -1)], &mut out);
        assert_eq!(out, vec![2.0, 10.0]);
    }
}
