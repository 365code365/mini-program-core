//! 触摸序列的**目标锁定**：`touchmove` / `touchend` / `touchcancel` 派发给 `touchstart` 时摸到的那个节点
//! （和它那条捕获-冒泡链），与微信、DOM 的触摸事件一致 —— 手指移出了它、或者它在 touchmove 里被
//! setData 挪走了，这串触摸照样归它。
//!
//! 从前每个事件都按手指**当前**的位置重新命中：跟手拖动的元素（movable-view、自己写的拖拽排序、
//! 侧滑删除的行）一旦被夹在边界、手指滑出它的框，后面的 touchmove 与 touchend 就全丢了 ——
//! 松手回弹不会发生，下一次按下前它都以为手指还按着。
//!
//! 锁定存在渲染器里：换页时渲染器整个重建，锁定跟着丢掉，不会把上一页的处理函数派到新页上。
//! 派发时按 `component_id` 换成本帧的那条绑定（dataset、位置是最新的）；节点已经没了就用按下时的。
use super::*;

/// 被锁定的事件类型（`longpress` 不在内：它不动手指，按当前位置命中就是按下的那个）
const CAPTURED: [&str; 3] = ["touchmove", "touchend", "touchcancel"];

pub(crate) struct TouchCapture {
    identifier: u32,
    chains: [Vec<EventBinding>; 3],
    target: Option<EventBinding>,
}

impl WxmlRenderer {
    /// `touchstart` 时调：按按下点算好这串触摸后续三种事件的派发链与 target
    pub fn capture_touch(&self, identifier: u32, x: f32, y: f32, scope: Option<bool>) {
        let chains = CAPTURED.map(|t| self.dispatch_chain(x, y, t, scope));
        let target = self.innermost_binding_at(x, y, scope);
        if let Ok(mut g) = self.touch_capture.lock() {
            *g = Some(TouchCapture { identifier, chains, target });
        }
    }

    /// 这串触摸锁定的派发链与 target。不是锁定的事件类型、不是这串触摸、或者根本没锁过时给 None
    /// （调用方按当前位置命中）
    pub fn captured_touch(&self, identifier: u32, event_type: &str) -> Option<(Vec<EventBinding>, Option<EventBinding>)> {
        let i = CAPTURED.iter().position(|t| *t == event_type)?;
        let g = self.touch_capture.lock().ok()?;
        let c = g.as_ref().filter(|c| c.identifier == identifier)?;
        // 只认写了 `id` 的节点：没写的 component_id 是「标签_坐标」，节点挪了就对不上，
        // 而同一位置换上来的别的列表项会对上 —— 那会把事件派给它（拖拽排序时正好如此）
        let same = |n: &EventBinding, b: &EventBinding| !b.id.is_empty() && n.component_id == b.component_id;
        let fresh = |b: &EventBinding| {
            self.event_bindings
                .iter()
                .find(|n| same(n, b) && n.event_type == b.event_type && n.handler == b.handler && n.phase == b.phase)
                .cloned()
                .unwrap_or_else(|| b.clone())
        };
        let target = c.target.as_ref().map(|t| self.event_bindings.iter().find(|n| same(n, t)).cloned().unwrap_or_else(|| t.clone()));
        Some((c.chains[i].iter().map(fresh).collect(), target))
    }

    /// 这串触摸结束（touchend / touchcancel 派发完）
    pub fn release_touch(&self, identifier: u32) {
        if let Ok(mut g) = self.touch_capture.lock() {
            if g.as_ref().map(|c| c.identifier == identifier).unwrap_or(false) {
                *g = None;
            }
        }
    }
}
