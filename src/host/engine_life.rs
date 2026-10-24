//! [`MiniEngine`] 的生命周期、动画时钟与引擎时钟（从 `engine.rs` 按职责拆出，纯搬迁）。
use super::engine::MiniEngine;
use super::engine_options::EngineOptions;

impl MiniEngine {
    /// 固定 CSS 动画时钟（秒）。**只给确定性测试用**：正常运行时动画由真实时间驱动，
    /// 而快照对比需要把 `@keyframes` 求值到同一个时刻，否则两次跑的相位不同、
    /// 逐像素对比永远对不上（桌面侧对应 `--time` 参数）。
    pub fn set_animation_time(&mut self, seconds: f32) {
        self.anim_time = Some(seconds);
        if let Some(r) = &mut self.renderer {
            r.set_animation_time(seconds);
        }
        if let Some(r) = &mut self.tabbar_renderer {
            r.set_animation_time(seconds);
        }
        self.mark_full();
    }

    /// 同 [`set_animation_time`](Self::set_animation_time)，但**不**强制整帧：动画照常按
    /// 局部重画推进，只是时钟由调用方逐帧给。逐帧对照测试
    /// （`tests/sdk_damage_equivalence.rs`）靠它让两个引擎的动画相位一帧不差 ——
    /// 用 `set_animation_time` 的话每一帧都成了整帧，局部重画那条路根本没走到。
    pub fn set_animation_clock(&mut self, seconds: f32) {
        self.anim_time = Some(seconds);
        if let Some(r) = &mut self.renderer {
            r.set_animation_time(seconds);
        }
        if let Some(r) = &mut self.tabbar_renderer {
            r.set_animation_time(seconds);
        }
    }

    /// 回前台。前后台没变时不派发：Flutter 的 `AppLifecycleState` 一次切后台会来 hidden + paused 两条，
    /// 启动时还有一条 resumed；从前每条都派发，切一次后台 `App.onHide` 两三次、启动时多一次 `App.onShow`
    /// （微信每次只一次）。宿主各自去重容易漏，引擎这里统一挡住
    pub fn on_show(&mut self) {
        if !std::mem::replace(&mut self.backgrounded, false) {
            return;
        }
        // 进后台时压起来的那张侧滑截图（on_hide）解开，侧滑一开始就要直接显示
        self.unpack_last_shot();
        self.app.eval("__dispatchApp('onShow')").ok();
        // 与微信一致：回前台时当前页也收到 onShow（组件 pageLifetimes.show 跟着）——从前只派发 App 级，
        // 页面里「切后台再回来就刷新」的写法（route 页的生命周期流水）在本引擎上不生效。先 App 后 Page
        self.app.eval("typeof __dispatchPage === 'function' && __dispatchPage('onShow')").ok();
        // 回前台只要重新合成出一帧：画布一直在（进后台不释放），onShow 里的 setData 走自己的失效分析。
        // 从前这里 `mark_full`：每次从桌面回来都整页重画，一加 PLY110 上那一帧 39ms（画 23ms + 整张重传）
        self.dmg.compose_dirty = true;
    }

    pub fn on_hide(&mut self) {
        if std::mem::replace(&mut self.backgrounded, true) {
            return;
        }
        // 切后台：先当前页 onHide，再 App.onHide（与 on_show 的顺序相反）
        self.app.eval("typeof __dispatchPage === 'function' && __dispatchPage('onHide')").ok();
        self.app.eval("__dispatchApp('onHide')").ok();
        // 进后台放掉图片的缩放结果与 mip（最多 32 + 24MB）：回前台只重新合成（on_show），不重画页面，
        // 用不到它们；之后哪张图要重画再按需重采样。解码图与画布都留着，回来不用重新解码、重画
        crate::canvas::clear_scaled_image_caches();
        // 大块渐变的行缓存也放掉（回前台只重新合成，用不到；最多 8MB）
        crate::renderer::components::gradient::row_cache::clear();
        // 合成缓冲与 RGBA 读出缓冲（各一屏，1080×2200 上 9.5MB）也放掉：回前台 on_show 本来就整帧重新合成，
        // 合成时按尺寸重新分配（engine_frame.rs `compose_frame`）。GPU 上屏时卸 surface 那一下还会 CPU 合成一帧
        // （engine_gpu.rs `detach_native_surface`），这块缓冲在后台整屏驻留着，谁也不读。
        // 后台期间取像素：长度对不上，`pixels_into` 返回 0、截图返回 None（都是已有的「没有帧」分支）
        self.compose = Vec::new();
        self.rgba = Vec::new();
        // 侧滑要用的最后一张截图在后台也用不到：压起来（shot_pack.rs，一屏 9.5MB → 几百 KB），on_show 再解开。
        // 解开写一屏内存（几 ms），落在回前台那一帧，不在滑动途中
        if let Some(s) = self.back_shots.last_mut() {
            s.pack();
        }
        self.rgba_stale = true;
        self.dmg.compose_dirty = true;
        #[cfg(feature = "gpu")]
        {
            self.cpu_frame_pending = None;
        }
        // `MINI_MEM_LOG=1`：进后台时打一行内存分项，放完之后量（真机上按 Home 就能看到后台留着多少，engine_memory.rs）
        if std::env::var_os("MINI_MEM_LOG").is_some() {
            crate::compositor::diag::emit(&format!("🧠 {}", self.memory_report_line()));
        }
    }

    pub fn trim_memory(&mut self) {
        crate::trim_memory();
        // 逻辑层也回收一遍（截图除了侧滑要用的那张本来就压着，engine_edge_back.rs）
        self.app.collect_garbage();
    }

    /// 引擎的单调时钟（毫秒）。宿主给过时间就沿着宿主的时钟走（上次给的值 + 之后流逝的
    /// 真实时间），否则从实例创建算起。从前这里是墙上时钟（`SystemTime`）：系统改时会让
    /// 长按、动画计时跳变，而且与宿主传进 `pump` 的时间不是一个量纲。
    pub(crate) fn mono_ms(&self) -> u64 {
        match self.host_clock {
            Some((ms, at)) => ms + at.elapsed().as_millis() as u64,
            None => self.clock_origin.elapsed().as_millis() as u64,
        }
    }

    /// 把本实例的安全区、屏幕尺寸与深浅色放进本线程，供之后的 WXSS 解析求 `env()` 与 `@media` 用
    pub(crate) fn apply_env_context(opts: &EngineOptions, width: u32, height: u32) {
        let base = crate::parser::wxss::EnvContext::default();
        let dark = match opts.system_info.get("theme").and_then(|v| v.as_str()) {
            Some(t) => t.eq_ignore_ascii_case("dark"),
            None => base.dark,
        };
        crate::parser::wxss::set_env_context(crate::parser::wxss::EnvContext {
            insets: opts.safe_area.map(|s| s.as_array()).unwrap_or([0.0; 4]),
            screen_width: width as f32,
            screen_height: height as f32,
            dark,
        });
    }
}
