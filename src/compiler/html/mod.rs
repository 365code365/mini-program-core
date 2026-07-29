//! HTML 目标编译器
//!
//! 把小程序编译成结构化、可在浏览器直接运行的 HTML 工程：
//!
//! ```text
//! dist-html/
//!   index.html                启动页（手机外壳 + 页面列表 + iframe 预览）
//!   common/  base.css · app.css · app.js · runtime.js
//!   assets/  静态资源
//!   pages/<name>/  index.html · <name>.css · <name>.js
//! ```
//!
//! 每个页面内嵌 WXML AST + 页面逻辑，配合 `runtime.js` 实现**响应式**：
//! `setData` 会按模板重新渲染 DOM，支持事件绑定、`model:` 双向数据绑定、`wx.*` 跳转等。

mod transpile;
mod runtime;

pub use transpile::{
    base_css, convert_rpx, make_html_doc, wxml_to_html, wxml_to_html_pretty, wxss_to_css,
};
pub use runtime::RUNTIME_JS;

use crate::compiler::{AppSource, CompileTarget, EmittedFile};
use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use serde_json::{json, Value as JsonValue};
use std::path::Path;

const WIDTH: u32 = 375;

/// HTML 工程编译目标
pub struct HtmlTarget;

impl HtmlTarget {
    pub fn new() -> Self { HtmlTarget }
}

impl Default for HtmlTarget {
    fn default() -> Self { Self::new() }
}

impl CompileTarget for HtmlTarget {
    fn name(&self) -> &str { "html" }

    fn compile(&self, app: &AppSource) -> Result<Vec<EmittedFile>, String> {
        let mut files: Vec<EmittedFile> = Vec::new();

        // ── 公共资源 ──
        files.push(EmittedFile::text("common/base.css", base_css()));
        files.push(EmittedFile::text(
            "common/app.css",
            format!("/* 由 app.wxss 编译 */\n{}", wxss_to_css(&app.app_wxss)),
        ));
        files.push(EmittedFile::text("common/app.js", app.app_js.clone()));
        // 内置图标表（编译期从 icon_data 那张 WeUI 字形表生成）拼在运行时前面，
        // 保证运行时动态创建的 <icon> 与编译期静态输出的完全一样。
        files.push(EmittedFile::text(
            "common/runtime.js",
            format!(
                "{}\n{}",
                crate::renderer::components::icon_data::icon_svg_js_table(),
                RUNTIME_JS
            ),
        ));

        // ── 静态资源 ──
        collect_assets(Path::new(&app.root).join("assets").as_path(), "assets", &mut files);

        // ── tabBar 配置（导出为固定底部导航栏，各 tab 页面均注入）──
        let tb = app.config.get("tabBar");
        let tab_list = app.tab_list();
        let tab_color = tb.and_then(|t| t.get("color")).and_then(|c| c.as_str()).unwrap_or("#999999");
        let tab_sel = tb.and_then(|t| t.get("selectedColor")).and_then(|c| c.as_str()).unwrap_or("#FF6B35");
        let tab_bg = tb.and_then(|t| t.get("backgroundColor")).and_then(|c| c.as_str()).unwrap_or("#ffffff");
        // custom:true 时使用开发者的 custom-tab-bar 组件（微信语义），其样式单独产出
        let use_custom_tab_bar = app.uses_custom_tab_bar();
        if use_custom_tab_bar {
            if let Some(bar) = &app.custom_tab_bar {
                files.push(EmittedFile::text(
                    "common/tabbar.css",
                    format!(
                        "/* 由 custom-tab-bar/index.wxss 编译 */\n{}\n\
                         /* 自定义 tabBar 固定在视口底部（对齐 375 宽） */\n\
                         .wx-custom-tabbar{{position:fixed;left:50%;transform:translateX(-50%);\
                         bottom:0;width:{w}px;max-width:100%;z-index:500;}}\n",
                        wxss_to_css(&bar.wxss),
                        w = WIDTH
                    ),
                ));
            }
        }

        // ── 逐页面 ──
        let mut nav_items = String::new();
        for page in &app.pages {
            let (dir, leaf) = page.route.rsplit_once('/').unwrap_or(("", page.route.as_str()));
            let depth = if dir.is_empty() { 0 } else { dir.split('/').count() };
            let rel = "../".repeat(depth);

            let page_css = wxss_to_css(&page.wxss);
            files.push(EmittedFile::text(
                format!("{}/{}.css", dir, leaf),
                format!("/* 由 {}.wxss 编译 */\n{}", leaf, page_css),
            ));
            let has_js = !page.js.is_empty();
            if has_js {
                files.push(EmittedFile::text(format!("{}/{}.js", dir, leaf), page.js.clone()));
            }

            // tabBar：当前页在 list 中则注入底部导航
            let tab_index = tab_list.iter().position(|(p, _)| p == &page.route);
            let is_tab = tab_index.is_some();
            let tabbar_html = match (is_tab, use_custom_tab_bar, &app.custom_tab_bar) {
                // 自定义 tabBar：按当前页设置 selected，渲染组件自身 WXML
                (true, true, Some(bar)) => {
                    build_custom_tabbar(bar, tab_index.unwrap_or(0), &rel)
                }
                (true, _, _) => build_tabbar(&tab_list, &page.route, &rel, tab_color, tab_sel, tab_bg),
                _ => String::new(),
            };

            // 静态首屏（无 JS 时的回退视图）
            let body = wxml_to_html_pretty(&page.wxml, &page.data);
            // 供 runtime 响应式重渲染的 WXML AST
            let ast = nodes_to_json(&page.wxml);
            let tabbar_css = is_tab && use_custom_tab_bar;
            let html = build_page_html(&page.route, leaf, &rel, &body, &ast, has_js, &tabbar_html, is_tab, tabbar_css, page.enable_pull_down_refresh);
            files.push(EmittedFile::text(format!("{}/{}.html", dir, leaf), html));

            nav_items.push_str(&format!(
                "<button class=\"nav-item\" data-src=\"{d}/{l}.html\">{p}</button>\n",
                d = dir, l = leaf, p = page.route
            ));
        }

        // ── 启动页 ──
        let first = app.pages.first().map(|p| p.route.clone()).unwrap_or_default();
        let (fdir, fleaf) = first.rsplit_once('/').unwrap_or(("", first.as_str()));
        let first_src = format!("{}/{}.html", fdir, fleaf);
        files.push(EmittedFile::text(
            "index.html",
            build_launcher(&nav_items, &first_src, app.pages.len()),
        ));

        Ok(files)
    }
}

/// 收集静态资源目录为 EmittedFile
fn collect_assets(src: &Path, prefix: &str, out: &mut Vec<EmittedFile>) {
    if !src.is_dir() { return; }
    let entries = match std::fs::read_dir(src) { Ok(e) => e, Err(_) => return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let rel = format!("{}/{}", prefix, name);
        if path.is_dir() {
            collect_assets(&path, &rel, out);
        } else if let Ok(bytes) = std::fs::read(&path) {
            out.push(EmittedFile::bytes(rel, bytes));
        }
    }
}

/// WXML 节点树 → 紧凑 JSON AST（供浏览器端运行时解释重渲染）
fn nodes_to_json(nodes: &[WxmlNode]) -> JsonValue {
    JsonValue::Array(nodes.iter().filter_map(node_to_json).collect())
}

fn node_to_json(node: &WxmlNode) -> Option<JsonValue> {
    match node.node_type {
        WxmlNodeType::Text => Some(json!({ "t": "tx", "x": node.text_content })),
        WxmlNodeType::Element => {
            let attrs: serde_json::Map<String, JsonValue> = node.attributes.iter()
                .map(|(k, v)| (k.clone(), JsonValue::String(v.clone())))
                .collect();
            let children: Vec<JsonValue> = node.children.iter().filter_map(node_to_json).collect();
            Some(json!({ "t": "el", "n": node.tag_name, "a": attrs, "c": children }))
        }
        WxmlNodeType::Comment => None,
    }
}

/// 自定义 tabBar：渲染 `custom-tab-bar` 组件的 WXML，`selected` 置为当前页下标。
///
/// 组件内的 `bindtap="switchTab"` 会被转成 `data-tap`，其点击行为由页面 runtime 处理；
/// 这里额外把每项的 `data-ds-path` 用作静态导航兜底（无 JS 时也能跳转）。
fn build_custom_tabbar(bar: &crate::compiler::TabBarSource, selected: usize, rel: &str) -> String {
    let mut data = bar.data.clone();
    if let Some(obj) = data.as_object_mut() {
        obj.insert("selected".to_string(), json!(selected));
    }
    let inner = wxml_to_html(&bar.wxml, &data);
    // 为每个 tab 项补一个编译期算好的可跳转地址（保留原 data-ds-path 供事件 dataset 使用）
    let inner = add_tabbar_href(&inner, rel);
    // 自带跳转脚本：tabBar 位于 #app 之外，其 bindtap 是「组件方法」，页面 runtime 只
    // 能分发页面方法，因此点击不会切页。这里用一段自包含的委托脚本完成 switchTab 语义，
    // 即使页面没有 JS 也能正常切换。
    format!(
        "<nav class=\"wx-custom-tabbar\">{inner}</nav>\n\
<script>\n\
(function(){{var bar=document.currentScript&&document.currentScript.previousElementSibling;\n\
 if(!bar||!bar.classList.contains('wx-custom-tabbar')){{bar=document.querySelector('.wx-custom-tabbar');}}\n\
 if(!bar)return;\n\
 bar.addEventListener('click',function(e){{\n\
  var el=e.target.closest?e.target.closest('[data-tabbar-href]'):null;\n\
  if(!el)return;\n\
  var href=el.getAttribute('data-tabbar-href');\n\
  if(href){{e.preventDefault();e.stopPropagation();window.location.href=href;}}\n\
 }},true);}})();\n\
</script>"
    )
}

/// 给带 `data-ds-path="<route>"` 的 tab 项追加 `data-tabbar-href="<rel><route>.html"`。
fn add_tabbar_href(html: &str, rel: &str) -> String {
    const KEY: &str = " data-ds-path=\"";
    let mut out = String::with_capacity(html.len() + 64);
    let mut rest = html;
    while let Some(pos) = rest.find(KEY) {
        let value_start = pos + KEY.len();
        let Some(end_offset) = rest[value_start..].find('"') else { break };
        let value_end = value_start + end_offset;
        let route = &rest[value_start..value_end];
        out.push_str(&rest[..value_end + 1]);
        if !route.is_empty() {
            out.push_str(&format!(
                " data-tabbar-href=\"{}{}.html\"",
                rel,
                route.trim_start_matches('/')
            ));
        }
        rest = &rest[value_end + 1..];
    }
    out.push_str(rest);
    out
}

/// tabBar 底部导航（固定，位于 #app 之外，不受 setData 重渲染影响）
fn build_tabbar(list: &[(String, String)], current: &str, rel: &str, color: &str, sel: &str, bg: &str) -> String {
    let mut items = String::new();
    for (i, (path, text)) in list.iter().enumerate() {
        let active = path == current;
        let c = if active { sel } else { color };
        items.push_str(&format!(
            "<a class=\"wx-tabbar-item\" href=\"{rel}{path}.html\" style=\"color:{c}\">{icon}<span class=\"wx-tabbar-text\">{text}</span></a>",
            rel = rel, path = path, c = c, icon = tabbar_icon(i, text), text = text
        ));
    }
    format!("<nav class=\"wx-tabbar\" style=\"background:{bg}\">{items}</nav>", bg = bg, items = items)
}

/// 按文本/序号选择 tab 图标（描边式 SVG，用 currentColor 跟随选中态着色）
fn tabbar_icon(idx: usize, text: &str) -> &'static str {
    let home = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M3 10.5 12 3l9 7.5\"/><path d=\"M5 9.5V21h14V9.5\"/></svg>";
    let grid = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linejoin=\"round\"><rect x=\"3\" y=\"3\" width=\"7\" height=\"7\" rx=\"1\"/><rect x=\"14\" y=\"3\" width=\"7\" height=\"7\" rx=\"1\"/><rect x=\"3\" y=\"14\" width=\"7\" height=\"7\" rx=\"1\"/><rect x=\"14\" y=\"14\" width=\"7\" height=\"7\" rx=\"1\"/></svg>";
    let cart = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><circle cx=\"9\" cy=\"20\" r=\"1.4\"/><circle cx=\"18\" cy=\"20\" r=\"1.4\"/><path d=\"M2 3h3l2.2 12h11l1.8-8H6\"/></svg>";
    let user = "<svg viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><circle cx=\"12\" cy=\"8\" r=\"4\"/><path d=\"M4 21c0-4 3.5-6.5 8-6.5s8 2.5 8 6.5\"/></svg>";
    if text.contains("首页") || text.contains("主页") { return home; }
    if text.contains("分类") || text.contains("类目") { return grid; }
    if text.contains("购物车") || text.contains("车") { return cart; }
    if text.contains("我") || text.contains("用户") || text.contains("个人") { return user; }
    match idx { 0 => home, 1 => grid, 2 => cart, _ => user }
}

/// 生成单个页面的 HTML（分离引用 base.css / app.css / <page>.css + 内嵌 AST + runtime + tabBar）
fn build_page_html(route: &str, leaf: &str, rel: &str, body: &str, ast: &JsonValue, has_js: bool, tabbar: &str, is_tab: bool, tabbar_css: bool, enable_pull_down: bool) -> String {
    let logic = if has_js {
        format!("<script src=\"./{}.js\"></script>\n", leaf)
    } else {
        String::new()
    };
    // tab 页给内容留出底部导航高度，避免被遮挡
    let pad = if is_tab { "padding-bottom:50px;" } else { "" };
    let tabbar_link = if tabbar_css {
        format!("  <link rel=\"stylesheet\" href=\"{rel}common/tabbar.css\">\n")
    } else {
        String::new()
    };
    format!(
"<!doctype html>
<html lang=\"zh\">
<head>
  <meta charset=\"utf-8\">
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
  <title>{route}</title>
  <link rel=\"stylesheet\" href=\"{rel}common/base.css\">
  <link rel=\"stylesheet\" href=\"{rel}common/app.css\">
  <link rel=\"stylesheet\" href=\"./{leaf}.css\">
{tabbar_link}  <style>
    #app{{width:{w}px;min-height:100vh;margin:0 auto;background:#f5f6f8;position:relative;overflow:hidden;{pad}}}
  </style>
</head>
<body>
<div id=\"app\">
{body}</div>
{tabbar}
<script>
window.__PAGE_ROUTE__ = {route_json};
window.__PAGE_PULL_DOWN__ = {pull_down};
window.__WXML__ = {ast};
</script>
<script src=\"{rel}common/runtime.js\"></script>
<script src=\"{rel}common/app.js\"></script>
{logic}</body>
</html>
",
        route = route, rel = rel, leaf = leaf, w = WIDTH, body = body, pad = pad, tabbar = tabbar,
        tabbar_link = tabbar_link,
        route_json = serde_json::to_string(route).unwrap(),
        pull_down = enable_pull_down,
        ast = serde_json::to_string(ast).unwrap(),
        logic = logic,
    )
}

/// 启动页：手机外壳 + 侧栏页面列表 + iframe 预览
fn build_launcher(nav_items: &str, first_src: &str, n: usize) -> String {
    format!(
"<!doctype html>
<html lang=\"zh\">
<head>
<meta charset=\"utf-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
<title>Mini App · HTML 工程</title>
<style>
  *{{box-sizing:border-box;}}
  body{{margin:0;font-family:-apple-system,system-ui,\"PingFang SC\",sans-serif;background:#1c1f26;color:#e8eaed;display:flex;min-height:100vh;}}
  aside{{width:260px;flex-shrink:0;padding:24px 18px;border-right:1px solid #2c313c;overflow:auto;}}
  aside h1{{font-size:16px;margin:0 0 4px;color:#9ecbff;}}
  aside p{{font-size:12px;color:#8b93a3;margin:0 0 18px;}}
  .nav-item{{display:block;width:100%;text-align:left;margin:4px 0;padding:9px 12px;border:0;border-radius:8px;background:#252a34;color:#cfd6e4;font-size:13px;cursor:pointer;transition:.15s;}}
  .nav-item:hover{{background:#2f3644;}}
  .nav-item.active{{background:#07c160;color:#fff;}}
  main{{flex:1;display:flex;flex-direction:column;align-items:center;justify-content:center;padding:32px;}}
  .phone{{width:390px;height:800px;background:#000;border-radius:44px;padding:12px;box-shadow:0 30px 80px rgba(0,0,0,.55);}}
  .phone iframe{{width:100%;height:100%;border:0;border-radius:32px;background:#fff;}}
  .hint{{margin-top:16px;font-size:12px;color:#6b7280;max-width:390px;text-align:center;line-height:1.6;}}
</style>
</head>
<body>
<aside>
  <h1>Mini App · HTML 工程</h1>
  <p>共 {n} 个页面 · 点击预览</p>
  {nav}
</aside>
<main>
  <div class=\"phone\"><iframe id=\"preview\" src=\"{first}\"></iframe></div>
  <div class=\"hint\">结构化 HTML 工程：common/ 公共资源 · pages/ 各页面(html·css·js) · assets/ 静态资源。<br>页面内支持事件、setData 响应式重渲染、model 双向绑定与 wx 跳转。</div>
</main>
<script>
  var frame=document.getElementById('preview');
  var items=[].slice.call(document.querySelectorAll('.nav-item'));
  function activate(el){{items.forEach(function(i){{i.classList.remove('active');}});el.classList.add('active');frame.src=el.getAttribute('data-src');}}
  items.forEach(function(el){{el.addEventListener('click',function(){{activate(el);}});}});
  if(items[0])items[0].classList.add('active');
</script>
</body>
</html>
",
        n = n, nav = nav_items, first = first_src,
    )
}
