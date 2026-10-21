//! 页面内自定义组件的挂载与对账：执行组件 js → **每处使用一个实例** → 组件 data 进入渲染作用域。
//!
//! 光把组件模板展开是不够的：模板里的 `{{a}}` 来自**组件实例自己的 data**，
//! 而框架型产物（uni-app / Taro）是在组件的 `attached` 里挂载自己的组件树并触发
//! 首次 `setData` 的。不建实例的话组件模板里全是空值 ——
//! 表现就是「底部导航整条空白，图标和文字都没有」。
//!
//! 实例跟着「这一帧的节点树里有哪些组件」走：模板展开把每处组件的实例键、带类型的属性、
//! 宿主上的事件绑定列出来（`TemplateEngine::collect_components`），交给逻辑层
//! `__reconcileComponents` 建实例 / 改属性（触发 observer）/ detached。
//! [`render_data`] 是宿主取渲染数据的**唯一入口**，SDK 引擎与桌面窗体都走它 ——
//! 两边各写一份的话迟早一边忘了对账（静态截图还一模一样，没人发现）。

use crate::parser::template::{ComponentTemplates, TemplateEngine};
use crate::parser::wxml::{WxmlNode, WxmlParser};
use crate::runtime::MiniApp;
use crate::using_components::ComponentSource;
use serde_json::Value as JsonValue;

/// 对账轮数上限。每一轮只能多看见「刚建出来的组件」里的子组件（它们要等外层有了 data
/// 才展开得出来），所以 N 层嵌套要 N+1 轮；超过这个数视为不收敛（多半是 observer 里
/// setData 又改了属性来源，来回震荡），按最后一轮的数据渲染并打日志。
const MAX_ROUNDS: usize = 8;

/// 挂载页面声明的全部自定义组件：执行组件 js、登记「标签 → 组件路径」，然后立刻对账一次，
/// 让实例（含嵌套组件、属性已到位）在**首帧之前**就建好 —— 组件不会先空一帧。
pub fn mount_page_components(app: &mut MiniApp, comps: &[ComponentSource], page_nodes: &[WxmlNode]) {
    let mut templates = ComponentTemplates::new();
    for c in comps {
        // 以「组件路径」为注册键执行组件 js：`Component({...})` 会登记到该路径。
        // 这个标记同时告诉 `Component()`「这是组件不是页面」，
        // 否则组件 js 会被当成页面注册，把当前页面顶掉。
        app.eval(&format!("__setPendingComponentPath({})", js_str(&c.module_path))).ok();
        let loaded = app.load_module_script(&c.module_path, &c.js);
        app.eval("__setPendingComponentPath('')").ok();
        if let Err(e) = loaded {
            eprintln!("⚠️  组件 {} 的 js 执行失败: {}", c.tag, e);
            continue;
        }
        app.eval(&format!("__registerPageComponent({}, {})", js_str(&c.tag), js_str(&c.module_path))).ok();
        // 与渲染器里那份同源同解析器：实例键按结构位置算，两边的树必须一模一样
        if let Ok(nodes) = WxmlParser::new(&c.wxml).parse() {
            templates.insert(c.tag.clone(), nodes);
        }
    }
    let _ = render_data(app, page_nodes, &templates);
}

/// 取渲染数据（页面 data + 各组件实例 data），顺带把组件实例对齐到这一帧的节点树。
///
/// 没有组件的页面就是一次 `__getRenderData()`，与从前一字不差。有组件时：
/// 收集出现位置 → 对账 → 有变化（建了实例、属性变了、有实例被回收）就重取数据再来一轮，
/// 直到稳定。属性没变的那一轮不会触发任何 observer，也不会让数据变脏。
pub fn render_data(app: &MiniApp, nodes: &[WxmlNode], templates: &ComponentTemplates) -> JsonValue {
    let mut data = fetch(app);
    if templates.is_empty() {
        return data;
    }
    for round in 0..MAX_ROUNDS {
        let list = TemplateEngine::collect_components(nodes, &data, templates);
        let payload = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into());
        // JSON 本身就是合法的 JS 字面量，直接嵌进去，省一次字符串转义 + JSON.parse
        let changed = app
            .eval(&format!("__reconcileComponents({{\"list\":{payload}}})"))
            .map(|s| s.trim().trim_matches('"') == "1")
            .unwrap_or(false);
        if !changed {
            // 对账里的 setData（新实例的 attached、属性 observer）已经反映在刚取的数据里了，
            // 把它们留下的「脏」标记吃掉，免得下一帧再白做一趟取数据 + 对账
            if round > 0 {
                app.take_data_dirty();
            }
            return data;
        }
        // 框架的渲染多在微任务里（Vue 的调度器）：泵完再取，组件这一帧就是新内容
        app.pump_jobs();
        data = fetch(app);
    }
    eprintln!("⚠️  组件对账 {MAX_ROUNDS} 轮仍未稳定（属性来回变化？），按最后一轮的数据渲染");
    data
}

fn fetch(app: &MiniApp) -> JsonValue {
    app.eval("__getRenderData()")
        .map(|s| serde_json::from_str(&s).unwrap_or(serde_json::json!({})))
        .unwrap_or(serde_json::json!({}))
}

fn js_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "''".to_string())
}

#[cfg(test)]
mod tests {
    use crate::parser::template::{camel_case, is_property_attr};

    /// 属性名规则从这里搬去了模板引擎（展开时就要用），规则本身不变
    #[test]
    fn dashed_attrs_become_camel_case() {
        assert_eq!(camel_case("u-p"), "uP");
        assert_eq!(camel_case("my-long-name"), "myLongName");
        assert_eq!(camel_case("plain"), "plain");
    }

    #[test]
    fn property_attr_rules() {
        for k in ["u-p", "u-i", "title", "count"] {
            assert!(is_property_attr(k), "{k} 是属性");
        }
        for k in ["class", "style", "id", "hidden", "eS", "eA", "slot", "bind:__l", "bindtap", "catch:x", "data-id", "wx:if", "change:eS", "capture-bindtap", "mut-bindtap"] {
            assert!(!is_property_attr(k), "{k} 不是属性");
        }
    }
}
