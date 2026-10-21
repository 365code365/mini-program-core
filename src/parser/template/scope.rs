//! 模板片段的作用域：哪个 `<template is>` 能看见哪些 `<template name>`。
//!
//! 「作用域根」是一组兄弟节点：页面/组件文件的顶层、`<include>` 的子节点（被引入文件）、
//! `<import>` 的子节点（被 import 的文件）。`wxml_file` 加载时已经把定义与 import 都上提
//! 到了各自文件的顶层，所以建表只扫一层：
//!
//! 1. 先收 `<import>` 节点的**直接** `<template name>` 子节点 —— 被 import 文件自己 import
//!    的模板在更深一层，收不到，这就是官方说的「不会间接引入」；多个 import 重名时后写的赢；
//! 2. 再收本层的 `<template name>`，覆盖同名的 import（本文件定义优先）。
//!
//! 每个定义记住**它定义在哪个文件**（`file`）：模板体里再写 `<template is>`，查的是定义
//! 文件的表，不是调用方的表（与微信编译产物按定义文件查模板一致）。
//!
//! 文档没写重名时的优先级，上面 1/2 两条是我们的取舍，有测试锁着。

use super::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// 模板嵌套展开的深度上限。glass-easel 允许模板递归（比如画树），但一个没有终止条件的
/// 自引用模板会无限展开 —— 渲染在每帧布局里跑，卡住就是整个页面卡死。
pub(super) const MAX_TEMPLATE_DEPTH: u32 = 16;

/// 单次渲染里 `<template is>` 展开的总次数上限。深度上限挡不住「一个模板里引用自己两次」
/// 这种指数爆炸（2^16 次还能忍，3^16 就是四千万），总量封顶兜住最坏情况；
/// 正常页面（几百条列表 × 每条一两个模板）离这个数很远。
pub(super) const MAX_TEMPLATE_EXPANSIONS: u32 = 50_000;

/// 一个模板定义：函数体 + 定义它的那个文件（作用域根）
#[derive(Clone, Copy)]
pub(super) struct Def<'a> {
    pub(super) body: &'a [WxmlNode],
    pub(super) file: &'a [WxmlNode],
}

pub(super) type Table<'a> = HashMap<&'a str, Def<'a>>;

/// 一次渲染共享的上下文
pub(super) struct Cx<'a> {
    pub(super) comps: &'a ComponentTemplates,
    /// 作用域根 → 模板表。按切片地址缓存：同一个模板在 `wx:for` 里展开几百次，
    /// 它的定义文件只建一次表
    cache: RefCell<HashMap<(usize, usize, usize), Rc<Table<'a>>>>,
    budget: Cell<u32>,
    /// 渲染数据里的 `$comp`（实例键 → 实例 data）。放在上下文里而不是一层层塞进作用域：
    /// 从前每展开一个组件就把整张 `$comp` 克隆进它的作用域，`wx:for` 出 N 个组件就是 N² 份拷贝
    comp_data: Option<&'a serde_json::Map<String, JsonValue>>,
    /// 渲染数据里的 `$copt`（标签名 → 组件选项，目前只有 `multipleSlots`）
    comp_opts: Option<&'a serde_json::Map<String, JsonValue>>,
    /// 当前节点在「所属模板」里的结构位置（实例键的一部分，见 `component.rs`）。
    /// 只有登记了组件时才记 —— 没有组件的页面一个字节都不多做
    path: RefCell<Vec<Seg>>,
    track: bool,
    /// 收集模式：只为拿到组件出现位置（宿主对账用），不做普通元素的插值
    pub(super) collect: Option<RefCell<Vec<super::component::Occurrence>>>,
    /// 作用域根 → 它定义的 wxs 模块（按切片地址缓存，同 `cache`）
    wxs_cache: RefCell<HashMap<(usize, usize), Option<Rc<crate::parser::expr::wxs::WxsScope>>>>,
    /// 这次渲染里见过 wxs 模块没有。没见过就不必在每组兄弟节点上切换求值作用域 ——
    /// 不用 wxs 的页面一点额外开销都没有
    pub(super) wxs_seen: Cell<bool>,
    /// 收集模式下的祖先栈：(归属实例键, 冒泡路径帧)。组件出现时按归属过滤出它的冒泡路径
    chain: RefCell<Vec<(String, super::component::ChainFrame)>>,
}

/// 结构位置的一段：第几个兄弟、`wx:for` 的第几项（没写 `wx:key`）、`wx:key` 给出的项身份、
/// 进入了 slot 内容
#[derive(Clone)]
pub(super) enum Seg {
    Child(u32),
    Iter(u32),
    Key(Rc<str>),
    Slot,
}

/// 当前所处的作用域
pub(super) struct Frame<'a, 'r> {
    /// 这里能用的模板
    pub(super) table: Rc<Table<'a>>,
    /// 「数据根」：页面 data、组件实例 data、或模板的 `data`。`<include>` 的内容只认它，
    /// 不认 `wx:for` 之类的临时变量（glass-easel 语义，见 `fragment.rs`）
    pub(super) root: &'r JsonValue,
    /// 已经嵌套展开了几层模板
    pub(super) depth: u32,
    /// 这段节点归谁所有：页面是空串，组件模板里是该组件实例的键。
    /// 事件派发、实例键前缀都看它；slot 内容沿用**使用方**的值
    pub(super) owner: &'r str,
    /// 正在展开的组件实例给 `<slot>` 留的使用方信息（页面顶层为 None）
    pub(super) slot: Option<&'r super::slot::SlotCx<'a, 'r>>,
    /// 已经嵌套了几层组件（自引用组件的兜底，见 `MAX_COMPONENT_DEPTH`）
    pub(super) cdepth: u32,
    /// 这段节点所在的 WXML 文件里定义的 wxs 模块（没有为 None）。官方规定模块只在定义它的
    /// 文件里可见：模板片段用定义模板的文件的、slot 内容用使用方的（见 `wxs_scope.rs`）
    pub(super) wxs: Option<Rc<crate::parser::expr::wxs::WxsScope>>,
}

/// 组件嵌套层数上限。组件可以递归使用自己（树形控件），没有终止条件时会无限展开
pub(super) const MAX_COMPONENT_DEPTH: u32 = 32;

impl<'a> Cx<'a> {
    pub(super) fn new(comps: &'a ComponentTemplates, data: &'a JsonValue) -> Self {
        let comp_data = data.get(COMPONENT_DATA_KEY).and_then(|v| v.as_object());
        let comp_opts = data.get(COMPONENT_OPTIONS_KEY).and_then(|v| v.as_object());
        Self {
            comps,
            cache: RefCell::new(HashMap::new()),
            budget: Cell::new(MAX_TEMPLATE_EXPANSIONS),
            comp_data,
            comp_opts,
            path: RefCell::new(Vec::new()),
            track: !comps.is_empty(),
            collect: None,
            wxs_cache: RefCell::new(HashMap::new()),
            wxs_seen: Cell::new(false),
            chain: RefCell::new(Vec::new()),
        }
    }
    /// 进入一个带事件绑定的元素（收集模式）。返回是否真的压了栈，出来时原样交给 [`Self::pop_chain`]
    pub(super) fn push_chain(&self, owner: &str, frame: Option<super::component::ChainFrame>) -> bool {
        match frame {
            Some(fr) => {
                self.chain.borrow_mut().push((owner.to_string(), fr));
                true
            }
            None => false,
        }
    }
    pub(super) fn pop_chain(&self, pushed: bool) {
        if pushed {
            self.chain.borrow_mut().pop();
        }
    }
    /// 一处组件使用的冒泡路径：宿主自己 + 栈里归属同一使用方的祖先（由内向外）
    pub(super) fn chain_for(&self, owner: &str, host: Option<super::component::ChainFrame>) -> Vec<super::component::ChainFrame> {
        let mut out: Vec<_> = host.into_iter().collect();
        out.extend(self.chain.borrow().iter().rev().filter(|(o, _)| o == owner).map(|(_, f)| f.clone()));
        out
    }

    /// 一个作用域根（WXML 文件顶层）定义的 wxs 模块
    pub(super) fn wxs_of(&self, list: &[WxmlNode]) -> Option<Rc<crate::parser::expr::wxs::WxsScope>> {
        let key = (list.as_ptr() as usize, list.len());
        if let Some(s) = self.wxs_cache.borrow().get(&key) {
            return s.clone();
        }
        let s = super::wxs_scope::scan(list);
        if s.is_some() {
            self.wxs_seen.set(true);
        }
        self.wxs_cache.borrow_mut().insert(key, s.clone());
        s
    }

    pub(super) fn collecting(&self) -> bool {
        self.collect.is_some()
    }

    /// 页面里写了 `wx:key` 时也记结构位置：带 key 的列表项要靠它得到稳定身份（见 `for_loop.rs`）
    pub(super) fn track_keys_in(&mut self, nodes: &[WxmlNode]) {
        if !self.track && super::for_loop::has_wx_key(nodes) {
            self.track = true;
        }
    }

    /// 此刻是否在某个带 `wx:key` 的列表项里（含组件实例本身就是带 key 的列表项的情形）
    pub(super) fn keyed_identity(&self, owner: &str) -> Option<String> {
        if !self.track {
            return None;
        }
        let in_keyed = self.path.borrow().iter().any(|s| matches!(s, Seg::Key(_))) || owner.contains("[#");
        in_keyed.then(|| format!("{owner}|{}", self.site()))
    }

    /// 实例 data：先按实例键找；找不到再按标签名（`__mountPageComponent` 的老式单实例）
    pub(super) fn instance_data(&self, key: &str, tag: &str) -> Option<&'a JsonValue> {
        let m = self.comp_data?;
        m.get(key).or_else(|| m.get(tag))
    }

    /// 组件是否声明了 `options.multipleSlots`
    pub(super) fn multiple_slots(&self, tag: &str) -> bool {
        self.comp_option(tag, "multipleSlots")
    }
    /// 组件选项（`$copt[标签][名]` 为真）：`virtualHost` / `dynamicSlots` / `multipleSlots`
    pub(super) fn comp_option(&self, tag: &str, name: &str) -> bool {
        self.comp_opts
            .and_then(|m| m.get(tag))
            .and_then(|o| o.get(name))
            .map(expr::is_truthy)
            .unwrap_or(false)
    }

    /// 进入一段结构位置；返回值离开作用域时自动退出
    pub(super) fn enter(&self, seg: Seg) -> PathGuard<'_, 'a> {
        if self.track {
            self.path.borrow_mut().push(seg);
            PathGuard(Some(self))
        } else {
            PathGuard(None)
        }
    }

    /// 换一套结构位置（进组件模板 / 进 slot 内容时用），返回旧的以便换回
    pub(super) fn swap_path(&self, p: Vec<Seg>) -> Vec<Seg> {
        std::mem::replace(&mut *self.path.borrow_mut(), p)
    }

    pub(super) fn path_snapshot(&self) -> Vec<Seg> {
        self.path.borrow().clone()
    }

    /// 结构位置的文本形式：`3.0[2].1`、slot 内容记作 `s`
    pub(super) fn site(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        for (i, seg) in self.path.borrow().iter().enumerate() {
            match seg {
                Seg::Child(n) => {
                    if i > 0 {
                        s.push('.');
                    }
                    let _ = write!(s, "{n}");
                }
                Seg::Iter(n) => {
                    let _ = write!(s, "[{n}]");
                }
                Seg::Key(k) => {
                    let _ = write!(s, "[#{k}]");
                }
                Seg::Slot => s.push_str(".s"),
            }
        }
        s
    }

    /// 一个作用域根的模板表
    pub(super) fn table_of(&self, list: &'a [WxmlNode]) -> Rc<Table<'a>> {
        self.cached((0, list.as_ptr() as usize, list.len()), || build_table(list))
    }

    /// `<include>` 子树的表：被引入文件自己的模板优先，其次仍能用引入方的
    /// （「嵌入相当于拷贝到当前位置」，拷过来的内容理应还看得见当前文件的模板）
    pub(super) fn include_table(&self, parent: &Rc<Table<'a>>, list: &'a [WxmlNode]) -> Rc<Table<'a>> {
        let own = self.table_of(list);
        if own.is_empty() {
            return Rc::clone(parent);
        }
        let key = (Rc::as_ptr(parent) as usize, list.as_ptr() as usize, list.len());
        self.cached(key, || {
            let mut t = (**parent).clone();
            t.extend(own.iter().map(|(k, v)| (*k, *v)));
            t
        })
    }

    /// 扣一次展开额度；用完返回 false
    pub(super) fn take_budget(&self) -> bool {
        let left = self.budget.get();
        if left == 0 {
            return false;
        }
        self.budget.set(left - 1);
        true
    }

    fn cached(&self, key: (usize, usize, usize), build: impl FnOnce() -> Table<'a>) -> Rc<Table<'a>> {
        if let Some(t) = self.cache.borrow().get(&key) {
            return Rc::clone(t);
        }
        let t = Rc::new(build());
        self.cache.borrow_mut().insert(key, Rc::clone(&t));
        t
    }
}

/// `Cx::enter` 的退出守卫
pub(super) struct PathGuard<'c, 'a>(Option<&'c Cx<'a>>);

impl Drop for PathGuard<'_, '_> {
    fn drop(&mut self) {
        if let Some(cx) = self.0 {
            cx.path.borrow_mut().pop();
        }
    }
}

/// `<template name="x">` → `Some("x")`
pub(super) fn def_name(n: &WxmlNode) -> Option<&str> {
    if n.node_type == WxmlNodeType::Element && n.tag_name == "template" {
        n.attributes.get("name").map(|s| s.trim())
    } else {
        None
    }
}

fn build_table(list: &[WxmlNode]) -> Table<'_> {
    let mut t = Table::new();
    for n in list {
        if n.node_type == WxmlNodeType::Element && n.tag_name == "import" {
            for d in &n.children {
                if let Some(name) = def_name(d) {
                    t.insert(name, Def { body: &d.children, file: &n.children });
                }
            }
        }
    }
    for n in list {
        if let Some(name) = def_name(n) {
            t.insert(name, Def { body: &n.children, file: list });
        }
    }
    t
}

/// 同一条警告只打一次：模板展开在每次布局重建时都跑，找不到的模板名会刷屏
pub(super) fn warn_once(msg: String) {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut s) = seen.lock() {
        if s.len() < 256 && s.insert(msg.clone()) {
            eprintln!("⚠️ {msg}");
        }
    }
}
