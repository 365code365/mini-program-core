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
        })
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
                "hostApis":["chooseImage"],"nativeComponents":["video","map"]}"#,
        )
        .unwrap();
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
