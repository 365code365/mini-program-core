//! `<image>` 建树时处理的两个属性（官方 component/image）：
//!
//! - `forceHttps`（WebView 特有，3.9.1）：自动把 http 链接换成 https。改的是节点上的 `src`，
//!   之后的加载、bindload / binderror、动图续帧都按换过的地址走；
//! - `show-menu-by-longpress`（默认 false）：长按图片弹菜单（发送给朋友 / 收藏 / 保存图片 …）。
//!   挂一条 `longpress` 绑定，处理函数名 `__imgmenu:<src>`：逻辑层 `__dispatchHandler` 认这个前缀，
//!   交给 media.js 的 `__imageMenu`（宿主在 hostApis 里接了 `image.showMenu` 就交宿主出原生菜单，
//!   没接就弹引擎的 actionSheet）。派发成功算「长按被处理」，这次触摸不再发 tap（与微信一样）。
use super::super::base::*;
use crate::parser::wxml::WxmlNode;
use std::collections::HashMap;

/// 长按菜单的处理函数名前缀（js/prelude/page-components.js 的 `__dispatchHandler` 认它）
pub const MENU_HANDLER: &str = "__imgmenu:";

/// 生效的 src（forceHttps 时 http:// → https://，并写回 attrs）
pub(super) fn effective_src(node: &WxmlNode, attrs: &mut HashMap<String, String>) -> String {
    let src = node.get_attr("src").unwrap_or("").to_string();
    let force = node.get_attr("forceHttps").or_else(|| node.get_attr("force-https")).is_some_and(attr_is_true);
    match src.strip_prefix("http://") {
        Some(rest) if force => {
            let s = format!("https://{rest}");
            attrs.insert("src".into(), s.clone());
            s
        }
        _ => src,
    }
}

/// show-menu-by-longpress：在节点的绑定后面加一条 longpress（数据集与节点上的 `data-*` 相同）
pub(super) fn longpress_menu(node: &WxmlNode, src: &str, events: &mut Vec<EventBind>) {
    if src.is_empty() || !bool_attr_of(node, "show-menu-by-longpress") {
        return;
    }
    let data = node
        .attributes
        .iter()
        .filter_map(|(k, v)| k.strip_prefix("data-").map(|n| (dataset_key(n), v.clone())))
        .collect();
    let owner = node.get_attr(crate::parser::template::COMPONENT_OWNER_ATTR).unwrap_or("").to_string();
    events.push(EventBind {
        event_type: "longpress".into(),
        handler: format!("{MENU_HANDLER}{src}"),
        data,
        is_catch: false,
        phase: EventPhase::Bubble,
        mut_bind: false,
        owner,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(attrs: &[(&str, &str)]) -> WxmlNode {
        let mut n = crate::parser::wxml::WxmlParser::new("<image/>").parse().unwrap().remove(0);
        for (k, v) in attrs {
            n.attributes.insert(k.to_string(), v.to_string());
        }
        n
    }

    #[test]
    fn force_https_只换_http() {
        let mut a = HashMap::new();
        assert_eq!(effective_src(&node(&[("src", "http://a.com/x.png"), ("forceHttps", "true")]), &mut a), "https://a.com/x.png");
        assert_eq!(a["src"], "https://a.com/x.png", "节点上的 src 也换了（加载与事件按它走）");
        let mut b = HashMap::new();
        assert_eq!(effective_src(&node(&[("src", "http://a.com/x.png")]), &mut b), "http://a.com/x.png", "默认 false 不换");
        assert_eq!(effective_src(&node(&[("src", "/img/a.png"), ("forceHttps", "true")]), &mut b), "/img/a.png");
    }

    #[test]
    fn 长按菜单只在开了属性时挂() {
        let mut ev = Vec::new();
        longpress_menu(&node(&[("src", "/a.png")]), "/a.png", &mut ev);
        assert!(ev.is_empty());
        longpress_menu(&node(&[("src", "/a.png"), ("show-menu-by-longpress", "true"), ("data-id", "7")]), "/a.png", &mut ev);
        assert_eq!((ev[0].event_type.as_str(), ev[0].handler.as_str()), ("longpress", "__imgmenu:/a.png"));
        assert_eq!(ev[0].data.get("id").map(|s| s.as_str()), Some("7"));
    }
}
