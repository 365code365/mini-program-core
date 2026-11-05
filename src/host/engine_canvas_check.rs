//! `<canvas binderror>`（官方 component/canvas：「当发生错误时触发 error 事件，detail = {errMsg}」）：
//! 同一页面里 `canvas-id` 重复（旧接口 `wx.createCanvasContext(id)` 按它找画布，第二个永远画不上）。
//!
//! 按布局代数查一次（换了布局才重查），重复的第二个起各派一次 `error`；已经报过的同一个节点不再报。
//! 不在建树时判：增量布局（layout_incremental.rs）不会重跑没变的节点，那里看不全一页的 canvas。
//! errMsg 的原文官方没写（待验）。
use super::engine::MiniEngine;
use serde_json::json;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct State {
    generation: Option<u64>,
    reported: HashSet<String>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

impl MiniEngine {
    /// 每帧一次（`tick_scroll_views`）；没换布局时只是一次比较
    pub(super) fn tick_canvas_check(&mut self) {
        let Some(r) = self.renderer.take() else { return };
        let generation = r.layout_generation();
        if STATE.with(|s| s.borrow().generation == Some(generation)) {
            self.renderer = Some(r);
            return;
        }
        let hits = r.query_selector("canvas", true, &self.interaction);
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut dups = Vec::new();
        for h in &hits {
            let Some(id) = h.attrs.get("canvas-id").map(|s| s.trim()).filter(|s| !s.is_empty()) else { continue };
            let n = seen.entry(id.to_string()).or_insert(0);
            *n += 1;
            if *n > 1 {
                dups.push((h.component_id.clone(), id.to_string()));
            }
        }
        let fresh: Vec<(String, String)> = STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.generation = Some(generation);
            let alive: HashSet<String> = dups.iter().map(|(c, _)| c.clone()).collect();
            s.reported.retain(|c| alive.contains(c));
            dups.into_iter().filter(|(c, _)| s.reported.insert(c.clone())).collect()
        });
        let now = self.mono_ms();
        let mut any = false;
        for (cid, id) in fresh {
            let detail = json!({ "errMsg": format!("canvas:fail canvas-id \"{id}\" 在同一页面里重复，旧接口只认第一个") });
            any |= super::own_events::dispatch_own(&mut self.app, &r, &cid, "error", detail, now);
        }
        self.renderer = Some(r);
        if any {
            self.app.pump_jobs();
            if self.app.take_data_dirty() {
                self.page_data_dirty = true;
            }
        }
    }
}

/// 换页：下一页重新查
pub(super) fn reset() {
    STATE.with(|s| *s.borrow_mut() = State::default());
}
