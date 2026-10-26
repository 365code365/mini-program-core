//! 分栏的第二步：两栏都能点。点另一栏时，那一栏的页面变成**活页面**（[`MiniEngine::frameset_focus`]）。
//!
//! 引擎一次只有一个活页面（渲染、命中、增量重画、输入框都只认它，engine_frameset.rs 的文件头）。
//! 所以「点左栏」= 把栈顶两页对调：左边那页换上来当活页面，右边那页留一张图放进另一栏。对调只动引擎侧
//! （页面栈最后两项、留着的视图、侧滑图、导航栏改动那两格）与逻辑层的 `__currentPage`
//! （事件处理函数、`setData` 取数据、组件实例都跟着它走）；逻辑层的页面栈**不动**，
//! `getCurrentPages()` 仍是真实的打开顺序，不派 onShow / onHide（两页一直都在屏上）。
//!
//! 对调着（左栏是活页面、右栏还开着一页）时的路由，按「右栏是左栏打开的详情」来：
//! - 左栏 `navigateTo`：右栏那页关掉（onUnload），新页进右栏 —— 左栏点列表里的另一项，右栏换成它；
//! - 返回 / 其它路由：先对调回来，再照常走（返回关掉的是右栏那页）；
//! - 窗口窄回单栏：先对调回来（单栏显示栈顶那页）。
use super::engine::MiniEngine;
use super::page_keep::SavedView;

impl MiniEngine {
    /// 宿主：这次按下落在 `left` 那一栏。那一栏是另一栏（显示的是图）且有页面时把它换成活页面，返回 true ——
    /// 之后的指针坐标按这一栏算。已经是活页面那一栏、占位栏、没分栏时什么都不做，返回 false
    pub fn frameset_focus(&mut self, left: bool) -> bool {
        if !self.frameset.active || self.page_stack.len() < 2 || self.frameset_live_left() == left {
            return false;
        }
        self.frameset_swap();
        true
    }

    /// 对调着的话先对调回来（返回、switchTab、redirectTo、reLaunch、窄回单栏之前调）
    pub(super) fn frameset_unswap(&mut self) {
        if self.frameset.swapped {
            self.frameset_swap();
        }
    }

    /// 对调着时左栏 `navigateTo`：右栏那页（逻辑层的栈顶，引擎里的倒数第二项）关掉，新页再进右栏
    pub(super) fn frameset_drop_right(&mut self) {
        if !self.frameset.swapped {
            return;
        }
        self.frameset.swapped = false;
        let n = self.page_stack.len();
        if n < 2 {
            return;
        }
        self.page_stack.remove(n - 2);
        self.keep.saved.pop();
        self.back_shots.pop();
        if self.events.nav.len() >= n {
            self.events.nav.remove(n - 2);
        }
        self.unpack_last_shot();
        self.app.eval("__framesetDropTop()").ok();
        self.frameset.side = None;
    }

    /// 栈顶两页对调（见文件头）
    fn frameset_swap(&mut self) {
        let n = self.page_stack.len();
        if n < 2 || self.keep.saved.len() < n - 1 {
            return;
        }
        let t0 = std::time::Instant::now();
        self.pointer_cancel();
        crate::host::input::stop_running_flings(&mut self.scroll, &mut self.interaction);
        self.route.finish();
        // 输入框聚焦着就收起键盘（它属于要放到另一栏去的那页）
        if let Some(f) = self.interaction.focused_input.take() {
            self.emit_input_blur(&f.id, &f.value);
        }
        // 要放到另一栏的这页：留一张此刻的整帧（另一栏显示它；浮层不进图、去左栏的不带胶囊）、留视图与数据
        let to_left = self.frameset.swapped;
        if self.side_shot_needs_repaint(to_left) {
            self.frameset_render_for_side(to_left);
        }
        let shot = self.snapshot_frame();
        let vp = self.viewport_height();
        let mut leaving = SavedView::take(&mut self.scroll, &mut self.renderer, &mut self.interaction, vp);
        leaving.data = Some(self.page_data.clone());
        let Some(coming) = self.keep.saved.pop() else { return };
        self.keep.saved.push(leaving);
        self.page_stack.swap(n - 1, n - 2);
        let k = self.events.nav.len();
        if k >= 2 {
            self.events.nav.swap(k - 1, k - 2);
        }
        if let Some(last) = self.back_shots.last_mut() {
            *last = shot.unwrap_or(crate::host::edge_back::PageShot::empty());
        }
        self.frameset.swapped = !self.frameset.swapped;
        self.frameset.side = None;
        // 宿主按下时已经按「点的那一栏是活页面」挪了坐标：这一帧一定要报一次摆法，哪怕最后又回到原样
        // （左栏 navigateTo：先对调到左、再关右栏开新页回到右 —— 前后状态一样，不报的话宿主一直以为活页面在左）
        self.frameset.reported = None;
        // 逻辑层：事件、setData、组件实例都按 __currentPage
        let below = if self.frameset.swapped { 1 } else { 0 };
        self.app.eval(&format!("__framesetFocus({below})")).ok();
        // 换上来那页的数据先放回去（逻辑层下一次重画时再取最新的）：换完紧接着就 navigateTo 的话，
        // 盖住它时留的是这份 —— 从前还是右栏那页的，左栏重画出来一片空字段
        self.page_data = coming.data.clone().unwrap_or_default();
        self.remount_top_components();
        self.put_back_view(Some(coming));
        self.needs_redraw = true;
        self.pump_notes.add("分栏：换活页面", Some(t0));
    }
}
