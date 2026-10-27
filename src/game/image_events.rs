//! pump 里与图片有关的一步（输入之后、rAF 之前，game-doc/02 §5.3）：收后台解码结果，交回 JS 的 onload / onerror；
//! 缓存全被引用还超预算时发 `wx.onMemoryWarning`。

use super::GameEngine;
use std::time::Duration;

/// 固定时钟（无头）下最多等解码多久：等完了再往下走，「设 src → 下一帧 onload」才逐次可复现
const FIXED_CLOCK_WAIT: Duration = Duration::from_secs(10);

impl GameEngine {
    /// 系统内存告警（Android onTrimMemory / onLowMemory，iOS didReceiveMemoryWarning；`mr_game_memory_warning`）：
    /// 放掉没人引用的图片解码结果、跑一遍 JS GC；`level` 是 5 / 10 / 15（Android 前台的三档，官方 onMemoryWarning 的取值）时
    /// 派发 wx.onMemoryWarning({level})，其它值（后台那几档）只回收不派发
    pub fn memory_warning(&mut self, level: i32) {
        // GC 之后 FinalizationRegistry 的回调是排进 job 队列的（Image 对象在那里通知原生放掉引用）：先跑完再回收
        self.collect_garbage();
        self.app.js_runtime().lock().unwrap().pump_jobs();
        let freed = self.images.borrow_mut().trim_unreferenced();
        self.host_log(&format!("🧹 内存告警 level={level}：放掉图片缓存 {}KB", freed / 1024));
        if matches!(level, 5 | 10 | 15) {
            self.guards.watchdog.arm();
            let r = self.app.eval(&format!("typeof __dispatchMemoryWarning === 'function' && __dispatchMemoryWarning({level})"));
            self.guards.watchdog.disarm();
            if let Err(e) = r {
                self.report_error(super::error_kind(&e, "memory"), &e);
            }
            self.after_js();
        }
    }

    pub(super) fn dispatch_images(&mut self) {
        let fixed = self.clock.borrow().is_fixed();
        let (ready, logs, warn) = {
            let mut s = self.images.borrow_mut();
            s.poll(fixed.then_some(FIXED_CLOCK_WAIT));
            (s.take_ready(), s.take_logs(), s.take_memory_warning())
        };
        for l in logs {
            self.host_log(&l);
        }
        if ready.is_empty() && !warn {
            return;
        }
        let mut errors = Vec::new();
        self.guards.watchdog.arm();
        for r in ready {
            let json = match &r.result {
                Ok((w, h)) => serde_json::json!({ "ok": true, "w": w, "h": h }),
                Err(e) => serde_json::json!({ "ok": false, "errMsg": e }),
            };
            if let Err(e) = self.app.call_str2("__gameImageDone", &r.id.to_string(), &json.to_string()) {
                errors.push(e);
            }
        }
        if warn {
            if let Err(e) = self.app.call_num("__dispatchMemoryWarning", &[10.0]) {
                errors.push(e);
            }
        }
        self.guards.watchdog.disarm();
        for e in errors {
            self.report_error(super::error_kind(&e, "image"), &e);
        }
        self.after_js();
    }

    /// 真正解码过几次（GT-I4 / I5）
    pub fn image_decode_count(&self) -> u64 {
        self.images.borrow().decode_count()
    }

    /// 图片缓存的字节数与改预算（GT-I6 用）
    pub fn image_cache_bytes(&self) -> usize {
        self.images.borrow().cached_bytes()
    }
    pub fn set_image_budget(&mut self, bytes: usize) {
        self.images.borrow_mut().set_budget(bytes);
    }
}
