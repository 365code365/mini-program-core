//! 挂起 / 恢复：小程序 A 打开小程序 B 时，A 不销毁、留在内存里（与微信一致），B 关掉后 A 原样回来 ——
//! 页面栈、滚动位置、列表里已经拉下来的数据、输入框里的字、JS 里的变量都还在，不用重开、不用盖 loading。
//!
//! 为什么要「挂起」而不是两个引擎一起跑：引擎有几样状态是进程级的（包根 `page_loader::set_app_path`、
//! 数据目录与命名空间、tabBar 高度、按组件 id 存的 swiper / picker / canvas / 视频等状态表，见
//! `renderer/components/park.rs`），B 建引擎时会把它们换成自己的。所以同一时刻只能有一个引擎在跑：
//! - [`MiniEngine::suspend`]：派发 `onHide`（页面先、App 后），把上面这些状态记下 / 整份带走，之后 `pump` 什么都不做；
//! - [`MiniEngine::resume`]：把它们还原（B 留下的丢掉），设上「本次进入」的参数（scene 1038 + referrerInfo），
//!   派发 `onShow`（App 先、页面后），重新合成一帧。画布在挂起期间一直在（`on_hide` 不释放），回来不用重画。
//!
//! 线程局部（thread_local）的状态不用管：宿主给每个引擎一条自己的线程（Android `EngineCore`、iOS `EngineSession`）。
//! 宿主要保证：B 建引擎之前 A 已经挂起，A 恢复之前 B 已经销毁完。
use super::engine::MiniEngine;

/// 一个挂起的小程序带走 / 记下的进程级状态
pub(crate) struct Parked {
    app_path: std::path::PathBuf,
    data: (Option<std::path::PathBuf>, Option<String>),
    tabbar_h: u32,
    comps: crate::renderer::components::park::Parked,
}

impl MiniEngine {
    /// 挂起（见文件头）。已经挂着返回 false
    pub fn suspend(&mut self) -> bool {
        if self.parked.is_some() {
            return false;
        }
        self.on_hide();
        self.parked = Some(Box::new(Parked {
            app_path: super::page_loader::get_app_path(),
            data: crate::sys::data_dir::raw_settings(),
            tabbar_h: super::tabbar::tabbar_height(),
            comps: crate::renderer::components::park::park(),
        }));
        true
    }

    /// 恢复（见文件头）。`enter`：`wx.getEnterOptionsSync()` / `App.onShow(options)` 拿到的参数
    /// （`{path, query, scene, referrerInfo}`），不给就沿用启动参数。没挂着返回 false
    pub fn resume(&mut self, enter: Option<&serde_json::Value>) -> bool {
        let Some(p) = self.parked.take() else { return false };
        // 包根同时登记给图片等包内资源的路径解析（set_app_path 里一起设）
        super::page_loader::set_app_path(p.app_path);
        crate::sys::data_dir::restore_settings(p.data);
        super::tabbar::set_tabbar_height(p.tabbar_h);
        crate::renderer::components::park::unpark(p.comps);
        // 解码图按包内路径（`/images/a.png`）缓存：B 的同名图不是 A 的那张。画布上 A 的像素还在，之后要重画的再按需解码
        crate::renderer::components::clear_image_caches();
        crate::canvas::clear_scaled_image_caches();
        if let Some(e) = enter {
            self.app.eval(&format!("globalThis.__hostEnterOptions = {e};")).ok();
        }
        self.on_show();
        true
    }

    /// 挂着（`suspend` 之后、`resume` 之前）：`pump` 不做事，宿主也不该给它 surface 与输入
    pub fn is_suspended(&self) -> bool {
        self.parked.is_some()
    }
}
