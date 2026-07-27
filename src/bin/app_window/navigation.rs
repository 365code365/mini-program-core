//! 页面导航逻辑

use std::collections::HashMap;
use mini_render::parser::wxml::WxmlNode;
use mini_render::parser::wxss::StyleSheet;
use mini_render::parser::template::ComponentTemplates;
use mini_render::using_components::ComponentSource;

/// 页面信息
pub struct PageInfo {
    pub path: String,
    pub wxml: String,
    pub wxss: String,
    pub js: String,
    /// 页面 json 的 `enablePullDownRefresh`（缺省继承 app.json 的 `window` 配置）。
    /// 微信里下拉回弹一直有，但只有开了这个开关才出现刷新指示器并回调
    /// `onPullDownRefresh`，所以两者要分开处理。
    pub enable_pull_down_refresh: bool,
    /// 页面 json `usingComponents` 声明的自定义组件（三件套源码）
    pub components: Vec<ComponentSource>,
}

/// 页面栈中的页面实例
pub struct PageInstance {
    pub path: String,
    pub query: HashMap<String, String>,
    pub wxml_nodes: Vec<WxmlNode>,
    pub stylesheet: StyleSheet,
    /// 自定义组件模板（标签名 → 组件 WXML），渲染时按标签展开
    pub component_templates: ComponentTemplates,
}

/// 导航请求类型
#[derive(Clone)]
pub enum NavigationRequest {
    NavigateTo { url: String },
    NavigateBack,
    SwitchTab { url: String },
    /// 关闭当前页再打开目标页（页面栈深度不变）
    RedirectTo { url: String },
    /// 关闭所有页面再打开目标页
    ReLaunch { url: String },
}

/// 解析 URL，返回路径和查询参数
pub fn parse_url(url: &str) -> (String, HashMap<String, String>) {
    let url = url.trim_start_matches('/');
    let mut query = HashMap::new();
    let (path, query_str) = if let Some(pos) = url.find('?') {
        (&url[..pos], Some(&url[pos+1..]))
    } else {
        (url, None)
    };
    if let Some(qs) = query_str {
        for pair in qs.split('&') {
            if let Some(eq_pos) = pair.find('=') {
                let key = &pair[..eq_pos];
                let value = &pair[eq_pos+1..];
                query.insert(key.to_string(), value.to_string());
            }
        }
    }
    (path.to_string(), query)
}

/// 原样返回页面节点。
///
/// 这里曾按「class 含 tabbar」直接删节点，用来兜自定义 tabBar 被写进页面的情况。
/// 但编译到 H5 时并不会做这种删除，页面里任何带 `tabbar` 字样的类名（比如
/// `.tabbar-tip`）都会在原生端凭空消失 —— 两端不一致的隐患，故不再过滤。
pub fn remove_manual_tabbar(nodes: &[WxmlNode]) -> Vec<WxmlNode> {
    return nodes.to_vec();
    #[allow(unreachable_code)]
    {
    use mini_render::parser::wxml::WxmlNodeType;
    
    fn filter_node(node: &WxmlNode) -> Option<WxmlNode> {
        if node.node_type != WxmlNodeType::Element {
            return Some(node.clone());
        }
        
        let class = node.attributes.get("class").map(|s| s.as_str()).unwrap_or("");
        if class.contains("tabbar") {
            return None;
        }
        
        let mut new_node = WxmlNode::new_element(&node.tag_name);
        new_node.attributes = node.attributes.clone();
        new_node.children = node.children.iter()
            .filter_map(|c| filter_node(c))
            .collect();
        Some(new_node)
    }
    
    nodes.iter().filter_map(|n| filter_node(n)).collect()
}
}
