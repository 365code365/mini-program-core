//! `<form>`：点了 `form-type` 按钮之后，找到它所在的表单并列出表单里的控件。
//!
//! 依据官方 component/form.html：点 `form-type="submit"` 的 button 时，把表单里带 `name`
//! 的 switch / input / textarea / checkbox(-group) / radio(-group) / slider / picker 的值
//! 按 `{name: value}` 提交；`form-type="reset"` 重置表单。
//!
//! 为什么走布局树而不是本帧的交互元素表：元素表只登记**视口附近**画到的节点
//! （视口裁剪），长表单滚过之后上面那几项根本不在表里，提交时就会悄悄丢字段。
//! 布局树里每个节点都在；控件的当前值（用户拨过 / 输入过）在 `InteractionManager.states`，
//! 按与登记时同一套算法得出的组件 id 去取（`get_component_id`）。

use super::*;

/// 按钮的 `form-type`
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FormAction {
    Submit,
    Reset,
}

/// 表单里的一个控件。`id` 是组件 id（`InteractionManager.states` 的键）
#[derive(Clone, Debug)]
pub enum FormControl {
    /// input / textarea：属性里的初始值
    Text { id: String, initial: String },
    Switch { id: String, initial: bool },
    Slider { id: String, initial: f64, min: f64 },
    /// picker 自己不留状态（选中值由页面在 bindchange 里写回 `value`），直接给属性值，类型按 mode
    Picker { value: serde_json::Value },
    /// checkbox-group（`multi`）/ radio-group 里的成员：(组件 id, value, 属性里是否选中)
    Group { multi: bool, items: Vec<(String, String, bool)> },
    /// 带 `wx://form-field` 的自定义组件：值是组件实例 data 里的 `value`（逻辑层按实例键取）
    Component { key: String },
}

/// 组件宿主上内置表单 behavior 的标记（`__ff`，见 `parser::template::FORM_FIELD_ATTR`）
fn ff_has(n: &RenderNode, what: &str) -> bool {
    n.attrs.get(crate::parser::template::FORM_FIELD_ATTR).map(|v| v.split_whitespace().any(|x| x == what)).unwrap_or(false)
}

/// 进入一个自定义组件宿主：它的模板内部是不是「表单看不见」的影子树。
/// 微信里 form 只收自己这棵树里的控件；组件内部的控件要组件带 `wx://form-field-group`
/// （按钮要 `wx://form-field-button`）才算。写进组件 slot 的内容属于使用方，不受影响
fn enters_shadow(n: &RenderNode, unless: &str) -> bool {
    n.attrs.contains_key(crate::parser::template::COMPONENT_INSTANCE_ATTR) && !ff_has(n, unless)
}

fn is_slot_content(n: &RenderNode) -> bool {
    n.attrs.contains_key(crate::parser::template::SLOT_CONTENT_ATTR)
}

#[derive(Clone, Debug)]
pub struct FormField {
    /// 没写 `name` 的控件不提交，但重置照样要管
    pub name: Option<String>,
    pub control: FormControl,
}

/// 一次 form-type 按钮点击命中的表单
#[derive(Clone, Debug)]
pub struct FormHit {
    pub action: FormAction,
    /// 表单里的控件，文档顺序
    pub fields: Vec<FormField>,
}

/// 遍历时一个祖先表单：(节点, 传给它的原点, 是否在覆盖层里)
type FormFrame<'a> = (&'a RenderNode, (f32, f32), bool);

impl WxmlRenderer {
    /// (x, y) 上是不是一个 `form-type` 按钮，是的话给出它所在的表单。
    /// `fixed` 与命中坐标系一致：覆盖层是视口坐标，正常流是内容坐标。
    pub fn form_button_at(&self, x: f32, y: f32, fixed: bool, interaction: &InteractionManager) -> Option<FormHit> {
        let cache = self.cache.as_ref()?;
        let mut forms: Vec<FormFrame> = Vec::new();
        let mut best: Option<(FormAction, FormFrame)> = None;
        for n in &cache.render_nodes {
            self.find_form_button(n, &cache.taffy, (0.0, 0.0), (false, false), (x, y, fixed), interaction, &mut forms, &mut best);
        }
        let (action, (form, origin, in_fixed)) = best?;
        let mut fields = Vec::new();
        self.collect_form_fields(form, &cache.taffy, origin, (in_fixed, false), interaction, None, &mut fields);
        Some(FormHit { action, fields })
    }

    /// 节点的逻辑包围盒与组件 id，以及它的子节点该用的原点（scroll-view 按当前滚动偏移挪）
    pub(super) fn form_geom(&self, n: &RenderNode, taffy: &Tree, origin: (f32, f32), interaction: &InteractionManager) -> Option<(GeoRect, String, (f32, f32))> {
        let l = taffy.layout(n.taffy_node).ok()?;
        let sf = self.scale_factor;
        // fixed 子树的根按覆盖层钉住它的原点算（视口坐标，与 query.rs 同一条）。从前用它在正常流里的位置：
        // 页面比视口高时 `bottom:0` 的底栏在内容最底下，点底栏里的 label / navigator / 表单按钮全落空
        // （阅读器的主题 radio 包在 label 里，点字不切换）
        let pinned = (n.style.is_fixed && self.fixed_origins.0 == self.layout_generation)
            .then(|| self.fixed_origins.1.get(&n.taffy_node).copied())
            .flatten();
        let origin = pinned.unwrap_or(origin);
        let (x, y) = (origin.0 + l.location.x, origin.1 + l.location.y);
        let rect = GeoRect::new(x / sf, y / sf, l.size.width / sf, l.size.height / sf);
        let id = Self::get_component_id(n, &rect);
        let (mut cx, mut cy) = (x, y);
        if n.tag == "scroll-view" {
            if let Some(c) = interaction.get_scroll_controller(&id) {
                // 与绘制期 `y + (-scroll * sf)` 同一个算式，子节点算出来的组件 id 才对得上
                if super::scroll_views::scroll_axes(&n.attrs).0 {
                    cx += -c.get_position() * sf;
                } else {
                    cy += -c.get_position() * sf;
                }
            }
        }
        Some((rect, id, (cx, cy)))
    }

    #[allow(clippy::too_many_arguments)]
    fn find_form_button<'a>(
        &self,
        n: &'a RenderNode,
        taffy: &Tree,
        origin: (f32, f32),
        (in_fixed, shadow): (bool, bool),
        pt: (f32, f32, bool),
        interaction: &InteractionManager,
        forms: &mut Vec<FormFrame<'a>>,
        best: &mut Option<(FormAction, FormFrame<'a>)>,
    ) {
        let Some((rect, _, child_origin)) = self.form_geom(n, taffy, origin, interaction) else { return };
        let fixed = in_fixed || n.style.is_fixed;
        // 影子树的边界：form 自己所在的那棵树里的按钮才是它的（组件模板里的 form 管组件里的按钮）
        let shadow = shadow && !is_slot_content(n) && n.tag != "form";
        if n.tag == "button" && !shadow && fixed == pt.2 && rect.contains(&crate::Point::new(pt.0, pt.1)) {
            let action = match n.attrs.get("form-type").map(|s| s.trim()) {
                Some("submit") => Some(FormAction::Submit),
                Some("reset") => Some(FormAction::Reset),
                _ => None,
            };
            let disabled = super::scroll_views::bool_attr(&n.attrs, "disabled");
            if let (Some(action), Some(form), false) = (action, forms.last(), disabled) {
                // 后访问到的盖在上面（与命中表「取最后一个」一致）
                *best = Some((action, *form));
            }
        }
        let pushed = n.tag == "form";
        if pushed {
            forms.push((n, origin, in_fixed));
        }
        let child_shadow = shadow || enters_shadow(n, "button");
        for c in &n.children {
            self.find_form_button(c, taffy, child_origin, (fixed, child_shadow), pt, interaction, forms, best);
        }
        if pushed {
            forms.pop();
        }
    }

    /// `group`：正在一个 checkbox-group / radio-group 里（成员记进它）
    #[allow(clippy::too_many_arguments)]
    fn collect_form_fields(
        &self,
        n: &RenderNode,
        taffy: &Tree,
        origin: (f32, f32),
        (in_fixed, shadow): (bool, bool),
        interaction: &InteractionManager,
        mut group: Option<&mut Vec<(String, String, bool)>>,
        out: &mut Vec<FormField>,
    ) {
        let Some((_, id, child_origin)) = self.form_geom(n, taffy, origin, interaction) else { return };
        let fixed = in_fixed || n.style.is_fixed;
        let shadow = !is_slot_content(n) && shadow;
        let name = n.attrs.get("name").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        // 组件内部（影子树）的控件 form 看不见：只往下找 slot 内容与更深处
        if shadow {
            let child_shadow = shadow;
            for c in &n.children {
                self.collect_form_fields(c, taffy, child_origin, (fixed, child_shadow), interaction, group.as_deref_mut(), out);
            }
            return;
        }
        // wx://form-field 的组件：它自己就是一个控件
        if ff_has(n, "field") {
            if let Some(key) = n.attrs.get(crate::parser::template::COMPONENT_INSTANCE_ATTR) {
                out.push(FormField { name: name.clone(), control: FormControl::Component { key: key.clone() } });
            }
        }
        let child_shadow = enters_shadow(n, "group");
        let control = match n.tag.as_str() {
            "input" | "textarea" => Some(FormControl::Text { id: id.clone(), initial: Self::input_value_attr(n) }),
            "switch" => Some(FormControl::Switch { id: id.clone(), initial: n.style.custom_data > 0.5 }),
            "slider" => {
                // 与绘制同一份解析：value 夹到 [min, max] 并按 step 吸附
                let a = crate::renderer::components::SliderAttrs::from_attrs(&n.attrs);
                Some(FormControl::Slider { id: id.clone(), initial: a.value, min: a.min })
            }
            "picker" => Some(FormControl::Picker { value: picker_value(n) }),
            "checkbox" | "radio" => {
                if let Some(g) = group.as_deref_mut() {
                    g.push((id.clone(), Self::input_value_attr(n), n.style.custom_data > 0.5));
                }
                None
            }
            _ => None,
        };
        if let Some(control) = control {
            out.push(FormField { name: name.clone(), control });
            // 叶子控件（picker 除外：它包着的是展示用的子节点，不是表单控件）
            if n.tag != "picker" {
                return;
            }
        }
        if n.tag == "checkbox-group" || n.tag == "radio-group" {
            let mut items = Vec::new();
            for c in &n.children {
                self.collect_form_fields(c, taffy, child_origin, (fixed, child_shadow), interaction, Some(&mut items), out);
            }
            out.push(FormField { name, control: FormControl::Group { multi: n.tag == "checkbox-group", items } });
            return;
        }
        for c in &n.children {
            self.collect_form_fields(c, taffy, child_origin, (fixed, child_shadow), interaction, group.as_deref_mut(), out);
        }
    }
}

/// picker 的属性值，类型与 `bindchange` 的 `detail.value` 一致：
/// selector 是数字下标、multiSelector / region 是数组、time / date 是字符串
fn picker_value(n: &RenderNode) -> serde_json::Value {
    use serde_json::{json, Value};
    let raw = n.attrs.get("value").map(|s| s.trim()).unwrap_or("");
    match n.attrs.get("mode").map(|s| s.as_str()).unwrap_or("selector") {
        "multiSelector" | "region" => match crate::renderer::components::parse_attr_json(raw) {
            v @ Value::Array(_) => v,
            _ => json!([]),
        },
        "time" | "date" => json!(raw),
        _ => json!(raw.parse::<i64>().unwrap_or(0)),
    }
}
