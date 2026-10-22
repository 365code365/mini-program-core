//! `engine_damage.rs` 用的损伤区几何：并、合并成几块、夹进裁剪窗，以及补条带的两个常量。
//! **纯搬迁**（engine_damage.rs 超 500 行时拆出来），一行逻辑没改。
use crate::renderer::VIEWPORT_CULL_MARGIN_PX;
use crate::Rect as GeoRect;

pub(super) fn union(a: Option<GeoRect>, b: GeoRect) -> Option<GeoRect> {
    if b.width <= 0.0 || b.height <= 0.0 {
        return a;
    }
    Some(match a {
        None => b,
        Some(a) => {
            let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
            let (x1, y1) = ((a.x + a.width).max(b.x + b.width), (a.y + a.height).max(b.y + b.height));
            GeoRect::new(x0, y0, x1 - x0, y1 - y0)
        }
    })
}

/// 滚动中裁剪窗里没画的行攒到这么多（物理像素）就顺手补上，不等视口真滑出条带。
/// 等到滑出再补，一次就是 400 多行（余量那么宽）—— 滑动里每隔一百多 pt 一个尖峰。
///
/// 从前是余量的 1/4（100 行）。一加 PLY110 上补一条的耗时随行数线性涨（首页渐变头图那截每行约
/// 0.08~0.1ms），100~190 行一条就是 7~16ms，快甩时每五六帧一次掉帧。改成 1/16（25 行）：每帧补的
/// 行数接近这一帧滚过的行数，耗时摊到每一帧（一帧 2~3ms）
pub(super) const EAGER_STRIP_PX: f32 = VIEWPORT_CULL_MARGIN_PX / 16.0;

/// 一帧最多分几趟画（每一趟都要把裁剪窗里的节点树走一遍，只是窗外不画）
pub(super) const MAX_PASSES: usize = 4;

/// 损伤区合并：两块并起来不比分开大多少（≤ 1.5 倍）就并；最后剩不超过 [`MAX_PASSES`] 块。
/// 不并的话一个滑出条带的帧（补条带 + 两个动画节点）要分三趟，并太狠又把半屏拖进来。
pub(super) fn merge_rects(mut rects: Vec<GeoRect>) -> Vec<GeoRect> {
    let area = |r: &GeoRect| r.width * r.height;
    loop {
        let n = rects.len();
        // 找「并起来多出的面积」最小的一对
        let mut best: Option<(usize, usize, f32)> = None;
        for i in 0..n {
            for j in i + 1..n {
                let u = union(Some(rects[i]), rects[j]).unwrap_or(rects[i]);
                let waste = area(&u) - area(&rects[i]) - area(&rects[j]);
                if best.map(|b| waste < b.2).unwrap_or(true) {
                    best = Some((i, j, waste));
                }
            }
        }
        let Some((i, j, waste)) = best else { return rects };
        let u = union(Some(rects[i]), rects[j]).unwrap_or(rects[i]);
        if n <= MAX_PASSES && area(&u) > 1.5 * (area(&rects[i]) + area(&rects[j])) && waste > 0.0 {
            return rects;
        }
        rects[i] = u;
        rects.swap_remove(j);
    }
}

/// `r` 夹进 `[x0,x1)×[y0,y1)`；夹完是空的返回 None
pub(super) fn clamp_rect(r: GeoRect, x0: f32, x1: f32, y0: f32, y1: f32) -> Option<GeoRect> {
    let (a, b) = (r.x.max(x0), (r.x + r.width).min(x1));
    let (c, d) = (r.y.max(y0), (r.y + r.height).min(y1));
    (b > a && d > c).then(|| GeoRect::new(a, c, b - a, d - c))
}
