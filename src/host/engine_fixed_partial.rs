//! 覆盖层（fixed 层）重画：整层（`render_fixed_layer`，从 engine_frame.rs 搬来，那边超了 500 行）与
//! 只重画一块（只有点击态 / 光标 / 覆盖层里的 scroll-view 那几块变了，engine_damage.rs `fixed_partial`）。
//!
//! 为什么要它：支付密码键盘这种「全屏半透明遮罩 + 底部面板 + 一排键」整个画在覆盖层里，按一下键
//! 点击态一亮一灭，从前每次整层清掉重画（一加 PLY110 上 9~13ms）再整层 12MB 传给 GPU —— 招财银行
//! 转账页「点击输入很不灵敏」。这里只清、只画那个键的盒子：节点树照样整棵走一遍（命中区与事件绑定
//! 每次都要重建），画的时候裁剪区外的不画（与页面画布的局部重画同一套：`set_damage_clip` + 画布裁剪），
//! 所以这块里的像素与整层重画逐字节相同；GPU 镜像只补传这几行（`LayerCanvas::write_rows`）。
use super::engine::MiniEngine;
use crate::{Color, Rect as GeoRect};

impl MiniEngine {
    /// 重画 fixed 覆盖层，并记下它的动画标记、事件绑定与遮挡区域（跳过它的帧要补回来）
    pub(super) fn render_fixed_layer(&mut self, vp: f32) {
        // fixed 层是页面视口坐标：引擎画了导航栏时它比画面矮一截（engine_navbar.rs）
        self.ensure_fixed_canvas();
        // 右上角胶囊画在这一层的最上面（为什么放这里见 capsule.rs）。有引擎画的导航栏时
        // 胶囊在导航栏画布里（paint_nav），这里不画
        let capsule = if self.nav_rows() > 0 {
            None
        } else {
            self.capsule_spec().map(|(r, dark)| (r, dark, self.capsule_pressed()))
        };
        let dpr = self.dpr;
        let prev_rows = self.fixed_rows;
        // 只有点击态 / 光标那几块变了（engine_damage.rs `fixed_partial`）：只清、只画这一块，GPU 只补传这几行
        if let Some(rect) = self.dmg.fixed_partial.take().filter(|_| capsule.is_none() && prev_rows.is_some()) {
            return self.render_fixed_partial(vp, rect);
        }
        let Self { renderer, fixed_canvas, page_stack, page_data, interaction, dmg, .. } = self;
        let fixed_canvas = fixed_canvas.write();
        // 只清上次画过的那几行：其余行本来就全透明（上次的 `opaque_row_span` 量出来的；画布新建时整张透明）。
        // 从前整张清：覆盖层整屏 1080×2200，只有一个右下角的悬浮按钮，也要清 950 万字节、再整张扫一遍 ——
        // 「节点与滚动」页滚动时悬浮按钮上的 scrollTop 一变，这一帧覆盖层就要 6ms（一加 PLY110）
        if let Some((a, b)) = prev_rows {
            fixed_canvas.clear_band(a as i32, b as i32 + 1, Color::new(0, 0, 0, 0));
        }
        if let (Some(r), Some(page)) = (renderer.as_mut(), page_stack.last()) {
            // 在两趟之间清零，才能分辨「动画在页面里」还是「在覆盖层里」
            r.set_animations_active(false);
            r.render_fixed_elements(fixed_canvas, &page.wxml_nodes, page_data, interaction, vp);
            dmg.fixed_layer_animates = r.has_active_animations();
            r.set_animations_active(dmg.page_animates || dmg.fixed_layer_animates);
            dmg.cached_fixed_bindings = r.fixed_event_bindings();
            dmg.cached_fixed_regions = r.fixed_hit_regions().to_vec();
        }
        let had_capsule = capsule.is_some();
        if let Some((rect, dark, pressed)) = capsule {
            fixed_canvas.reset_clip();
            crate::host::capsule::draw(fixed_canvas, rect, dpr, dark, pressed);
        }
        // 有导航栏时胶囊的「已画」由 paint_nav 记（换前景色会让导航栏的签名变，必然重画）
        if had_capsule || self.nav_rows() == 0 {
            self.capsule_painted();
        }
        self.dmg.fixed_dirty = false;
        // 覆盖层真正画过的行区间：`present_to_buffer` 只合成这几行，
        // 传 None 会**整层跳过** —— 底部吸底操作栏、弹窗遮罩就全丢了。
        let (opaque, nonzero) = crate::host::render::row_spans(self.fixed_canvas.read());
        self.fixed_rows = opaque;
        self.release_idle_rows(Some(nonzero));
        self.note_fixed_area_positions();
    }

    pub(super) fn render_fixed_partial(&mut self, vp: f32, rect: GeoRect) {
        let Self { renderer, fixed_canvas, page_stack, page_data, interaction, dmg, .. } = self;
        let (cw, ch) = (fixed_canvas.read().width(), fixed_canvas.read().height());
        let rect = crate::geometry::snap_out(rect, cw, ch);
        if rect.width <= 0.0 || rect.height <= 0.0 {
            dmg.fixed_dirty = false;
            return;
        }
        let (y0, y1) = (rect.y.max(0.0) as u32, ((rect.y + rect.height).ceil() as u32).min(ch));
        let canvas = fixed_canvas.write_rows(y0, y1);
        canvas.reset_clip();
        canvas.clear_area(&rect, Color::new(0, 0, 0, 0));
        canvas.clip_rect(rect);
        if let (Some(r), Some(page)) = (renderer.as_mut(), page_stack.last()) {
            r.set_damage_clip(Some(rect));
            r.set_animations_active(false);
            r.render_fixed_elements(canvas, &page.wxml_nodes, page_data, interaction, vp);
            dmg.fixed_layer_animates = r.has_active_animations();
            r.set_animations_active(dmg.page_animates || dmg.fixed_layer_animates);
            r.set_damage_clip(None);
            dmg.cached_fixed_bindings = r.fixed_event_bindings();
            dmg.cached_fixed_regions = r.fixed_hit_regions().to_vec();
        }
        canvas.reset_clip();
        dmg.fixed_dirty = false;
        // 覆盖层画过的行：这一块只可能在原来的区间里加行，并起来（多合成几行全透明的不影响结果）
        self.fixed_rows = match self.fixed_rows {
            Some((a, b)) => Some((a.min(y0), b.max(y1.saturating_sub(1)))),
            None => Some((y0, y1.saturating_sub(1))),
        };
        self.note_fixed_area_positions();
    }

    /// 覆盖层画过之后：记下覆盖层里每个 scroll-view 这次画到的位置（页面那一趟只记页面里的，
    /// 覆盖层自己重画时不记的话，下一帧它们永远「动了」）
    pub(super) fn note_fixed_area_positions(&mut self) {
        for (id, c) in &self.interaction.scroll_controllers {
            if self.interaction.scroll_area_bounds(id).is_some_and(|(_, fixed)| fixed) {
                self.dmg.drawn_area_positions.insert(id.clone(), c.get_position());
            }
        }
    }
}
