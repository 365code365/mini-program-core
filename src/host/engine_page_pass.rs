//! 页面画布那一趟前后的记账（从 `engine_frame.rs` 拆出，**纯搬迁**，两个函数改成 `pub(super)` 供它调用）：
//! 整条带重画之前按内容高换好画布；画完之后记动画包围盒、页面动画标记、scroll-view 位置、布局偏移。
use super::engine::MiniEngine;
use crate::Canvas;

impl MiniEngine {
    /// 整条带重画之前：布局（结果进缓存）并按内容高换好画布。高度对的话什么也不做
    pub(super) fn presize_canvas(&mut self, scroll: f32, vp: f32) {
        let (Some(r), Some(page)) = (self.renderer.as_mut(), self.page_stack.last()) else { return };
        r.plan_frame(&page.wxml_nodes, &self.page_data, Some((scroll, vp)));
        let Some(content_h) = r.cached_content_height().filter(|h| *h > 1.0) else { return };
        let required = (content_h * self.dpr).ceil() as u32;
        if required > 0 && required != self.canvas.read().height() {
            let width = self.canvas.read().width();
            self.canvas.replace(Canvas::new(width, required));
        }
    }

    /// setData 让内容**变高**、失效分析却只给了一块矩形时（列表追加，见 `invalidate_grow.rs`）：
    /// 画布接着长高，已经画好的那条带原样搬过去，滚动范围跟着放大。
    ///
    /// 从前内容高一变就整帧：换一块新画布、整条带重画、整张重传。这里只多一次条带大小的拷贝
    /// （一屏多一点，几兆字节），新露出来的那截由这一帧的矩形 / 下一帧的补条带画上
    pub(super) fn grow_canvas_to_content(&mut self) {
        let Some(content_h) = self.renderer.as_ref().and_then(|r| r.cached_content_height()).filter(|h| *h > 1.0) else {
            return;
        };
        let (width, old_h) = (self.canvas.read().width(), self.canvas.read().height());
        let required = (content_h * self.dpr).ceil() as u32;
        if required <= old_h {
            return;
        }
        let mut fresh = Canvas::new(width, required);
        if let Some((b0, b1)) = self.drawn_band {
            // 条带以外的行本来就不保证是对的（滑回去之前会补画），只搬条带
            let y0 = (b0.floor().max(0.0) as usize).min(old_h as usize);
            let y1 = (b1.ceil().max(0.0) as usize).min(old_h as usize);
            let row = width as usize;
            if y1 > y0 {
                fresh.pixels_mut()[y0 * row..y1 * row].copy_from_slice(&self.canvas.read().pixels()[y0 * row..y1 * row]);
            }
        }
        // 换代数：GPU 镜像据此把条带整段重传（与内容高变了、整帧时一样，只是不用重画）
        self.canvas.replace(fresh);
        let vp = self.viewport_height();
        self.scroll.update_content_height(content_h, vp);
    }

    /// 页面画布画完之后：动画包围盒、页面动画标记、scroll-view 位置、布局偏移的记账
    pub(super) fn after_page_pass(&mut self, anim_complete: bool, scroll: f32) {
        if let Some(r) = self.renderer.as_mut() {
            let bounds = r.take_animated_bounds();
            // 只有这一趟的裁剪范围覆盖了全部动画节点，收集到的包围盒才是完整的（与桌面
            // 窗体同一条规则）。纯 setData 的局部帧只碰了一小块，拿它覆盖旧记录、或拿它的
            // 「有没有动画」覆盖旧结论，都可能让外面的动画就此冻住 —— 那种帧只增不减
            if anim_complete {
                self.dmg.last_animated_bounds = bounds;
                self.dmg.page_animates = r.has_active_animations();
            } else {
                // 只增不减：这一趟里新动起来的（局部重画画到的 swiper 刚开始翻页）也记上，
                // 不然下一帧「在动却没有包围盒」只能整帧
                for b in bounds {
                    if !self.dmg.last_animated_bounds.contains(&b) {
                        self.dmg.last_animated_bounds.push(b);
                    }
                }
                self.dmg.page_animates |= r.has_active_animations();
            }
            // 覆盖层这一帧可能不重画：渲染器的「有动画」要把它那份并回来，宿主据此续帧
            r.set_animations_active(self.dmg.page_animates || self.dmg.fixed_layer_animates);
        }
        self.dmg.drawn_area_positions.clear();
        for (id, c) in &self.interaction.scroll_controllers {
            self.dmg.drawn_area_positions.insert(id.clone(), c.get_position());
        }
        self.dmg.layout_scroll = Some(scroll);
    }
}
