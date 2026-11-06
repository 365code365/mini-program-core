//! 「关于小程序」：胶囊左半「•••」点开后要展示的那几项（名字、AppID、简介、入口页、页数、引擎版本）。
//!
//! 微信里这里是「关于 XX」页（名称、主体、简介、服务类目…），那些大多是平台侧的数据，引擎拿不到；
//! 能从小程序包里拿到的只有 `project.config.json` 与 `app.json`。三端共用这一份：
//! - 桌面窗体：点「•••」弹一个 Modal 展示（演示用）；
//! - SDK：启动后发一条 `{type:"app", about}` 事件，宿主自己决定怎么展示（Flutter / iOS / Android 的默认实现是弹框）；
//! - H5 导出：嵌进每个页面（`window.__APP_ABOUT__`），点「•••」弹 `wx.showModal`。
use serde_json::{json, Value};
use std::path::Path;

fn project_config(root: &Path) -> Value {
    std::fs::read_to_string(root.join("project.config.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null)
}

fn text(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(str::trim).filter(|x| !x.is_empty()).map(str::to_string)
}

/// 小程序的 AppID（`project.config.json` 的 `appid`）。没有、或是开发者工具「游客模式」的
/// 占位 `touristappid` 时是空串。`wx.getAccountInfoSync()` 与关于信息共用这一份
pub fn project_app_id(root: &Path) -> String {
    text(&project_config(root), "appid").filter(|a| a != "touristappid").unwrap_or_default()
}

/// 读小程序根目录下的 `project.config.json` + 已解析的 app.json，拼出关于信息
pub fn about(root: &Path, app_json: &Value) -> Value {
    let project = project_config(root);
    let mut pages: Vec<String> = string_list(app_json.get("pages"));
    // 分包里的页面也算（`subPackages` / 旧写法 `subpackages`，页面路径相对分包 root）
    for key in ["subPackages", "subpackages"] {
        for sp in app_json.get(key).and_then(|v| v.as_array()).into_iter().flatten() {
            let root = sp.get("root").and_then(|r| r.as_str()).unwrap_or("").trim_end_matches('/');
            for p in string_list(sp.get("pages")) {
                pages.push(if root.is_empty() { p } else { format!("{root}/{p}") });
            }
        }
    }
    let name = text(&project, "projectname")
        .or_else(|| app_json.get("window").and_then(|w| text(w, "navigationBarTitleText")))
        .or_else(|| root.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_default();
    // 开发者工具新建项目时写进去的默认 description（「项目配置文件，详见文档：…」）不是小程序简介
    let description = text(&project, "description").filter(|d| !d.starts_with("项目配置文件")).unwrap_or_default();
    let app_id = project_app_id(root);
    json!({
        "name": name,
        "appId": app_id,
        "description": description,
        "entryPage": app_json.get("entryPagePath").and_then(|v| v.as_str()).map(str::to_string)
            .or_else(|| pages.first().cloned()).unwrap_or_default(),
        "pageCount": pages.len(),
        "engine": format!("mini-render {}", env!("CARGO_PKG_VERSION")),
    })
}

fn string_list(v: Option<&Value>) -> Vec<String> {
    v.and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// 演示用的弹框文案：（标题，正文）。桌面窗体的 Modal 与 H5 的 `wx.showModal` 用同一份
pub fn dialog_text(about: &Value) -> (String, String) {
    let s = |k: &str| about.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let name = s("name");
    let mut lines = Vec::new();
    if !s("description").is_empty() {
        lines.push(s("description"));
    }
    lines.push(format!("AppID：{}", if s("appId").is_empty() { "未设置".to_string() } else { s("appId") }));
    let pages = about.get("pageCount").and_then(|v| v.as_u64()).unwrap_or(0);
    lines.push(format!("入口：{}（共 {pages} 页）", s("entryPage")));
    lines.push(format!("引擎：{}", s("engine")));
    (if name.is_empty() { "关于小程序".to_string() } else { format!("关于{name}") }, lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str, project: Option<&str>) -> std::path::PathBuf {
        let d = std::env::temp_dir().join("mini-render-about-test").join(name);
        std::fs::remove_dir_all(&d).ok();
        std::fs::create_dir_all(&d).unwrap();
        if let Some(p) = project {
            std::fs::write(d.join("project.config.json"), p).unwrap();
        }
        d
    }

    #[test]
    fn 名字与_appid_取_project_config_分包页面也算() {
        let d = dir("full", Some(r#"{"projectname":"云茶园","appid":"wx123","description":"一杯好茶"}"#));
        let app = json!({"pages":["pages/a/a","pages/b/b"],"subPackages":[{"root":"pkg/","pages":["c/c"]}]});
        let a = about(&d, &app);
        assert_eq!(a["name"], "云茶园");
        assert_eq!(a["appId"], "wx123");
        assert_eq!(a["description"], "一杯好茶");
        assert_eq!(a["entryPage"], "pages/a/a");
        assert_eq!(a["pageCount"], 3);
        let (title, body) = dialog_text(&a);
        assert_eq!(title, "关于云茶园");
        assert!(body.starts_with("一杯好茶\nAppID：wx123\n入口：pages/a/a（共 3 页）"), "{body}");
    }

    #[test]
    fn 缺省值_工具默认描述与游客_appid_不算() {
        let d = dir("tool-default", Some(r#"{"appid":"touristappid","description":"项目配置文件，详见文档：https://…"}"#));
        let a = about(&d, &json!({"pages":["p/i"],"window":{"navigationBarTitleText":"标题当名字"}}));
        assert_eq!(a["name"], "标题当名字");
        assert_eq!(a["appId"], "");
        assert_eq!(a["description"], "");
        assert!(dialog_text(&a).1.starts_with("AppID：未设置"));
        // 连 project.config.json 都没有：名字落到目录名
        let d = dir("bare-dir", None);
        assert_eq!(about(&d, &json!({"pages":[]}))["name"], "bare-dir");
    }
}
