//! fx 蒙版（`fx2d::paint_with_fx`）的缓冲复用与包围盒。
//!
//! 为什么要有：从前每次渐变 / 阴影 / 合成模式的填充、每段走 fx 的文字都 `Canvas::new` 一张**整画布**
//! 大小的蒙版、清空、整张取覆盖率、整张跑浮点合成 —— O(画布面积)，跟形状多大无关。iPhone 上萌宠小岛
//! 的 UI 面板（渐变按钮，2x 离屏画布）一次 fill 2~4ms，一帧 14~40 次，就是 50~80ms 的慢帧。
//!
//! 现在：蒙版按尺寸缓存在线程里（不变式：缓存里的蒙版**全透明、无裁剪、无平移**），画完扫一遍找出
//! 被写过的包围盒（按整个像素值判「不是全 0」，不只看 alpha —— 混合可能留下 rgb≠0、a=0 的像素），
//! 只在框内取覆盖率、合成，用完只把框内清回 0 再还回缓存。框外覆盖率本来就是 0，旧代码在那里
//! `continue`，所以结果逐字节相同。阴影与会动形状外像素的合成模式仍走整画布（见 `paint_with_fx`）。

use crate::{Canvas, Color};
use std::cell::RefCell;

/// 缓存几张（不同尺寸的离屏画布轮流画时不来回分配）。一张 2x 全屏约 4MB
const KEEP: usize = 3;

thread_local! {
    static MASKS: RefCell<Vec<Canvas>> = const { RefCell::new(Vec::new()) };
}

/// 取一张全透明的 w×h 蒙版
pub(super) fn take(w: u32, h: u32) -> Canvas {
    let hit = MASKS.with(|m| {
        let mut m = m.borrow_mut();
        let i = m.iter().position(|c| c.width() == w && c.height() == h)?;
        Some(m.remove(i))
    });
    hit.unwrap_or_else(|| {
        let mut c = Canvas::new(w, h);
        c.clear(Color::TRANSPARENT);
        c
    })
}

/// 还回缓存：调用方必须已经把它清回全透明
pub(super) fn give_back(mask: Canvas) {
    MASKS.with(|m| {
        let mut m = m.borrow_mut();
        m.insert(0, mask);
        m.truncate(KEEP);
    });
}

/// 设备坐标里的形状外接框（浮点）→ 光栅器可能写到的整数像素框 `[x0, x1) × [y0, y1)`，已与画布求交。
/// 外扩 2px：扫描线填充按 `min_y - 1 .. max_y + 1` 行、每段区间左右各 1px 走（canvas/paths.rs）
pub(super) fn pixel_region(b: (f32, f32, f32, f32), w: u32, h: u32) -> (usize, usize, usize, usize) {
    let c = |v: f32, hi: u32| (v.max(0.0) as u32).min(hi) as usize;
    let (x0, y0) = (c((b.0 - 2.0).floor(), w), c((b.1 - 2.0).floor(), h));
    let (x1, y1) = (c((b.2 + 2.0).ceil(), w), c((b.3 + 2.0).ceil(), h));
    (x0, y0, x1.max(x0), y1.max(y0))
}

/// 框 `region` 里被写过（不是全 0）的像素的包围盒 `[x0, x1) × [y0, y1)`；一个都没有时 None。
/// 调用方保证框外没被写过（整画布，或画蒙版时裁剪到了这个框）
pub(super) fn dirty_bbox(mask: &Canvas, region: (usize, usize, usize, usize)) -> Option<(usize, usize, usize, usize)> {
    let w = mask.width() as usize;
    let (rx0, ry0, rx1, ry1) = region;
    if w == 0 || rx1 <= rx0 || ry1 <= ry0 {
        return None;
    }
    let (mut x0, mut x1, mut y0, mut y1) = (usize::MAX, 0, usize::MAX, 0);
    for y in ry0..ry1 {
        let row = &mask.pixels()[y * w + rx0..y * w + rx1];
        let Some(first) = row.iter().position(|p| *p != Color::TRANSPARENT) else { continue };
        let first = first + rx0;
        // 从行首切到框右沿：rposition 给的就是绝对列号
        let row = &mask.pixels()[y * w..y * w + rx1];
        let last = row.iter().rposition(|p| *p != Color::TRANSPARENT).unwrap_or(first);
        x0 = x0.min(first);
        x1 = x1.max(last + 1);
        y0 = y0.min(y);
        y1 = y + 1;
    }
    (y0 != usize::MAX).then_some((x0, y0, x1, y1))
}

/// 框内的覆盖率（alpha），按框宽排
pub(super) fn coverage(mask: &Canvas, (x0, y0, x1, y1): (usize, usize, usize, usize)) -> Vec<u8> {
    let w = mask.width() as usize;
    let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0));
    for y in y0..y1 {
        out.extend(mask.pixels()[y * w + x0..y * w + x1].iter().map(|p| p.a));
    }
    out
}

/// 把框内清回全透明（框外按 `dirty_bbox` 的定义本来就是 0）
pub(super) fn clear_rect(mask: &mut Canvas, (x0, y0, x1, y1): (usize, usize, usize, usize)) {
    let w = mask.width() as usize;
    for y in y0..y1 {
        mask.pixels_mut()[y * w + x0..y * w + x1].fill(Color::TRANSPARENT);
    }
}

/// 路径所有点（含贝塞尔控制点）的外接框：曲线落在控制点的凸包里，展平后的折线点都在曲线上，所以保守。
/// 只给**填充**用：描边的斜接没有长度上限（canvas/paths.rs），外扩多少都不保证
pub(super) fn path_hull(path: &crate::Path) -> Option<(f32, f32, f32, f32)> {
    use crate::gfx::path::PathCommand::*;
    let mut b = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    let mut add = |p: &crate::gfx::geometry::Point| {
        b = (b.0.min(p.x), b.1.min(p.y), b.2.max(p.x), b.3.max(p.y));
    };
    for c in path.commands() {
        match c {
            MoveTo(p) | LineTo(p) => add(p),
            QuadTo(a, p) => {
                add(a);
                add(p);
            }
            CubicTo(a, b2, p) => {
                add(a);
                add(b2);
                add(p);
            }
            Close => {}
        }
    }
    (b.0 <= b.2 && b.0.is_finite() && b.1.is_finite() && b.2.is_finite() && b.3.is_finite()).then_some(b)
}
