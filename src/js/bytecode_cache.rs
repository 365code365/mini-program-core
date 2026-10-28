//! JS 字节码缓存：小游戏包里的大脚本编译一次，之后从磁盘读字节码，跳过解析与编译。
//!
//! 为什么要：Cocos Creator 导出的引擎脚本 3MB（`cocos-js/_virtual_cc-*.js`），一加 PLY110 上光
//! `new Function(源码)` 就 360~390ms，每次冷启动都要付；字节码读回来只是反序列化。
//!
//! 做法：`JS_Eval(… JS_EVAL_FLAG_COMPILE_ONLY)` 编出函数对象 → `JS_WriteObject(BYTECODE)` 落盘 →
//! 下次 `JS_ReadObject` + `JS_EvalFunction`。缓存键 = 源码（含包装头）+ 引擎指纹的 64 位哈希：
//! 包更新了、引擎升级了（QuickJS 版本、本 crate 版本）都会换键，旧文件不会被读到。
//! 读回失败（文件坏了、字节码版本对不上）一律退回正常编译，并把坏文件删掉。
//! 文件头再存一份源码哈希与长度：哈希撞上但内容不同的极端情况也能认出来。
//!
//! 只缓存 ≥ [`MIN_BYTES`] 的源码（小文件解析本来就快，存成一堆小文件反而慢）。
//! 目录 `<数据目录>/jscache/`，总量超过 [`MAX_TOTAL`] 时按修改时间删最旧的。
//! `MINI_NO_JS_CACHE=1` 关掉（对照 / 排查用）。
//!
//! 安全：QuickJS 的字节码读取器不防恶意输入，所以缓存只放在宿主给的私有数据目录里，
//! 而且只读本进程自己写出、头部校验通过的文件。
use rquickjs::{qjs, Ctx, Value};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

pub const MIN_BYTES: usize = 16 * 1024;
const MAX_TOTAL: u64 = 96 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"MRQJSBC1";
/// 文件名与 eval 一致（栈里显示 eval_script:行:列），缓存与否不改变报错文本
const FILENAME: &[u8] = b"eval_script\0";

fn fingerprint() -> &'static str {
    static F: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    F.get_or_init(|| {
        // SAFETY: JS_GetVersion 返回静态的 C 字符串
        let qv = unsafe { std::ffi::CStr::from_ptr(qjs::JS_GetVersion()) }.to_string_lossy().into_owned();
        format!("{qv}/{}/{}", env!("CARGO_PKG_VERSION"), std::mem::size_of::<usize>())
    })
}

fn key_of(src: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    fingerprint().hash(&mut h);
    src.hash(&mut h);
    h.finish()
}

pub fn enabled() -> bool {
    std::env::var("MINI_NO_JS_CACHE").map(|v| v != "1").unwrap_or(true)
}

fn dir() -> PathBuf {
    crate::data_dir::data_dir().join("jscache")
}

/// 执行一段脚本（全局作用域，与 `ctx.eval` 同义），能用缓存就用缓存。
/// 返回 (结果, 是否命中)。异常照常以 `Err(rquickjs::Error::Exception)` 返回
pub fn eval<'js>(ctx: &Ctx<'js>, src: &str) -> (rquickjs::Result<Value<'js>>, bool) {
    if src.len() < MIN_BYTES || !enabled() {
        return (ctx.eval(src), false);
    }
    let key = key_of(src);
    let path = dir().join(format!("{key:016x}.qbc"));
    if let Some(fun) = read(ctx, &path, key, src.len()) {
        return (run(ctx, fun), true);
    }
    // 编译（只编不跑）→ 落盘 → 跑
    let Ok(csrc) = std::ffi::CString::new(src) else { return (ctx.eval(src), false) };
    let raw = ctx.as_raw().as_ptr();
    // SAFETY: raw 是当前上下文；源码以 NUL 结尾、长度不含 NUL；返回值所有权归我们
    let fun = unsafe {
        qjs::JS_Eval(raw, csrc.as_ptr(), src.len() as _, FILENAME.as_ptr() as *const _, (qjs::JS_EVAL_TYPE_GLOBAL | qjs::JS_EVAL_FLAG_COMPILE_ONLY) as i32)
    };
    // SAFETY: fun 是刚拿到的、引用计数归我们的值
    let fun = unsafe { Value::from_raw(ctx.clone(), fun) };
    if fun.is_exception() {
        // 语法错误：交给 rquickjs 把异常取出来（与 ctx.eval 同一个错误形状）
        return (Err(rquickjs::Error::Exception), false);
    }
    write(ctx, &path, key, src.len(), &fun);
    (run(ctx, fun), false)
}

fn run<'js>(ctx: &Ctx<'js>, fun: Value<'js>) -> rquickjs::Result<Value<'js>> {
    let raw = ctx.as_raw().as_ptr();
    // SAFETY: JS_EvalFunction 接管（并释放）fun 的那一份引用：交出去之后 forget，免得 Value 的 Drop 再减一次
    let r = unsafe { qjs::JS_EvalFunction(raw, fun.as_raw()) };
    std::mem::forget(fun);
    // SAFETY: 返回值所有权归我们
    let v = unsafe { Value::from_raw(ctx.clone(), r) };
    if v.is_exception() {
        Err(rquickjs::Error::Exception)
    } else {
        Ok(v)
    }
}

fn read<'js>(ctx: &Ctx<'js>, path: &Path, key: u64, src_len: usize) -> Option<Value<'js>> {
    let bytes = std::fs::read(path).ok()?;
    let head = MAGIC.len() + 16;
    let ok = bytes.len() > head
        && &bytes[..8] == MAGIC
        && bytes[8..16] == key.to_le_bytes()
        && bytes[16..24] == (src_len as u64).to_le_bytes();
    if !ok {
        let _ = std::fs::remove_file(path);
        return None;
    }
    let body = &bytes[head..];
    let raw = ctx.as_raw().as_ptr();
    // SAFETY: body 是本进程写出、头部校验过的字节码；JS_ReadObject 只读这段内存
    let v = unsafe { qjs::JS_ReadObject(raw, body.as_ptr(), body.len() as _, qjs::JS_READ_OBJ_BYTECODE as i32) };
    // SAFETY: 返回值所有权归我们
    let v = unsafe { Value::from_raw(ctx.clone(), v) };
    if v.is_exception() {
        // 字节码版本对不上之类：清掉这次的异常，删文件，退回编译
        let _ = ctx.catch();
        let _ = std::fs::remove_file(path);
        return None;
    }
    Some(v)
}

fn write(ctx: &Ctx<'_>, path: &Path, key: u64, src_len: usize, fun: &Value<'_>) {
    let raw = ctx.as_raw().as_ptr();
    let mut size: qjs::size_t = 0;
    // SAFETY: fun 是有效的函数对象；返回的缓冲由 js_malloc 分配，用完 js_free
    let p = unsafe { qjs::JS_WriteObject(raw, &mut size, fun.as_raw(), qjs::JS_WRITE_OBJ_BYTECODE as i32) };
    if p.is_null() {
        let _ = ctx.catch();
        return;
    }
    // SAFETY: p 指向 size 字节
    let body = unsafe { std::slice::from_raw_parts(p, size as usize) };
    let mut out = Vec::with_capacity(body.len() + 24);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&key.to_le_bytes());
    out.extend_from_slice(&(src_len as u64).to_le_bytes());
    out.extend_from_slice(body);
    // SAFETY: 与 JS_WriteObject 同一个上下文分配的
    unsafe { qjs::js_free(raw, p as *mut _) };
    let d = dir();
    if std::fs::create_dir_all(&d).is_err() {
        return;
    }
    // 先写临时文件再改名：写到一半被杀，下次读到的也不会是半个文件
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, &out).is_ok() && std::fs::rename(&tmp, path).is_ok() {
        prune(&d);
    }
}

/// 总量超过 MAX_TOTAL：按修改时间从旧到新删，删到一半以下
fn prune(d: &Path) {
    let Ok(rd) = std::fs::read_dir(d) else { return };
    let mut files: Vec<(std::time::SystemTime, u64, PathBuf)> = rd
        .flatten()
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            Some((m.modified().ok()?, m.len(), e.path()))
        })
        .collect();
    let total: u64 = files.iter().map(|f| f.1).sum();
    if total <= MAX_TOTAL {
        return;
    }
    files.sort();
    let mut left = total;
    for (_, len, p) in files {
        if left <= MAX_TOTAL / 2 {
            break;
        }
        if std::fs::remove_file(&p).is_ok() {
            left -= len;
        }
    }
}

#[cfg(test)]
mod tests {
    /// 编译一次落盘、第二次从字节码读回：结果相同、第二次命中；坏文件退回编译并改写
    #[test]
    fn 缓存命中_结果相同_坏文件自愈() {
        // 不改进程级的数据目录（别的测试并行在用）：用默认目录，只动自己这一个文件
        let rt = crate::js::JsRuntime::new().unwrap();
        // 够大才走缓存
        let pad = format!("/* {} {} */", std::process::id(), "x".repeat(super::MIN_BYTES));
        let src = format!("{pad}\nglobalThis.__bc = (globalThis.__bc || 0) + 1; (function f(n){{ return n < 2 ? n : f(n-1) + f(n-2); }})(15)");
        let (a, hit_a) = rt.eval_cached(&src);
        let (b, hit_b) = rt.eval_cached(&src);
        assert_eq!((a.as_deref(), hit_a), (Ok("610"), false));
        assert_eq!((b.as_deref(), hit_b), (Ok("610"), true));
        assert_eq!(rt.eval("__bc").unwrap(), "2", "命中缓存时脚本照样执行（副作用发生两次）");
        // 弄坏缓存文件：读不回就重新编译，结果照样对
        let file = super::dir().join(format!("{:016x}.qbc", super::key_of(&src)));
        std::fs::write(&file, b"MRQJSBC1 broken").unwrap();
        let (c, hit_c) = rt.eval_cached(&src);
        assert_eq!((c.as_deref(), hit_c), (Ok("610"), false));
        // 语法错误：与 eval 同一种报错
        let bad = format!("{pad}\nfunction (");
        let (e, _) = rt.eval_cached(&bad);
        assert!(e.unwrap_err().contains("SyntaxError"));
        let _ = std::fs::remove_file(&file);
    }
}
