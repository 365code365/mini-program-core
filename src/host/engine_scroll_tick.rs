//! [`MiniEngine`] 每帧的 scroll-view 事件 → `wx.pageScrollTo` → `onPageScroll`，以及测试用的 scroll-view 偏移查询。
//!
//! 从 `scroll_view.rs` 原样搬出（那边超过 500 行；那边留与宿主无关、桌面窗体也用的函数）。
use super::scroll_view as sv;

impl super::engine::MiniEngine {
    /// 当前页面定义了 `onPageScroll`：没定义就不派发（每页问一次逻辑层，结果按页记住）。
    ///
    /// 与微信一致（官方文档：「请只在需要的时候才在 page 中定义此方法，不要定义空方法」——
    /// 没定义时渲染层根本不往逻辑层发滚动事件）。从前每滚一帧都 `eval` 一段派发脚本：
    /// 一加 PLY110 上快甩时 QuickJS 光解析这段就占引擎线程 7%，页面根本没有这个回调
    pub(super) fn page_wants_scroll_events(&mut self) -> bool {
        let key = (self.page_stack.len(), self.page_opened_ms, self.current_route());
        if let Some((k, v)) = &self.page_scroll_handler {
            if *k == key {
                return *v;
            }
        }
        let v = self
            .app
            .eval("(typeof __currentPage === 'object' && __currentPage && typeof __currentPage.onPageScroll === 'function') ? 1 : 0")
            .map(|r| r.trim() == "1")
            .unwrap_or(true);
        self.page_scroll_handler = Some((key, v));
        v
    }
}

/// `InteractionManager::scroll_views` 里给页面 `onPageScroll` 记「上次报的位置」用的键。
/// 组件 id 不会是这个（NUL 开头），换页时随 `clear_page_state` 一起清
const PAGE_KEY: &str = "\u{0}page";

impl super::engine::MiniEngine {
    /// 某个 scroll-view 当前的滚动偏移（逻辑 px；`id` 是它的 `id` 属性或组件 id）。
    /// 诊断与测试用：宿主不需要它也能工作
    pub fn scroll_view_offset(&self, id: &str) -> Option<f32> {
        self.interaction.get_scroll_controller(id).map(|c| c.get_position())
    }
    /// 所有 scroll-view 的偏移（按 id 排好）。测试用：两个引擎比「滚到同一处没有」
    pub fn scroll_view_offsets(&self) -> Vec<(String, f32)> {
        let mut v: Vec<(String, f32)> = self.interaction.scroll_controllers.iter().map(|(k, c)| (k.clone(), c.get_position())).collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }

    /// SDK 每帧：scroll-view 事件 → `wx.pageScrollTo` → `onPageScroll`。返回是否要重画
    pub(super) fn tick_scroll_views(&mut self, now_ms: u64) -> bool {
        // swiper 的换页动画与自动播放按宿主给的时钟走（同一串 pump 时间 → 同一串画面）
        crate::renderer::components::swiper_set_clock(now_ms.max(1));
        // scroll-view 的事件（bindscroll / scrolltolower / refresher…）派完不用整页重画：回调里的 setData
        // 在本函数末尾记成数据脏，交给失效分析；scroll-view 自己的位移由 engine_damage 只重画它的盒子。
        // 从前派过事件就 `mark_full`：带 bindscroll 的列表（外卖店铺页、点餐页按滚动高亮左侧分类）
        // 滚着的每一帧都整页重画，一加 PLY110 上 14~34ms、屏上重复 25.7%
        let mut redraw = false;
        sv::tick(&mut self.app, self.renderer.as_ref(), &mut self.interaction, now_ms);
        // swiper / picker-view 的事件（change / transition / animationfinish …）派完不用整页重画：
        // 回调里 setData 走下面的失效分析，swiper 自己的位移由 engine_swiper.rs 只重画它的盒子，
        // picker-view 的吸附动画照旧按「在动」整帧。从前这里 `redraw = true` → `mark_full`：
        // 轮播页每次自动翻页那一帧整页重画（一加上 60ms，bindchange 里改了一行字）
        super::swiper::tick(&mut self.app, self.renderer.as_ref(), now_ms);
        // picker 面板弹着时跟上页面改的 range（multiSelector 联动）
        if super::picker_sheet::sync_columns(&mut self.picker_sheet, self.renderer.as_ref()) {
            redraw = true;
        }
        // VideoContext 命令、video 的组件事件、控制条（内置播放器那边变了要重画；同层渲染只重画视频框）
        if super::video::tick(&mut self.app, self.renderer.as_ref(), &self.interaction, now_ms) {
            redraw = true;
            self.dmg.compose_dirty = true;
        }
        // LivePlayerContext 的命令（同层渲染的直播，engine_live_player.rs）
        self.tick_live_players();
        // 相机帧 / 扫码 / voip-room（engine_camera.rs）
        self.tick_cameras();
        // 小窗：回到原页收起、宿主交了新帧重新合成（engine_pip.rs；没有小窗时一次判空）
        self.tick_pip();
        // canvas-id 重复 → binderror（engine_canvas_check.rs）
        self.tick_canvas_check();
        // 全屏那一层（画在合成那一步）：新帧 / 控制条变了只重新合成
        self.sync_fullscreen(now_ms);
        // SelectorQuery / ScrollViewContext / IntersectionObserver（与桌面共用）
        let vp = self.viewport_height();
        let view = super::selector_query::PageView {
            scroll_top: self.scroll.get_position(),
            width: self.width as f32,
            height: vp,
            content_height: self.scroll.get_max_scroll() + vp,
        };
        if super::selector_query::tick(&mut self.app, self.renderer.as_ref(), &mut self.interaction, view) {
            redraw = true;
        }
        let route = self.current_route();
        super::observers::tick(&mut self.app, self.renderer.as_ref(), &self.interaction, &route, view, now_ms);
        // 操作菜单 / 图片预览的请求与动画（画在合成那一步，只要重新合成）
        // 预览层的关闭键放在状态栏下面、「当前 / 总数」放在底部安全区上面
        let bottom = self.opts.safe_area.map(|s| s.bottom).unwrap_or(0.0);
        super::ui_overlay::image_preview::set_insets(self.opts.status_bar_height(), bottom);
        if super::ui_overlay::take_sheet_requests(&mut self.app) {
            self.dmg.compose_dirty = true;
        }
        // wx.setBackgroundColor / wx.hideKeyboard
        let (wa_redraw, _) = super::window_api::take_requests(&mut self.app, self.renderer.as_ref(), &mut self.interaction);
        if wa_redraw {
            redraw = true;
        }
        // 原生 tabBar 的运行时 API：改了就重画 tabBar；显隐变了视口高度跟着变，重新排版
        let was_hidden = super::tabbar_api::hidden();
        if super::tabbar_api::take_requests(&mut self.app, self.config.tab_bar.as_ref()) {
            self.dmg.tabbar_route = None;
            redraw = true;
            if super::tabbar_api::hidden() != was_hidden {
                self.update_renderers();
            }
        }
        if sv::apply_page_scroll(&mut self.app, self.renderer.as_ref(), &self.interaction, &mut self.scroll) {
            redraw = true;
        }
        let pos = self.scroll.get_position();
        let wants = self.page_wants_scroll_events();
        let st = sv::state(&mut self.interaction.scroll_views, PAGE_KEY);
        let mut last = if st.tracked { st.reported_pos } else { 0.0 };
        st.tracked = true;
        if wants && sv::page_scroll(&mut self.app, pos, &mut last) {
            self.scroll_moved = true;
        }
        sv::state(&mut self.interaction.scroll_views, PAGE_KEY).reported_pos = last;
        // 事件处理函数里的 setData 这一帧就生效（不等下一帧开头）。
        //
        // 只记「数据脏了」，**不**要整页重画：与别处的 setData 一样交给失效分析（只重画变了的节点）。
        // 从前这里 `redraw = true` → 调用方 `mark_full`：onPageScroll / IntersectionObserver 回调里
        // 一 setData 就整页重画 —— 一加 PLY110 上滑「节点与滚动」页 42~71 帧 `整帧·结构`、
        // 每帧 24~31ms，屏上重复 9~11%（tests/sdk_damage_equivalence.rs 有这一条）
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
        // 重画之前：scroll-into-view 绑定的字段这一帧里变过又变回来，也要再滚一次
        sv::rearm_into_view(&self.app, self.renderer.as_ref(), &mut self.interaction, self.page_data_dirty);
        redraw
    }
}
