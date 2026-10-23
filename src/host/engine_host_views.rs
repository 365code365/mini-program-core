//! [`MiniEngine`] 与宿主之间的**通用**同层组件（创建选项 `sameLayer` 里除 `video` 以外的标签，如 `<map>`）。
//!
//! 画面怎么进来、怎么画见 `components::host_view`。这里管与宿主的三条约定：
//! - **`sameLayerViews` 事件**：页面上有哪些同层组件（`key` / `tag` / `attrs` / 逻辑宽高 / 设备像素 `box`）。
//!   只在布局树整棵重建过、或换了渲染器之后重新数一遍，变了才发 —— 宿主据此建 / 改 / 放自己的离屏控件，
//!   按 `box` 画好交帧。滚动不发（滚动时画面在引擎画布里跟着走，宿主什么都不用做）。
//! - **`nativeViewTap` 事件**：点落在同层组件的框里（视图本地逻辑坐标 `x` / `y`），宿主自己判点到了什么
//!   （地图的标记 → 回传 `markertap`）。拖动照样滚页面。
//! - **回传事件**：宿主的 `mr_app_native_view_event(key, name, detail)` 派发给节点自己的 `bind<name>`
//!   （`dispatch_native_view_event` 在盖上去的控件里没找到这个键时转到这里）。
use super::engine::MiniEngine;
use crate::renderer::components::host_view;
use crate::renderer::{QueryHit, WxmlRenderer};
use crate::ui::interaction::InteractionManager;
use serde_json::{json, Value};
use std::cell::RefCell;

thread_local! {
    /// 上次数的时候布局树的代数、上次报的列表（变了才发）。按线程存：一个线程一个引擎
    static SYNC: RefCell<(Option<u64>, Option<String>)> = const { RefCell::new((None, None)) };
}

/// 页面上所有的同层组件（文档顺序；键重复的只留第一个）
fn views(r: &WxmlRenderer, interaction: &InteractionManager) -> Vec<(String, String, QueryHit)> {
    let mut out: Vec<(String, String, QueryHit)> = Vec::new();
    for tag in host_view::tags() {
        for v in r.query_selector(&tag, true, interaction) {
            let key = host_view::key(&tag, &v.attrs);
            if !out.iter().any(|(k, _, _)| *k == key) {
                out.push((key, tag.clone(), v));
            }
        }
    }
    out
}

/// 点在某个同层组件的框里：攒一条 `nativeViewTap`（下一帧发）。返回是否点中（`fixed` 与坐标系一致）。
/// 不吃掉这一下：节点与祖先上的 bindtap 照常派发（`host/tap.rs` 先派发再调这里）
pub fn on_tap(renderer: &WxmlRenderer, interaction: &InteractionManager, x: f32, y: f32, fixed: bool) -> bool {
    if host_view::tags().is_empty() {
        return false;
    }
    let p = crate::Point::new(x, y);
    let hit = views(renderer, interaction).into_iter().rev().find(|(_, _, v)| v.is_fixed == fixed && v.rect.contains(&p));
    let Some((key, tag, v)) = hit else { return false };
    host_view::queue_tap(&key, &tag, x - v.rect.x, y - v.rect.y);
    true
}

impl MiniEngine {
    /// 每次 pump 末尾（`sync_host_events`）
    pub(super) fn sync_host_views(&mut self) {
        if host_view::tags().is_empty() {
            return;
        }
        for ev in host_view::take_taps() {
            self.events.push(ev);
        }
        let Some(r) = self.renderer.as_ref() else { return };
        let generation = r.layout_generation();
        let stale = host_view::take_stale();
        if !stale && SYNC.with(|s| s.borrow().0) == Some(generation) {
            return;
        }
        SYNC.with(|s| s.borrow_mut().0 = Some(generation));
        let dpr = self.dpr;
        let found = views(r, &self.interaction);
        let keys: Vec<String> = found.iter().map(|(k, _, _)| k.clone()).collect();
        let list: Vec<Value> = found
            .into_iter()
            .map(|(key, tag, v)| {
                json!({
                    "key": key, "tag": tag, "attrs": v.attrs,
                    "width": v.rect.width, "height": v.rect.height,
                    // 宿主按这个尺寸画（一个宿主像素对一个屏幕像素），引擎只剩整行拷贝
                    "box": [(v.rect.width * dpr).round(), (v.rect.height * dpr).round()],
                    "fixed": v.is_fixed,
                })
            })
            .collect();
        let sig = serde_json::to_string(&list).unwrap_or_default();
        if SYNC.with(|s| s.borrow().1.as_deref() == Some(sig.as_str())) {
            return;
        }
        SYNC.with(|s| s.borrow_mut().1 = Some(sig));
        // 页面上已经没有的：帧（几 MB 一张）一起丢掉
        host_view::retain(&keys);
        self.events.push(json!({ "type": "sameLayerViews", "views": list }));
    }

    /// 宿主回传的组件事件（`markertap` …）。返回是否派发给了节点上的绑定
    pub(super) fn host_view_event(&mut self, key: &str, name: &str, detail: &Value) -> bool {
        if host_view::tags().is_empty() {
            return false;
        }
        let Some(r) = self.renderer.take() else { return false };
        let hit = views(&r, &self.interaction).into_iter().find(|(k, _, _)| k == key);
        let mut found = false;
        if let Some((_, _, v)) = hit {
            let t = self.mono_ms();
            found = super::own_events::dispatch_own(&mut self.app, &r, &v.component_id, name, detail.clone(), t);
            self.app.pump_jobs();
        }
        self.renderer = Some(r);
        // 只标数据脏：重画多大由失效分析定
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
        found
    }
}
