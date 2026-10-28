//! FileSystemManager 的 G4.6 部分（game-doc/04、05 GT-FS1~FS3）：二进制直通、`readCompressedFile`、`readZipEntry`、用户文件配额。
//! 小程序与小游戏共用（都是官方 API）。从 ops.rs 拆出来，那边只管路径权限与基本文件操作。
//!
//! ## 二进制直通
//! 从前 ArrayBuffer 写入换成 `{__b64}`、读出（没给 encoding）回 base64，再在 JS 里解一遍。现在 `__native_fs_raw(op, json, bytes?)`
//! 直接收 / 回 ArrayBuffer：写入的字节先放进本线程的 [`RAW_IN`]，`payload()` 看到 `{"__raw":true}` 就从那里取；
//! 读出的字节放进 [`RAW_OUT`]，结果 JSON 里写 `"raw":true`，入口函数再把它作为第二个返回值交回 JS。
//! 用线程局部是因为 `__native_fs` 是同步调用，进出都在同一条线程上、中间不会穿插别的调用。
//!
//! ## 配额
//! `USER_DATA_PATH` 下所有文件合计不超过 200MB（官方：本地用户文件上限 200MB）。写操作在写之前按「目录现有大小 − 被覆盖文件 + 新内容」算，
//! 超了返回 `the maximum size of the file storage limit is exceeded`（官方错误码 1300202 的文案；真机的完整串待取证）。

use serde_json::{json, Value};
use std::cell::RefCell;
use std::io::Read;

thread_local! {
    pub(super) static RAW_IN: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
    pub(super) static RAW_OUT: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
}

/// 用户文件合计上限
pub const USER_QUOTA: u64 = 200 * 1024 * 1024;
pub const QUOTA_ERR: &str = "the maximum size of the file storage limit is exceeded";

/// `USER_DATA_PATH` 下的总字节数
fn usr_bytes() -> u64 {
    fn walk(p: &std::path::Path) -> u64 {
        let Ok(rd) = std::fs::read_dir(p) else { return 0 };
        rd.flatten()
            .map(|e| match e.metadata() {
                Ok(m) if m.is_dir() => walk(&e.path()),
                Ok(m) => m.len(),
                Err(_) => 0,
            })
            .sum()
    }
    super::resolve(super::USR).map(|r| walk(&r.real)).unwrap_or(0)
}

/// 写 `adding` 字节到 `target`（追加时 `append`）之后会不会超配额。测试可以用 `MINI_FS_QUOTA_BYTES` 压低上限
pub(super) fn check_quota(target: &std::path::Path, adding: u64, append: bool) -> Result<(), String> {
    let quota = std::env::var("MINI_FS_QUOTA_BYTES").ok().and_then(|v| v.parse().ok()).unwrap_or(USER_QUOTA);
    let old = if append { 0 } else { std::fs::metadata(target).map(|m| m.len()).unwrap_or(0) };
    if usr_bytes().saturating_sub(old) + adding > quota {
        return Err(QUOTA_ERR.into());
    }
    Ok(())
}

/// 读出的内容：没给 encoding 时走直通（`raw`），给了按编码转字符串
pub(super) fn content_raw(bytes: Vec<u8>, enc: &str) -> Result<Value, String> {
    if enc.is_empty() {
        RAW_OUT.with(|o| *o.borrow_mut() = Some(bytes));
        return Ok(json!({ "raw": true }));
    }
    let text = super::codec::decode(&bytes, enc).ok_or_else(|| format!("invalid encoding \"{enc}\""))?;
    Ok(json!({ "data": text }))
}

/// `readCompressedFile`：`compressionAlgorithm` 只有 `br`（官方）
pub(super) fn read_compressed(real: &std::path::Path, path: &str, algo: &str) -> Result<Value, String> {
    if algo != "br" {
        return Err(format!("invalid compressionAlgorithm \"{algo}\""));
    }
    let data = std::fs::read(real).map_err(|_| format!("no such file or directory, open {path}"))?;
    let mut out = Vec::new();
    brotli_decompressor::Decompressor::new(&data[..], 64 * 1024)
        .read_to_end(&mut out)
        .map_err(|e| format!("decompress fail: {e}"))?;
    content_raw(out, "")
}

/// `readZipEntry`：`entries` 是 `"all"` 或 `[{path, encoding, position, length}]`。
/// 返回 `{entries: {path: {data | errMsg}}}`；没给 encoding 的条目，数据按顺序放进 `bins`（JS 侧换成 ArrayBuffer）
pub(super) fn read_zip_entry(real: &std::path::Path, path: &str, entries: &Value, enc_all: &str) -> Result<Value, String> {
    let data = std::fs::read(real).map_err(|_| format!("no such file or directory, open {path}"))?;
    let all = super::zip::entries(&data)?;
    let mut out = serde_json::Map::new();
    let mut bins: Vec<Vec<u8>> = Vec::new();
    let mut put = |name: &str, body: Option<&Vec<u8>>, enc: &str, pos: Option<u64>, len: Option<u64>, out: &mut serde_json::Map<String, Value>| {
        let Some(body) = body else {
            out.insert(name.into(), json!({ "errMsg": format!("no such file or directory, open {name}") }));
            return;
        };
        let start = (pos.unwrap_or(0) as usize).min(body.len());
        let end = len.map(|l| (start + l as usize).min(body.len())).unwrap_or(body.len());
        let slice = &body[start..end];
        if enc.is_empty() {
            out.insert(name.into(), json!({ "bin": bins.len() }));
            bins.push(slice.to_vec());
        } else {
            match super::codec::decode(slice, enc) {
                Some(t) => out.insert(name.into(), json!({ "data": t })),
                None => out.insert(name.into(), json!({ "errMsg": format!("invalid encoding \"{enc}\"") })),
            };
        }
    };
    let find = |p: &str| all.iter().find(|(n, _)| n == p.trim_start_matches('/')).and_then(|(_, b)| b.as_ref());
    if entries.as_str() == Some("all") {
        for (name, body) in &all {
            if body.is_some() {
                put(name, body.as_ref(), enc_all, None, None, &mut out);
            }
        }
    } else {
        for e in entries.as_array().ok_or("entries 应当是 'all' 或数组")? {
            let p = e.get("path").and_then(Value::as_str).unwrap_or("");
            let enc = e.get("encoding").and_then(Value::as_str).unwrap_or("");
            let n = |k: &str| e.get(k).and_then(Value::as_f64).filter(|v| *v >= 0.0).map(|v| v as u64);
            put(p, find(p), enc, n("position"), n("length"), &mut out);
        }
    }
    // 多个二进制条目：拼成一块交出去，JS 按偏移切（一次过桥）
    let offsets: Vec<usize> = bins.iter().scan(0usize, |acc, b| { let o = *acc; *acc += b.len(); Some(o) }).collect();
    let lens: Vec<usize> = bins.iter().map(|b| b.len()).collect();
    if !bins.is_empty() {
        RAW_OUT.with(|o| *o.borrow_mut() = Some(bins.concat()));
    }
    Ok(json!({ "entries": out, "binOffsets": offsets, "binLens": lens, "raw": !lens.is_empty() }))
}

/// `__native_fs_raw(op, json, bytes?)` → `[json, ArrayBuffer | null]`
pub fn register(rt: &crate::js::JsRuntime) -> Result<(), String> {
    use rquickjs::{Array, Value as JsValue};
    rt.register_raw("__native_fs_raw", move |ctx, a| {
        let text = |v: Option<&JsValue>| v.and_then(|x| x.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
        let op = text(a.0.first());
        let json_in = text(a.0.get(1));
        if let Some(v) = a.0.get(2).filter(|v| !v.is_undefined() && !v.is_null()) {
            let bytes = crate::js::binary::require_bytes(&ctx, Some(v), "data", |b| b.to_vec())?;
            RAW_IN.with(|i| *i.borrow_mut() = Some(bytes));
        }
        let out = super::call(&op, &json_in);
        RAW_IN.with(|i| i.borrow_mut().take());
        let bin = RAW_OUT.with(|o| o.borrow_mut().take());
        let arr = Array::new(ctx.clone())?;
        arr.set(0, out)?;
        match bin {
            Some(b) => arr.set(1, crate::js::binary::array_buffer(&ctx, b)?)?,
            None => arr.set(1, JsValue::new_null(ctx.clone()))?,
        }
        Ok(arr.into_value())
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn br_解压往返_坏数据报错() {
        use std::io::{Read, Write};
        let src: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let mut br = Vec::new();
        {
            let mut w = brotli::CompressorWriter::new(&mut br, 4096, 5, 22);
            w.write_all(&src).unwrap();
        }
        assert!(br.len() < src.len() / 10, "样本要真的被压缩过");
        let dir = std::env::temp_dir().join(format!("br-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.br");
        std::fs::write(&f, &br).unwrap();
        let v = super::read_compressed(&f, "a.br", "br").unwrap();
        assert_eq!(v["raw"], true);
        assert_eq!(super::RAW_OUT.with(|o| o.borrow_mut().take()), Some(src));
        assert!(super::read_compressed(&f, "a.br", "gzip").is_err(), "只认 br");
        std::fs::write(&f, b"not brotli at all").unwrap();
        assert!(super::read_compressed(&f, "a.br", "br").is_err());
        let mut junk = Vec::new();
        let _ = brotli_decompressor::Decompressor::new(&b""[..], 16).read_to_end(&mut junk);
        std::fs::remove_dir_all(&dir).ok();
    }
}
