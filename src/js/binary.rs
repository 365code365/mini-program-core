//! 二进制过桥（game-doc/02 §6）：JS 的 ArrayBuffer / TypedArray / DataView 与 Rust 字节之间直接转换，
//! 不再经 base64 字符串（从前所有二进制都走纯 JS 的逐字节 base64，见 prelude/codec.js）。
//!
//! 借出的字节只在当次原生调用里有效：调用期间不能回调 JS（回调可能 detach / 改写这块内存），
//! 所以 [`with_bytes`] 只给一个闭包作用域，要跨调用保存就自己拷成 `Vec<u8>`。

use rquickjs::{qjs, ArrayBuffer, Ctx, Exception, Object, Result as JsResult, Value};

/// 单次过桥的上限：内存紧张时一次拷贝就能把进程推过线，宁可明确报 RangeError
pub const MAX_BRIDGE_BYTES: usize = 256 * 1024 * 1024;

/// 取出一个二进制值的字节（ArrayBuffer 整块；TypedArray / DataView 按 byteOffset / byteLength 切），交给 `f`。
/// 不是二进制、或者 buffer 已经 detached，返回 None
pub fn with_bytes<'js, R>(v: &Value<'js>, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
    let (ab, off, len) = view(v)?;
    let bytes = ab.as_bytes()?;
    let end = off.checked_add(len)?;
    bytes.get(off..end).map(f)
}

/// 二进制值 → (底下的 ArrayBuffer, 字节偏移, 字节长度)。
///
/// 先按类判、再取：`ArrayBuffer::from_value` 对不是 ArrayBuffer 的值走 `JS_GetArrayBuffer`，后者会**抛一个 TypeError**
/// （建错误对象、拼回溯、格式化文案）再返回空 —— WebGL 的 uniform / bufferData 传的都是 TypedArray，
/// 每次调用都白白抛一次，桌面实测 `uniformMatrix4fv` 一次 3.4µs 里大半花在这。TypedArray 直接问引擎要
/// buffer / 偏移 / 长度（`JS_GetTypedArrayBuffer`），不再按名字读三个属性；DataView 照旧读属性
fn view<'js>(v: &Value<'js>) -> Option<(ArrayBuffer<'js>, usize, usize)> {
    let raw = v.as_raw();
    // SAFETY：只读值的标签 / 类
    if unsafe { qjs::JS_IsArrayBuffer(raw) } {
        let ab = ArrayBuffer::from_value(v.clone())?;
        let n = ab.len();
        return Some((ab, 0, n));
    }
    let obj: &Object = v.as_object()?;
    if unsafe { qjs::JS_GetTypedArrayType(raw) } >= 0 {
        let (mut off, mut len, mut bpe) = (0usize, 0usize, 0usize);
        // SAFETY：上面判过是 TypedArray（不会抛）；返回的 buffer 是新引用，交给 Value 管
        let b = unsafe { qjs::JS_GetTypedArrayBuffer(v.ctx().as_raw().as_ptr(), raw, &mut off as *mut usize as _, &mut len as *mut usize as _, &mut bpe as *mut usize as _) };
        if unsafe { qjs::JS_IsException(b) } {
            return None;
        }
        let ab = ArrayBuffer::from_value(unsafe { Value::from_raw(v.ctx().clone(), b) })?;
        return Some((ab, off, len));
    }
    // DataView：没有对应的 C 接口，按属性读（不是二进制的普通对象读不到 buffer，在这里就返回）
    let buffer: Value = obj.get("buffer").ok()?;
    if !unsafe { qjs::JS_IsArrayBuffer(buffer.as_raw()) } {
        return None;
    }
    let ab = ArrayBuffer::from_value(buffer)?;
    let off: usize = obj.get::<_, f64>("byteOffset").ok()? as usize;
    let len: usize = obj.get::<_, f64>("byteLength").ok()? as usize;
    Some((ab, off, len))
}

/// 拷一份出来（要跨调用保存时）
pub fn bytes_of(v: &Value<'_>) -> Option<Vec<u8>> {
    with_bytes(v, |b| b.to_vec())
}

/// Rust 字节 → 新的 ArrayBuffer（一次拷贝）。超过 [`MAX_BRIDGE_BYTES`] 抛 RangeError
pub fn array_buffer<'js>(ctx: &Ctx<'js>, bytes: Vec<u8>) -> JsResult<Value<'js>> {
    if bytes.len() > MAX_BRIDGE_BYTES {
        return Err(Exception::throw_range(ctx, &format!("单次过桥的二进制不能超过 {} 字节，实际 {}", MAX_BRIDGE_BYTES, bytes.len())));
    }
    Ok(ArrayBuffer::new(ctx.clone(), bytes)?.into_value())
}

/// 要求参数是二进制：不是的话抛 TypeError（文案带上参数名，排查时一眼能看出传错了什么）
pub fn require_bytes<'js, R>(ctx: &Ctx<'js>, v: Option<&Value<'js>>, what: &str, f: impl FnOnce(&[u8]) -> R) -> JsResult<R> {
    let v = v.ok_or_else(|| Exception::throw_type(ctx, &format!("{what} 必填（ArrayBuffer / TypedArray）")))?;
    if let Some(len) = with_bytes(v, |b| b.len()) {
        if len > MAX_BRIDGE_BYTES {
            return Err(Exception::throw_range(ctx, &format!("单次过桥的二进制不能超过 {MAX_BRIDGE_BYTES} 字节，实际 {len}")));
        }
    }
    with_bytes(v, f).ok_or_else(|| Exception::throw_type(ctx, &format!("{what} 应当是 ArrayBuffer / TypedArray / DataView（且未 detached）")))
}

/// 可写地借出一个二进制值的字节（readPixels / getBufferSubData 往里写）。规则与 [`with_bytes`] 相同：
/// 只在当次原生调用里有效，期间不能回调 JS
pub fn with_bytes_mut<'js, R>(v: &Value<'js>, f: impl FnOnce(&mut [u8]) -> R) -> Option<R> {
    let (ab, off, len) = view(v)?;
    let raw = ab.as_raw()?;
    let end = off.checked_add(len)?;
    if end > raw.len {
        return None;
    }
    // SAFETY: raw 指向这块 ArrayBuffer 的存储（未 detached，as_raw 判过），[off, end) 在界内；
    // 借用期间不回调 JS，没有别人会同时读写或释放它
    Some(f(unsafe { std::slice::from_raw_parts_mut(raw.ptr.as_ptr().add(off), len) }))
}
