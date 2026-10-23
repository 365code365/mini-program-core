//! 宿主在**创建时**交给引擎的选项（`mr_app_create_ex` 的 JSON），以及由它算出的
//! 逻辑层系统信息。
//!
//! ## 为什么要在创建时给
//! `app.js` 在 `MiniEngine::new` 里就执行了，而 `onLaunch` 里读 `wx.getSystemInfoSync()`
//! 算导航栏高度、用 `typeof wx.chooseImage` 探测能力是常态 —— 创建之后再设就晚了。
//!
//! ## 默认值不变
//! 宿主什么都不给时，系统信息只把**尺寸**换成引擎的真实尺寸（375×667@2 时与从前
//! 写死的那份逐字段相同），安全区、状态栏仍是「无刘海机型」的 0 / 20。
use serde_json::{json, Map, Value};

/// 安全区内缩（逻辑像素）。刘海、灵动岛、底部横条、横屏时的侧边。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SafeAreaInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl SafeAreaInsets {
    fn from_json(v: &Value) -> Option<Self> {
        let o = v.as_object()?;
        let f = |k: &str| o.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0).max(0.0) as f32;
        Some(Self { top: f("top"), right: f("right"), bottom: f("bottom"), left: f("left") })
    }

    pub fn as_array(&self) -> [f32; 4] {
        [self.top, self.right, self.bottom, self.left]
    }
}

/// 创建选项。所有字段都可省。
#[derive(Clone, Debug, Default)]
pub struct EngineOptions {
    /// 合并进 `wx.getSystemInfoSync()` 的字段（`platform` / `system` / `brand` / `model` /
    /// `language` / `theme` / `statusBarHeight` …）。尺寸相关字段由引擎按真实尺寸算，
    /// 这里给了会覆盖。
    pub system_info: Map<String, Value>,
    /// 宿主给了安全区：状态栏高度取 `top`，`env(safe-area-inset-*)` 取真值
    pub safe_area: Option<SafeAreaInsets>,
    /// 宿主实现的 `wx.*` 能力（如 `chooseImage`）。逻辑层会把它们装成真函数，
    /// 调用经事件 `hostCall` 交给宿主，宿主用 `mr_app_host_reply` 回结果。
    pub host_apis: Vec<String>,
    /// 由宿主用原生控件承载的组件标签（`video` / `map` / `web-view` / `camera` …）。
    /// 引擎只算位置并发 `nativeViews` 事件，不自己画也不自己解码。
    pub native_components: Vec<String>,
    /// 同层渲染的组件标签（`"sameLayer": ["video"]`）：宿主只解码，帧交给引擎画进页面
    /// （`mr_video_frame_*`），滚动时与页面在同一张图里走，不会像盖上去的控件那样错开。
    /// 播放命令发 `videoCommand` 事件、页面上有哪些视频发 `videos` 事件；与 `nativeComponents`
    /// 同时列了的以这里为准。`video` 见 `components::video::host_feed`；别的标签（`map` …）走通用的
    /// `components::host_view`：宿主按 `sameLayerViews` 事件把控件画进离屏缓冲、帧交给引擎（`mr_native_view_frame_rgba`）
    pub same_layer: Vec<String>,
    /// 引擎在 `navigationStyle: custom` 的页面上画右上角胶囊（`"menuButton": true`），
    /// 点击发 `menuButton` 事件。默认关；普通页面的胶囊在宿主的导航栏里（见 `capsule.rs`）
    pub menu_button: bool,
    /// 引擎在普通页面（非 custom）顶上画默认导航栏（`"navigationBar": true`，见 `navbar.rs`）：
    /// 宿主把引擎画面铺满整屏（顶到状态栏），不再自己画导航栏。默认关（桌面窗体与老宿主不变）
    pub navigation_bar: bool,
    /// 页面转场动画（`"routeAnimation": true`）：navigateTo 新页从右边滑入、返回时当前页向右
    /// 滑出，300ms（见 `route_anim.rs`）。默认关：无头测试与老宿主照旧「一换就到」
    pub route_animation: bool,
}

impl EngineOptions {
    /// 解析 `mr_app_create_ex` 的 JSON。空串 / `null` 等于全默认；格式错返回 Err。
    pub fn from_json(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if text.is_empty() || text == "null" {
            return Ok(Self::default());
        }
        let v: Value = serde_json::from_str(text).map_err(|e| format!("选项 JSON 解析失败: {e}"))?;
        let o = v.as_object().ok_or("选项必须是 JSON 对象")?;
        let strings = |k: &str| -> Vec<String> {
            o.get(k)
                .and_then(|a| a.as_array())
                .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
                .unwrap_or_default()
        };
        Ok(Self {
            system_info: o.get("systemInfo").and_then(|s| s.as_object()).cloned().unwrap_or_default(),
            safe_area: o.get("safeArea").and_then(SafeAreaInsets::from_json),
            host_apis: strings("hostApis"),
            native_components: strings("nativeComponents"),
            same_layer: strings("sameLayer"),
            menu_button: o.get("menuButton").and_then(|v| v.as_bool()).unwrap_or(false),
            navigation_bar: o.get("navigationBar").and_then(|v| v.as_bool()).unwrap_or(false),
            route_animation: o.get("routeAnimation").and_then(|v| v.as_bool()).unwrap_or(false),
        })
    }

    /// 状态栏高度（逻辑像素）：宿主给了安全区取 `top`，否则按无刘海机型的 20
    pub fn status_bar_height(&self) -> f32 {
        self.safe_area.map(|s| s.top).unwrap_or(20.0)
    }

    /// `<video>` 走同层渲染（宿主解码、引擎画）
    pub fn same_layer_video(&self) -> bool {
        self.same_layer.iter().any(|t| t == "video")
    }

    /// 走通用同层渲染的标签（`sameLayer` 去掉 video：video 有自己的播放器约定）
    pub fn same_layer_views(&self) -> Vec<String> {
        self.same_layer.iter().filter(|t| t.as_str() != "video").cloned().collect()
    }

    /// 真正由宿主盖控件的标签：`nativeComponents` 去掉同层渲染的那几个
    pub fn overlay_components(&self) -> Vec<String> {
        self.native_components.iter().filter(|t| !self.same_layer.contains(t)).cloned().collect()
    }

    /// 宿主报的平台（`systemInfo.platform`，缺省 `devtools`）
    pub fn platform(&self) -> &str {
        self.system_info.get("platform").and_then(|v| v.as_str()).unwrap_or("devtools")
    }
}

/// 交给逻辑层的系统信息（`__hostSystemInfo`，`device.js` 合并进 `getSystemInfoSync`）
pub fn system_info(width: u32, height: u32, dpr: f32, opts: &EngineOptions) -> Value {
    let (w, h) = (width as f32, height as f32);
    let (status_bar, insets) = match opts.safe_area {
        Some(s) => (s.top, s),
        // 无刘海机型：状态栏 20，安全区从状态栏下沿开始（与从前写死的那份一致）
        None => (20.0, SafeAreaInsets { top: 20.0, ..Default::default() }),
    };
    let mut info = json!({
        "screenWidth": w,
        "screenHeight": h,
        "windowWidth": w,
        "windowHeight": h,
        "pixelRatio": dpr,
        "devicePixelRatio": dpr,
        "statusBarHeight": status_bar,
        "safeArea": {
            "left": insets.left,
            "right": w - insets.right,
            "top": insets.top,
            "bottom": h - insets.bottom,
            "width": (w - insets.left - insets.right).max(0.0),
            "height": (h - insets.top - insets.bottom).max(0.0),
        },
        "safeAreaInsets": {
            "top": insets.top, "right": insets.right, "bottom": insets.bottom, "left": insets.left,
        },
        "deviceOrientation": if w > h { "landscape" } else { "portrait" },
    });
    if let Some(obj) = info.as_object_mut() {
        for (k, v) in &opts.system_info {
            obj.insert(k.clone(), v.clone());
        }
    }
    info
}

/// 把系统信息装进逻辑层（`app.js` 执行之前调一次，尺寸 / 安全区变了再调）
pub fn install_system_info(app: &crate::runtime::MiniApp, info: &Value) {
    // `globalThis.`：eval 跑在严格模式下，给未声明的全局赋值会抛 ReferenceError（静默失败过一次）
    if let Err(e) = app.eval(&format!("globalThis.__hostSystemInfo = {info};")) {
        eprintln!("⚠️  安装系统信息失败: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 默认尺寸下与从前写死的那份一致() {
        let v = system_info(375, 667, 2.0, &EngineOptions::default());
        assert_eq!(v["windowWidth"], json!(375.0));
        assert_eq!(v["windowHeight"], json!(667.0));
        assert_eq!(v["pixelRatio"], json!(2.0));
        assert_eq!(v["statusBarHeight"], json!(20.0));
        assert_eq!(v["safeArea"]["top"], json!(20.0));
        assert_eq!(v["safeArea"]["bottom"], json!(667.0));
        assert_eq!(v["safeArea"]["height"], json!(647.0));
        assert_eq!(v["safeAreaInsets"]["top"], json!(20.0));
        assert_eq!(v["deviceOrientation"], json!("portrait"));
    }

    #[test]
    fn 宿主给的安全区与字段生效() {
        let o = EngineOptions::from_json(
            r#"{"safeArea":{"top":47,"bottom":34},"systemInfo":{"platform":"ios","model":"iPhone 15"},
                "hostApis":["chooseImage"],"nativeComponents":["video","map"],"menuButton":true}"#,
        )
        .unwrap();
        assert!(o.menu_button);
        let d = EngineOptions::from_json("{}").unwrap();
        assert!(!d.menu_button, "胶囊默认不画");
        assert!(!d.navigation_bar && !d.route_animation, "导航栏与转场默认关");
        assert_eq!((o.status_bar_height(), o.platform()), (47.0, "ios"));
        assert_eq!((d.status_bar_height(), d.platform()), (20.0, "devtools"));
        let n = EngineOptions::from_json(r#"{"navigationBar":true,"routeAnimation":true}"#).unwrap();
        assert!(n.navigation_bar && n.route_animation);
        let v = system_info(393, 852, 3.0, &o);
        assert_eq!(v["statusBarHeight"], json!(47.0));
        assert_eq!(v["safeArea"]["bottom"], json!(818.0));
        assert_eq!(v["safeArea"]["height"], json!(771.0));
        assert_eq!(v["platform"], json!("ios"));
        assert_eq!(o.host_apis, vec!["chooseImage".to_string()]);
        assert_eq!(o.native_components, vec!["video".to_string(), "map".to_string()]);
    }

    #[test]
    fn 坏选项报错_空选项全默认() {
        assert!(EngineOptions::from_json("[1]").is_err());
        assert!(EngineOptions::from_json("{").is_err());
        assert!(EngineOptions::from_json("").unwrap().host_apis.is_empty());
        assert!(EngineOptions::from_json("null").unwrap().safe_area.is_none());
    }
}
