//! list-builder / grid-builder 的 `bind:itembuild` / `bind:itemdispose`（官方：detail `{index}`，index 是项序号 number）。
//!
//! 每帧问渲染器各构造器的可见窗（`WxmlRenderer::builder_windows`，沿路径走 + 二分，不遍历列表），
//! 与上一帧的窗比：离开的项先发 itemdispose、新进来的项再发 itembuild，各自按序号升序。
//! 第一次看到一个构造器时窗里的每一项都发一次 itembuild（「首屏按需渲染」的那批）。
//! 记录按「页面路由 + 构造器的键」放在线程局部（一个引擎实例一条线程，同 observers）；页面上不再有的构造器直接丢掉记录
//! （微信那边页面销毁时会不会给剩下的项发 itemdispose 没写 —— 待验，这里不发）。
//!
//! SDK 与桌面窗体共用：挂在 `observers::tick` 里（两边每帧都调它）。
use crate::renderer::{BuilderWindow, WxmlRenderer};
use crate::runtime::MiniApp;
use crate::ui::interaction::InteractionManager;
use serde_json::json;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static BUILT: RefCell<HashMap<String, (usize, usize)>> = RefCell::new(HashMap::new());
}

/// 引擎销毁 / 重新启动小程序时清空
pub fn clear() {
    BUILT.with(|b| b.borrow_mut().clear());
}

/// 一帧。返回是否派发过（回调里可能 setData）
pub fn tick(app: &mut MiniApp, r: &WxmlRenderer, interaction: &InteractionManager, route: &str, page_scroll: f32, view_h: f32, t_ms: u64) -> bool {
    let wins = r.builder_windows(interaction, page_scroll, view_h);
    let route = route.trim_start_matches('/');
    if wins.is_empty() {
        BUILT.with(|b| b.borrow_mut().retain(|k, _| !k.starts_with(route)));
        return false;
    }
    let mut fires: Vec<(&BuilderWindow, &'static str, usize)> = Vec::new();
    let mut seen: Vec<String> = Vec::with_capacity(wins.len());
    BUILT.with(|b| {
        let mut b = b.borrow_mut();
        for w in &wins {
            let key = format!("{route}|{}", w.key);
            let prev = b.get(&key).copied().unwrap_or((0, 0));
            let now = w.range.unwrap_or((0, 0));
            if prev != now && !w.binds.is_empty() {
                let inside = |r: (usize, usize), i: usize| i >= r.0 && i < r.1;
                for i in prev.0..prev.1 {
                    if !inside(now, i) {
                        fires.push((w, "itemdispose", i));
                    }
                }
                for i in now.0..now.1 {
                    if !inside(prev, i) {
                        fires.push((w, "itembuild", i));
                    }
                }
            }
            b.insert(key.clone(), now);
            seen.push(key);
        }
        b.retain(|k, _| !k.starts_with(route) || seen.contains(k));
    });
    // 一帧里进出窗口的项可能有十几个（快速甩动）：拼成一段脚本只 eval 一次。从前一项一次 eval，
    // 每次都要 QuickJS 重新解析一段脚本，外加一趟 JSON 解析 / 再序列化。各项照旧按序派发、各自 try（__dispatchEvent 里）
    let mut script = String::new();
    for (w, name, index) in &fires {
        push_dispatch(&mut script, w, name, *index, t_ms);
    }
    if !script.is_empty() {
        app.eval(&script).ok();
    }
    !fires.is_empty()
}

/// 组件事件：只触发构造器自己身上的绑定（不冒泡），target / currentTarget 都是构造器。
/// 没有触点：`touches` / `changedTouches` 是空数组（在 JS 里换掉，省一趟 JSON 解析）
fn push_dispatch(script: &mut String, w: &BuilderWindow, name: &str, index: usize, t_ms: u64) {
    use std::fmt::Write;
    for b in w.binds.iter().filter(|b| b.event_type == name) {
        let node = super::touch::NodeInfo { id: &w.id, dataset: &b.data, offset_left: 0.0, offset_top: 0.0 };
        let ev = super::touch::event_json(name, &node, &node, (0.0, 0.0), 0, t_ms, json!({ "index": index }));
        let h = serde_json::to_string(&b.handler).unwrap_or_else(|_| "''".into());
        let o = serde_json::to_string(&b.owner).unwrap_or_else(|_| "''".into());
        if crate::renderer::draw_profile::scroll_log() {
            eprintln!("📜 {} {name} -> {} index={index}", w.key, b.handler);
        }
        let _ = writeln!(script, "__dispatchEvent({h}, Object.assign({ev}, {{ touches: [], changedTouches: [] }}), {o});");
    }
}
