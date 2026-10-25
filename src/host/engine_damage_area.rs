//! 动了的 scroll-view 要重画哪：页面上的按它们在页面画布上的盒子并起来，覆盖层里的另算（只重画覆盖层那块）。
//! 从 engine_damage.rs 搬出（那边超了 500 行）。
use super::engine::MiniEngine;
use super::engine_damage_rects::union;
use crate::Rect as GeoRect;

/// scroll-view 动了之后要重画多少
pub(super) enum AreaDamage {
    Nothing,
    /// 页面上要重画的一块（可能同时还有覆盖层里的一块）
    Rect(GeoRect, Option<GeoRect>),
    /// 只有覆盖层里的 scroll-view 动了（覆盖层画布的设备像素）
    Fixed(GeoRect),
    Full,
}

impl MiniEngine {
    /// 相对上次画进页面画布时，哪些 scroll-view 的位置变了（与桌面 `scroll_area_damage` 一致）
    pub(super) fn scroll_area_damage(&self) -> AreaDamage {
        let mut acc = None;
        let mut fixed_acc: Option<GeoRect> = None;
        let mut moved_any = false;
        for (id, c) in &self.interaction.scroll_controllers {
            let pos = c.get_position();
            let moved = self.dmg.drawn_area_positions.get(id).map(|p| (p - pos).abs() > 0.001).unwrap_or(true);
            if !moved {
                continue;
            }
            moved_any = true;
            // 本帧没登记（滚出视口了）的不知道画在哪
            let Some((b, fixed)) = self.interaction.scroll_area_bounds(id) else {
                return AreaDamage::Full;
            };
            let s = self.dpr;
            let r = GeoRect::new(b.x * s, b.y * s, b.width * s, b.height * s);
            if fixed {
                // 覆盖层里的 scroll-view（点餐页「已选菜品」抽屉、弹窗里的列表）：页面一个像素不用动，
                // 只重画覆盖层里它那块（engine_fixed_partial.rs）。从前整条带 + 覆盖层，一加上每帧 18~29ms、整层 24MB 上传
                moved_any = true;
                fixed_acc = union(fixed_acc, r);
                continue;
            }
            acc = union(acc, r);
        }
        match (acc, fixed_acc) {
            (Some(r), f) => AreaDamage::Rect(r, f),
            (None, Some(f)) => AreaDamage::Fixed(f),
            // 动了但宽高为 0（还没布局出来）：保守整帧
            (None, None) if moved_any => AreaDamage::Full,
            (None, None) => AreaDamage::Nothing,
        }
    }
}
