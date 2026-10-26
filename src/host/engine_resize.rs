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
use crate::host::CONTENT_HEIGHT;
use crate::renderer::WxmlRenderer;
use crate::Canvas;

impl MiniEngine {
    /// 改尺寸（逻辑像素）与像素比。没变返回 false；变了重建并立刻出一帧，
    /// 之后 `pixel_size()` / 像素出口都是新尺寸。
    pub fn resize(&mut self, width: u32, height: u32, dpr: f32) -> bool {
        let (width, height) = (width.max(1), height.max(1));
        let dpr = if dpr > 0.1 { dpr } else { 1.0 };
        // 分栏（engine_frameset.rs）：宿主给的是整个窗口宽，够宽时引擎按一栏的宽排
        self.frameset.total_width = width;
        let (split, width) = self.frameset.pane_width(width);
        if split != self.frameset.active {
            self.frameset.active = split;
            self.frameset.side = None;
        }
        if (width, height) == (self.width, self.height) && (dpr - self.dpr).abs() < 1e-6 {
            return false;
        }
        let width_changed = width != self.width;
        let only_height = !width_changed && (dpr - self.dpr).abs() < 1e-6;
        self.pointer_cancel();
        crate::host::input::stop_running_flings(&mut self.scroll, &mut self.interaction);
        let keep_scroll = self.scroll.get_position();

        self.width = width;
        self.height = height;
        self.dpr = dpr;
        self.refit_back_shots(only_height);
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
    ///
    /// 视频全屏时宿主收起状态栏报来的新值先按住（engine_video_feed.rs）：用了要整页重排样式，退出又排回来
    pub fn set_safe_area(&mut self, insets: SafeAreaInsets) -> bool {
        if crate::renderer::components::fullscreen_video().is_some() {
            self.video_feed.held_safe_area = (self.opts.safe_area != Some(insets)).then_some(insets);
            return false;
        }
        self.video_feed.held_safe_area = None;
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
        // windowHeight 按这一页（有引擎画的导航栏时减掉它，engine_navbar.rs）
        let info = self.page_system_info();
        engine_options::install_system_info(&self.app, &info);
        self.forget_window_height();
        // 转场按旧尺寸留的图对不上了：直接收手
        self.route.finish();
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
        // 留着的 tab 页同样重来；被盖住 / 切走的页留的渲染器是按旧尺寸建的，丢掉（回来时新建）
        let tabs: Vec<String> = self.keep.tabs.keys().cloned().collect();
        for path in tabs {
            let Some(info) = self.pages.get(&path) else { continue };
            if let Ok((_, sheet)) = self.page_styles(info) {
                if let Some((page, _)) = self.keep.tabs.get_mut(&path) {
                    page.stylesheet = sheet;
                }
            }
        }
        self.keep.forget_renderers();
        Self::apply_env_context(&self.opts, self.width, self.height);
        if let Some(ct) = self.custom_tabbar.as_mut() {
            // 只按新尺寸重解样式（WXML 与尺寸无关，data 以主上下文里的活实例为准，都不动）
            if let Some(sheet) = crate::host::page_loader::parse_custom_tabbar_stylesheet(&self.app_wxss) {
                ct.stylesheet = sheet;
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
        // tabBar 高含底部安全区（engine.rs::tabbar_logical_height）：set_safe_area 也走这里
        let tab_px = ((self.tabbar_h() * dpr) as u32).max(1);
        self.tabbar_canvas = LayerCanvas::new(Canvas::new(px(self.width), tab_px));
        // 被盖住 / 切走的页留的渲染器按旧尺寸建的（屏高进 `100vh` 与 fixed 视口）：回来时新建
        self.keep.forget_renderers();
        self.update_renderers();
        self.drawn_band = None;
        self.fixed_rows = None;
        self.compose.clear();
        self.rgba.clear();
        self.rgba_stale = true;
        self.page_data_dirty = true;
        self.mark_full();
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

impl MiniEngine {
    /// 侧滑返回留的那几张上一页图跟着改尺寸。
    ///
    /// 只有高度变了（宽与像素比没变）时按**底边对齐**改高：在 Flutter 里高度变化来自顶上的
    /// AppBar 显隐（普通页 ↔ `navigationStyle: custom` 页），上一页在屏幕上正好是下面那一截。
    /// 多出来的顶上那条填那一页的导航栏颜色（宿主的 AppBar 就是这个色），少了就从顶上裁掉。
    /// 不处理的话 `edge_back::compose` 见尺寸对不上就整条画暗底：进了 custom 页再侧滑，左边一片黑。
    /// 宽或像素比变了（转屏）旧图对不上任何位置，丢掉（侧滑时露暗底，与从前一样）。
    fn refit_back_shots(&mut self, only_height: bool) {
        let (w, h) = self.pixel_size();
        for i in 0..self.back_shots.len() {
            let fill = self.page_stack.get(i).and_then(|p| self.nav_bar_rgb(&p.path));
            let shot = &mut self.back_shots[i];
            // 压着的先解开，改完尺寸再压回去
            let was_packed = shot.packed.is_some();
            shot.unpack();
            if shot.pixels.is_empty() || (shot.width, shot.height) == (w, h) {
                if was_packed {
                    shot.pack();
                }
                continue;
            }
            if !only_height || shot.width != w {
                shot.pixels.clear();
                (shot.width, shot.height) = (0, 0);
                continue;
            }
            *shot = refit_bottom(shot, h, fill);
            if was_packed {
                shot.pack();
            }
        }
    }

    /// 某一页导航栏的背景色（app.json 的 window ← 页面 json）。0xRRGGBB
    fn nav_bar_rgb(&self, route: &str) -> Option<u32> {
        let page = self.pages.get(route).and_then(|p| p.window.get("navigationBarBackgroundColor"));
        let hex = page.or_else(|| self.app_window.get("navigationBarBackgroundColor"))?.as_str()?;
        let hex = hex.strip_prefix('#')?;
        let hex: String = if hex.len() == 3 { hex.chars().flat_map(|c| [c, c]).collect() } else { hex.to_string() };
        u32::from_str_radix(hex.get(..6)?, 16).ok()
    }
}

/// 底边对齐改高：`h` 比原图高就在顶上补 `fill`（没给就重复原图第一行），矮就从顶上裁
fn refit_bottom(shot: &crate::host::edge_back::PageShot, h: u32, fill: Option<u32>) -> crate::host::edge_back::PageShot {
    let w = shot.width as usize;
    let (old_h, new_h) = (shot.height as usize, h as usize);
    let mut pixels = Vec::with_capacity(w * new_h);
    if new_h >= old_h {
        let pad = new_h - old_h;
        match fill {
            Some(c) => pixels.resize(w * pad, c),
            None => (0..pad).for_each(|_| pixels.extend_from_slice(&shot.pixels[..w])),
        }
        pixels.extend_from_slice(&shot.pixels);
    } else {
        pixels.extend_from_slice(&shot.pixels[(old_h - new_h) * w..]);
    }
    crate::host::edge_back::PageShot::new(shot.width, h, pixels)
}

#[cfg(test)]
mod refit_tests {
    use super::refit_bottom;
    use crate::host::edge_back::PageShot;

    #[test]
    fn 变高顶上补导航栏色_变矮从顶上裁_都是底边对齐() {
        let shot = PageShot::new(2, 2, vec![1, 2, 3, 4]);
        let tall = refit_bottom(&shot, 4, Some(9));
        assert_eq!((tall.height, tall.pixels), (4, vec![9, 9, 9, 9, 1, 2, 3, 4]));
        let tall = refit_bottom(&shot, 3, None);
        assert_eq!(tall.pixels, vec![1, 2, 1, 2, 3, 4], "没给颜色：重复第一行");
        let short = refit_bottom(&shot, 1, Some(9));
        assert_eq!((short.height, short.pixels), (1, vec![3, 4]));
    }
}
