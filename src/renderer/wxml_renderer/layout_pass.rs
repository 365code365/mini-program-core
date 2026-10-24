//! 排版那几步：页面根的样式、排一遍 + 第二遍修正、带文字度量的 taffy 求解、量内容高。
//!
//! 从 layout.rs 搬出来（**纯搬迁**，一行逻辑没改）：增量布局之后那个文件到了 548 行。
//! 整棵重建、增量重建（layout_incremental.rs）与增量的校验（layout_verify.rs）都走这里，
//! 三处的排法因此只有一份。
use super::*;

impl WxmlRenderer {
    /// 页面根的样式（宽 = 画布宽、高由内容驱动但至少 `viewport_h`）。整棵重建与增量重建的校验共用
    pub(super) fn root_style(&self, viewport_h: f32) -> Style {
        let canvas_w = self.screen_width * self.scale_factor;
        Style {
            // 页面根：高度由内容驱动（要能滚），但**至少一屏**。
            //
            // 这等价于 CSS 的 `min-height: 100vh` —— 编译出的 H5 端一直是这么写的
            // （`#app{min-height:100vh}`），而这边只写了 `height:auto`。差别在内容不足
            // 一屏的页面上肉眼可见：页面根上写 `flex:1` 的容器（框架产物的标准结构）
            // 撑不满视口，底下漏出 `page` 的背景色 —— tea-app 登录页底部就露出一条
            // 12px 的浅米色，而设计稿/微信里整屏都是深绿。
            size: Size { width: length(canvas_w), height: auto() },
            min_size: Size { width: auto(), height: length(viewport_h) },
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    /// 页面根的最小高（视口高，见上面建根节点那段的注释），物理像素
    pub(super) fn root_min_height(&self, viewport: Option<(f32, f32)>) -> f32 {
        viewport
            .map(|(_, h)| h * self.scale_factor)
            .filter(|h| *h > 1.0)
            .or(self.page_style().explicit_height)
            .unwrap_or(self.screen_height * self.scale_factor)
    }

    /// 排版一遍 + 第二遍修正（换行补高、绝对定位包含块、瀑布流），返回第一遍排完的时刻（分段计时用）。
    /// 修正改的样式记进 `layout_touched`（这是缓存那棵树），增量重建时先还原
    pub(super) fn layout_pass(&self, taffy: &mut Tree, root: NodeId, render_nodes: &[RenderNode], viewport_h: f32, record: bool) -> std::time::Instant {
        self.compute_with_text(taffy, root, Size::MAX_CONTENT);
        let t_layout1 = std::time::Instant::now();
        self.record_touch.set(record);
        // 第二遍：按实际宽度修正换行文本高度、按实际包含块高度修正绝对定位高度，再重新布局
        let mut need_relayout = self.correct_overflow(taffy, render_nodes);
        need_relayout |= self.correct_absolute_heights(taffy, render_nodes, viewport_h);
        if need_relayout { self.compute_with_text(taffy, root, Size::MAX_CONTENT); }
        // 瀑布流 grid-view：子项高度定了才能按最矮列摆放（components::grid_view），摆完再排一遍
        if crate::renderer::components::grid_view::masonry_pass(taffy, render_nodes, self.scale_factor) { self.compute_with_text(taffy, root, Size::MAX_CONTENT); }
        self.record_touch.set(false);
        t_layout1
    }

    /// 测量给定 WXML+数据的内容总高度（逻辑像素），用于自适应画布尺寸。
    pub fn measure_content_height(&self, nodes: &[WxmlNode], data: &JsonValue) -> f32 {
        let rendered = crate::parser::TemplateEngine::render_with_components(nodes, data, &self.component_templates);
        let mut taffy = Tree::new();
        let mut render_nodes = Vec::new();
        // 同 `update_layout_if_needed`：继承起点是 `page { … }`（字号影响内容高）
        let page = self.page_style();
        for (sib_i, node) in rendered.iter().enumerate() {
            if let Some(rn) = self.build_tree(&mut taffy, node, &[], &page.inherited, sib_i, rendered.len(), false) {
                render_nodes.push(rn);
            }
        }
        let child_ids: Vec<NodeId> = render_nodes.iter().map(|n| n.taffy_node).collect();
        let root = taffy.new_with_children(
            Style {
                size: Size { width: length(self.screen_width * self.scale_factor), height: auto() },
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &child_ids,
        ).unwrap();
        self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        let mut need_relayout = self.correct_overflow(&mut taffy, &render_nodes);
        need_relayout |= self.correct_absolute_heights(
            &mut taffy,
            &render_nodes,
            self.screen_height * self.scale_factor,
        );
        if need_relayout { self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT); }
        if crate::renderer::components::grid_view::masonry_pass(&mut taffy, &render_nodes, self.scale_factor) { self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT); }
        taffy.layout(root).unwrap().size.height / self.scale_factor
    }
    
    /// 第二遍布局修正：首遍 `compute_layout` 后文本节点的实际宽度已知，
    /// 对「宽度为百分比/100%」等在 build 阶段无法预知换行的文本重新计算换行行数，
    /// 据此修正其盒子高度，避免多行文本被压成一行高而与后续兄弟节点重叠。
    /// 返回是否有节点高度被修改（需要重新 compute_layout）。
    ///
    /// 这是对标准 CSS「文本按可用宽度自动换行、盒子高度随行数增长」语义的补齐。
    /// 用文本度量闭包计算布局：仅带 TextMeasure 上下文的（block/auto 宽）文本按可用
    /// 宽度解析换行与高度；其它叶子沿用各自 Style 里的显式尺寸（known dimensions）。
    pub(super) fn compute_with_text(&self, taffy: &mut Tree, root: NodeId, available: Size<AvailableSpace>) {
        let tr = self.text_renderer.as_deref();
        // taffy 0.12 的度量回调多了一个 `&Style` 参数（叶子自己的样式），
        // 文本度量用不到它 —— 换行只取决于「已知尺寸 + 可用宽度」。
        //
        // 其余叶子回答 0：非替换元素（空的 `<view>` 等）的内容尺寸本来就是 0，
        // 所以 `min-height:auto` 允许它们被压缩 —— 音乐播放器那个 4px 高的进度条里
        // 塞一个 13px 的圆把手，就是靠这条被压回去的。
        // **替换元素**（image/canvas/video/input）不一样：它们有固有尺寸，
        // 由各自的 `build` 显式写上 `min_size`（见 `ImageComponent::build`），
        // 不靠这里兜——否则「所有定尺寸叶子都不可压」会误伤前者。
        let _ = taffy.compute_layout_with_measure(
            root,
            available,
            |known, avail, _id, ctx: Option<&mut TextMeasure>, _style| match ctx {
                Some(tm) => measure_text_node(known, avail, tm, tr),
                None => Size {
                    width: known.width.unwrap_or(0.0),
                    height: known.height.unwrap_or(0.0),
                },
            },
        );
    }

}
