//! H5 导出里的右上角胶囊：`navigationStyle: custom` 的页面静态写进 HTML（`examples/compare`
//! 截图时把页面脚本关掉了，靠运行时插入的话双端对比里就没有它）。
//!
//! 几何与配色与原生端同一组数（`src/host/capsule.rs`）：375 宽视口里 281,24 87×32，
//! `navigationBarTextStyle: white` 用半透明黑底白图标，否则半透明白底黑图标。
//! 点击在运行时（`runtime/08e_io.js`）接：先派发可取消的 `wx-menu-button` 事件给嵌入页面，
//! 没人拦就走默认 ——「•••」弹关于（`window.__APP_ABOUT__`），「⊙」返回（`history.back()`）。
use serde_json::Value as JsonValue;

/// 视口宽（与 `#app` 一致）
const VIEWPORT: f32 = 375.0;

/// 胶囊的 HTML（含它自己的样式）；不是 custom 导航栏的页面返回空串
pub fn capsule_html(window: &JsonValue) -> String {
    if window.get("navigationStyle").and_then(|v| v.as_str()) != Some("custom") {
        return String::new();
    }
    let dark = window.get("navigationBarTextStyle").and_then(|v| v.as_str()) == Some("white");
    // 配色（与 capsule.rs 的 palette 逐项对应，alpha = 那边的 u8 / 255）
    let (bg, border, divider, icon) = if dark {
        ("rgba(0,0,0,.149)", "rgba(255,255,255,.251)", "rgba(255,255,255,.251)", "#fff")
    } else {
        ("rgba(255,255,255,.6)", "rgba(0,0,0,.078)", "rgba(0,0,0,.2)", "#000")
    };
    // 定位：fixed，水平按视口居中再偏到右边（窗口比 375 宽时与 #app 对齐）
    let left = 281.0 - VIEWPORT / 2.0;
    format!(
        "<style>\
.wx-capsule{{position:fixed;top:24px;left:50%;margin-left:{left}px;width:87px;height:32px;z-index:4000;display:flex;}}\
.wx-capsule svg{{position:absolute;left:0;top:0;pointer-events:none;}}\
.wx-capsule i{{flex:1;cursor:pointer;}}\
</style>\n\
<div class=\"wx-capsule\" data-wx-capsule>\
<svg width=\"87\" height=\"32\" viewBox=\"0 0 87 32\">\
<rect width=\"87\" height=\"32\" rx=\"16\" fill=\"{bg}\"/>\
<rect x=\".25\" y=\".25\" width=\"86.5\" height=\"31.5\" rx=\"15.75\" fill=\"none\" stroke=\"{border}\" stroke-width=\".5\"/>\
<rect x=\"43.25\" y=\"7\" width=\".5\" height=\"18\" fill=\"{divider}\"/>\
<g fill=\"{icon}\"><circle cx=\"15.55\" cy=\"16\" r=\"1.7\"/><circle cx=\"21.75\" cy=\"16\" r=\"2.4\"/><circle cx=\"27.95\" cy=\"16\" r=\"1.7\"/>\
<circle cx=\"65.25\" cy=\"16\" r=\"2.7\"/></g>\
<circle cx=\"65.25\" cy=\"16\" r=\"6.45\" fill=\"none\" stroke=\"{icon}\" stroke-width=\"1.9\"/>\
</svg><i data-act=\"menu\" title=\"关于\"></i><i data-act=\"close\" title=\"返回\"></i></div>\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn 只有_custom_页面有胶囊_深浅按前景色() {
        assert!(capsule_html(&json!({})).is_empty());
        assert!(capsule_html(&json!({"navigationStyle": "default"})).is_empty());
        let light = capsule_html(&json!({"navigationStyle": "custom"}));
        assert!(light.contains("data-wx-capsule") && light.contains("rgba(255,255,255,.6)"));
        assert!(light.contains("data-act=\"menu\"") && light.contains("data-act=\"close\""));
        // 281 - 187.5：在 375 宽的视口里落在 281
        assert!(light.contains("margin-left:93.5px") && light.contains("top:24px"));
        let dark = capsule_html(&json!({"navigationStyle": "custom", "navigationBarTextStyle": "white"}));
        assert!(dark.contains("rgba(0,0,0,.149)") && dark.contains("fill=\"#fff\""));
    }
}
