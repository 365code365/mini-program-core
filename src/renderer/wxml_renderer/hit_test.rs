//! 命中测试与事件绑定表。
//!
//! 绘制期把「哪个矩形上挂了什么处理器」记进 `event_bindings`，命中时按注册顺序
//! 逆序找最深的那个，再沿祖先链冒泡（`catch` 截断）。
//!
//! 覆盖层与正常流是**两套坐标**（视口 vs 内容），所以命中入口按 `is_fixed` 分开：
//! 混在一起查会让页面顶部的普通元素冒充覆盖层，把弹窗的点击漏到下层。
//! `<picker>` 的可点区域另存一张表 —— `bindchange` 不是可命中事件，
//! 事件绑定表里找不到它。

use super::*;
use crate::renderer::components::EventPhase;

impl WxmlRenderer {
    /// 本帧登记的 picker（宿主据此把点击变成选择面板）
    pub fn picker_regions(&self) -> &[PickerBinding] {
        &self.picker_regions
    }

    /// 命中某个 picker（逻辑坐标；`fixed_only` 时只看覆盖层里的）
    pub fn picker_hit(&self, x: f32, y: f32, fixed_only: bool) -> Option<&PickerBinding> {
        self.picker_regions.iter().rev().find(|p| {
            !p.disabled
                && (!fixed_only || p.is_fixed)
                && x >= p.bounds.x
                && x <= p.bounds.x + p.bounds.width
                && y >= p.bounds.y
                && y <= p.bounds.y + p.bounds.height
        })
    }

    /// 从一个 `<picker>` 节点抽出宿主弹面板需要的全部信息
    pub(super) fn build_picker_binding(
        node: &RenderNode,
        bounds: &GeoRect,
        id: &str,
        disabled: bool,
        is_fixed: bool,
    ) -> PickerBinding {
        let mode = node.attrs.get("mode").cloned().unwrap_or_else(|| "selector".to_string());
        let range_key = node.attrs.get("range-key").cloned();
        let raw_range = node.attrs.get("range").map(|s| s.as_str()).unwrap_or("[]");
        let parsed = crate::renderer::components::parse_attr_json(raw_range);

        let mut range = Vec::new();
        let mut multi_range = Vec::new();
        match &parsed {
            JsonValue::Array(items) if mode == "multiSelector" => {
                for col in items {
                    multi_range.push(Self::picker_column(col, range_key.as_deref()));
                }
            }
            JsonValue::Array(_) => {
                range = Self::picker_column(&parsed, range_key.as_deref());
            }
            _ => {}
        }

        let change_handler = node
            .events
            .iter()
            .find(|e| e.event_type == "change")
            .map(|e| e.handler.clone());

        PickerBinding {
            id: id.to_string(),
            bounds: *bounds,
            mode,
            range,
            multi_range,
            value: node.attrs.get("value").cloned().unwrap_or_default(),
            fields: node.attrs.get("fields").cloned().unwrap_or_default(),
            start: node.attrs.get("start").cloned().unwrap_or_default(),
            end: node.attrs.get("end").cloned().unwrap_or_default(),
            change_handler,
            disabled,
            is_fixed,
        }
    }

    /// 把一列选项（字符串数组，或对象数组 + `range-key`）转成显示文本
    pub(super) fn picker_column(value: &JsonValue, range_key: Option<&str>) -> Vec<String> {
        let Some(items) = value.as_array() else { return Vec::new() };
        items
            .iter()
            .map(|item| match (item, range_key) {
                (JsonValue::Object(_), Some(key)) => item
                    .get(key)
                    .map(crate::parser::expr::render_value)
                    .unwrap_or_default(),
                _ => crate::parser::expr::render_value(item),
            })
            .collect()
    }

    pub(super) fn get_component_id(node: &RenderNode, bounds: &GeoRect) -> String {
        if let Some(id) = node.attrs.get("id") {
            if !id.is_empty() {
                return id.clone();
            }
        }
        // 用整数格式化：`{:.0}` 走浮点转十进制，逐节点逐帧调用时开销不可忽略
        format!("{}_{}_{}", node.tag, bounds.x as i32, bounds.y as i32)
    }

    pub fn get_event_bindings(&self) -> &[EventBinding] { 
        &self.event_bindings 
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<&EventBinding> {
        self.event_bindings.iter().rev().find(|b| b.bounds.contains(&crate::Point::new(x, y)))
    }

    /// 视口坐标 (x, y) 是否落在 `position: fixed` 覆盖层上。
    ///
    /// 这是「弹窗不穿透」的判定依据：覆盖层画在页面之上，落在它范围内的点击
    /// 必须由它消费掉，即使那块区域没有绑事件（例如只写了半透明遮罩）。
    pub fn fixed_layer_hit(&self, x: f32, y: f32) -> bool {
        let p = crate::Point::new(x, y);
        self.fixed_hit_regions.iter().any(|r| r.contains(&p))
    }

    /// 只在覆盖层里做命中冒泡（视口坐标）
    pub fn hit_test_bubble_fixed(&self, x: f32, y: f32, event_type: &str) -> Vec<EventBinding> {
        self.hit_test_bubble_filtered(x, y, event_type, true)
    }

    /// 只在正常流里做命中冒泡（内容坐标，已含滚动偏移）
    pub fn hit_test_bubble_flow(&self, x: f32, y: f32, event_type: &str) -> Vec<EventBinding> {
        self.hit_test_bubble_filtered(x, y, event_type, false)
    }

    /// 覆盖层的事件绑定（供宿主在「跳过覆盖层重绘」的帧里补回来）
    pub fn fixed_event_bindings(&self) -> Vec<EventBinding> {
        self.event_bindings.iter().filter(|b| b.is_fixed).cloned().collect()
    }

    /// 覆盖层的遮挡区域（同上，供宿主缓存）
    pub fn fixed_hit_regions(&self) -> &[GeoRect] {
        &self.fixed_hit_regions
    }

    /// 把上一帧缓存的覆盖层绑定与遮挡区域补回来。
    ///
    /// 事件绑定每帧都会清空重建，而覆盖层在「内容没变」的帧里是跳过重绘的 ——
    /// 不补回来的话，那些帧里弹窗就没有任何绑定，点击会直接穿到下层页面。
    pub fn restore_fixed_bindings(&mut self, bindings: &[EventBinding], regions: &[GeoRect]) {
        self.event_bindings.extend(bindings.iter().cloned());
        self.fixed_hit_regions.clear();
        self.fixed_hit_regions.extend_from_slice(regions);
    }
    
    /// 命中测试并返回事件冒泡链。
    ///
    /// 对齐官方语义：`tap` 事件从最内层节点向外冒泡，`catchtap` 阻止继续冒泡。
    /// 由于渲染层是扁平的绑定列表，这里用包围盒面积升序近似节点由内到外的层级
    /// （子节点面积必然 <= 父节点）。遇到 `is_catch` 的绑定后停止（含该项）。
    ///
    /// 只返回与 `event_type` 匹配的绑定（点击对应 "tap"）。
    pub fn hit_test_bubble(&self, x: f32, y: f32, event_type: &str) -> Vec<EventBinding> {
        let p = crate::Point::new(x, y);
        let mut matched: Vec<EventBinding> = self.event_bindings.iter()
            .filter(|b| b.event_type == event_type && b.bounds.contains(&p))
            .cloned()
            .collect();
        Self::bubble_chain(&mut matched)
    }

    /// 同 [`Self::hit_test_bubble`]，但只看覆盖层或只看正常流的绑定
    pub(super) fn hit_test_bubble_filtered(
        &self,
        x: f32,
        y: f32,
        event_type: &str,
        fixed: bool,
    ) -> Vec<EventBinding> {
        let p = crate::Point::new(x, y);
        let mut matched: Vec<EventBinding> = self.event_bindings.iter()
            .filter(|b| b.is_fixed == fixed && b.event_type == event_type && b.bounds.contains(&p))
            .cloned()
            .collect();
        Self::bubble_chain(&mut matched)
    }

    /// 按微信语义排好序的**派发链**：先捕获（由外向内）再冒泡（由内向外）。
    ///
    /// - `catch*` / `capture-catch:*`：触发后终止后续传播（含它自己）
    /// - `mut-bind:*`：互斥绑定，一条触发后其余 mut-bind 不再触发，但 `bind`/`catch` 照旧
    /// - `scope`：`Some(true)` 只看 `position:fixed` 覆盖层，`Some(false)` 只看正常流，
    ///   `None` 两者都看（覆盖层的坐标系是视口，正常流含滚动偏移，所以调用方通常要分开问）
    ///
    /// 调用方只需按顺序逐条调用返回的绑定即可 —— 传播语义全在这里收口，
    /// 免得每个输入路径各写一遍（此前 fixed 分支只取了链首一条，等于覆盖层内不冒泡）。
    pub fn dispatch_chain(
        &self,
        x: f32,
        y: f32,
        event_type: &str,
        scope: Option<bool>,
    ) -> Vec<EventBinding> {
        let p = crate::Point::new(x, y);
        let mut matched: Vec<EventBinding> = self
            .event_bindings
            .iter()
            .filter(|b| {
                scope.map(|s| b.is_fixed == s).unwrap_or(true)
                    && b.event_type == event_type
                    && b.bounds.contains(&p)
            })
            .cloned()
            .collect();
        // 面积升序 ≈ 由内向外（子节点面积必然不大于父节点）
        matched.sort_by(|a, b| {
            let area_a = a.bounds.width * a.bounds.height;
            let area_b = b.bounds.width * b.bounds.height;
            area_a.partial_cmp(&area_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut chain = Vec::new();
        // ── 捕获阶段：由外向内 ──
        for b in matched.iter().filter(|b| b.phase == EventPhase::Capture).rev() {
            let stop = b.is_catch;
            chain.push(b.clone());
            if stop {
                return chain;
            }
        }
        // ── 冒泡阶段：由内向外 ──
        let mut mut_fired = false;
        for b in matched.iter().filter(|b| b.phase == EventPhase::Bubble) {
            if b.mut_bind {
                if mut_fired {
                    continue;
                }
                mut_fired = true;
            }
            let stop = b.is_catch;
            chain.push(b.clone());
            if stop {
                break;
            }
        }
        chain
    }

    /// 命中点上**最内层**的那条绑定（不限事件类型）。
    ///
    /// 事件对象里的 `target` 指「真正被摸到的节点」，与「挂着处理函数的节点」
    /// （`currentTarget`）是两回事：`touchend` 只绑在外层容器上时，
    /// `target` 仍应是里面那个列表项 —— 页面靠 `e.target.dataset.id` 取行号。
    /// 完全没有任何绑定的节点这里找不到（渲染层只留了绑定表），返回 None。
    pub fn innermost_binding_at(&self, x: f32, y: f32, scope: Option<bool>) -> Option<EventBinding> {
        let p = crate::Point::new(x, y);
        self.event_bindings
            .iter()
            .filter(|b| scope.map(|s| b.is_fixed == s).unwrap_or(true) && b.bounds.contains(&p))
            .min_by(|a, b| {
                let area_a = a.bounds.width * a.bounds.height;
                let area_b = b.bounds.width * b.bounds.height;
                area_a.partial_cmp(&area_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// 命中点上是否存在该事件的 `catch` 绑定（含捕获阶段）。
    ///
    /// 手势层要用它：遮罩上的 `catchtouchmove` 就是「不许滚动下层页面」的标准写法。
    pub fn has_catch_for(&self, x: f32, y: f32, event_type: &str) -> bool {
        let p = crate::Point::new(x, y);
        self.event_bindings
            .iter()
            .any(|b| b.is_catch && b.event_type == event_type && b.bounds.contains(&p))
    }

    pub(super) fn bubble_chain(matched: &mut Vec<EventBinding>) -> Vec<EventBinding> {
        let matched = std::mem::take(matched);
        let mut matched = matched;
        
        // 面积升序：最内层（最小）在前
        matched.sort_by(|a, b| {
            let area_a = a.bounds.width * a.bounds.height;
            let area_b = b.bounds.width * b.bounds.height;
            area_a.partial_cmp(&area_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // 冒泡：从内到外，遇 catch 停止
        let mut chain = Vec::new();
        for b in matched {
            let stop = b.is_catch;
            chain.push(b);
            if stop {
                break;
            }
        }
        chain
    }
    

    /// 按 class 找节点，返回它的布局尺寸（物理像素）。布局回归用例用它断言
    /// 「这一格有没有被压窄/折行」，比逐像素比图更能说明问题出在哪。
    #[cfg(test)]
    pub fn node_size_by_class(&self, class: &str) -> Option<(f32, f32)> {
        fn walk(taffy: &Tree, n: &RenderNode, class: &str) -> Option<(f32, f32)> {
            let hit = n
                .attrs
                .get("class")
                .map(|c| c.split_whitespace().any(|c| c == class))
                .unwrap_or(false);
            if hit {
                let l = taffy.layout(n.taffy_node).ok()?;
                return Some((l.size.width, l.size.height));
            }
            n.children.iter().find_map(|c| walk(taffy, c, class))
        }
        let cache = self.cache.as_ref()?;
        cache.render_nodes.iter().find_map(|n| walk(&cache.taffy, n, class))
    }

    /// 获取事件绑定数量
    pub fn event_count(&self) -> usize {
        self.event_bindings.len()
    }
    
    /// 打印所有事件绑定（调试用）
    pub fn debug_events(&self) {
        for (i, binding) in self.event_bindings.iter().enumerate() {
            println!("   [{}] {} -> {} bounds=({:.1},{:.1},{:.1},{:.1}) data={:?}", 
                i, binding.event_type, binding.handler,
                binding.bounds.x, binding.bounds.y, binding.bounds.width, binding.bounds.height,
                binding.data);
        }
    }
}
