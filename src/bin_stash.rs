//! 长连接收到的二进制（WebSocket 帧、TCP / UDP 包）不经 base64 过桥（game-doc/02 §6.3、05 GT-N5）。
//!
//! 后台线程把字节放进这里（按「种类 + 连接 id」排队），事件 JSON 里只带一个 `"bin": true`；
//! JS 收到事件后调 `__native_bin_take(种类, id)` 拿 `ArrayBuffer`（js/binary.rs 直接构造，零 base64）。
//! 发送方向：`__native_socket_send_bin` / `__native_tcp_write_bin` / `__native_udp_send_bin` / `__native_udp_write_bin`
//! 直接收 ArrayBuffer / TypedArray。从前收发都是 base64：1MB 的帧要编一遍、当 JS 字符串字面量 eval 一遍、再解一遍。
//!
//! 种类：1 = WebSocket，2 = TCP / UDP。连接关掉时调用方清掉它的队列（[`clear`]）。

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

pub const WS: u8 = 1;
pub const NET: u8 = 2;

fn store() -> &'static Mutex<HashMap<(u8, u32), VecDeque<Vec<u8>>>> {
    static S: OnceLock<Mutex<HashMap<(u8, u32), VecDeque<Vec<u8>>>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn stash(kind: u8, id: u32, bytes: Vec<u8>) {
    if let Ok(mut s) = store().lock() {
        s.entry((kind, id)).or_default().push_back(bytes);
    }
}

pub fn take(kind: u8, id: u32) -> Option<Vec<u8>> {
    let mut s = store().lock().ok()?;
    let q = s.get_mut(&(kind, id))?;
    let v = q.pop_front();
    if q.is_empty() {
        s.remove(&(kind, id));
    }
    v
}

pub fn clear(kind: u8, id: u32) {
    if let Ok(mut s) = store().lock() {
        s.remove(&(kind, id));
    }
}

/// JS 入口：取字节、二进制发送
pub fn register(rt: &crate::js::JsRuntime) -> Result<(), String> {
    use crate::js::binary::{array_buffer, require_bytes};
    use rquickjs::{IntoJs, Value};
    let num = |v: Option<&Value>| v.and_then(|x| x.as_number()).unwrap_or(0.0);
    let text = |v: Option<&Value>| v.and_then(|x| x.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
    rt.register_raw("__native_bin_take", move |ctx, a| match take(num(a.0.first()) as u8, num(a.0.get(1)) as u32) {
        Some(b) => array_buffer(&ctx, b),
        None => Ok(Value::new_null(ctx)),
    })?;
    // 返回 "" 或错误串（与字符串版同一个约定）
    rt.register_raw("__native_socket_send_bin", move |ctx, a| {
        let id = num(a.0.first()) as u32;
        let bytes = require_bytes(&ctx, a.0.get(1), "data", |b| b.to_vec())?;
        crate::websocket::send_bytes(id, bytes).err().unwrap_or_default().into_js(&ctx)
    })?;
    rt.register_raw("__native_tcp_write_bin", move |ctx, a| {
        let id = num(a.0.first()) as u32;
        let bytes = require_bytes(&ctx, a.0.get(1), "data", |b| b.to_vec())?;
        crate::net_socket::tcp_write_bytes(id, bytes).err().unwrap_or_default().into_js(&ctx)
    })?;
    // (id, address, port, bytes, broadcast)
    rt.register_raw("__native_udp_send_bin", move |ctx, a| {
        let id = num(a.0.first()) as u32;
        let bytes = require_bytes(&ctx, a.0.get(3), "message", |b| b.to_vec())?;
        let bc = a.0.get(4).and_then(|v| v.as_bool()).unwrap_or(false);
        crate::net_socket::udp_send_bytes(id, &text(a.0.get(1)), num(a.0.get(2)) as u16, bytes, bc).err().unwrap_or_default().into_js(&ctx)
    })?;
    rt.register_raw("__native_udp_write_bin", move |ctx, a| {
        let id = num(a.0.first()) as u32;
        let bytes = require_bytes(&ctx, a.0.get(1), "message", |b| b.to_vec())?;
        crate::net_socket::udp_write_bytes(id, bytes).err().unwrap_or_default().into_js(&ctx)
    })?;
    Ok(())
}
