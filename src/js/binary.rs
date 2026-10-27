//! 二进制过桥（game-doc/02 §6）：JS 的 ArrayBuffer / TypedArray / DataView 与 Rust 字节之间直接转换，
//! 不再经 base64 字符串（从前所有二进制都走纯 JS 的逐字节 base64，见 prelude/codec.js）。
//!
//! 借出的字节只在当次原生调用里有效：调用期间不能回调 JS（回调可能 detach / 改写这块内存），
//! 所以 [`with_bytes`] 只给一个闭包作用域，要跨调用保存就自己拷成 `Vec<u8>`。

use rquickjs::{ArrayBuffer, Ctx, Exception, Object, Result as JsResult, Value};

/// 单次过桥的上限：内存紧张时一次拷贝就能把进程推过线，宁可明确报 RangeError
pub const MAX_BRIDGE_BYTES: usize = 256 * 1024 * 1024;

/// 取出一个二进制值的字节（ArrayBuffer 整块；TypedArray / DataView 按 byteOffset / byteLength 切），交给 `f`。
/// 不是二进制、或者 buffer 已经 detached，返回 None
pub fn with_bytes<'js, R>(v: &Value<'js>, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
    if let Some(ab) = ArrayBuffer::from_value(v.clone()) {
        return ab.as_bytes().map(f);
    }
    // 所有视图（Uint8Array…Float64Array、DataView）都有这三个属性：一处处理，不按元素类型分十几个分支
    let obj: &Object = v.as_object()?;
    let buffer: Value = obj.get("buffer").ok()?;
    let ab = ArrayBuffer::from_value(buffer)?;
    let off: usize = obj.get::<_, f64>("byteOffset").ok()? as usize;
    let len: usize = obj.get::<_, f64>("byteLength").ok()? as usize;
    let bytes = ab.as_bytes()?;
    let end = off.checked_add(len)?;
    bytes.get(off..end).map(f)
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
