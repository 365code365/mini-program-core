//! wxs 模块的执行方：把渲染层表达式引擎发起的 `{{m.f(x)}}` 交给逻辑层的 QuickJS（`wxs.js`）。
//!
//! 渲染与逻辑层跑在**同一个线程**上，渲染期间不持有运行时的锁，所以这里能直接同步调进去。
//! 仍然只用 `try_lock`：万一哪天有人在 JS 原生回调里触发了渲染，拿不到锁就给 `null`
//! （那一处少个值），而不是把整个线程锁死。

use crate::js::JsRuntime;
use crate::parser::expr::wxs::WxsHost;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, Weak};

pub(super) struct AppWxsHost {
    /// 弱引用：宿主挂在线程局部变量上，不能让它把已经销毁的应用的 JS 运行时续命
    runtime: Weak<Mutex<JsRuntime>>,
    /// 已经交给 JS 定义过的模块 id
    defined: RefCell<HashSet<String>>,
    /// 模块导出的常量（`{{m.PI}}`）：模块是单例、导出在定义时就定了，取一次就够
    exports: RefCell<HashMap<String, Value>>,
}

impl AppWxsHost {
    pub(super) fn new(runtime: &Arc<Mutex<JsRuntime>>) -> Self {
        Self {
            runtime: Arc::downgrade(runtime),
            defined: RefCell::new(HashSet::new()),
            exports: RefCell::new(HashMap::new()),
        }
    }

    /// 拿着锁执行一段 JS；模块还没定义过就先定义
    fn eval(&self, id: &str, bundle: &str, code: &str) -> Option<String> {
        let runtime = self.runtime.upgrade()?;
        let rt = match runtime.try_lock() {
            Ok(rt) => rt,
            Err(_) => {
                eprintln!("⚠️ wxs：逻辑层正忙（在 JS 回调里触发了渲染？），{id} 这次求值为 null");
                return None;
            }
        };
        if !self.defined.borrow().contains(id) {
            // 包是加载期生成的 JSON，本身就是合法的 JS 字面量
            let lit = if bundle.trim_start().starts_with('{') { bundle } else { "{}" };
            rt.eval(&format!("__wxsDefine({}, {lit})", js_str(id))).ok()?;
            self.defined.borrow_mut().insert(id.to_string());
        }
        rt.eval(code).ok()
    }
}

impl WxsHost for AppWxsHost {
    fn call(&self, id: &str, bundle: &str, path: &[String], args: &[Value]) -> Value {
        let path_json = serde_json::to_string(path).unwrap_or_else(|_| "[]".into());
        let args_json = serde_json::to_string(args).unwrap_or_else(|_| "[]".into());
        let code = format!("__wxsCall({}, {path_json}, {args_json})", js_str(id));
        self.eval(id, bundle, &code)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null)
    }

    fn exports(&self, id: &str, bundle: &str) -> Value {
        if let Some(v) = self.exports.borrow().get(id) {
            return v.clone();
        }
        let v = self
            .eval(id, bundle, &format!("__wxsExports({})", js_str(id)))
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null);
        self.exports.borrow_mut().insert(id.to_string(), v.clone());
        v
    }
}

fn js_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "''".into())
}
