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
        files.push(EmittedFile::text("common/runtime.js", RUNTIME_JS));

        // ── 静态资源 ──
        collect_assets(Path::new(&app.root).join("assets").as_path(), "assets", &mut files);

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

            // 静态首屏（无 JS 时的回退视图）
            let body = wxml_to_html_pretty(&page.wxml, &page.data);
            // 供 runtime 响应式重渲染的 WXML AST
            let ast = nodes_to_json(&page.wxml);
            let html = build_page_html(&page.route, leaf, &rel, &body, &ast, has_js);
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

/// 生成单个页面的 HTML（分离引用 base.css / app.css / <page>.css + 内嵌 AST + runtime）
fn build_page_html(route: &str, leaf: &str, rel: &str, body: &str, ast: &JsonValue, has_js: bool) -> String {
    let logic = if has_js {
        format!("<script src=\"./{}.js\"></script>\n", leaf)
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
  <style>
    #app{{width:{w}px;min-height:100vh;margin:0 auto;background:#f5f6f8;position:relative;overflow:hidden;}}
  </style>
</head>
<body>
<div id=\"app\">
{body}</div>
<script>
window.__PAGE_ROUTE__ = {route_json};
window.__WXML__ = {ast};
</script>
<script src=\"{rel}common/runtime.js\"></script>
<script src=\"{rel}common/app.js\"></script>
{logic}</body>
</html>
",
        route = route, rel = rel, leaf = leaf, w = WIDTH, body = body,
        route_json = serde_json::to_string(route).unwrap(),
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
