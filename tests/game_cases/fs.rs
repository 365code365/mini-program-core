//! GT-FS1~FS5（game-doc/05 §3）：ArrayBuffer 直通、readCompressedFile(br)、readZipEntry、配额、路径越权、storage 上限。
use super::*;

const JS: &str = r#"
var T = require('_lib/t.js');
var fs = wx.getFileSystemManager(), U = wx.env.USER_DATA_PATH, r = {};
function fnv(buf) { var a = new Uint8Array(buf), h = 0x811c9dc5; for (var i = 0; i < a.length; i++) { h ^= a[i]; h = Math.imul(h, 16777619) >>> 0; } return h >>> 0; }
function err(f) { try { f(); return null; } catch (e) { return String(e.message); } }
var b64 = 0, e0 = __b64encode, d0 = __b64decode;
globalThis.__b64encode = function (x) { b64++; return e0(x); };
globalThis.__b64decode = function (x) { b64++; return d0(x); };
// FS1：2MB ArrayBuffer 写进去读出来，相同、不经 base64
var big = new Uint8Array(2 * 1024 * 1024); for (var i = 0; i < big.length; i++) big[i] = (i * 13 + 1) & 255;
fs.writeFileSync(U + '/big.bin', big.buffer);
var back = fs.readFileSync(U + '/big.bin');
r.fs1 = { isAB: back instanceof ArrayBuffer, len: back.byteLength, same: fnv(back) === fnv(big.buffer), b64: b64 };
r.fs1slice = new Uint8Array(fs.readFileSync(U + '/big.bin', undefined, 3, 2)).join(',');
r.fs1text = fs.readFileSync(U + '/big.bin', 'hex', 0, 2);
// FS2
var c = fs.readCompressedFileSync({ filePath: 'data/a.br', compressionAlgorithm: 'br' });
r.fs2 = { isAB: c instanceof ArrayBuffer, len: c.byteLength, text: String.fromCharCode.apply(null, new Uint8Array(c.slice(0, 5))) };
r.fs2bad = err(function () { fs.readCompressedFileSync({ filePath: 'data/a.br', compressionAlgorithm: 'gzip' }); });
fs.readCompressedFile({ filePath: 'data/a.br', compressionAlgorithm: 'br', success: function (res) { T.ev('fs2async', { len: res.data.byteLength }); } });
// readZipEntry
fs.readZipEntry({ filePath: 'data/z.zip', entries: [{ path: 'a.txt', encoding: 'utf-8' }, { path: 'd/b.bin' }, { path: 'nope' }],
  success: function (res) {
    var e = res.entries;
    T.ev('zip', { a: e['a.txt'].data, b: new Uint8Array(e['d/b.bin'].data).join(','), nope: e['nope'].errMsg });
  } });
fs.readZipEntry({ filePath: 'data/z.zip', entries: 'all', encoding: 'utf-8', success: function (res) { T.ev('zipall', { keys: Object.keys(res.entries).sort() }); } });
// FS5：越权
r.fs5 = [err(function () { fs.writeFileSync('../x.txt', 'a'); }), err(function () { fs.writeFileSync('wxfile://store/x', 'a'); }),
         err(function () { fs.writeFileSync('game.js', 'a'); }), err(function () { fs.readFileSync('/etc/passwd', 'utf8'); })];
// FS3：配额（测试把上限压到 3MB）：再写 2MB 超了
r.fs3 = err(function () { fs.writeFileSync(U + '/big2.bin', big.buffer); });
r.fs3ok = err(function () { fs.writeFileSync(U + '/big.bin', big.buffer); }); // 覆盖同名文件不算新增
fs.writeFile({ filePath: U + '/big3.bin', data: big.buffer, fail: function (e) { T.ev('fs3async', e); } });
// FS4：storage
var mb = new Array(1024 * 1024 + 1).join('a');
r.fs4item = err(function () { wx.setStorageSync('k', mb); });
var half = new Array(900 * 1024).join('b');
var total = null;
for (var k = 0; k < 12 && !total; k++) total = err(function () { wx.setStorageSync('t' + k, half); });
r.fs4total = total; r.fs4n = k;
r.fs4ok = err(function () { wx.setStorageSync('t0', 'small'); });
wx.setStorage({ key: 'k', data: mb, fail: function (e) { T.ev('fs4async', e); } });
T.ev('r', r);
T.ready('fs');
"#;

#[test]
fn fs1_fs5_文件与存储() {
    let _g = serial();
    std::env::set_var("MINI_FS_QUOTA_BYTES", (3 * 1024 * 1024).to_string());
    let mut h = GameHarness::open_inline("fs", "", Opts::default());
    let data = h.dir.join("data");
    std::fs::create_dir_all(&data).unwrap();
    {
        use std::io::Write;
        let mut br = Vec::new();
        let mut w = brotli::CompressorWriter::new(&mut br, 4096, 5, 22);
        w.write_all(&b"hello brotli ".repeat(1000)).unwrap();
        drop(w);
        std::fs::write(data.join("a.br"), br).unwrap();
    }
    std::fs::write(data.join("z.zip"), stored_zip(&[("a.txt", b"zip text"), ("d/b.bin", &[1, 2, 3])])).unwrap();
    let r = h.engine.eval(&format!("(function(){{ var module = {{}}; {JS} }})()"));
    std::env::remove_var("MINI_FS_QUOTA_BYTES");
    r.unwrap();
    h.vsyncs(3);
    h.assert_no_fail();
    let r = h.events("r").pop().expect("没有结果");
    assert_eq!(r["fs1"], serde_json::json!({ "isAB": true, "len": 2 * 1024 * 1024, "same": true, "b64": 0 }), "FS1");
    assert_eq!(r["fs1slice"], "40,53", "position 3、length 2：(3*13+1)&255=40, (4*13+1)=53");
    assert_eq!(r["fs1text"], "010e");
    assert_eq!(r["fs2"], serde_json::json!({ "isAB": true, "len": 13000, "text": "hello" }), "FS2");
    assert!(r["fs2bad"].as_str().unwrap().contains("compressionAlgorithm"));
    assert_eq!(h.events("fs2async")[0]["len"], 13000);
    let z = h.events("zip").pop().expect("readZipEntry 没回调");
    assert_eq!((z["a"].as_str(), z["b"].as_str()), (Some("zip text"), Some("1,2,3")));
    assert!(z["nope"].as_str().unwrap().contains("fail"));
    assert_eq!(h.events("zipall")[0]["keys"], serde_json::json!(["a.txt", "d/b.bin"]));
    for (i, e) in r["fs5"].as_array().unwrap().iter().enumerate() {
        let e = e.as_str().unwrap_or_else(|| panic!("FS5 第 {i} 项应当失败"));
        assert!(e.contains("permission denied") || e.contains("no such file"), "FS5 第 {i} 项：{e}");
    }
    assert!(r["fs3"].as_str().unwrap().contains("the maximum size of the file storage limit is exceeded"), "FS3：{}", r["fs3"]);
    assert!(r["fs3ok"].is_null(), "覆盖同名文件不算新增：{}", r["fs3ok"]);
    assert!(h.events("fs3async")[0]["errMsg"].as_str().unwrap().starts_with("writeFile:fail the maximum size"));
    assert!(r["fs4item"].as_str().unwrap().contains("exceed storage item max length"), "FS4 单键：{}", r["fs4item"]);
    assert!(r["fs4total"].as_str().unwrap().contains("exceed storage max size"), "FS4 总量：{}", r["fs4total"]);
    assert_eq!(r["fs4n"], 12, "900KB 一个，第 12 个超 10MB");
    assert!(r["fs4ok"].is_null(), "改小已有的键不受总量限制");
    assert!(h.events("fs4async")[0]["errMsg"].as_str().unwrap().starts_with("setStorage:fail exceed"));
}

/// 只含存储条目的 zip（与 wxfs::zip 的单测同一个做法）
fn stored_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, body) in files {
        let off = out.len() as u32;
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(body);
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&(body.len() as u32).to_le_bytes());
        central.extend_from_slice(&(body.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        central.extend_from_slice(&off.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let cd_off = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_off.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}
