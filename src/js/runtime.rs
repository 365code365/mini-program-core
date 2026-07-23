//! QuickJS 运行时

use rquickjs::{Context, Runtime, Function, Object, Value, Ctx, Result as JsResult, function::Rest};
use std::cell::RefCell;
use std::rc::Rc;

/// JS 运行时
pub struct JsRuntime {
    runtime: Runtime,
    context: Context,
}

impl JsRuntime {
    pub fn new() -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|e| e.to_string())?;
        let context = Context::full(&runtime).map_err(|e| e.to_string())?;
        
        Ok(Self { runtime, context })
    }
    
    /// 泵出所有待处理的微任务（Promise / async 的 job 队列）
    ///
    /// QuickJS 中 Promise 的 then/catch、async 函数的续体都会进入 job 队列，
    /// 必须由宿主主动执行。之前的实现从不调用它，导致 Promise/async 永远不 resolve。
    pub fn pump_jobs(&self) {
        // 每轮最多执行有限次，避免恶意/异常代码造成死循环
        let mut guard = 0u32;
        while self.runtime.is_job_pending() {
            match self.runtime.execute_pending_job() {
                Ok(_) => {}
                Err(_) => break,
            }
            guard += 1;
            if guard > 100_000 {
                break;
            }
        }
    }
    
    /// 执行 JS 代码
    pub fn eval(&self, code: &str) -> Result<String, String> {
        let result = self.context.with(|ctx| {
            let result: JsResult<Value> = ctx.eval(code);
            match result {
                Ok(val) => Ok(js_value_to_string(&ctx, &val)),
                Err(e) => {
                    // 尝试获取更详细的错误信息：若是 JS 抛出的异常，取回异常对象
                    if matches!(e, rquickjs::Error::Exception) {
                        let exc = ctx.catch();
                        Err(format!("JS Exception: {}", js_value_to_string(&ctx, &exc)))
                    } else {
                        Err(format!("{:?}", e))
                    }
                }
            }
        });
        // 执行完同步代码后，泵出微任务队列
        self.pump_jobs();
        result
    }
    
    /// 执行 JS 文件
    pub fn eval_file(&self, path: &str) -> Result<String, String> {
        let code = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        self.eval(&code)
    }
    
    /// 注册一个 CommonJS 模块（不立即执行，等 require 时才执行工厂函数）
    ///
    /// `path` 为相对项目根目录的路径（可带或不带 .js 后缀），
    /// `source` 为模块源码。源码会被包裹进 `function(module, exports, require)`。
    pub fn define_module(&self, path: &str, source: &str) -> Result<(), String> {
        // path 用 {:?} 生成带转义的合法 JS 字符串字面量；source 作为原始代码注入函数体
        let code = format!(
            "__defineModule({:?}, function(module, exports, require) {{\n{}\n}});",
            path, source
        );
        self.eval(&code).map(|_| ())
    }
    
    /// 加载（require）一个模块，触发其工厂函数执行
    pub fn require_module(&self, path: &str) -> Result<(), String> {
        let code = format!("__require('', {:?});", path);
        self.eval(&code).map(|_| ())
    }
    
    /// 注册全局函数（简化版，使用闭包包装）
    pub fn register_function<F>(&self, name: &str, func: F) -> Result<(), String>
    where
        F: Fn(Vec<String>) -> String + 'static,
    {
        // 将函数存储在 Rc<RefCell> 中以便在闭包中使用
        let func = Rc::new(RefCell::new(func));
        let name_owned = name.to_string();
        
        self.context.with(|ctx| {
            let global = ctx.globals();
            let func_clone = func.clone();
            
            // 使用 Rest<Value> 来接收可变数量的参数
            let js_func = Function::new(ctx.clone(), move |ctx: Ctx, args: Rest<Value>| -> JsResult<String> {
                let string_args: Vec<String> = args.0
                    .iter()
                    .map(|v| js_value_to_string(&ctx, v))
                    .collect();
                
                let f = func_clone.borrow();
                let result = f(string_args);
                Ok(result)
            });
            
            match js_func {
                Ok(f) => global.set(&name_owned, f).map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            }
        })
    }
    
    /// 调用 JS 函数
    pub fn call_function(&self, name: &str, args: &[&str]) -> Result<String, String> {
        // 构建调用代码
        let args_str = args
            .iter()
            .map(|s| format!("\"{}\"", s.replace("\"", "\\\"")))
            .collect::<Vec<_>>()
            .join(", ");
        
        let code = format!("{}({})", name, args_str);
        self.eval(&code)
    }
    
    /// 设置全局变量
    pub fn set_global(&self, name: &str, value: &str) -> Result<(), String> {
        self.context.with(|ctx| {
            let global = ctx.globals();
            global.set(name, value).map_err(|e| e.to_string())
        })
    }
    
    /// 获取全局变量
    pub fn get_global(&self, name: &str) -> Result<String, String> {
        self.context.with(|ctx| {
            let global = ctx.globals();
            let val: JsResult<Value> = global.get(name);
            match val {
                Ok(v) => Ok(js_value_to_string(&ctx, &v)),
                Err(e) => Err(e.to_string()),
            }
        })
    }
}

/// 将 JS Value 转换为字符串。
///
/// 对于对象/数组，使用 JS 侧的 `JSON.stringify` 做序列化，
/// 这样 native 侧就能拿到完整的对象数据，而不再是之前的 `"[object]"`。
fn js_value_to_string(ctx: &Ctx, val: &Value) -> String {
    if val.is_undefined() {
        "undefined".to_string()
    } else if val.is_null() {
        "null".to_string()
    } else if let Some(s) = val.as_string() {
        s.to_string().unwrap_or_default()
    } else if let Some(b) = val.as_bool() {
        b.to_string()
    } else if let Some(n) = val.as_int() {
        n.to_string()
    } else if let Some(n) = val.as_float() {
        // JS 数字语义：整数值不带小数点
        format_js_number(n)
    } else if val.is_array() || val.is_object() {
        json_stringify(ctx, val).unwrap_or_else(|| "[object Object]".to_string())
    } else {
        "[unknown]".to_string()
    }
}

/// 用 JS 侧 JSON.stringify 序列化对象/数组
fn json_stringify(ctx: &Ctx, val: &Value) -> Option<String> {
    let globals = ctx.globals();
    let json: Object = globals.get("JSON").ok()?;
    let stringify: Function = json.get("stringify").ok()?;
    let res: Value = stringify.call((val.clone(),)).ok()?;
    if let Some(s) = res.as_string() {
        s.to_string().ok()
    } else {
        None
    }
}

/// 格式化 JS 浮点数，去除多余的 .0
fn format_js_number(n: f64) -> String {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create JS runtime")
    }
}
