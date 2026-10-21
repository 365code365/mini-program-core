//! `wx.setClipboardData` / `wx.getClipboardData`。
//!
//! 引擎里先存一份（无头、测试、SDK 都有确定的行为），再按宿主决定要不要同步到系统剪贴板：
//! - 桌面窗体启动时调 [`use_system`]：读写都走系统剪贴板（arboard）；
//! - 移动端 SDK：写的时候另发一条宿主事件 `{type:'clipboard', data}`（见 prelude/host.js），
//!   宿主收到后写进系统剪贴板；读拿引擎里这份（宿主可以在 `hostApis` 里点名 getClipboardData 自己实现）。
//! 无头与测试从不碰系统剪贴板 —— 跑一遍测试不能把开发者剪贴板里的东西冲掉。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static TEXT: Mutex<String> = Mutex::new(String::new());
static SYSTEM: AtomicBool = AtomicBool::new(false);

/// 桌面窗体：读写同步到系统剪贴板
pub fn use_system(on: bool) {
    SYSTEM.store(on, Ordering::Relaxed);
}

pub fn set(text: &str) {
    *TEXT.lock().unwrap() = text.to_string();
    #[cfg(feature = "desktop")]
    if SYSTEM.load(Ordering::Relaxed) {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(text.to_string());
        }
    }
}

pub fn get() -> String {
    #[cfg(feature = "desktop")]
    if SYSTEM.load(Ordering::Relaxed) {
        if let Ok(t) = arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
            return t;
        }
    }
    TEXT.lock().unwrap().clone()
}
