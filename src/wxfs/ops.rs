//! 文件操作本体：`call(op, json)` 按 op 分派，返回 JSON 串（形状见 `super` 的模块注释）。

use super::{codec, resolve, to_virtual, zip, Resolved, STORE};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

type R = Result<Value, String>;

fn s<'a>(a: &'a Value, k: &str) -> &'a str {
    a.get(k).and_then(Value::as_str).unwrap_or("")
}

fn n(a: &Value, k: &str) -> Option<u64> {
    a.get(k).and_then(Value::as_f64).filter(|v| *v >= 0.0).map(|v| v as u64)
}

fn b(a: &Value, k: &str) -> bool {
    a.get(k).and_then(Value::as_bool).unwrap_or(false)
}

fn not_found(op: &str, p: &str) -> String {
    format!("no such file or directory, {op} {p}")
}

fn readable(p: &str, op: &str) -> Result<Resolved, String> {
    resolve(p).ok_or_else(|| not_found(op, p))
}

fn writable(p: &str, op: &str) -> Result<Resolved, String> {
    let r = resolve(p).ok_or_else(|| format!("permission denied, {op} {p}"))?;
    if !r.writable {
        return Err(format!("permission denied, {op} {p}"));
    }
    Ok(r)
}

fn io_err(e: std::io::Error, op: &str, p: &str) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => not_found(op, p),
        std::io::ErrorKind::AlreadyExists => format!("file already exists {p}"),
        std::io::ErrorKind::PermissionDenied => format!("permission denied, {op} {p}"),
        _ => format!("{e}, {op} {p}"),
    }
}

/// 写入内容：`data` 是字符串（按 encoding）或 `{"__b64":"…"}`（JS 侧把 ArrayBuffer 换成的）
fn payload(a: &Value) -> Result<Vec<u8>, String> {
    match a.get("data") {
        // ArrayBuffer 直通（extra.rs）：字节在本线程的 RAW_IN 里
        Some(Value::Object(o)) if o.get("__raw").is_some() => super::extra::RAW_IN.with(|i| i.borrow_mut().take()).ok_or("invalid data".into()),
        Some(Value::Object(o)) => o.get("__b64").and_then(Value::as_str).and_then(codec::unb64).ok_or("invalid data".into()),
        Some(Value::String(t)) => codec::encode(t, s(a, "encoding")).ok_or_else(|| format!("invalid encoding \"{}\"", s(a, "encoding"))),
        Some(Value::Null) | None => Ok(Vec::new()),
        Some(other) => Ok(other.to_string().into_bytes()),
    }
}

/// 读出的内容：给了 encoding 返回字符串，没给返回 base64（JS 侧还原成 ArrayBuffer）
fn content(bytes: &[u8], enc: &str) -> R {
    if enc.is_empty() {
        return Ok(json!({ "b64": codec::b64(bytes) }));
    }
    let text = codec::decode(bytes, enc).ok_or_else(|| format!("invalid encoding \"{enc}\""))?;
    Ok(json!({ "data": text }))
}

fn millis(t: std::io::Result<std::time::SystemTime>) -> u64 {
    t.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0)
}

fn stats_of(m: &fs::Metadata) -> Value {
    // 微信 Stats：mode 带文件类型位（S_IFDIR 0o040000 / S_IFREG 0o100000），时间是秒
    let mode = if m.is_dir() { 0o040000 | 0o755 } else { 0o100000 | 0o644 };
    json!({
        "mode": mode, "size": if m.is_dir() { 0 } else { m.len() },
        "lastAccessedTime": millis(m.accessed()), "lastModifiedTime": millis(m.modified()),
        "isDir": m.is_dir()
    })
}

fn walk(base: &Path, rel: &str, out: &mut Vec<Value>) {
    let Ok(m) = fs::metadata(base.join(rel)) else { return };
    out.push(json!({ "path": if rel.is_empty() { "/".to_string() } else { format!("/{rel}") }, "stats": stats_of(&m) }));
    if m.is_dir() {
        if let Ok(rd) = fs::read_dir(base.join(rel)) {
            let mut names: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
            names.sort();
            for name in names {
                walk(base, &if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") }, out);
            }
        }
    }
}

// ─────────────── 文件描述符（open / read / write / close）───────────────

struct Fd {
    file: fs::File,
    append: bool,
}

fn fds() -> &'static Mutex<(u32, HashMap<u32, Fd>)> {
    static F: OnceLock<Mutex<(u32, HashMap<u32, Fd>)>> = OnceLock::new();
    F.get_or_init(|| Mutex::new((0, HashMap::new())))
}

fn open(a: &Value) -> R {
    let p = s(a, "filePath");
    let flag = if s(a, "flag").is_empty() { "r" } else { s(a, "flag") };
    let mut o = fs::OpenOptions::new();
    let excl = flag.contains('x');
    let wants_write = flag != "r";
    let r = if wants_write { writable(p, "open")? } else { readable(p, "open")? };
    match flag.trim_end_matches('s') {
        "r" => o.read(true),
        "r+" => o.read(true).write(true),
        "w" | "wx" => o.write(true).truncate(true).create(!excl).create_new(excl),
        "w+" | "wx+" => o.read(true).write(true).truncate(true).create(!excl).create_new(excl),
        "a" | "ax" => o.append(true).create(!excl).create_new(excl),
        "a+" | "ax+" => o.read(true).append(true).create(!excl).create_new(excl),
        _ => return Err(format!("invalid flag \"{flag}\"")),
    };
    let file = o.open(&r.real).map_err(|e| io_err(e, "open", p))?;
    let mut t = fds().lock().unwrap();
    t.0 += 1;
    let fd = t.0;
    t.1.insert(fd, Fd { file, append: flag.starts_with('a') });
    Ok(json!({ "fd": fd.to_string() }))
}

fn with_fd<T>(a: &Value, f: impl FnOnce(&mut Fd) -> Result<T, String>) -> Result<T, String> {
    let fd: u32 = s(a, "fd").parse().map_err(|_| "bad file descriptor".to_string())?;
    let mut t = fds().lock().unwrap();
    let e = t.1.get_mut(&fd).ok_or("bad file descriptor")?;
    f(e)
}

// ─────────────── 分派 ───────────────

/// `__native_fs(op, json)` 的入口
pub fn call(op: &str, args_json: &str) -> String {
    let a: Value = serde_json::from_str(args_json).unwrap_or(Value::Null);
    let r = dispatch(op, &a);
    match r {
        Ok(mut v) => {
            if let Value::Object(m) = &mut v {
                m.insert("ok".into(), Value::Bool(true));
                v.to_string()
            } else {
                json!({ "ok": true, "value": v }).to_string()
            }
        }
        Err(e) => json!({ "ok": false, "err": e }).to_string(),
    }
}

fn dispatch(op: &str, a: &Value) -> R {
    // 微信里 USER_DATA_PATH 总是存在（readdir / writeFile 到根下不需要先 mkdir）
    if let Some(r) = resolve(super::USR) {
        fs::create_dir_all(&r.real).ok();
    }
    match op {
        "access" => {
            let p = s(a, "path");
            let r = readable(p, "access")?;
            r.real.exists().then(|| json!({})).ok_or_else(|| not_found("access", p))
        }
        "stat" => {
            let p = s(a, "path");
            let r = readable(p, "stat")?;
            let m = fs::metadata(&r.real).map_err(|e| io_err(e, "stat", p))?;
            if b(a, "recursive") && m.is_dir() {
                let mut out = Vec::new();
                walk(&r.real, "", &mut out);
                return Ok(json!({ "list": out }));
            }
            Ok(json!({ "stats": stats_of(&m) }))
        }
        "readFile" => {
            let p = s(a, "filePath");
            let r = readable(p, "open")?;
            if r.real.is_dir() {
                return Err(format!("illegal operation on a directory, open {p}"));
            }
            let mut bytes = fs::read(&r.real).map_err(|e| io_err(e, "open", p))?;
            let start = (n(a, "position").unwrap_or(0) as usize).min(bytes.len());
            let end = n(a, "length").map(|l| (start + l as usize).min(bytes.len())).unwrap_or(bytes.len());
            bytes = bytes[start..end].to_vec();
            if b(a, "raw") {
                return super::extra::content_raw(bytes, s(a, "encoding"));
            }
            content(&bytes, s(a, "encoding"))
        }
        "writeFile" | "appendFile" => {
            let p = s(a, "filePath");
            let r = writable(p, "open")?;
            let data = payload(a)?;
            if op == "appendFile" && !r.real.exists() {
                return Err(not_found("open", p));
            }
            super::extra::check_quota(&r.real, data.len() as u64, op == "appendFile")?;
            let mut f = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(op == "appendFile")
                .truncate(op == "writeFile")
                .open(&r.real)
                .map_err(|e| io_err(e, "open", p))?;
            f.write_all(&data).map_err(|e| io_err(e, "write", p))?;
            Ok(json!({}))
        }
        "copyFile" => {
            let (sp, dp) = (s(a, "srcPath"), s(a, "destPath"));
            let src = readable(sp, "copyFile")?;
            let dst = writable(dp, "copyFile")?;
            super::extra::check_quota(&dst.real, fs::metadata(&src.real).map(|m| m.len()).unwrap_or(0), false)?;
            fs::copy(&src.real, &dst.real).map_err(|e| io_err(e, "copyFile", sp))?;
            Ok(json!({}))
        }
        "rename" => {
            let (op_, np) = (s(a, "oldPath"), s(a, "newPath"));
            let old = writable(op_, "rename")?;
            let new = writable(np, "rename")?;
            fs::rename(&old.real, &new.real).map_err(|e| io_err(e, "rename", op_))?;
            Ok(json!({}))
        }
        "unlink" => {
            let p = s(a, "filePath");
            let r = writable(p, "unlink")?;
            if r.real.is_dir() {
                return Err(format!("operation not permitted, unlink {p}"));
            }
            fs::remove_file(&r.real).map_err(|e| io_err(e, "unlink", p))?;
            Ok(json!({}))
        }
        "mkdir" => {
            let p = s(a, "dirPath");
            let r = writable(p, "mkdir")?;
            if r.real.exists() {
                return Err(format!("file already exists {p}"));
            }
            let res = if b(a, "recursive") { fs::create_dir_all(&r.real) } else { fs::create_dir(&r.real) };
            res.map_err(|e| io_err(e, "mkdir", p))?;
            Ok(json!({}))
        }
        "rmdir" => {
            let p = s(a, "dirPath");
            let r = writable(p, "rmdir")?;
            if !r.real.is_dir() {
                return Err(if r.real.exists() { format!("not a directory {p}") } else { not_found("rmdir", p) });
            }
            let res = if b(a, "recursive") { fs::remove_dir_all(&r.real) } else { fs::remove_dir(&r.real) };
            res.map_err(|e| if r.real.exists() { format!("directory not empty {p}") } else { io_err(e, "rmdir", p) })?;
            Ok(json!({}))
        }
        "readdir" => {
            let p = s(a, "dirPath");
            let r = readable(p, "scandir")?;
            if !r.real.is_dir() {
                return Err(if r.real.exists() { format!("not a directory {p}") } else { not_found("scandir", p) });
            }
            let mut files: Vec<String> = fs::read_dir(&r.real)
                .map_err(|e| io_err(e, "scandir", p))?
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            files.sort();
            Ok(json!({ "files": files }))
        }
        "truncate" => {
            let p = s(a, "filePath");
            let r = writable(p, "open")?;
            let f = fs::OpenOptions::new().write(true).open(&r.real).map_err(|e| io_err(e, "open", p))?;
            f.set_len(n(a, "length").unwrap_or(0)).map_err(|e| io_err(e, "truncate", p))?;
            Ok(json!({}))
        }
        "getFileInfo" => {
            let p = s(a, "filePath");
            let r = readable(p, "open")?;
            let bytes = fs::read(&r.real).map_err(|e| io_err(e, "open", p))?;
            let digest = if s(a, "digestAlgorithm").eq_ignore_ascii_case("sha1") { codec::sha1_hex(&bytes) } else { codec::md5_hex(&bytes) };
            Ok(json!({ "size": bytes.len(), "digest": digest }))
        }
        "saveFile" => {
            let tp = s(a, "tempFilePath");
            let src = readable(tp, "open")?;
            if !src.real.is_file() {
                return Err(not_found("open", tp));
            }
            let (virt, real) = if s(a, "filePath").is_empty() {
                let name = src.real.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
                let v = format!("{STORE}/{name}");
                (v.clone(), resolve(&v).ok_or("bad path")?.real)
            } else {
                let fp = s(a, "filePath");
                (fp.to_string(), writable(fp, "open")?.real)
            };
            if let Some(dir) = real.parent() {
                fs::create_dir_all(dir).ok();
            }
            // 微信 saveFile 是「移动」：临时文件之后不可再用
            if fs::rename(&src.real, &real).is_err() {
                fs::copy(&src.real, &real).map_err(|e| io_err(e, "saveFile", tp))?;
                fs::remove_file(&src.real).ok();
            }
            Ok(json!({ "savedFilePath": virt }))
        }
        "getSavedFileList" => {
            let dir = resolve(STORE).ok_or("bad path")?.real;
            let mut list = Vec::new();
            if let Ok(rd) = fs::read_dir(&dir) {
                for e in rd.flatten() {
                    if let Ok(m) = e.metadata() {
                        if m.is_file() {
                            let path = to_virtual(&e.path()).unwrap_or_default();
                            list.push(json!({ "filePath": path, "size": m.len(), "createTime": millis(m.modified()) }));
                        }
                    }
                }
            }
            list.sort_by(|x, y| x["filePath"].as_str().cmp(&y["filePath"].as_str()));
            Ok(json!({ "fileList": list }))
        }
        "removeSavedFile" => {
            let p = s(a, "filePath");
            let r = readable(p, "unlink")?;
            if !p.starts_with(STORE) && !r.writable {
                return Err(format!("permission denied, unlink {p}"));
            }
            fs::remove_file(&r.real).map_err(|e| io_err(e, "unlink", p))?;
            Ok(json!({}))
        }
        "unzip" => {
            let (zp, tp) = (s(a, "zipFilePath"), s(a, "targetPath"));
            let src = readable(zp, "unzip")?;
            let dst = writable(tp, "unzip")?;
            let data = fs::read(&src.real).map_err(|e| io_err(e, "unzip", zp))?;
            let entries = zip::entries(&data)?;
            for (name, body) in entries {
                if !zip::safe_rel(name.trim_end_matches('/')) {
                    return Err(format!("unzip fail: illegal entry {name}"));
                }
                let out = dst.real.join(name.trim_end_matches('/'));
                match body {
                    None => fs::create_dir_all(&out).map_err(|e| io_err(e, "unzip", tp))?,
                    Some(bytes) => {
                        if let Some(d) = out.parent() {
                            fs::create_dir_all(d).map_err(|e| io_err(e, "unzip", tp))?;
                        }
                        fs::write(&out, bytes).map_err(|e| io_err(e, "unzip", tp))?;
                    }
                }
            }
            Ok(json!({}))
        }
        "readCompressedFile" => {
            let p = s(a, "filePath");
            let r = readable(p, "open")?;
            super::extra::read_compressed(&r.real, p, s(a, "compressionAlgorithm"))
        }
        "readZipEntry" => {
            let p = s(a, "filePath");
            let r = readable(p, "open")?;
            super::extra::read_zip_entry(&r.real, p, a.get("entries").unwrap_or(&Value::Null), s(a, "encoding"))
        }
        "open" => open(a),
        "close" => {
            let fd: u32 = s(a, "fd").parse().map_err(|_| "bad file descriptor".to_string())?;
            fds().lock().unwrap().1.remove(&fd).map(|_| json!({})).ok_or("bad file descriptor".into())
        }
        "fstat" => with_fd(a, |f| Ok(json!({ "stats": stats_of(&f.file.metadata().map_err(|e| e.to_string())?) }))),
        "ftruncate" => with_fd(a, |f| {
            f.file.set_len(n(a, "length").unwrap_or(0)).map_err(|e| e.to_string())?;
            Ok(json!({}))
        }),
        "read" => with_fd(a, |f| {
            if let Some(pos) = n(a, "position") {
                f.file.seek(SeekFrom::Start(pos)).map_err(|e| e.to_string())?;
            }
            let len = n(a, "length").unwrap_or(0) as usize;
            let mut buf = vec![0u8; len];
            let got = f.file.read(&mut buf).map_err(|e| e.to_string())?;
            buf.truncate(got);
            Ok(json!({ "b64": codec::b64(&buf), "bytesRead": got }))
        }),
        "write" => with_fd(a, |f| {
            let data = payload(a)?;
            if let (Some(pos), false) = (n(a, "position"), f.append) {
                f.file.seek(SeekFrom::Start(pos)).map_err(|e| e.to_string())?;
            }
            f.file.write_all(&data).map_err(|e| e.to_string())?;
            Ok(json!({ "bytesWritten": data.len() }))
        }),
        _ => Err(format!("unknown operation {op}")),
    }
}
