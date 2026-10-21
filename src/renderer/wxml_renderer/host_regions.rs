//! 交给宿主的两类区域：输入框的属性（弹哪种软键盘）与原生组件的摆放。
//!
//! 与 `picker_regions` 同一套登记方式：绘制期在 `register_interactive_element` 里随手记下，
//! 坐标已经是逻辑像素（正常流是内容坐标、fixed 是视口坐标），每帧随绘制重建。
//! 只有被画到的节点会登记（视口 ± 裁剪余量）—— 屏幕外的原生组件本来也不该显示。
//!
//! ## 原生组件
//! 宿主在创建选项里点名的标签（`video` / `map` / `web-view` / `camera` …）由宿主用自己的
//! 控件承载（微信里它们就是原生层）。引擎只报位置与属性；`video` 自己的解码与绘制
//! 在 `components::video` 里按 `is_host_native` 跳过，免得两边各解一份。
use super::*;

/// 输入框的宿主可见属性（聚焦时随 `inputFocus` 事件发出去）
#[derive(Debug, Clone)]
pub struct InputRegion {
    /// 与 `get_component_id` 一致（输入事件按它找绑定）
    pub component_id: String,
    /// `input` 或 `textarea`
    pub tag: String,
    /// `type` / `password` / `placeholder` / `maxlength` / `confirm-type` / `confirm-hold` /
    /// `adjust-position` / `cursor-spacing`（只抄这几个，别把整张属性表拷来拷去）
    pub attrs: HashMap<String, String>,
    pub is_fixed: bool,
}

/// 一个由宿主承载的原生组件在本帧的位置
#[derive(Debug, Clone)]
pub struct NativeRegion {
    pub tag: String,
    /// 与 `get_component_id` 一致：有 `id` 属性就是它，否则含坐标（不稳定，宿主别拿它当键）
    pub component_id: String,
    /// 稳定键：`id` 属性优先，否则「标签#第 n 个」（文档顺序）
    pub key: String,
    /// 节点属性（`src` / `autoplay` / `latitude` …，模板求值之后的值）
    pub attrs: HashMap<String, String>,
    /// 逻辑像素：正常流是内容坐标，fixed 是视口坐标
    pub bounds: GeoRect,
    pub is_fixed: bool,
    /// 节点上的事件绑定 (事件名, 处理函数, 所属组件, dataset)：宿主回传 `ended` 之类时用
    pub events: Vec<(String, String, String, HashMap<String, String>)>,
}

const INPUT_ATTRS: &[&str] = &[
    "type",
    "password",
    "placeholder",
    "maxlength",
    "confirm-type",
    "confirm-hold",
    "adjust-position",
    "cursor-spacing",
];

impl WxmlRenderer {
    /// 宿主承载的原生组件标签（换页重建渲染器时由宿主设）
    pub fn set_native_components(&mut self, tags: &[String]) {
        self.native_tags = tags.to_vec();
    }

    /// 这个标签是否由宿主承载
    pub fn is_host_native(&self, tag: &str) -> bool {
        self.native_tags.iter().any(|t| t == tag)
    }

    /// 本帧画到的原生组件（文档顺序）
    pub fn native_regions(&self) -> &[NativeRegion] {
        &self.native_regions
    }

    /// 按组件 id 找输入框属性
    pub fn input_region(&self, component_id: &str) -> Option<&InputRegion> {
        self.input_regions.iter().find(|r| r.component_id == component_id)
    }

    /// 绘制期登记（`register_interactive_element` 调）
    pub(super) fn register_host_regions(
        &mut self,
        node: &RenderNode,
        bounds: &GeoRect,
        component_id: &str,
        is_fixed: bool,
    ) {
        let tag = node.tag.as_str();
        if tag == "input" || tag == "textarea" {
            let attrs = INPUT_ATTRS
                .iter()
                .filter_map(|k| node.attrs.get(*k).map(|v| (k.to_string(), v.clone())))
                .collect();
            self.input_regions.retain(|r| r.component_id != component_id);
            self.input_regions.push(InputRegion {
                component_id: component_id.to_string(),
                tag: tag.to_string(),
                attrs,
                is_fixed,
            });
            return;
        }
        if !self.is_host_native(tag) {
            return;
        }
        // 同一节点一帧里可能被登记两次（scroll-view 子树会再走一遍），按组件 id 去重
        if self.native_regions.iter().any(|r| r.component_id == component_id && r.is_fixed == is_fixed) {
            return;
        }
        let key = match node.attrs.get("id").filter(|s| !s.is_empty()) {
            Some(id) => id.clone(),
            None => {
                let nth = self.native_regions.iter().filter(|r| r.tag == tag).count();
                format!("{tag}#{nth}")
            }
        };
        let events = node
            .events
            .iter()
            .map(|e| (e.event_type.clone(), e.handler.clone(), e.owner.clone(), e.data.clone()))
            .collect();
        self.native_regions.push(NativeRegion {
            tag: tag.to_string(),
            component_id: component_id.to_string(),
            key,
            attrs: node.attrs.clone(),
            bounds: *bounds,
            is_fixed,
            events,
        });
    }

    /// 正常流重画前：只留覆盖层登记的（覆盖层可能整帧不重画，由它自己重建）
    pub(super) fn retain_fixed_host_regions(&mut self) {
        self.input_regions.retain(|r| r.is_fixed);
        self.native_regions.retain(|r| r.is_fixed);
    }

    /// 覆盖层重画前：清掉覆盖层上一次登记的
    pub(super) fn clear_fixed_host_regions(&mut self) {
        self.input_regions.retain(|r| !r.is_fixed);
        self.native_regions.retain(|r| !r.is_fixed);
        self.scroll_views.retain(|s| !s.is_fixed);
        self.swiper_regions.retain(|s| !s.is_fixed);
    }
}
