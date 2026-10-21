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
        self.needs_redraw = true;
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

    pub fn on_show(&mut self) {
        self.app.eval("__dispatchApp('onShow')").ok();
        self.needs_redraw = true;
    }

    pub fn on_hide(&mut self) {
        self.app.eval("__dispatchApp('onHide')").ok();
    }

    pub fn trim_memory(&mut self) {
        crate::trim_memory();
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
