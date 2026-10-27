// g-wasm（G10.5）：WXWebAssembly 解释器（wasmi）与 QuickJS 同一份算法对比：fib(25) 与 2000 个整数插入排序。
// 每轮一行 T:EV bench {wasm_ms, js_ms}。bench.wasm 由 bench.wat 编出（cargo run --example 用 wat crate 编，见提交记录）
var T = require('_lib/t.js');
function jsFib(n) { return n < 2 ? n : jsFib(n - 1) + jsFib(n - 2); }
function jsSort(a) { for (var i = 1; i < a.length; i++) { var v = a[i], j = i - 1; while (j >= 0 && a[j] > v) { a[j + 1] = a[j]; j--; } a[j + 1] = v; } }
WXWebAssembly.instantiate('bench.wasm', {}).then(function (r) {
    var ex = r.instance.exports, rounds = Number(T.q.rounds || 3);
    for (var k = 0; k < rounds; k++) {
        var t0 = Date.now(), f1 = ex.fib(25), t1 = Date.now(), f2 = jsFib(25), t2 = Date.now();
        var m = new Int32Array(ex.mem.buffer), a = new Int32Array(2000);
        for (var i = 0; i < 2000; i++) a[i] = m[i] = (i * 7919) % 2003;
        var t3 = Date.now(); ex.sort(2000); var t4 = Date.now(); jsSort(a); var t5 = Date.now();
        var same = true; m = new Int32Array(ex.mem.buffer); for (i = 0; i < 2000; i++) if (m[i] !== a[i]) same = false;
        T.ev('bench', { fib: [f1 === f2, t1 - t0, t2 - t1], sort: [same, t4 - t3, t5 - t4] });
    }
    T.ev('done', {});
}, function (e) { T.ev('fail', { e: String(e) }); });
T.ready('g-wasm');
