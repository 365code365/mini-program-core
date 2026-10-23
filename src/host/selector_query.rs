//! `wx.createSelectorQuery()` 与 scroll-view 的 `ScrollViewContext`：宿主侧（SDK 与桌面共用）。
//!
//! 依据官方 api/wxml/SelectorQuery.html、NodesRef.html、ScrollViewContext.html。
//! 逻辑层 `exec()` 把一串查询交过来（`__native_host_request('selectorQuery')`），这里按布局树
//! 算好、调 `__resolveSelectorQuery(id, results)` 回给逻辑层 —— 与微信一样是**异步**的。
//! 还没布局（页面刚建、onLoad 里就查）时留到下一帧再答，免得回一串 null。
//!
//! 坐标：`boundingClientRect` 是**视口**坐标（正常流的节点减去页面滚动；fixed 节点本来就是视口坐标），
//! scroll-view 里的节点按它当前的滚动偏移算（`query_selector` 已处理）。
//! 取舍：`computedStyle` 只认节点 `style` 属性里写了的项与 width / height；`in(component)` 不收窄范围
//! （本引擎 id / class 是页面级的）；`mark` 给空对象。

use crate::renderer::{QueryHit, WxmlRenderer};
use crate::runtime::MiniApp;
use crate::ui::interaction::InteractionManager;
use serde_json::{json, Map, Value};

/// 页面视口：滚动位置、可视区尺寸、页面内容高度（逻辑像素）
#[derive(Clone, Copy, Debug)]
pub struct PageView {
    pub scroll_top: f32,
    pub width: f32,
    pub height: f32,
    pub content_height: f32,
}

/// 每帧一次：答 SelectorQuery、执行 ScrollViewContext。返回是否要重画（scroll-view 被程序滚动了）
pub fn tick(app: &mut MiniApp, renderer: Option<&WxmlRenderer>, interaction: &mut InteractionManager, view: PageView) -> bool {
    let queries = app.take_host_requests("selectorQuery");
    let sv_ops = app.take_host_requests("scrollView");
    let Some(r) = renderer.filter(|r| r.has_layout()) else {
        // 还没布局：原样留到下一帧
        for q in queries {
            app.push_host_request("selectorQuery", q);
        }
        for q in sv_ops {
            app.push_host_request("scrollView", q);
        }
        return false;
    };
    for req in queries {
        let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
        let list = req.get("queries").and_then(Value::as_array).cloned().unwrap_or_default();
        let results: Vec<Value> = list.iter().map(|q| answer(r, interaction, view, q)).collect();
        let js = format!("__resolveSelectorQuery({id}, {})", Value::Array(results));
        if let Err(e) = app.eval(&js) {
            eprintln!("⚠️ SelectorQuery 回调失败：{e}");
        }
    }
    let mut moved = false;
    for op in sv_ops {
        moved |= scroll_view_op(r, interaction, &op);
    }
    moved
}

/// 一条查询：`select` 给对象或 null，`selectAll` 给数组
fn answer(r: &WxmlRenderer, im: &InteractionManager, view: PageView, q: &Value) -> Value {
    let fields = q.get("fields").cloned().unwrap_or(Value::Null);
    if q.get("viewport").and_then(Value::as_bool).unwrap_or(false) {
        return viewport_fields(&fields, view);
    }
    let sel = q.get("selector").and_then(Value::as_str).unwrap_or("");
    let all = q.get("all").and_then(Value::as_bool).unwrap_or(false);
    let hits = r.query_selector(sel, all, im);
    if all {
        Value::Array(hits.iter().map(|h| node_fields(r, im, view, h, &fields)).collect())
    } else {
        hits.first().map(|h| node_fields(r, im, view, h, &fields)).unwrap_or(Value::Null)
    }
}

fn want(f: &Value, k: &str) -> bool {
    f.get(k).map(|v| v.as_bool().unwrap_or(!v.is_null())).unwrap_or(false)
}

fn viewport_fields(f: &Value, v: PageView) -> Value {
    let mut o = Map::new();
    if want(f, "id") {
        o.insert("id".into(), json!(""));
    }
    if want(f, "dataset") {
        o.insert("dataset".into(), json!({}));
    }
    if want(f, "rect") {
        for (k, x) in [("left", 0.0), ("top", 0.0), ("right", v.width), ("bottom", v.height)] {
            o.insert(k.into(), json!(x));
        }
    }
    if want(f, "size") {
        o.insert("width".into(), json!(v.width));
        o.insert("height".into(), json!(v.height));
    }
    if want(f, "scrollOffset") {
        o.insert("scrollLeft".into(), json!(0.0));
        o.insert("scrollTop".into(), json!(v.scroll_top));
        o.insert("scrollWidth".into(), json!(v.width));
        o.insert("scrollHeight".into(), json!(v.content_height.max(v.height)));
    }
    Value::Object(o)
}

/// 节点在视口里的矩形 (left, top, right, bottom)
pub fn client_rect(h: &QueryHit, scroll_top: f32) -> (f32, f32, f32, f32) {
    let top = if h.is_fixed { h.rect.y } else { h.rect.y - scroll_top };
    (h.rect.x, top, h.rect.x + h.rect.width, top + h.rect.height)
}

fn node_fields(r: &WxmlRenderer, im: &InteractionManager, view: PageView, h: &QueryHit, f: &Value) -> Value {
    let mut o = Map::new();
    if want(f, "id") {
        o.insert("id".into(), json!(h.id));
    }
    if want(f, "dataset") {
        o.insert("dataset".into(), json!(h.dataset));
    }
    if want(f, "mark") {
        o.insert("mark".into(), json!({}));
    }
    if want(f, "rect") {
        let (l, t, rt, b) = client_rect(h, view.scroll_top);
        for (k, x) in [("left", l), ("top", t), ("right", rt), ("bottom", b)] {
            o.insert(k.into(), json!(x));
        }
    }
    if want(f, "size") {
        o.insert("width".into(), json!(h.rect.width));
        o.insert("height".into(), json!(h.rect.height));
    }
    if want(f, "scrollOffset") {
        let pos = im.get_scroll_controller(&h.component_id).map(|c| c.get_position()).unwrap_or(0.0);
        let info = r.scroll_views().iter().find(|s| s.component_id == h.component_id);
        let horizontal = info.map(|s| s.horizontal).unwrap_or(false);
        let (cw, ch) = info.map(|s| (s.content_width, s.content_height)).unwrap_or((h.rect.width, h.rect.height));
        o.insert("scrollLeft".into(), json!(if horizontal { pos } else { 0.0 }));
        o.insert("scrollTop".into(), json!(if horizontal { 0.0 } else { pos }));
        o.insert("scrollWidth".into(), json!(cw.max(h.rect.width)));
        o.insert("scrollHeight".into(), json!(ch.max(h.rect.height)));
    }
    if let Some(props) = f.get("properties").and_then(Value::as_array) {
        for p in props.iter().filter_map(Value::as_str) {
            let v = h.attrs.get(p).or_else(|| h.attrs.get(&kebab(p))).map(|s| typed(s)).unwrap_or(Value::Null);
            o.insert(p.to_string(), v);
        }
    }
    if let Some(styles) = f.get("computedStyle").and_then(Value::as_array) {
        let inline = inline_style(h.attrs.get("style").map(|s| s.as_str()).unwrap_or(""));
        for p in styles.iter().filter_map(Value::as_str) {
            let k = kebab(p);
            let v = match k.as_str() {
                "width" => format!("{}px", h.rect.width),
                "height" => format!("{}px", h.rect.height),
                _ => inline.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone()).unwrap_or_default(),
            };
            o.insert(p.to_string(), json!(v));
        }
    }
    if want(f, "node") {
        let node = match h.tag.as_str() {
            "canvas" if h.attrs.get("type").map(|s| s.trim()) == Some("2d") => {
                json!({ "kind": "canvas", "id": h.id, "width": h.rect.width, "height": h.rect.height })
            }
            "scroll-view" => json!({ "kind": "scroll-view", "id": h.component_id }),
            // 引擎内置组件（builtin_components）的实例：DraggableSheetContext 等由组件实例自己提供
            "draggable-sheet" | "snapshot" | "open-container" => json!({ "kind": h.tag, "id": h.id }),
            _ => Value::Null,
        };
        if node.is_null() {
            o.insert("node".into(), Value::Null);
        } else {
            o.insert("__node".into(), node);
        }
    }
    if want(f, "context") {
        let ctx = match h.tag.as_str() {
            "video" => json!({ "kind": "video", "id": h.id }),
            "canvas" => json!({ "kind": "canvas", "id": h.attrs.get("canvas-id").cloned().unwrap_or_default() }),
            // 官方 NodesRef.context 还支持这几个：EditorContext（内置组件）、MapContext / LivePlayerContext（宿主）
            "editor" | "map" | "live-player" => json!({ "kind": h.tag, "id": h.id }),
            _ => Value::Null,
        };
        if ctx.is_null() {
            o.insert("context".into(), Value::Null);
        } else {
            o.insert("__ctx".into(), ctx);
        }
    }
    Value::Object(o)
}

/// `scrollX` → `scroll-x`
fn kebab(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// 属性值按 JSON 认得出的类型给（数字、布尔、数组…），认不出就是字符串
fn typed(s: &str) -> Value {
    serde_json::from_str(s.trim()).unwrap_or_else(|_| json!(s))
}

fn inline_style(s: &str) -> Vec<(String, String)> {
    s.split(';')
        .filter_map(|d| {
            let (k, v) = d.split_once(':')?;
            Some((k.trim().to_ascii_lowercase(), v.trim().to_string()))
        })
        .collect()
}

/// ScrollViewContext：`{id(组件 id), op, …}`。返回是否动了滚动位置
fn scroll_view_op(r: &WxmlRenderer, im: &mut InteractionManager, op: &Value) -> bool {
    let id = op.get("id").and_then(Value::as_str).unwrap_or("");
    let Some(info) = r.scroll_views().iter().find(|s| s.component_id == id).cloned() else {
        eprintln!("⚠️ ScrollViewContext：scroll-view {id:?} 不在当前画面里");
        return false;
    };
    let animated = op.get("animated").and_then(Value::as_bool).unwrap_or(true);
    let dur = if animated {
        op.get("duration").and_then(Value::as_f64).unwrap_or(300.0) as f32 / 1000.0
    } else {
        0.0
    };
    let target = match op.get("op").and_then(Value::as_str).unwrap_or("") {
        "scrollTo" => {
            let k = if info.horizontal { "left" } else { "top" };
            match op.get(k).and_then(Value::as_f64) {
                Some(v) => v as f32,
                None => return false,
            }
        }
        "scrollIntoView" => {
            let sel = op.get("selector").and_then(Value::as_str).unwrap_or("");
            let Some(hit) = r.query_selector(sel, false, im).into_iter().next() else {
                eprintln!("⚠️ ScrollViewContext.scrollIntoView：{sel:?} 没找到");
                return false;
            };
            let pos = im.get_scroll_controller(id).map(|c| c.get_position()).unwrap_or(0.0);
            let offset = op.get("offset").and_then(Value::as_f64).unwrap_or(0.0) as f32;
            // query 给的子节点位置已经减过当前滚动量：加回来就是它在内容里的偏移
            let rel = if info.horizontal { hit.rect.x - info.bounds.x } else { hit.rect.y - info.bounds.y };
            rel + pos + offset
        }
        // triggerRefresh / closeRefresh：页面改 `refresher-triggered` 能做到同样的事（T14-8），
        // 程序入口要接进 host::scroll_view 的刷新状态机，暂未做（G-4）
        other => {
            eprintln!("⚠️ ScrollViewContext.{other} 未实现（请改用 refresher-triggered 属性）");
            return false;
        }
    };
    let Some(c) = im.get_scroll_controller_mut(id) else { return false };
    c.animate_to(target, dur);
    true
}
