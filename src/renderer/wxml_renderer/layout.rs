//! 布局树构建、度量与缓存。
//!
//! 从 WXML 节点树 + 数据建出 `RenderNode` 树与 taffy 布局树（样式在这一趟解析），
//! 跑 flexbox 求解，再对「换行后真实高度超过单行高度」的文本做一次修正回填。
//!
//! 建树 + 布局是整个渲染里最贵的一段，所以结果按数据指纹缓存：
//! 数据没变（滚动、动画帧、按压）就直接复用上一次的树。

use super::*;

/// 建一个孩子：(渲染器, 树, 孩子, 祖先链, 继承的文字样式, 兄弟序号, 兄弟数, 有无定位祖先)
pub(super) type ChildBuilder<'a> =
    dyn FnMut(&WxmlRenderer, &mut Tree, &WxmlNode, &[ElementDesc], &InheritedText, usize, usize, bool) -> Option<RenderNode> + 'a;

impl WxmlRenderer {
    /// 需要时重建布局树，并返回这次数据变化的重绘范围（见 [`FramePlan`]）。
    pub(super) fn update_layout_if_needed(
        &mut self,
        nodes: &[WxmlNode],
        data: &JsonValue,
        viewport: Option<(f32, f32)>,
    ) -> FramePlan {
        // 检查视口是否变化（用于虚拟列表）
        let viewport_changed = self.current_viewport != viewport;
        // 布局只跟视口**高**有关（见下面「只有滚动偏移变了」那段）：偏移变了照样可以增量重建
        let height_changed = self.current_viewport.map(|v| v.1) != viewport.map(|v| v.1);
        
        if let Some(cache) = &self.cache {
            // 整棵 JSON 的比较只做一次（下面两个分支都要用）
            let same_data = cache.data == *data;
            if same_data && !viewport_changed {
                return FramePlan::Unchanged; // Cache hit!
            }
            // 只有滚动偏移变了：布局树原样复用，**但这一帧仍按整帧重画**。
            //
            // 参与布局的只有视口高（页面根的 min-height、绝对定位包含块的折算），
            // 滚动偏移一个字节都不进建树 / 样式 / taffy（`build_tree`、`compute_with_text`、
            // reflow 都只借 `&self`，没有副作用）。从前把 (偏移, 高) 整个当缓存键：
            // 页面上有 CSS 动画时每一帧都带着新偏移来问布局，于是滚动期间**每帧**整页重建
            // （sample-app 首页 14ms/帧：模板 1 + 建树样式 4 + 布局 6.6 + 换行修正 2.2）。
            //
            // 为什么不顺手返回 Unchanged、让动画帧走局部重绘：绘制期的视口裁剪按**当前**
            // 偏移剪子树，而已绘制条带是按画它那一刻的偏移算的。偏移变了之后只重画动画
            // 损伤区，条带里落在新裁剪窗外的那部分会被清成底色却没人重画 —— 往回滚时露白。
            // 所以这里只省掉重建，重画范围与以前完全一样（整帧 + scroll-view 缓存作废），
            // 像素逐字节不变；「滚动中只补画新露出的条带」是另一项优化，要单独证明。
            if same_data && !height_changed {
                self.current_viewport = viewport;
                self.scroll_cache.mark_all_dirty();
                return self.full_because(|| "数据没变、只换了滚动偏移".into());
            }
        }
        
        // 更新当前视口
        self.current_viewport = viewport;
        
        // 数据变化，标记所有 scroll-view 缓存为脏
        self.scroll_cache.mark_all_dirty();
        
        // 重建耗时诊断（`MINI_LAYOUT_LOG=1`）：一次 setData 会走完整条
        // 「模板求值 → 建树/样式 → 布局」流水线，是交互卡顿的主要来源，
        // 分段计时能直接指出该优化哪一段。
        let log_timing = std::env::var("MINI_LAYOUT_LOG").is_ok();
        let t_start = std::time::Instant::now();
        let rendered = crate::parser::TemplateEngine::render_with_components(nodes, data, &self.component_templates);
        let t_template = std::time::Instant::now();
        // 数据变了、视口高没变：先试增量重建（没变的子树原样留着，layout_incremental.rs）。
        // 偏移也变了的那一帧重画范围照旧按整帧（与整棵重建时一样，见下面 `plan`）
        let rendered = if !height_changed {
            match self.rebuild_incremental(rendered, data, viewport, viewport_changed, t_start, t_template, log_timing) {
                Ok(plan) => return plan,
                Err(rendered) => rendered,
            }
        } else {
            rendered
        };
        let mut taffy = Tree::new();
        let mut render_nodes = Vec::new();
        
        // `page { color / font-size / font-family … }` 是整页的文字基线，要作为
        // 继承链的起点。之前从内置默认值起步，于是在 page 上定义字号/字色的应用
        // 整体字形与色调都对不上。
        let page = self.page_style();
        for (sib_i, node) in rendered.iter().enumerate() {
            if let Some(rn) = self.build_tree(&mut taffy, node, &[], &page.inherited, sib_i, rendered.len(), false) {
                render_nodes.push(rn);
            }
        }
        let t_build = std::time::Instant::now();
        
        // 构建正常布局树（包含所有节点，fixed 元素也参与布局计算）
        let child_ids: Vec<NodeId> = render_nodes.iter().map(|n| n.taffy_node).collect();
        let viewport_h = self.root_min_height(viewport);
        let root = taffy.new_with_children(self.root_style(viewport_h), &child_ids).unwrap();
        
        // 新的一棵树：布局之后改样式的记录作废，从这里起记这棵（layout_incremental.rs）
        self.layout_touched.borrow_mut().clear();
        let t_layout1 = self.layout_pass(&mut taffy, root, &render_nodes, viewport_h, true);
        // 注：根节点保持 `height: auto`（内容高驱动滚动）。
        //
        // 曾经试过再补一趟「把根高度换成 max(视口, 内容高) 的确定值」来给百分比高度
        // 提供参照物，结果 flex 的分配规则跟着变了 —— canvas 页与组件页整页错位，
        // 与 H5 的差异从 5.4%/3.8% 恶化到 12.5%/10.2%。已回退。
        //
        // 百分比高度塌成 0 的问题改在建树期解决：绝对定位元素在没有定位祖先时
        // 按视口折算高度（见 `components::base` 里的包含块修正），不动布局结构。
        let ms = |a: std::time::Instant, b: std::time::Instant| (b - a).as_secs_f32() * 1000.0;
        let t_fix = std::time::Instant::now();
        if log_timing {
            eprintln!(
                "⏱  重建布局：模板 {:.2}ms  建树+样式 {:.2}ms  布局一遍 {:.2}ms  换行修正 {:.2}ms",
                ms(t_start, t_template),
                ms(t_template, t_build),
                ms(t_build, t_layout1),
                ms(t_layout1, t_fix),
            );
        }
        
        // 获取实际内容高度
        let root_layout = taffy.layout(root).unwrap();
        let content_height = root_layout.size.height / self.scale_factor;

        // 与上一棵树比对，算出这次只需要重画哪一块。
        // 视口变了走的是虚拟列表，结构本来就会变，直接整帧。
        let plan = match (&self.cache, viewport_changed) {
            (Some(old), false) => self.plan_from_diff(
                super::invalidate::OldLayout { nodes: &old.render_nodes, layouts: &old.taffy, content_height: old.content_height, moved: &HashMap::new() },
                &render_nodes,
                &taffy,
                content_height,
            ),
            (None, _) => self.full_because(|| "第一次布局".into()),
            (Some(_), true) => self.full_because(|| "视口（滚动偏移 / 高）与上次布局时不同".into()),
        };
        self.rebuild_laps = Some([
            ms(t_start, t_template),
            ms(t_template, t_build),
            ms(t_build, t_layout1),
            ms(t_layout1, t_fix),
            ms(t_fix, std::time::Instant::now()),
        ]);
        
        self.cache = Some(CachedLayout {
            render_nodes,
            taffy,
            content_height,
            data: data.clone(),
            rendered,
            root,
        });
        self.layout_generation += 1;
        plan
    }

    /// 先把布局算好，并告诉调用方这一帧的重绘范围。
    ///
    /// 必须与绘制分成两步：宿主要按这个范围决定「清多大画布」，
    /// 而清屏发生在绘制之前 —— 边画边算范围就来不及了。
    /// 之后的绘制调用会命中布局缓存，不会重复这趟开销。
    pub fn plan_frame(
        &mut self,
        nodes: &[WxmlNode],
        data: &JsonValue,
        viewport: Option<(f32, f32)>,
    ) -> FramePlan {
        self.update_layout_if_needed(nodes, data, viewport)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn build_tree(&self, taffy: &mut Tree, node: &WxmlNode, ancestors: &[ElementDesc], inherited: &InheritedText, sib_index: usize, sib_count: usize, positioned_ancestor: bool) -> Option<RenderNode> {
        self.build_tree_with(taffy, node, ancestors, inherited, sib_index, sib_count, positioned_ancestor, &mut |r, t, c, anc, inh, k, n, pos| {
            r.build_tree(t, c, anc, inh, k, n, pos)
        })
    }

    /// [`build_tree`](Self::build_tree)，孩子由 `child` 建（整棵重建就是递归 build_tree；
    /// 增量重建换成「没变的孩子原样留着」，layout_incremental.rs）。参数与 build_tree 一一对应
    #[allow(clippy::too_many_arguments)]
    pub(super) fn build_tree_with(&self, taffy: &mut Tree, node: &WxmlNode, ancestors: &[ElementDesc], inherited: &InheritedText, sib_index: usize, sib_count: usize, positioned_ancestor: bool, child: &mut ChildBuilder<'_>) -> Option<RenderNode> {
        let sf = self.scale_factor;
        
        if node.node_type == WxmlNodeType::Text {
            let text = node.text_content.trim();
            if text.is_empty() { return None; }
            // 原始文本节点继承父级的字号/颜色/字重/对齐/行高
            let fs = inherited.font_size;
            // 字族也是继承来的：裸文本（`<view>文字</view>`）同样要按父级的 font-family
            // 度量与绘制，否则宋体页面里这类文字会用黑体量宽度、再用宋体画
            let family = crate::text_family::renderer_for_family(inherited.font_family.as_deref());
            let text_tr = family.as_deref().or(self.text_renderer.as_deref());
            // 默认行高取字体自然行高（≈浏览器 normal），无字体时回退 1.2 倍
            let natural_lh = text_tr
                .map(|tr| tr.natural_line_height_for(text, fs * sf) / sf)
                .unwrap_or(fs * crate::text::NORMAL_LINE_HEIGHT_FACTOR);
            let line_h = inherited.line_height.unwrap_or(natural_lh);
            let tw = match text_tr {
                Some(tr) => tr.measure_text_weighted(
                    text,
                    fs * sf,
                    inherited.letter_spacing * sf,
                    false,
                ),
                None => self.measure_text(text, fs * sf),
            };
            // 居中/右对齐的文本撑满可用宽度，绘制时再按对齐做偏移（否则无法居中）；否则取整到整像素即可
            // （换行判定另有亚像素容差）。撑满时再垫一个「不小于字宽」：父节点宽度靠内容撑（居中的弹层、
            // 横排里没写宽的标签）时 100% 没有参照物、按 0 排，`.btn{text-align:center}` 只剩内边距那么宽
            let fill = matches!(inherited.align, TextAlign::Center | TextAlign::Right);
            let (width_dim, min_w) = if fill { (percent(1.0_f32), length(tw.ceil())) } else { (length(tw.ceil()), auto()) };
            let tn = taffy.new_leaf(Style {
                size: Size { width: width_dim, height: length(line_h * sf) },
                min_size: Size { width: min_w, height: auto() },
                ..Default::default()
            }).unwrap();
            return Some(RenderNode {
                tag: "#text".into(), 
                text: text.into(), 
                attrs: HashMap::new(),
                taffy_node: tn,
                style: NodeStyle {
                    font_size: fs,
                    text_color: inherited.color,
                    font_weight: inherited.weight,
                    text_align: inherited.align,
                    line_height: inherited.line_height,
                    letter_spacing: inherited.letter_spacing,
                    font_family: inherited.font_family.clone(),
                    opacity: 1.0,
                    ..Default::default()
                },
                children: vec![], 
                events: vec![],
            });
        }
        
        if node.node_type != WxmlNodeType::Element { return None; }

        // virtualHost 组件模板的第一层节点：补回被虚化掉的宿主那几层祖先（见 VIRTUAL_HOST_ATTR）
        let vhost_chain = virtual_host_chain(node, ancestors);
        let ancestors = vhost_chain.as_deref().unwrap_or(ancestors);

        let tag = node.tag_name.as_str();
        let mut ctx = ComponentContext {
            scale_factor: sf,
            screen_width: self.screen_width,
            screen_height: self.screen_height,
            stylesheet: &self.stylesheet,
            taffy,
            ancestors: ancestors.to_vec(),
            inherited: inherited.clone(),
            sibling_index: sib_index,
            sibling_count: sib_count,
            has_positioned_ancestor: positioned_ancestor,
        };
        
        // 建树分派：标签 → 组件在 components::registry 那张表里（唯一登记点）
        let spec = crate::renderer::components::spec_for(tag);
        let mut render_node = (spec.build)(node, &mut ctx);
        
        // `<button>` 在微信里是普通容器：里面写 `<text class="…">` 时，那段文字的
        // 颜色/字号/字重由它自己的 CSS 说的算。一律当叶子会把子树整个吞掉、
        // 只用按钮自身的样式把收集到的文本画一遍 —— tea-app 登录页的「微信一键登录」
        // 因此变成继承来的深色 + 大一号字，而 `.wx-btn-t` 明明写了 `color:#fff; font-size:30rpx`。
        // 只有「带元素子节点」的按钮才按容器处理，`<button>纯文字</button>` 仍走叶子那条
        // 成熟路径（默认配色/内边距/最小高度都在那里）。
        let button_as_container = tag == "button"
            && node.children.iter().any(|c| c.node_type == WxmlNodeType::Element);

        if let Some(ref mut rn) = render_node {
            if !Self::is_leaf_component(tag) || button_as_container {
                // 扩展祖先链：当前节点作为子节点的父级，用于后代/子选择器匹配
                let mut child_ancestors = ancestors.to_vec();
                let node_classes: Vec<&str> = node.get_attr("class")
                    .map(|s| s.split_whitespace().collect())
                    .unwrap_or_default();
                // 同上：没有属性选择器就不带属性表，省掉祖先链克隆时的深拷贝
                static EMPTY_ATTRS: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
                let desc_attrs = if self.stylesheet.has_attr_selectors() {
                    &node.attributes
                } else {
                    EMPTY_ATTRS.get_or_init(HashMap::new)
                };
                child_ancestors.push(ElementDesc::new(
                    &node.tag_name,
                    node.get_attr("id"),
                    &node_classes,
                    desc_attrs,
                ));
                
                // 计算传递给子节点的继承文本样式（来自当前节点的计算样式）
                let child_inherited = InheritedText {
                    font_size: rn.style.font_size,
                    color: rn.style.text_color,
                    weight: rn.style.font_weight,
                    align: rn.style.text_align,
                    line_height: rn.style.line_height,
                    letter_spacing: rn.style.letter_spacing,
                    font_family: rn.style.font_family.clone(),
                };
                
                // 子节点是否「有定位祖先」：祖先链上已经有，或者**当前节点自己**被定位。
                // 绝对定位元素的百分比包含块由此决定（见 base.rs 里的包含块修正）。
                // `position: relative` 也算定位祖先（CSS 语义）。
                // 不能问 taffy —— 它的默认 position 就是 Relative，分不出 static。
                let self_positioned = rn.style.is_positioned;
                let child_positioned = positioned_ancestor || self_positioned;

                let mut children = vec![];
                for (sib_ci, c) in node.children.iter().enumerate() {
                    // slot 内容不吃组件的作用域样式（见 `slot_content_ancestors`）
                    let slot_chain = crate::parser::TemplateEngine::slot_content_ancestors(c, &child_ancestors);
                    let child_ancestors = slot_chain.as_deref().unwrap_or(&child_ancestors);
                    if let Some(cr) = child(self, ctx.taffy, c, child_ancestors, &child_inherited, sib_ci, node.children.len(), child_positioned) { 
                        children.push(cr); 
                    }
                }
                
                if !children.is_empty() {
                    let child_ids: Vec<NodeId> = children.iter().map(|c| c.taffy_node).collect();
                    // `build` 交出来的就是基础样式原样（view / block / scroll-view / 未登记标签）：
                    // 直接用，不再把选择器匹配与声明落地重做一遍（见 TagSpec::base_style_only）
                    let reused = if spec.base_style_only { ctx.taffy.style(rn.taffy_node).ok().cloned() } else { None };
                    let (mut ts, ns) = match reused {
                        Some(ts) => (ts, None),
                        None => {
                            let (ts, ns) = build_base_style(node, &mut ctx);
                            (ts, Some(ns))
                        }
                    };
                    
                    // 这里重新算了一遍基础样式，会覆盖组件 build 里设的布局，
                    // 所以需要容器语义的组件必须在这里再补一次（swiper / swiper-item）。
                    if tag == "swiper" {
                        // 方向、一屏几项、两侧边距与 item 尺寸（见 swiper_draw.rs）
                        self.apply_swiper_container_layout(node, &mut ts, &children, &mut ctx);
                    }
                    if tag == "picker-view" {
                        // 各列横排平分、撑满高度；列里的项照常纵向排（绘制时整列按滚轮位置偏移）
                        Self::apply_picker_view_layout(&mut ts, &children, &mut ctx);
                    }
                    if tag == "grid-view" { crate::renderer::components::grid_view::apply_layout(&node.attributes, &mut ts, ctx.scale_factor, ctx.screen_width); }
                    
                    // 对于 scroll-view，使用 Overflow::Visible 让子节点能够正确布局
                    // 裁剪在渲染时通过 canvas.clip_rect 处理
                    //
                    // 试过按 CSS 的滚动容器建模（`Overflow::Scroll`，与编译出的 H5 的
                    // `overflow-y:auto` 一致），图的是「滚动容器的自动最小尺寸为 0」这条
                    // 规则 —— 那样一行里的 scroll-view 就不会因为内部内容很宽而挤压兄弟。
                    // **结论是不能这么做**：本引擎的页面画布高度来自内容高，而滚动容器
                    // 不把内容尺寸往外传，于是「整页套一个 height:100% 的 scroll-view」
                    // 这种写法（uni-app 产物的常见结构）整页塌成空白 ——
                    // 实测 tea-app 21 个页面只剩下底部 tab。
                    if tag == "scroll-view" {
                        ts.overflow.x = taffy::style::Overflow::Visible;
                        ts.overflow.y = taffy::style::Overflow::Visible;
                        super::scroll_views::apply_padding_attr(&node.attributes, &mut ts, ctx.scale_factor);

                        // 横向滚动（与登记、绘制同一个判定，见 scroll_views::scroll_axes）
                        let (scroll_x, _) = super::scroll_views::scroll_axes(&node.attributes);
                        super::scroll_views::place_refresher_slot(node, &children, ctx.taffy);

                        if scroll_x {
                            // 横向滚动：子元素横向排列
                            ts.flex_direction = FlexDirection::Row;
                            ts.flex_wrap = FlexWrap::NoWrap;
                        }
                        
                        // 为 scroll-view 的子元素设置 flex-shrink: 0，防止被压缩
                        for child in &children {
                            if let Ok(mut style) = ctx.taffy.style(child.taffy_node).cloned() {
                                style.flex_shrink = 0.0;
                                ctx.taffy.set_style(child.taffy_node, style).ok();
                            }
                        }
                    }
                    
                    let new_tn = ctx.taffy.new_with_children(ts, &child_ids).unwrap();
                    // `build` 建的那个节点被上面这个顶替了：从树里拿掉（树在增量重建之间一直留着，
                    // 不拿掉每次 setData 都多一批没人引用的节点）。它名下的孩子已经改挂到新节点上，不会被带走
                    super::layout_incremental::remove_owned(ctx.taffy, rn.taffy_node);
                    rn.taffy_node = new_tn;
                    rn.children = children;
                    // 更新样式（保留原有样式中已设置的值，但用新样式覆盖）；复用的那种本来就是它
                    if let Some(ns) = ns {
                        rn.style = ns;
                    }
                }
            }
        }
        
        render_node
    }
    
    /// 取输入类组件的初始值：`value` 或双向绑定写法 `model:value`。
    pub(super) fn input_value_attr(node: &RenderNode) -> String {
        node.attrs
            .get("value")
            .or_else(|| node.attrs.get("model:value"))
            .cloned()
            .unwrap_or_default()
    }

    /// 绘制/交互登记时要不要遍历子节点。
    ///
    /// 叶子组件自己画完就结束；但**建好树以后带子节点的叶子**（`<button><text/></button>`
    /// 这种，见 build_tree 里的 `button_as_container`）说明它其实是按容器建的，
    /// 子树必须照常画出来 —— 否则页面上只剩按钮的底色。
    pub(super) fn draws_children(node: &RenderNode) -> bool {
        !Self::is_leaf_component(&node.tag) || !node.children.is_empty()
    }

    /// 是否是叶子组件（自己把内容合成掉，渲染器不再遍历子节点）。
    ///
    /// 清单在 `components::registry` 那张表里 —— 从前这里另写了一份 13 个标签的
    /// `matches!`，与 build / draw 的分派各自维护，漏一处就会「子节点画两遍」。
    pub(super) fn is_leaf_component(tag: &str) -> bool {
        crate::renderer::components::spec_for(tag).leaf
    }
    
    pub(super) fn measure_text(&self, text: &str, size: f32) -> f32 {
        self.text_renderer.as_deref()
            .map(|tr| tr.measure_text(text, size))
            .unwrap_or(text.chars().count() as f32 * size * 0.6)
    }

}

/// 按与 text.rs 绘制一致的贪心算法统计文本在给定可用宽度下的换行行数。
/// 用于第二遍布局修正文本盒子高度（含 `\n` 硬换行）。
///
/// 从前这里是 `components::count_wrapped_text_lines` 的一份逐行相同的拷贝（同样的硬换行切分、
/// 同样的逐字度量闭包、同样的 `max(1)`）；现在直接用那一份，顺带吃到它的记忆表
pub(super) fn count_wrapped_lines(tr: &TextRenderer, text: &str, max_width: f32, size: f32, letter_spacing: f32, bold: bool) -> usize {
    crate::renderer::components::count_wrapped_text_lines(tr, text, max_width, size, letter_spacing, bold)
}

/// virtualHost 组件模板的第一层节点：祖先链补上被虚化掉的宿主（由外向内）。不是这种节点给 None
fn virtual_host_chain(node: &WxmlNode, ancestors: &[ElementDesc]) -> Option<Vec<ElementDesc>> {
    let tags = node.get_attr(crate::parser::template::VIRTUAL_HOST_ATTR)?;
    let mut v = ancestors.to_vec();
    v.extend(tags.split_whitespace().map(|t| ElementDesc::new(t, None, &[], &HashMap::new())));
    Some(v)
}
