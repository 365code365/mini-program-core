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
            .find(|(et, ..)| et == "change")
            .map(|(_, h, ..)| h.clone());

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
