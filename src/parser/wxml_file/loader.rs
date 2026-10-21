//! `<include>` / `<import>` 的路径解析与递归展开，以及 `<template>` 定义的上提。
//!
//! 规则（官方文档 wxml/import.html、wxml/template.html）：
//! - `include`：把目标文件**除 `<template>` 与 `<wxs>` 外**的内容嵌到当前位置；
//! - `import`：只引入目标文件**自身定义**的模板，不间接引入目标文件 import 的模板；
//! - 路径相对写它的文件所在目录，`/` 开头相对小程序根；`.wxml` 后缀可省（两种都认）。
//!
//! 守卫：成环（A include B、B include A）与过深（>16 层）都只打一次警告、该处什么都不放；
//! 解析到小程序目录之外的路径一律拒绝 —— 包里的 WXML 不该能读到包外的文件。

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::using_components::{normalize, resolve_component_path};
use std::path::{Path, PathBuf};

/// 文件嵌套深度上限（include 与 import 共用）。正常工程两三层就到头了，
/// 16 层只是防止「菱形 import」之类的写法把展开结果放大到失控。
const MAX_FILE_DEPTH: usize = 16;

pub(super) struct Loader {
    /// 规范化后的小程序根目录
    root: PathBuf,
    /// 正在展开的文件链（判环用）
    stack: Vec<PathBuf>,
}

impl Loader {
    pub(super) fn new(app_root: &Path) -> Self {
        Self { root: normalize(app_root), stack: Vec::new() }
    }

    /// 处理一个已解析的文件：解析其中所有引用，并把 `<template name>` / `<import>`
    /// 上提到顶层末尾（按文档序，后出现的 import 覆盖先出现的 —— 见模板表的构建）。
    pub(super) fn process_file(&mut self, file: &Path, nodes: Vec<WxmlNode>) -> Vec<WxmlNode> {
        let file = normalize(file);
        self.stack.push(file.clone());
        let mut hoisted = Vec::new();
        let mut out = self.process_list(&file, nodes, &mut hoisted);
        out.extend(hoisted);
        self.stack.pop();
        out
    }

    fn process_list(
        &mut self,
        file: &Path,
        nodes: Vec<WxmlNode>,
        hoisted: &mut Vec<WxmlNode>,
    ) -> Vec<WxmlNode> {
        let mut out = Vec::with_capacity(nodes.len());
        for mut node in nodes {
            if node.node_type != WxmlNodeType::Element {
                out.push(node);
                continue;
            }
            match node.tag_name.as_str() {
                "template" if node.attributes.contains_key("name") => {
                    // 定义体里的 include 也相对本文件解析；嵌套定义同样上提
                    let body = std::mem::take(&mut node.children);
                    node.children = self.process_list(file, body, hoisted);
                    hoisted.push(node);
                }
                "import" => {
                    node.children = self.load_import(file, node.get_attr("src").unwrap_or(""));
                    hoisted.push(node);
                }
                "include" => {
                    node.children = self.load_include(file, node.get_attr("src").unwrap_or(""));
                    out.push(node);
                }
                // wxs 的内容是脚本，不是 WXML，不往里递归；`src` 与 `require` 在这里读进来打包
                "wxs" => {
                    super::wxs_bundle::bundle(&self.root, file, &mut node);
                    out.push(node);
                }
                _ => {
                    let kids = std::mem::take(&mut node.children);
                    node.children = self.process_list(file, kids, hoisted);
                    out.push(node);
                }
            }
        }
        out
    }

    /// `<include>`：被引入文件处理后的全部节点（定义与 wxs 留着，渲染时它们不出图）
    fn load_include(&mut self, from: &Path, src: &str) -> Vec<WxmlNode> {
        match self.load(from, src, "include") {
            Some((path, nodes)) => self.process_file(&path, nodes),
            None => Vec::new(),
        }
    }

    /// `<import>`：只留被引入文件顶层的 `<template name>`、`<import>` 与 `<wxs>`。
    /// 正文不渲染，丢掉；它的 import 留着是因为它自己的模板体里可能要用。
    fn load_import(&mut self, from: &Path, src: &str) -> Vec<WxmlNode> {
        let Some((path, nodes)) = self.load(from, src, "import") else {
            return Vec::new();
        };
        self.process_file(&path, nodes)
            .into_iter()
            .filter(|n| {
                n.node_type == WxmlNodeType::Element
                    && match n.tag_name.as_str() {
                        "template" => n.attributes.contains_key("name"),
                        "import" | "wxs" => true,
                        _ => false,
                    }
            })
            .collect()
    }

    /// 解析路径、判环判深、读文件并解析。任何一步失败都打警告并返回 None。
    fn load(&mut self, from: &Path, src: &str, kind: &str) -> Option<(PathBuf, Vec<WxmlNode>)> {
        let src = src.trim();
        let Some(path) = self.resolve(from, src) else {
            eprintln!("⚠️ WXML <{kind} src=\"{src}\">：路径无效或越出小程序目录（来自 {}）", from.display());
            return None;
        };
        if self.stack.contains(&path) {
            eprintln!("⚠️ WXML <{kind} src=\"{src}\">：引用成环，已忽略（来自 {}）", from.display());
            return None;
        }
        if self.stack.len() >= MAX_FILE_DEPTH {
            eprintln!("⚠️ WXML <{kind} src=\"{src}\">：嵌套超过 {MAX_FILE_DEPTH} 层，已忽略");
            return None;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                eprintln!("⚠️ WXML <{kind} src=\"{src}\">：找不到文件 {}（来自 {}）", path.display(), from.display());
                return None;
            }
        };
        match crate::parser::WxmlParser::new(&text).parse() {
            Ok(nodes) => Some((path, nodes)),
            Err(e) => {
                eprintln!("⚠️ WXML <{kind} src=\"{src}\">：{} 解析失败：{e}", path.display());
                None
            }
        }
    }

    /// `src` → 文件路径。`.wxml` 可省：没写后缀时优先找 `x.wxml`，再找 `x` 本身。
    fn resolve(&self, from: &Path, src: &str) -> Option<PathBuf> {
        // 动态路径（`{{}}`）微信也不支持：import/include 是编译期的
        if src.is_empty() || src.contains("{{") {
            return None;
        }
        let dir = from.parent().unwrap_or(&self.root);
        let base = resolve_component_path(&self.root, dir, src);
        if !base.starts_with(&self.root) {
            return None;
        }
        if base.extension().map(|e| e == "wxml").unwrap_or(false) {
            return Some(base);
        }
        let mut with_ext = base.clone().into_os_string();
        with_ext.push(".wxml");
        let with_ext = PathBuf::from(with_ext);
        if with_ext.is_file() || !base.is_file() {
            Some(with_ext)
        } else {
            Some(base)
        }
    }
}
