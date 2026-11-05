//! 输入框聚焦的参数与「按属性聚焦」（`focus` / `auto-focus`），SDK 与桌面共用。
//!
//! - [`fill_from_attrs`]：刚聚焦的输入框按渲染器登记的属性补齐编辑参数（maxlength / type / 多行 /
//!   confirm-type / confirm-hold / hold-keyboard）。点击聚焦（tap.rs `prepare_focused_input`）与按属性聚焦共用。
//! - [`take_attr_focus`]：取走这一帧 `focus` 属性变化排的请求（ui/interaction/focus_attr.rs），
//!   变成与点击同样的 `Focus` / `InputBlur` 结果交给宿主的统一派发（bindfocus / bindblur、弹收键盘）。
//!   按属性聚焦时光标放在 `cursor`（官方：「指定 focus 时的光标位置」），`selection-start/end`
//!   （「自动聚集时有效，需与 selection-end 搭配使用」）两个都 ≥0 时选中那一段；都没写光标在末尾。
use crate::renderer::WxmlRenderer;
use crate::ui::interaction::{FocusRequest, FocusedInput, InteractionManager, InteractionResult};

/// 按属性补齐编辑参数。`attr` 取渲染器登记的属性（`InputRegion.attrs`），`tag` 是 input / textarea
pub(crate) fn fill_from_attrs(input: &mut FocusedInput, tag: Option<&str>, attr: impl Fn(&str) -> Option<String>) {
    let truthy = |k: &str| attr(k).map(|v| crate::renderer::components::attr_is_true(&v)).unwrap_or(false);
    if let Some(t) = tag {
        input.multiline = t == "textarea";
    }
    if let Some(ml) = attr("maxlength").and_then(|s| s.trim().parse::<i32>().ok()) {
        input.maxlength = ml;
    }
    input.is_password = crate::renderer::components::is_password_input(attr("password").as_deref(), attr("type").as_deref());
    if let Some(t) = attr("type") {
        input.input_type = t;
    }
    // textarea 的右下角默认是「换行」（`return`），input 默认是「完成」
    input.confirm_type = attr("confirm-type")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| if input.multiline { "return" } else { "done" }.to_string());
    input.confirm_hold = truthy("confirm-hold");
    input.hold_keyboard = truthy("hold-keyboard");
}

/// 这一帧 `focus` 属性排的聚焦 / 失焦，按顺序交给宿主派发（先失焦旧的，再聚焦新的）
pub fn take_attr_focus(interaction: &mut InteractionManager, renderer: Option<&WxmlRenderer>) -> Vec<InteractionResult> {
    let Some(req) = interaction.take_focus_request() else { return Vec::new() };
    let mut out = Vec::new();
    match req {
        FocusRequest::Blur(id) => {
            if interaction.focused_input.as_ref().is_some_and(|f| f.id == id) {
                out.extend(interaction.blur_input());
            }
        }
        FocusRequest::Focus(id) => {
            if interaction.focused_input.as_ref().is_some_and(|f| f.id != id) {
                out.extend(interaction.blur_input());
            }
            let Some(r) = interaction.focus_by_id(&id) else { return out };
            let region = renderer.and_then(|r| r.input_region(&id));
            let attr = |k: &str| region.and_then(|r| r.attrs.get(k)).cloned();
            if let Some(f) = interaction.focused_input.as_mut() {
                fill_from_attrs(f, region.map(|r| r.tag.as_str()), attr);
                place_caret(f, &attr);
            }
            out.push(r);
        }
    }
    out
}

/// `cursor` / `selection-start` / `selection-end`（字符下标，-1 = 不指定）
fn place_caret(f: &mut FocusedInput, attr: &impl Fn(&str) -> Option<String>) {
    let num = |k: &str| attr(k).and_then(|s| s.trim().parse::<i64>().ok()).filter(|n| *n >= 0).map(|n| n as usize);
    let len = f.char_len();
    if let (Some(s), Some(e)) = (num("selection-start"), num("selection-end")) {
        let (s, e) = (s.min(len), e.min(len));
        if s != e {
            f.selection_start = Some(s.min(e));
            f.selection_end = Some(s.max(e));
        }
        f.cursor_pos = e;
        return;
    }
    if let Some(c) = num("cursor") {
        f.cursor_pos = c.min(len);
    }
}

/// 从一个输入框点到另一个：旧的先失焦（bindchange / bindblur），新的 bindfocus 随后由宿主派。
/// 从前旧的那个一声不吭就丢了焦点，页面在 bindblur 里做的校验 / 保存不执行（tap.rs 的各条点击出口调）
pub(crate) fn blur_switched_input(app: &mut crate::runtime::MiniApp, renderer: Option<&WxmlRenderer>) {
    if let Some((id, value)) = crate::ui::interaction::edit_note::take_switch() {
        super::tap::dispatch_input_event(app, renderer, &id, "blur", &value);
    }
}
