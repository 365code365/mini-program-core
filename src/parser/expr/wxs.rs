//! `{{ }}` 里调用 wxs 模块：`{{m.fmt(price)}}`、`{{m.PI}}`。
//!
//! 表达式引擎是纯 Rust 的（`eval(&Expr, &Value)`），而 wxs 模块是脚本 —— 真要跑它只能交给
//! 逻辑层的 QuickJS。WXS 是 ES5 的子集，同一段代码在 JS 引擎里的结果与微信一致
//! （官方 wxs/index.html：「WXS 与 JavaScript 是不同的语言，有自己的语法，并不和 JavaScript 一致」，
//! 但差异都是「WXS 不支持某些写法」，能在微信跑的 WXS 在 JS 里结果相同）。
//!
//! 这里只放三样东西，都是**线程局部**的（渲染与逻辑层在同一个线程上跑）：
//! - **宿主**（[`WxsHost`]）：真正执行脚本的一方，由 `MiniApp::init` 装上。没有宿主时
//!   （画廊、离屏渲染、单测里只建了渲染器）调用一律得 `null` —— 与从前完全一样；
//! - **模块代码表**：模块 id → 打包好的源码（`{"entry":…,"files":{路径: 代码}}`），由模板引擎
//!   扫描 `<wxs>` 节点时登记，宿主第一次调用某个模块时才交给 JS 定义；
//! - **当前作用域**：此刻正在求值的 WXML 文件里有哪些模块（模块名 → id）。官方规定 wxs 模块
//!   只在定义它的那个 WXML 文件里可见，模板片段只能用**定义模板的文件**里的模块，
//!   include / import 都不带模块 —— 作用域由模板引擎在进出每个作用域根时切换。

use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// 执行 wxs 模块的一方（逻辑层）
pub trait WxsHost {
    /// 调用模块 `id` 导出对象上 `path` 指到的函数（`m.a.f(x)` 的 `path` 是 `["a","f"]`）。
    /// 函数不存在、脚本抛错都返回 `null`。
    fn call(&self, id: &str, bundle: &str, path: &[String], args: &[Value]) -> Value;
    /// 模块导出对象里的非函数部分（`{{m.PI}}` 这类常量访问）
    fn exports(&self, id: &str, bundle: &str) -> Value;
}

/// 一个 WXML 文件里可见的模块：模块名 → 模块 id
#[derive(Debug, Default)]
pub struct WxsScope {
    pub modules: HashMap<String, String>,
}

thread_local! {
    static HOST: RefCell<Option<Rc<dyn WxsHost>>> = RefCell::new(None);
    static CODE: RefCell<HashMap<String, Rc<str>>> = RefCell::new(HashMap::new());
    static SCOPE: RefCell<Option<Rc<WxsScope>>> = const { RefCell::new(None) };
}

/// 装上 / 卸下本线程的宿主（`MiniApp::init` 调）。同一线程上后建的应用顶掉先建的。
pub fn set_host(host: Option<Rc<dyn WxsHost>>) {
    HOST.with(|h| *h.borrow_mut() = host);
}

/// 登记模块代码（模板引擎扫到 `<wxs>` 时调；同一个 id 内容相同，重复登记无害）
pub fn register(id: &str, bundle: &str) {
    CODE.with(|c| {
        let mut c = c.borrow_mut();
        if !c.contains_key(id) {
            c.insert(id.to_string(), Rc::from(bundle));
        }
    });
}

/// 进入一个作用域（离开时自动恢复上一个）
pub fn enter(scope: Option<Rc<WxsScope>>) -> ScopeGuard {
    let prev = SCOPE.with(|s| std::mem::replace(&mut *s.borrow_mut(), scope));
    ScopeGuard(Some(prev))
}

/// [`enter`] 的恢复守卫
pub struct ScopeGuard(Option<Option<Rc<WxsScope>>>);

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        if let Some(prev) = self.0.take() {
            SCOPE.with(|s| *s.borrow_mut() = prev);
        }
    }
}

/// 名字是不是当前文件里的 wxs 模块；是就给出模块 id
fn module_id(name: &str) -> Option<String> {
    SCOPE.with(|s| s.borrow().as_ref().and_then(|sc| sc.modules.get(name).cloned()))
}

fn code_of(id: &str) -> Option<Rc<str>> {
    CODE.with(|c| c.borrow().get(id).cloned())
}

fn host() -> Option<Rc<dyn WxsHost>> {
    HOST.with(|h| h.borrow().clone())
}

/// `{{m}}` / `{{m.PI}}`：名字是模块时给出导出对象（函数不在里面）；不是模块返回 None，
/// 调用方照常去数据里找。模块名优先于同名数据字段 —— 模块是编译期绑定到这个名字上的。
pub(super) fn module_value(name: &str) -> Option<Value> {
    let id = module_id(name)?;
    let Some(h) = host() else { return Some(Value::Null) };
    let code = code_of(&id).unwrap_or_else(|| Rc::from(""));
    Some(h.exports(&id, &code))
}

/// `{{m.f(x)}}`：`root` 是模块名、`path` 是成员路径。不是模块返回 None（调用方按旧行为给 null）
pub(super) fn call(root: &str, path: &[String], args: &[Value]) -> Option<Value> {
    let id = module_id(root)?;
    let Some(h) = host() else { return Some(Value::Null) };
    let code = code_of(&id).unwrap_or_else(|| Rc::from(""));
    Some(h.call(&id, &code, path, args))
}

/// 模块 id：打包源码的 FNV-1a 哈希。同一段代码在两个文件里得到同一个 id、共用一个模块实例 ——
/// wxs 模块应当是纯函数（官方：wxs 函数不能调用逻辑层函数，也不能改页面数据），共用不改变结果
pub fn module_id_of(bundle: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bundle.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("wxs:{h:016x}")
}
