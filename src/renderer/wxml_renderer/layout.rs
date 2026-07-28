//! 布局树构建、度量与缓存。
//!
//! 从 WXML 节点树 + 数据建出 `RenderNode` 树与 taffy 布局树（样式在这一趟解析），
//! 跑 flexbox 求解，再对「换行后真实高度超过单行高度」的文本做一次修正回填。
//!
//! 建树 + 布局是整个渲染里最贵的一段，所以结果按数据指纹缓存：
//! 数据没变（滚动、动画帧、按压）就直接复用上一次的树。

use super::*;
// taffy 0.12 的尺寸值是位压缩结构，读值走这几个封装（见 components::base）
use crate::renderer::components::dim_is_auto;

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
        
        if let Some(cache) = &self.cache {
            if cache.data == *data && !viewport_changed {
                return FramePlan::Unchanged; // Cache hit!
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
        let canvas_w = self.screen_width * self.scale_factor;
        let viewport_h = viewport
            .map(|(_, h)| h * self.scale_factor)
            .filter(|h| *h > 1.0)
            .or(page.explicit_height)
            .unwrap_or(self.screen_height * self.scale_factor);
        let root = taffy.new_with_children(
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
            },
            &child_ids,
        ).unwrap();
        
        self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        let t_layout1 = std::time::Instant::now();
        // 第二遍：按实际宽度修正换行文本高度、按实际包含块高度修正绝对定位高度，再重新布局
        let mut need_relayout = self.correct_overflow(&mut taffy, &render_nodes);
        need_relayout |= self.correct_absolute_heights(&mut taffy, &render_nodes, viewport_h);
        if need_relayout {
            self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        }
        // 注：根节点保持 `height: auto`（内容高驱动滚动）。
        //
        // 曾经试过再补一趟「把根高度换成 max(视口, 内容高) 的确定值」来给百分比高度
        // 提供参照物，结果 flex 的分配规则跟着变了 —— canvas 页与组件页整页错位，
        // 与 H5 的差异从 5.4%/3.8% 恶化到 12.5%/10.2%。已回退。
        //
        // 百分比高度塌成 0 的问题改在建树期解决：绝对定位元素在没有定位祖先时
        // 按视口折算高度（见 `components::base` 里的包含块修正），不动布局结构。
        if log_timing {
            let ms = |a: std::time::Instant, b: std::time::Instant| (b - a).as_secs_f32() * 1000.0;
            eprintln!(
                "⏱  重建布局：模板 {:.1}ms  建树+样式 {:.1}ms  布局一遍 {:.1}ms  换行修正 {:.1}ms",
                ms(t_start, t_template),
                ms(t_template, t_build),
                ms(t_build, t_layout1),
                ms(t_layout1, std::time::Instant::now()),
            );
        }
        
        // 获取实际内容高度
        let root_layout = taffy.layout(root).unwrap();
        let content_height = root_layout.size.height / self.scale_factor;

        // 与上一棵树比对，算出这次只需要重画哪一块。
        // 视口变了走的是虚拟列表，结构本来就会变，直接整帧。
        let plan = match (&self.cache, viewport_changed) {
            (Some(old), false) => self.plan_from_diff(old, &render_nodes, &taffy, content_height),
            _ => FramePlan::Full,
        };
        
        self.cache = Some(CachedLayout {
            render_nodes,
            taffy,
            content_height,
            data: data.clone(),
        });
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
        if need_relayout {
            self.compute_with_text(&mut taffy, root, Size::MAX_CONTENT);
        }
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

    #[allow(clippy::too_many_arguments)]
    pub(super) fn build_tree(&self, taffy: &mut Tree, node: &WxmlNode, ancestors: &[ElementDesc], inherited: &InheritedText, sib_index: usize, sib_count: usize, positioned_ancestor: bool) -> Option<RenderNode> {
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
            // 居中/右对齐的文本撑满可用宽度，绘制时再按对齐做偏移（否则无法居中）
            let width_dim: Dimension = if matches!(inherited.align, TextAlign::Center | TextAlign::Right) {
                percent(1.0)
            } else {
                // 取整到整像素即可（换行判定另有亚像素容差），不额外加宽
                length(tw.ceil())
            };
            let tn = taffy.new_leaf(Style {
                size: Size { width: width_dim, height: length(line_h * sf) },
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
        
        let mut render_node = match tag {
            "text" => TextComponent::build(node, &mut ctx),
            "button" => ButtonComponent::build(node, &mut ctx),
            "icon" => IconComponent::build(node, &mut ctx),
            "progress" => ProgressComponent::build(node, &mut ctx),
            "switch" => SwitchComponent::build(node, &mut ctx),
            "checkbox" => CheckboxComponent::build(node, &mut ctx),
            "checkbox-group" => CheckboxGroupComponent::build(node, &mut ctx),
            "radio" => RadioComponent::build(node, &mut ctx),
            "radio-group" => RadioGroupComponent::build(node, &mut ctx),
            "slider" => SliderComponent::build(node, &mut ctx),
            "input" | "textarea" => InputComponent::build(node, &mut ctx),
            "image" => ImageComponent::build(node, &mut ctx),
            "video" => VideoComponent::build(node, &mut ctx),
            "canvas" => CanvasComponent::build(node, &mut ctx),
            "swiper" => SwiperComponent::build(node, &mut ctx),
            "swiper-item" => SwiperItemComponent::build(node, &mut ctx),
            "rich-text" => RichTextComponent::build(node, &mut ctx),
            "picker" => PickerComponent::build(node, &mut ctx),
            "picker-view" => PickerViewComponent::build(node, &mut ctx),
            "picker-view-column" => PickerViewColumnComponent::build(node, &mut ctx),
            _ => ViewComponent::build(node, &mut ctx),
        };
        
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
                    if let Some(cr) = self.build_tree(ctx.taffy, c, &child_ancestors, &child_inherited, sib_ci, node.children.len(), child_positioned) { 
                        children.push(cr); 
                    }
                }
                
                if !children.is_empty() {
                    let child_ids: Vec<NodeId> = children.iter().map(|c| c.taffy_node).collect();
                    let (mut ts, ns) = build_base_style(node, &mut ctx);
                    
                    // 这里重新算了一遍基础样式，会覆盖组件 build 里设的布局，
                    // 所以需要容器语义的组件必须在这里再补一次（swiper / swiper-item）。
                    if tag == "swiper" {
                        let vertical = node.get_attr("vertical")
                            .map(|v| v == "true" || v == "{{true}}")
                            .unwrap_or(false);
                        ts.flex_direction = if vertical { FlexDirection::Column } else { FlexDirection::Row };
                        ts.flex_wrap = FlexWrap::NoWrap;
                        if dim_is_auto(ts.size.width) {
                            ts.size.width = percent(1.0);
                        }
                        if dim_is_auto(ts.size.height) {
                            ts.size.height = length(SwiperComponent::DEFAULT_HEIGHT * ctx.scale_factor);
                        }
                        // 每个 item 占满一屏且不收缩（对齐 HTML .wx-swiper-item{flex:0 0 100%}）
                        for child in &children {
                            if let Ok(mut style) = ctx.taffy.style(child.taffy_node).cloned() {
                                style.size = taffy::geometry::Size { width: percent(1.0), height: percent(1.0) };
                                style.min_size.width = percent(1.0);
                                style.flex_shrink = 0.0;
                                style.flex_grow = 0.0;
                                ctx.taffy.set_style(child.taffy_node, style).ok();
                            }
                        }
                    }
                    
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

                        // 检查是否是横向滚动
                        let scroll_x = node.get_attr("scroll-x")
                            .map(|s| s == "true" || s == "{{true}}")
                            .unwrap_or(false);

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
                    
                    rn.taffy_node = new_tn;
                    rn.children = children;
                    // 更新样式（保留原有样式中已设置的值，但用新样式覆盖）
                    rn.style = ns;
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

    pub(super) fn is_leaf_component(tag: &str) -> bool {
        // rich-text / picker 不再是叶子：rich-text 自建带样式的文本片段子树；
        // picker 渲染其子元素（触发视图，如“当前选择：xxx”）而非合成占位 UI。
        matches!(tag, 
            "text" | "button" | "icon" | "progress" | "switch" | 
            "checkbox" | "radio" | "slider" | "input" | "textarea" | "image" | "video" | "canvas" |
            "picker-view-column"
        )
    }
    
    pub(super) fn measure_text(&self, text: &str, size: f32) -> f32 {
        self.text_renderer.as_deref()
            .map(|tr| tr.measure_text(text, size))
            .unwrap_or(text.chars().count() as f32 * size * 0.6)
    }

}

/// 按与 text.rs 绘制一致的贪心算法统计文本在给定可用宽度下的换行行数。
/// 用于第二遍布局修正文本盒子高度（含 `\n` 硬换行）。
pub(super) fn count_wrapped_lines(tr: &TextRenderer, text: &str, max_width: f32, size: f32, letter_spacing: f32, bold: bool) -> usize {
    if max_width <= 0.0 {
        return text.split('\n').count().max(1);
    }
    let mut lines = 0usize;
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines += 1;
            continue;
        }
        let chars: Vec<char> = paragraph.chars().collect();
        let measure = |s: &[char]| -> f32 {
            s.iter().map(|c| tr.measure_char_weighted(*c, size, bold) + letter_spacing).sum()
        };
        lines += crate::renderer::components::wrap_paragraph_lines(&chars, max_width, measure).len();
    }
    lines.max(1)
}
