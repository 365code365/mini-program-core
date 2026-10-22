//! 页面导航逻辑

use std::collections::HashMap;
use crate::parser::wxml::WxmlNode;
use crate::parser::wxss::StyleSheet;
use crate::parser::template::ComponentTemplates;
use crate::using_components::ComponentSource;

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
    /// 页面 json 原样（`navigationBarTitleText` 等窗口配置）。宿主画导航栏要用，
    /// 与 app.json 的 `window` 合并后放进页面事件。没有页面 json 时是 `Null`。
    pub window: serde_json::Value,
}

/// 页面栈中的页面实例
pub struct PageInstance {
    pub path: String,
    pub query: HashMap<String, String>,
    pub wxml_nodes: Vec<WxmlNode>,
    pub stylesheet: StyleSheet,
    /// 自定义组件模板（标签名 → 组件 WXML），渲染时按标签展开
    pub component_templates: ComponentTemplates,
    /// 被新页盖住那一刻的页面滚动（位置, 内容高）。返回时按它恢复（微信里上一页的实例一直活着，
    /// 滚到哪就停在哪）。从前返回一律回到顶部
    pub saved_scroll: Option<(f32, f32)>,
}

impl PageInstance {
    /// 返回到这一页时的滚动控制器：有存过的位置就恢复（内容高先按存下的给，`max_scroll` 才不会把位置夹成 0；
    /// 第一帧渲染出真实内容高后照常更新），否则从顶上开始
    pub fn restored_scroll(&self, viewport: f32) -> crate::ui::ScrollController {
        match self.saved_scroll {
            Some((pos, content)) => {
                let mut s = crate::ui::ScrollController::new(content.max(viewport), viewport);
                s.set_position(pos);
                s
            }
            None => crate::ui::ScrollController::new(viewport, viewport),
        }
    }
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

/// 取走「`wx.navigateBack` 已经替宿主出过几层逻辑层的栈」，并清零。
///
/// 宿主每次处理返回都先调它（哪怕自己的栈只剩一页、什么都不做，也要取走，免得留到下一次）：
/// - 大于 0：这次返回是 `wx.navigateBack({ delta })` 发起的，逻辑层已经出过栈并派发了
///   onUnload，宿主只退自己的栈这么多层，**不要**再 [`pop_logic_page`]；
/// - 等于 0：宿主自己发起的（系统返回键、侧滑），宿主退一层，再 [`pop_logic_page`]。
///
/// 从前 SDK 不分这两种，一律再 `__popPage(1)`：从详情页 `wx.navigateBack()` 回来，逻辑层
/// 连退两层，回到的页面拿着**上上一页**的 data 渲染 —— 模板里的值全空、点什么都没反应
/// （方法也是上上一页的）。桌面窗体按「当前路由对不对得上」来防重复出栈，同一页面连开
/// 两层时会漏；而两边都拿出栈之后的 `__currentPage` 派发 onUnload，派给了要回去的那一页。
///
/// 不按两边栈的层数对齐：桌面 `--route` 启动时逻辑层多压了一个首页，层数本来就不相等。
pub fn take_logic_popped(app: &crate::runtime::MiniApp) -> usize {
    app.eval("typeof __takeLogicPopped==='function'?__takeLogicPopped():0")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// 宿主发起的返回：逻辑层的当前页派发 onUnload，再出一层（复用上一页的实例，不重载它的 js）。
/// 逻辑层只剩一页时不动（`__popPage` 本来就不出最后一页，onUnload 也不该派给留下的那页）
pub fn pop_logic_page(app: &crate::runtime::MiniApp) {
    app.eval("if(__pageStack.length>1){if(__currentPage&&__currentPage.onUnload)__currentPage.onUnload();__popPage(1);}")
        .ok();
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
    use crate::parser::wxml::WxmlNodeType;
    
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
