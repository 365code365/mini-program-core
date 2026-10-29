//! 小游戏的安全区与胶囊（右上角「••• | ⊙」）的位置：宿主报屏幕内缩（刘海、灵动岛、横条、横屏时的侧边），
//! 引擎按当前方向算出 `statusBarHeight` / `safeArea` / `safeAreaInsets` / `menuButton`，交给
//! `wx.getSystemInfoSync` / `getWindowInfo` / `getMenuButtonBoundingClientRect`；宿主的原生胶囊也按
//! [`GameEngine::menu_button_rect`] 摆（同一个数，游戏避让胶囊时不会差几个像素）。
//!
//! ## 公式（与小程序 capsule.rs 同一组尺寸：87×32、右边距 7）
//! - 竖屏：`top = statusBarHeight + 4`（Android 8，48 高导航栏里居中），`statusBarHeight = 内缩 top`；
//! - 横屏：手机横过来状态栏是收起的（iOS 横屏没有状态栏，Android 游戏页沉浸式），`statusBarHeight = 内缩 top`
//!   （iPhone 横屏是 0），胶囊 `top = 内缩 top + 8`；
//! - 两个方向都是 `right = windowWidth - 内缩 right - 7`：iPhone 横屏刘海在右边时胶囊让进安全区里。
//!
//! 宿主没报内缩（无头测试、老宿主）就不动：系统信息与从前逐字节相同，胶囊走 JS 里的老公式。
//! 宿主在 `systemInfo` 里显式给了的字段（`safeArea` / `statusBarHeight` / `menuButton`）一律以宿主为准。
use crate::host::SafeAreaInsets;
use serde_json::{json, Value};

pub const WIDTH: f32 = crate::host::capsule::WIDTH;
pub const HEIGHT: f32 = crate::host::capsule::HEIGHT;
const RIGHT_GAP: f32 = crate::host::capsule::RIGHT_GAP;
const LANDSCAPE_TOP_GAP: f32 = 8.0;

/// 胶囊矩形（逻辑像素）：(left, top, width, height)
pub fn menu_button(w: f32, insets: SafeAreaInsets, landscape: bool, platform: &str) -> (f32, f32, f32, f32) {
    let top = if landscape { insets.top + LANDSCAPE_TOP_GAP } else { insets.top + crate::host::capsule::top_gap(platform) };
    let right = w - insets.right - RIGHT_GAP;
    (right - WIDTH, top, WIDTH, HEIGHT)
}

/// 按宿主报的内缩改写系统信息（`info` 是 engine_options::system_info 的结果）。`host` 是宿主的 systemInfo 覆盖项
pub fn apply(info: &mut Value, host: &serde_json::Map<String, Value>, insets: Option<SafeAreaInsets>, landscape: bool) {
    let Some(i) = insets else { return };
    let w = info["windowWidth"].as_f64().unwrap_or(0.0) as f32;
    let h = info["windowHeight"].as_f64().unwrap_or(0.0) as f32;
    let platform = info["platform"].as_str().unwrap_or("").to_string();
    let mut set = |k: &str, v: Value| {
        if !host.contains_key(k) {
            info[k] = v;
        }
    };
    set("statusBarHeight", json!(i.top));
    set("safeArea", json!({
        "left": i.left, "right": w - i.right, "top": i.top, "bottom": h - i.bottom,
        "width": (w - i.left - i.right).max(0.0), "height": (h - i.top - i.bottom).max(0.0),
    }));
    set("safeAreaInsets", json!({ "top": i.top, "right": i.right, "bottom": i.bottom, "left": i.left }));
    let (l, t, mw, mh) = menu_button(w, i, landscape, &platform);
    set("menuButton", json!({ "left": l, "top": t, "width": mw, "height": mh, "right": l + mw, "bottom": t + mh }));
}

/// 宿主报的内缩（`{"top","right","bottom","left"}`，逻辑像素）
pub fn parse_insets(v: &Value) -> Option<SafeAreaInsets> {
    let o = v.as_object()?;
    let f = |k: &str| o.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0).max(0.0) as f32;
    Some(SafeAreaInsets { top: f("top"), right: f("right"), bottom: f("bottom"), left: f("left") })
}

impl super::GameEngine {
    /// 宿主的屏幕内缩变了（转屏、横条显隐）：重算系统信息。不派发事件（转屏的 onWindowResize 由 resize 发）
    pub fn set_safe_area(&mut self, insets: SafeAreaInsets) {
        if self.safe_area == Some(insets) {
            return;
        }
        self.safe_area = Some(insets);
        self.install_window_info();
    }

    /// 胶囊矩形（逻辑像素，left / top / width / height），宿主按它摆原生胶囊。
    /// 宿主在 systemInfo 里给了 menuButton 就是那个；没报内缩时与 JS 的老公式一致
    pub fn menu_button_rect(&self) -> (f32, f32, f32, f32) {
        let landscape = self.width > self.height;
        let platform = super::target_platform();
        if let Some(mb) = self.system_info_opts.get("menuButton").and_then(|v| v.as_object()) {
            let f = |k: &str| mb.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            return (f("left"), f("top"), f("width"), f("height"));
        }
        let insets = self.safe_area.unwrap_or_else(|| {
            let sb = self.system_info_opts.get("statusBarHeight").and_then(|v| v.as_f64()).filter(|v| *v != 0.0).unwrap_or(20.0) as f32;
            SafeAreaInsets { top: sb, ..Default::default() }
        });
        // 没报内缩：老公式不分横竖（statusBarHeight + 间距）
        let landscape = landscape && self.safe_area.is_some();
        menu_button(self.width as f32, insets, landscape, platform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(w: f32, h: f32, platform: &str) -> Value {
        json!({ "windowWidth": w, "windowHeight": h, "platform": platform, "statusBarHeight": 20.0 })
    }

    #[test]
    fn 竖屏_刘海屏的胶囊在状态栏下面() {
        let mut v = info(402.0, 874.0, "ios");
        let i = SafeAreaInsets { top: 62.0, right: 0.0, bottom: 34.0, left: 0.0 };
        apply(&mut v, &Default::default(), Some(i), false);
        assert_eq!(v["statusBarHeight"], json!(62.0));
        assert_eq!(v["safeArea"], json!({"left":0.0,"right":402.0,"top":62.0,"bottom":840.0,"width":402.0,"height":778.0}));
        assert_eq!(v["menuButton"], json!({"left":308.0,"top":66.0,"width":87.0,"height":32.0,"right":395.0,"bottom":98.0}));
    }

    #[test]
    fn 横屏_胶囊让开右边的刘海() {
        let mut v = info(874.0, 402.0, "ios");
        let i = SafeAreaInsets { top: 0.0, right: 62.0, bottom: 21.0, left: 62.0 };
        apply(&mut v, &Default::default(), Some(i), true);
        assert_eq!(v["statusBarHeight"], json!(0.0));
        assert_eq!(v["safeArea"]["left"], json!(62.0));
        assert_eq!(v["safeArea"]["right"], json!(812.0));
        assert_eq!(v["safeArea"]["bottom"], json!(381.0));
        // right = 874 - 62 - 7 = 805，top = 0 + 8
        assert_eq!(v["menuButton"]["right"], json!(805.0));
        assert_eq!(v["menuButton"]["top"], json!(8.0));
    }

    #[test]
    fn 安卓竖屏间距8_宿主显式给的字段不改() {
        let mut v = info(411.0, 891.0, "android");
        let mut host = serde_json::Map::new();
        host.insert("statusBarHeight".into(), json!(30));
        v["statusBarHeight"] = json!(30);
        apply(&mut v, &host, Some(SafeAreaInsets { top: 24.0, ..Default::default() }), false);
        assert_eq!(v["statusBarHeight"], json!(30));
        assert_eq!(v["menuButton"]["top"], json!(32.0));
    }

    #[test]
    fn 没报内缩不动() {
        let mut v = info(375.0, 667.0, "ios");
        let before = v.clone();
        apply(&mut v, &Default::default(), None, false);
        assert_eq!(v, before);
    }
}
