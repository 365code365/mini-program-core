//! 安全区与胶囊的 C ABI（引擎侧见 game/chrome.rs）
use super::{guarded, MRGame};

/// 屏幕内缩变了（转屏、横条显隐；逻辑像素）：引擎按当前方向重算 safeArea / statusBarHeight / 胶囊位置。
/// 建实例时用创建选项 `"safeArea": {"top","right","bottom","left"}` 给第一份
#[no_mangle]
pub extern "C" fn mr_game_set_safe_area(game: *mut MRGame, top: f32, right: f32, bottom: f32, left: f32) {
    let i = crate::host::SafeAreaInsets { top: top.max(0.0), right: right.max(0.0), bottom: bottom.max(0.0), left: left.max(0.0) };
    guarded(game, (), |e| e.set_safe_area(i));
}

/// 胶囊（右上角「••• | ⊙」）在视图里的矩形（逻辑像素），与 `wx.getMenuButtonBoundingClientRect` 同一个数：
/// 宿主按它摆原生胶囊。建实例 / resize / set_safe_area 之后再取。实例为空时写 0
#[no_mangle]
pub extern "C" fn mr_game_menu_button(game: *mut MRGame, x: *mut f32, y: *mut f32, w: *mut f32, h: *mut f32) {
    let r = guarded(game, (0.0, 0.0, 0.0, 0.0), |e| e.menu_button_rect());
    // SAFETY: 调用方给的输出指针，可为 NULL
    unsafe {
        for (p, v) in [(x, r.0), (y, r.1), (w, r.2), (h, r.3)] {
            if let Some(p) = p.as_mut() {
                *p = v;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::ffi::CString;

    #[test]
    fn 横竖屏的胶囊跟着内缩走() {
        let _l = (crate::data_dir::SCOPE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner()),
            crate::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner()));
        let d = std::env::temp_dir().join(format!("mr-ffi-chrome-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("game.json"), "{}").unwrap();
        std::fs::write(d.join("game.js"), "").unwrap();
        let c = |s: &str| CString::new(s).unwrap();
        let opts = c(r#"{"appId":"wxchrome","safeArea":{"top":62,"right":0,"bottom":34,"left":0}}"#);
        let g = mr_game_create(c(d.to_str().unwrap()).as_ptr(), c(d.with_extension("data").to_str().unwrap()).as_ptr(), 402, 874, 3.0, opts.as_ptr());
        assert!(!g.is_null());
        let rect = |g| {
            let (mut x, mut y, mut w, mut h) = (0f32, 0f32, 0f32, 0f32);
            mr_game_menu_button(g, &mut x, &mut y, &mut w, &mut h);
            (x, y, w, h)
        };
        let js = |g, s: &str| unsafe { std::ffi::CStr::from_ptr(mr_game_eval(g, c(s).as_ptr())) }.to_str().unwrap().to_string();
        // 竖屏：top = 62 + 4（非 Android 目标），right = 402 - 7
        let platform_gap = if cfg!(target_os = "android") { 8.0 } else { 4.0 };
        assert_eq!(rect(g), (308.0, 62.0 + platform_gap, 87.0, 32.0));
        assert_eq!(js(g, "JSON.stringify(wx.getMenuButtonBoundingClientRect().left)"), "308");
        // 转成横屏：先报内缩再 resize（宿主两种顺序都可能）
        mr_game_set_safe_area(g, 0.0, 62.0, 21.0, 62.0);
        mr_game_resize(g, 874, 402, 3.0, c("landscape").as_ptr());
        assert_eq!(rect(g), (874.0 - 62.0 - 7.0 - 87.0, 8.0, 87.0, 32.0));
        assert_eq!(js(g, "wx.getSystemInfoSync().safeArea.right"), "812");
        assert_eq!(js(g, "wx.getWindowInfo().statusBarHeight"), "0");
        assert_eq!(js(g, "wx.getMenuButtonBoundingClientRect().top"), "8");
        mr_game_destroy(g);
        let _ = std::fs::remove_dir_all(&d);
    }
}
