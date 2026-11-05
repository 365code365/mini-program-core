//! [`MiniEngine`] 的小窗（`picture-in-picture-mode`，同层渲染的 `<live-pusher>`；画法与命中在 components/pip.rs）。
//!
//! - **进**：离开页面之前（`push_page` 盖住它：push；`back_with` 关掉它：pop），页面上第一个属性里写了这个方向的
//!   live-pusher 进小窗，先给它派 `enterpictureinpicture`（原页还在眼前，绑定还在）。一次只有一个小窗；
//! - **宿主**：`sameLayerViews` 里一直报着它（`pip: true`，`box` 是小窗的设备像素），宿主的控件不放、推流不断，
//!   按小窗的尺寸交帧；引擎合成时画在右下角（每来一帧重新合成一次，页面不重画）；
//! - **出**：push 进的回到原页那一层（点小窗 = `wx.navigateBack` 回去，或者自己返回）→ 收起，组件照常画在页面里，
//!   画出来的那一帧之后派 `leavepictureinpicture`；点 × → 关掉（不在页面上了，宿主放掉控件，推流停；
//!   原页压在栈里看不见，这时不派 leave，官方没写这种情形，待验）。原页被 redirect / reLaunch 掉了也关。
use super::engine::MiniEngine;
use crate::renderer::components::host_view;
use crate::renderer::components::pip::{self, Pip, PipDir, PipHit};
use serde_json::{json, Value};
use std::cell::RefCell;

thread_local! {
    /// 刚收起、还没派 `leavepictureinpicture` 的组件 id 与等了几帧（回到原页那一帧页面还没画，绑定表是空的）
    static LEAVE: RefCell<Option<(String, u8)>> = const { RefCell::new(None) };
    /// 上次告诉逻辑层的「小窗开着没有」
    static ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `picture-in-picture-mode` → (push, pop)。字符串 `push` / `pop`，或数组（`["push", "pop"]`；
/// 模板求值出来是 JSON 文本，写死在属性里的可能是逗号串），空串 / `[]` 是不开
pub(super) fn pip_modes(v: &str) -> (bool, bool) {
    let v = v.trim();
    let list: Vec<String> = serde_json::from_str::<Vec<String>>(v).unwrap_or_else(|_| {
        v.split(',').map(|s| s.trim().trim_matches(|c| matches!(c, '[' | ']' | '"' | '\'' | ' ')).to_string()).collect()
    });
    (list.iter().any(|s| s == "push"), list.iter().any(|s| s == "pop"))
}

/// 小窗那一条（`sameLayerViews` 用）：有小窗、且它不在当前页的列表里时
pub(super) fn pip_entry(dpr: f32, listed: &[String]) -> Option<Value> {
    let p = pip::current().filter(|p| !listed.contains(&p.key))?;
    Some(json!({
        "key": p.key, "tag": p.tag, "attrs": p.attrs,
        "width": p.rect.width, "height": p.rect.height,
        "box": [(p.rect.width * dpr).round(), (p.rect.height * dpr).round()],
        "fixed": true, "pip": true,
    }))
}

impl MiniEngine {
    /// 小窗在窗口里的位置：贴右下角，让开底部安全区与 tabBar
    fn pip_rect(&self, aspect: f32) -> crate::Rect {
        let tab = if self.is_tabbar_page(&self.current_route()) { self.tabbar_h() } else { 0.0 };
        let bottom = self.opts.safe_area.map(|s| s.bottom.max(0.0)).unwrap_or(0.0) + tab;
        pip::place(self.width as f32, self.height as f32, bottom, aspect)
    }

    /// 离开当前页之前调（见模块注释）
    pub(super) fn pip_on_leave(&mut self, dir: PipDir) {
        if pip::visible() || !host_view::is_same_layer("live-pusher") {
            return;
        }
        let t = self.mono_ms();
        let Some(r) = self.renderer.as_ref() else { return };
        let want = |m: &String| {
            let (push, pop) = pip_modes(m);
            if dir == PipDir::Push { push } else { pop }
        };
        let found = super::engine_host_views::views(r, &self.interaction)
            .into_iter()
            .find(|(_, tag, v)| tag == "live-pusher" && v.attrs.get("picture-in-picture-mode").is_some_and(want));
        let Some((key, tag, v)) = found else { return };
        super::own_events::dispatch_own(&mut self.app, r, &v.component_id, "enterpictureinpicture", json!({}), t);
        let aspect = if v.rect.width > 0.0 { v.rect.height / v.rect.width } else { 16.0 / 9.0 };
        let rect = self.pip_rect(aspect);
        pip::enter(Pip {
            key,
            tag,
            attrs: v.attrs.clone(),
            component_id: v.component_id.clone(),
            dir,
            origin_depth: self.page_stack.len(),
            origin_route: self.current_route(),
            aspect,
            rect,
            drawn_seq: None,
        });
        self.dmg.compose_dirty = true;
    }

    /// 每帧（`tick_scroll_views`）：回到原页收起、原页没了关掉、摆位置、宿主交了新帧就重新合成、补发 leave
    pub(super) fn tick_pip(&mut self) {
        self.flush_pip_leave();
        self.sync_pip_active();
        let Some((dir, depth, route_ok, aspect, rect, drawn)) = pip::with_mut(|p| (p.dir, p.origin_depth, p.origin_route.clone(), p.aspect, p.rect, p.drawn_seq))
        else {
            return;
        };
        // LivePusherContext.exitPictureInPicture（host-objects.js 记下的请求；只在有小窗时问逻辑层）
        let asked = self.app.eval("(function(){var r=globalThis.__pipExitRequest;globalThis.__pipExitRequest=false;return r?1:0})()");
        if asked.is_ok_and(|s| s.trim() == "1") {
            pip::exit();
            host_view::mark_stale();
            self.dmg.compose_dirty = true;
            return;
        }
        let n = self.page_stack.len();
        if dir == PipDir::Push && n <= depth {
            let back_home = n == depth && self.current_route() == route_ok;
            if let Some(p) = pip::exit() {
                if back_home {
                    LEAVE.with(|l| *l.borrow_mut() = Some((p.component_id, 0)));
                }
            }
            host_view::mark_stale();
            self.dmg.compose_dirty = true;
            return;
        }
        let r = self.pip_rect(aspect);
        let key = pip::with_mut(|p| p.key.clone()).unwrap_or_default();
        if r != rect || host_view::seq_of(&key) != drawn {
            pip::with_mut(|p| p.rect = r);
            self.dmg.compose_dirty = true;
        }
    }

    /// `wx.checkIsPictureInPictureActive()`（同步接口，base-ext.js 读 `__pipActive`）：小窗开 / 关之后的下一帧告诉逻辑层，变了才 eval
    fn sync_pip_active(&mut self) {
        let on = pip::visible();
        if ACTIVE.with(|a| a.replace(on)) != on {
            self.app.eval(&format!("globalThis.__pipActive = {on}")).ok();
        }
    }

    /// 收起之后的 leave：页面画出来（绑定表有了）才派，最多等 10 帧
    fn flush_pip_leave(&mut self) {
        let Some((id, waited)) = LEAVE.with(|l| l.borrow_mut().take()) else { return };
        let Some(r) = self.renderer.as_ref() else { return };
        if r.get_event_bindings().is_empty() && waited < 10 {
            LEAVE.with(|l| *l.borrow_mut() = Some((id, waited + 1)));
            return;
        }
        let t = self.mono_ms();
        super::own_events::dispatch_own(&mut self.app, r, &id, "leavepictureinpicture", json!({}), t);
    }

    /// 指针按下 / 移动 / 抬起：按在小窗上的这串触摸归它（engine_overlay_input.rs 最先问）
    pub(super) fn pip_pointer(&mut self, phase: u8, x: f32, y: f32) -> bool {
        match phase {
            0 => pip::press(x, y),
            1 => pip::pressing(),
            _ => {
                let Some(hit) = pip::release(x, y) else { return false };
                match hit {
                    Some(PipHit::Close) => {
                        pip::exit();
                        host_view::mark_stale();
                        self.dmg.compose_dirty = true;
                    }
                    Some(PipHit::Body) => {
                        let back = pip::current().filter(|p| p.dir == PipDir::Push).map(|p| self.page_stack.len().saturating_sub(p.origin_depth));
                        if let Some(d) = back.filter(|d| *d > 0) {
                            self.app.eval(&format!("wx.navigateBack({{ delta: {d} }})")).ok();
                        }
                    }
                    None => {}
                }
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::pip_modes;

    #[test]
    fn 小窗模式的几种写法() {
        assert_eq!(pip_modes("push"), (true, false));
        assert_eq!(pip_modes(" pop "), (false, true));
        assert_eq!(pip_modes(r#"["push","pop"]"#), (true, true));
        assert_eq!(pip_modes("push,pop"), (true, true));
        assert_eq!(pip_modes("[]"), (false, false));
        assert_eq!(pip_modes(""), (false, false));
    }
}
