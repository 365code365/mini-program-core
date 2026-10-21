//! 尺寸、像素比、安全区变化：转屏、分屏、折叠屏、Flutter 里外层约束变了。
//!
//! 从前引擎创建之后尺寸就定死了：Android 的 View 在 `onSizeChanged` 里只是请求一帧，
//! 画面还是旧尺寸、贴在左上角；Flutter 的 `Texture` 会把旧尺寸的画面拉伸铺满。
//!
//! ## 要重建的东西（与 `new_with_options` 一一对应）
//! - 三块画布（页面 / fixed 覆盖层 / tabBar）按新的设备像素尺寸重分配；
//! - 两个渲染器：`WxmlRenderer` 没有改尺寸的入口，而且布局缓存的键里**没有屏宽**，
//!   原地改会静默复用旧布局 —— 所以与换页一样整个重建（`update_renderers`）；
//! - 自定义 tabBar 的高度按新宽度重新实测（rpx 随屏宽变）；
//! - 逻辑层的系统信息，并派发 `wx.onWindowResize` / 页面 `onResize`；
//! - 安全区或屏宽变了时，栈上每一页的样式表重新解析（`env()` 与 rpx 折算都在解析期）。
//!
//! 滚动位置保留（按新内容高夹住），进行中的手势取消（坐标系已经变了）。
use super::engine::MiniEngine;
use super::engine_options::{self, SafeAreaInsets};
use crate::compositor::LayerCanvas;
use crate::host::{tabbar_height, CONTENT_HEIGHT};
use crate::renderer::WxmlRenderer;
use crate::Canvas;

impl MiniEngine {
    /// 改尺寸（逻辑像素）与像素比。没变返回 false；变了重建并立刻出一帧，
    /// 之后 `pixel_size()` / 像素出口都是新尺寸。
    pub fn resize(&mut self, width: u32, height: u32, dpr: f32) -> bool {
        let (width, height) = (width.max(1), height.max(1));
        let dpr = if dpr > 0.1 { dpr } else { 1.0 };
        if (width, height) == (self.width, self.height) && (dpr - self.dpr).abs() < 1e-6 {
            return false;
        }
        let width_changed = width != self.width;
        self.pointer_cancel();
        crate::host::input::stop_running_flings(&mut self.scroll, &mut self.interaction);
        let keep_scroll = self.scroll.get_position();

        self.width = width;
        self.height = height;
        self.dpr = dpr;
        self.app.set_size(width, height);
        if let Some(sheet) = &mut self.picker_sheet {
            sheet.viewport = (width as f32, height as f32);
        }
        self.sync_system_info(true);
        if width_changed {
            // rpx 与 `env()` 的折算都跟屏宽走：样式表与 tabBar 高度要按新宽度重来
            self.restyle();
        }
        self.rebuild_surfaces();
        self.scroll.set_position(keep_scroll);
        self.render();
        true
    }

    /// 改安全区（逻辑像素）。宿主在拿到系统的 insets 后、以及转屏后调用。
    pub fn set_safe_area(&mut self, insets: SafeAreaInsets) -> bool {
        if self.opts.safe_area == Some(insets) {
            return false;
        }
        self.opts.safe_area = Some(insets);
        self.sync_system_info(false);
        self.restyle();
        self.rebuild_surfaces();
        self.render();
        true
    }

    /// 把系统信息重新装进逻辑层；`notify` 时派发窗口尺寸变化
    fn sync_system_info(&mut self, notify: bool) {
        let info = engine_options::system_info(self.width, self.height, self.dpr, &self.opts);
        engine_options::install_system_info(&self.app, &info);
        if notify {
            self.app.eval("__dispatchWindowResize()").ok();
            self.app.pump_jobs();
            if self.app.take_data_dirty() {
                self.page_data_dirty = true;
            }
        }
    }

    /// 栈上每一页（与自定义 tabBar）的样式表按当前安全区 / 屏宽重新解析，tabBar 高度重测
    fn restyle(&mut self) {
        let paths: Vec<String> = self.page_stack.iter().map(|p| p.path.clone()).collect();
        for (i, path) in paths.iter().enumerate() {
            let Some(info) = self.pages.get(path) else { continue };
            match self.page_styles(info) {
                Ok((_, sheet)) => self.page_stack[i].stylesheet = sheet,
                Err(e) => eprintln!("⚠️  重新解析 {path} 的样式失败: {e}"),
            }
        }
        Self::apply_env_context(&self.opts, self.width, self.height);
        if self.custom_tabbar.is_some() {
            if let Ok(Some(ct)) = crate::host::load_custom_tabbar_with_app_wxss(&self.app_wxss) {
                self.custom_tabbar = Some(ct);
            }
        }
        if let Some(ct) = &self.custom_tabbar {
            let probe = WxmlRenderer::new(ct.stylesheet.clone(), self.width as f32, self.height as f32);
            let measured = probe.measure_content_height(&ct.wxml_nodes, &ct.data);
            if measured > 1.0 {
                crate::host::set_tabbar_height(measured.round() as u32);
            }
        }
    }

    /// 画布与渲染器按当前尺寸重建，上一帧的绘制记录作废。
    ///
    /// 页面画布高先按 `CONTENT_HEIGHT` 给，下一次 `render()` 会按实测内容高再分配
    /// （与新建实例同一条路）。用 `replace` 而不是新建 `PageCanvas`：写入日志的代数要变，
    /// 以后按脏区上传的合成后端据此知道「整块换了」。
    fn rebuild_surfaces(&mut self) {
        let dpr = self.dpr;
        let px = |v: u32| (v as f32 * dpr) as u32;
        self.canvas.replace(Canvas::new(px(self.width), px(CONTENT_HEIGHT.max(self.height))));
        self.fixed_canvas = LayerCanvas::new(Canvas::new(px(self.width), px(self.height)));
        self.tabbar_canvas = LayerCanvas::new(Canvas::new(px(self.width), px(tabbar_height()).max(1)));
        self.update_renderers();
        self.drawn_band = None;
        self.fixed_rows = None;
        self.compose.clear();
        self.rgba.clear();
        self.rgba_stale = true;
        self.page_data_dirty = true;
        self.needs_redraw = true;
    }
}

impl MiniEngine {
    /// 宿主转来的「点了状态栏」（iOS）/「双击了标题栏」（安卓）：写了 `enable-back-to-top` 的纵向
    /// scroll-view 滚回顶部（官方 component/scroll-view.html）。返回是否有 scroll-view 动了
    pub fn tap_status_bar(&mut self) -> bool {
        let Some(r) = self.renderer.as_ref() else { return false };
        let ids: Vec<String> = r.scroll_views().iter().filter(|s| s.back_to_top).map(|s| s.component_id.clone()).collect();
        let mut moved = false;
        for id in ids {
            if let Some(c) = self.interaction.get_scroll_controller_mut(&id) {
                if c.get_position() > 0.5 {
                    c.animate_to(0.0, 0.3); // 与 scroll-with-animation / pageScrollTo 默认一样 300ms
                    moved = true;
                }
            }
        }
        moved
    }
}
