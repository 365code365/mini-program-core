//! GT-WASM（G10）：WXWebAssembly 的 JS API —— 路径、导入导出、i64 ↔ BigInt、Memory（零拷贝、grow 之后旧 buffer detached）、
//! Table / funcref、Global、陷入与异常、JS 导入函数里重入 wasm、.wasm.br。WAT 现场编成 .wasm 写进夹具目录。
use super::*;

fn open_wasm(name: &str, mods: &[(&str, &str)], js: &str) -> GameHarness {
    open_wasm_with(name, "{}", mods, js)
}

fn open_wasm_with(name: &str, game_json: &str, mods: &[(&str, &str)], js: &str) -> GameHarness {
    let mut h = GameHarness::open_inline_with(name, game_json, js, Opts::default());
    // 夹具目录已建好：把模块写进去后再跑一段（game.js 里只定义 run()）
    for (file, wat) in mods {
        let p = h.dir.join(file);
        if file.ends_with(".js") {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, wat).unwrap();
            continue;
        }
        let bin = wat::parse_str(wat).unwrap_or_else(|e| panic!("{file}: {e}"));
        if file.ends_with(".br") {
            let mut out = Vec::new();
            brotli::BrotliCompress(&mut bin.as_slice(), &mut out, &Default::default()).unwrap();
            std::fs::write(p, out).unwrap();
        } else {
            std::fs::write(p, bin).unwrap();
        }
    }
    if let Err(e) = h.engine.eval("run()") {
        panic!("run() 抛错：{e}\n{:?}", h.log);
    }
    h.vsyncs(3);
    if !game_json.contains("workers") && h.log.iter().all(|l| !l.contains("T:EV r ")) {
        panic!("没有结果：{:#?}\n{:?}", h.log.iter().rev().take(15).collect::<Vec<_>>(), h.errors);
    }
    h
}

const MATH: &str = r#"(module
  (import "env" "log" (func $log (param i32)))
  (import "env" "cb" (func $cb (param i32) (result i32)))
  (memory (export "mem") 1 4)
  (global $g (export "g") (mut i32) (i32.const 7))
  (table (export "tab") 2 funcref)
  (elem (i32.const 0) $add $twice)
  (func $add (export "add") (param i32 i32) (result i32) local.get 0 local.get 1 i32.add)
  (func $twice (export "twice") (param i32) (result i32) local.get 0 call $cb i32.const 2 i32.mul)
  (func (export "mul64") (param i64 i64) (result i64) local.get 0 local.get 1 i64.mul)
  (func (export "f") (param f64) (result f64 i32) local.get 0 f64.sqrt i32.const 9)
  (func (export "store") (param i32 i32) local.get 0 local.get 1 i32.store8)
  (func (export "grow") (param i32) (result i32) local.get 0 memory.grow)
  (func (export "trap") (result i32) i32.const 1 i32.const 0 i32.div_s)
  (func (export "callIndirect") (param i32 i32) (result i32) local.get 1 local.get 1 local.get 0 call_indirect (param i32 i32) (result i32))
  (func (export "say") i32.const 42 call $log)
  (func (export "bumpG") global.get $g i32.const 1 i32.add global.set $g)
)"#;

#[test]
fn wasm1_导入导出与类型() {
    let _g = serial();
    let h = open_wasm("wasm1", &[("m.wasm", MATH)], r#"
var T = require('_lib/t.js');
globalThis.run = function () {
    var logs = [], ex;
    var imports = { env: { log: function (x) { logs.push(x); }, cb: function (x) { return ex.add(x, 100); } } };
    WXWebAssembly.instantiate('m.wasm', imports).then(function (r) {
        ex = r.instance.exports;
        var r1 = { add: ex.add(2, 3), addWrap: ex.add(0x7fffffff, 1), len: ex.add.length, mul64: String(ex.mul64(3000000000n, 4n)),
            multi: ex.f(16), say: (ex.say(), logs), twice: ex.twice(1), g0: ex.g.value };
        ex.bumpG(); r1.g1 = ex.g.value; ex.g.value = 50; r1.g2 = ex.g.value;
        try { ex.mul64(1, 2); r1.i64num = 'ok'; } catch (e) { r1.i64num = e instanceof TypeError; }
        try { ex.trap(); } catch (e) { r1.trap = e instanceof WXWebAssembly.RuntimeError; }
        r1.tab = [ex.tab.length, ex.tab.get(0)(5, 6), ex.tab.get(1) !== null, ex.callIndirect(0, 21)];
        r1.mod = r.module.exports.length > 5;
        r1.frozen = Object.isFrozen(ex);
        T.ev('r', r1);
    }, function (e) { T.ev('fail', { e: String(e) }); });
};
T.ready('wasm1');
"#);
    let r = &h.events("r")[0];
    assert_eq!(r, &serde_json::json!({ "add": 5, "addWrap": -2147483648i64, "len": 2, "mul64": "12000000000", "multi": [4, 9], "say": [42],
        "twice": 202, "g0": 7, "g1": 8, "g2": 50, "i64num": true, "trap": true, "tab": [2, 11, true, 42], "mod": true, "frozen": true }), "{:?}", h.events("fail"));
}

#[test]
fn wasm2_内存零拷贝与grow() {
    let _g = serial();
    let h = open_wasm("wasm2", &[("m.wasm", MATH)], r#"
var T = require('_lib/t.js');
globalThis.run = function () {
    WXWebAssembly.instantiate('m.wasm', { env: { log: function () {}, cb: function (x) { return x; } } }).then(function (r) {
        var ex = r.instance.exports, m = ex.mem, b0 = m.buffer, v = new Uint8Array(b0);
        ex.store(10, 77);
        var o = { seen: v[10], same: m.buffer === b0, len0: b0.byteLength };
        v[20] = 5;
        o.wasmSees = new Uint8Array(m.buffer)[20];
        o.old = ex.grow(1);
        o.detached = b0.byteLength;
        o.len1 = m.buffer.byteLength;
        o.keep = new Uint8Array(m.buffer)[10];
        o.jsGrow = m.grow(1); o.len2 = m.buffer.byteLength;
        try { m.grow(5); } catch (e) { o.over = e instanceof RangeError; }
        var mm = new WXWebAssembly.Memory({ initial: 1, maximum: 2 }); o.own = mm.buffer.byteLength;
        T.ev('r', o);
    });
};
T.ready('wasm2');
"#);
    assert_eq!(h.events("r")[0], serde_json::json!({ "seen": 77, "same": true, "len0": 65536, "wasmSees": 5, "old": 1, "detached": 0,
        "len1": 131072, "keep": 77, "jsGrow": 2, "len2": 196608, "over": true, "own": 65536 }));
}

#[test]
fn wasm3_链接错误_路径_br_异常穿透() {
    let _g = serial();
    let h = open_wasm("wasm3", &[("m.wasm.br", MATH), ("bad.wasm", "(module)")], r#"
var T = require('_lib/t.js');
globalThis.run = function () {
    var out = {};
    var p = [];
    p.push(WXWebAssembly.instantiate('m.wasm.br', { env: { log: function () {}, cb: function () { throw new Error('boom'); } } }).then(function (r) {
        try { r.instance.exports.twice(1); } catch (e) { out.thrown = e.message; }
        out.br = r.instance.exports.add(1, 1);
    }));
    p.push(WXWebAssembly.instantiate('m.wasm.br', {}).catch(function (e) { out.noMod = e instanceof TypeError; }));
    p.push(WXWebAssembly.instantiate('m.wasm.br', { env: { log: 1, cb: function () {} } }).catch(function (e) { out.link = e instanceof WXWebAssembly.LinkError; }));
    p.push(WXWebAssembly.instantiate('../x.wasm', {}).catch(function (e) { out.escape = e instanceof WXWebAssembly.CompileError && e.message.indexOf('代码包') >= 0; }));
    p.push(WXWebAssembly.instantiate('nope.wasm', {}).catch(function (e) { out.missing = e instanceof WXWebAssembly.CompileError; }));
    p.push(WXWebAssembly.instantiate('bad.wasm', {}).then(function (r) { out.empty = Object.keys(r.instance.exports).length; }));
    Promise.all(p).then(function () { T.ev('r', out); });
};
T.ready('wasm3');
"#);
    assert_eq!(h.events("r")[0], serde_json::json!({ "thrown": "boom", "br": 2, "noMod": true, "link": true, "escape": true, "missing": true, "empty": 0 }));
}

/// WASM4：Worker 里用 WXWebAssembly（.wasm 在 Worker 目录外，路径相对包根）
#[test]
fn wasm4_worker() {
    let _g = serial();
    let base = mini_render::game::GameEngine::live_worker_threads();
    let mut h = open_wasm_with("wasm4", r#"{ "workers": "workers" }"#, &[("m.wasm", MATH), ("workers/index.js", r#"
WXWebAssembly.instantiate('m.wasm', { env: { log: function () {}, cb: function (x) { return x + 1; } } }).then(function (r) {
    worker.postMessage({ add: r.instance.exports.add(20, 22), twice: r.instance.exports.twice(4) });
}, function (e) { worker.postMessage({ err: String(e) }); });
"#)], r#"
var T = require('_lib/t.js');
globalThis.run = function () {
    var w = globalThis.w = wx.createWorker('workers/index.js');
    w.onMessage(function (m) { T.ev('r', m); });
};
T.ready('wasm4');
"#);
    h.until("T:EV r ", 200);
    assert_eq!(h.events("r")[0], serde_json::json!({ "add": 42, "twice": 10 }));
    // 线程收干净再走：后面的 Worker 用例按「进程里还活着几个 Worker 线程」判
    h.engine.eval("w.terminate()").unwrap();
    let t0 = std::time::Instant::now();
    while mini_render::game::GameEngine::live_worker_threads() > base && t0.elapsed().as_secs() < 5 {
        h.vsyncs(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// WASM5：g-wasm 基准夹具（结果与 JS 同一份算法一致；耗时只打印，记进验收文档）
#[test]
fn wasm5_基准夹具() {
    let _g = serial();
    let mut h = GameHarness::open("g-wasm", Opts { query: "rounds=2", ..Default::default() });
    h.until("T:EV done", 600);
    let b = h.events("bench");
    assert!(b.iter().all(|x| x["fib"][0] == true && x["sort"][0] == true), "{b:?}");
    eprintln!("g-wasm 基准（桌面 release）：{b:?}");
}
