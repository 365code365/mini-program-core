//! 滑块（`<slider>`）的交互状态：每帧登记、拖动、点按跳转、受控值。
//!
//! ## 语义（对齐微信）
//! - **点一下**轨道：滑块跳到手指处，发一次 `change`（走 tap 链路，见 [`InteractionManager::handle_click`]）；
//! - **横向拖**：跟手，每次值变化发 `changing`，松手发 `change`。拖动的归属由手势仲裁决定
//!   （`host::gesture` 的 `Slider` 目标）：横向锁定才交给滑块，**竖着划照样滚页面** ——
//!   从前一按到滑块就把整次触摸吃掉，手指落在滑块那一行上整页都滚不动；
//! - 从滑块圆点上抓起时保持抓取偏移，不会一抓就跳到手指正下方；从轨道别处拖起则直接跳到手指处。
//!
//! ## 受控值
//! 拖动/点按写进 `states`，同时记下「当时 WXML 里的 `value`」（`slider_origin`）。
//! 之后页面 `setData` 改了 `value`（与记下的不同）就以页面为准、丢掉交互状态；
//! 没改（页面没绑定、或绑定了但不回写）就保持用户拖到的位置。
//! 从前交互状态一旦写下就永远盖住 `value` —— 页面再怎么 setData 滑块都不动。
use super::*;
use crate::renderer::components::SliderAttrs;

/// 本帧登记的一个 slider（绘制时由渲染端写入）
#[derive(Clone, Copy, Debug)]
pub struct SliderSpec {
    pub attrs: SliderAttrs,
    /// 登记的盒子（正常流是内容坐标，覆盖层是视口坐标）
    pub bounds: Rect,
    /// 轨道左端与长度：与 `bounds` 同一坐标系，和绘制同一份算式（`slider_track_span`）
    pub track_x: f32,
    pub track_w: f32,
    pub is_fixed: bool,
}

impl SliderSpec {
    /// 横坐标 → 吸附后的值
    pub fn value_at(&self, x: f32) -> f64 {
        self.attrs.value_at(x, self.track_x, self.track_w)
    }

    /// 某个值对应的滑块圆心横坐标
    pub fn thumb_x(&self, v: f64) -> f32 {
        self.track_x + self.track_w * self.attrs.progress(v)
    }

    /// 盒子中心：派发 `changing` / `change` 的命中点（手指拖出盒子之外时事件照样要落到滑块上）
    pub fn center(&self) -> (f32, f32) {
        (self.bounds.x + self.bounds.width / 2.0, self.bounds.y + self.bounds.height / 2.0)
    }
}

/// 拖动中的滑块
#[derive(Clone, Debug)]
pub struct DraggingSlider {
    pub id: String,
    pub spec: SliderSpec,
    /// 从圆点上抓起时，圆心与手指的横向差（拖动全程保持）
    grab_dx: f32,
    /// 当前值
    pub value: f64,
    /// 值至少变过一次：松手才发 `change`，也据此吞掉随后的 tap
    pub moved: bool,
    /// 拖动开始时 WXML 里的 `value`（受控判断用）
    origin: f64,
}

/// 抓取判定半径：圆点半径之外再留一点余量（微信的抓取区是固定 28px 的方块，手指比鼠标粗）
fn grab_radius(block: f32) -> f32 {
    block.max(28.0) / 2.0 + 6.0
}

impl InteractionManager {
    /// 登记一个 slider（每帧）。页面改过 `value` 时在这里丢掉过期的交互状态。
    pub fn note_slider(&mut self, id: String, spec: SliderSpec) {
        let dragging = self.dragging_slider.as_ref().map(|d| d.id == id).unwrap_or(false);
        if !dragging && self.slider_value(&id, spec.attrs.value).is_none() && self.slider_origin.contains_key(&id) {
            self.states.remove(&id);
            self.slider_origin.remove(&id);
        }
        self.sliders.insert(id, spec);
    }

    /// 本页登记过的 slider
    pub fn slider_spec(&self, id: &str) -> Option<&SliderSpec> {
        self.sliders.get(id)
    }

    /// 本页登记过的所有 slider 的 id（测试与诊断用）
    pub fn slider_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.sliders.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// 交互状态里的值。页面已经改过 `value`（与写入状态时的不同）、又不在拖这一个时返回 `None`
    /// —— 调用方该用 WXML 的值。
    pub fn slider_value(&self, id: &str, attr_value: f64) -> Option<f64> {
        if let Some(d) = self.dragging_slider.as_ref().filter(|d| d.id == id) {
            return Some(d.value);
        }
        let v = self.states.get(id)?.value.trim().parse::<f64>().ok()?;
        match self.slider_origin.get(id) {
            Some(o) if (o - attr_value).abs() > 1e-9 => None,
            _ => Some(v),
        }
    }

    /// 直接设定一个 slider 的值（表单重置、点按跳转）
    pub fn set_slider_value(&mut self, id: &str, v: f64) {
        let origin = self.sliders.get(id).map(|s| s.attrs.value).unwrap_or(v);
        self.states.insert(id.to_string(), ComponentState { checked: false, value: SliderAttrs::format(v) });
        self.slider_origin.insert(id.to_string(), origin);
    }

    /// 命中点最上层的元素是一个可用的 slider 时返回它的 id（`fixed`：只看覆盖层，坐标是视口坐标）
    pub fn slider_at(&self, x: f32, y: f32, fixed: bool) -> Option<String> {
        let el = if fixed { self.hit_test_fixed(x, y) } else { self.hit_test_flow(x, y) }?;
        (el.interaction_type == InteractionType::Slider && self.sliders.contains_key(&el.id)).then(|| el.id.clone())
    }

    /// 手势锁定为横向、交给滑块：从按下点 `start_x` 判断是不是抓在圆点上，再推到当前 `x`。
    /// 返回值变了时的新值。
    pub fn begin_slider_drag(&mut self, id: &str, start_x: f32, x: f32) -> Option<f64> {
        let spec = *self.sliders.get(id)?;
        let current = self.slider_value(id, spec.attrs.value).unwrap_or(spec.attrs.value);
        let thumb = spec.thumb_x(current);
        let grab_dx = if (start_x - thumb).abs() <= grab_radius(spec.attrs.block_size) { thumb - start_x } else { 0.0 };
        self.dragging_slider = Some(DraggingSlider {
            id: id.to_string(),
            spec,
            grab_dx,
            value: current,
            moved: false,
            origin: spec.attrs.value,
        });
        self.drag_slider_to(x)
    }

    /// 拖动推进。返回值变了时的新值（没变不必重画、也不发 `changing`）。
    pub fn drag_slider_to(&mut self, x: f32) -> Option<f64> {
        let d = self.dragging_slider.as_mut()?;
        let v = d.spec.value_at(x + d.grab_dx);
        if v == d.value {
            return None;
        }
        d.value = v;
        d.moved = true;
        let (id, origin) = (d.id.clone(), d.origin);
        self.states.insert(id.clone(), ComponentState { checked: false, value: SliderAttrs::format(v) });
        self.slider_origin.insert(id, origin);
        Some(v)
    }

    /// 松手 / 取消：结束拖动，返回 `(id, 最终值, 是否真的拖动过)`
    pub fn end_slider_drag(&mut self) -> Option<(String, f64, bool)> {
        self.dragging_slider.take().map(|d| (d.id, d.value, d.moved))
    }

    /// 是否正在拖动滑块
    pub fn is_dragging_slider(&self) -> bool {
        self.dragging_slider.is_some()
    }

    /// 点一下 slider：跳到手指处（微信点按轨道就发一次 `change`，值没变也发）
    pub(super) fn tap_slider(&mut self, element: &InteractiveElement, x: f32) -> Option<InteractionResult> {
        let spec = *self.sliders.get(&element.id)?;
        let value = spec.value_at(x);
        self.set_slider_value(&element.id, value);
        Some(InteractionResult::SliderChange { id: element.id.clone(), value })
    }
}
