// 索敌 / 范围 / 伤害：网格分桶 + WXWebAssembly（wasm/combat.wasm，源码 wasm/combat.wat）。
//
// - 分桶：每个模拟步把活着的敌人按所在格子放进桶（桶 = 1 格，数组复用不分配）。塔索敌只看射程方框里的那几个桶，
//   候选写进 wasm 内存，再调一次 pick / within；场上 100 个敌人、30 座塔也只算几十个候选。
// - wasm：开局 instantiate，拿同一组随机输入与 JS 版对一遍（包括 JS 写内存 → wasm 读、wasm 写内存 → JS 读两个方向），
//   一致才用；没有 WXWebAssembly、加载失败或结果不一致，一律用同一套算法的 JS 版（__play().wasm 里看得到是哪个）。
// - 只在塔「冷却好了要开火」时才索敌（每秒几十次调用），不每步每塔都算。
var MAX = 512, OUT = 8192;

function jsImpl() {
  var buf = new ArrayBuffer(65536), f32 = new Float32Array(buf), i32 = new Int32Array(buf);
  return {
    f32: f32, i32: i32,
    pick: function (n, tx, ty, r2) {
      var bi = -1, best = -1;
      for (var k = 0; k < n; k++) {
        var dx = f32[k * 4] - tx, dy = f32[k * 4 + 1] - ty;
        if (Math.fround(dx * dx + dy * dy) <= r2 && f32[k * 4 + 2] > best) { best = f32[k * 4 + 2]; bi = k; }
      }
      return bi;
    },
    within: function (n, cx, cy, r2) {
      var cnt = 0;
      for (var k = 0; k < n; k++) {
        var dx = f32[k * 4] - cx, dy = f32[k * 4 + 1] - cy;
        if (Math.fround(dx * dx + dy * dy) <= r2) i32[OUT / 4 + cnt++] = k;
      }
      return cnt;
    },
    dmg: function (base, armor, pierce) { return pierce ? base : Math.max(base * 0.25, base - armor); }
  };
}

function make(COLS, ROWS) {
  var impl = jsImpl(), mode = 'js', calls = 0, note = '';
  var buckets = [], cand = new Array(MAX), hits = new Array(MAX);
  for (var b = 0; b < COLS * ROWS; b++) buckets.push([]);

  function init(done) {
    if (typeof WXWebAssembly === 'undefined' || !WXWebAssembly.instantiate) { note = '没有 WXWebAssembly'; done(mode); return; }
    WXWebAssembly.instantiate('wasm/combat.wasm', {}).then(function (r) {
      var ex = r.instance.exports, w = { f32: new Float32Array(ex.mem.buffer), i32: new Int32Array(ex.mem.buffer),
        pick: ex.pick, within: ex.within, dmg: ex.dmg };
      if (selfCheck(w, impl)) { impl = w; mode = 'wasm'; } else note = 'wasm 与 JS 结果不一致，用 JS 版';
      done(mode);
    }, function (e) { note = '加载失败：' + e; done(mode); });
  }
  // 同一组输入两边各算一遍
  function selfCheck(w, j) {
    try {
      var n = 40, ok = true;
      for (var k = 0; k < n * 4; k++) w.f32[k] = j.f32[k] = Math.fround(Math.random() * 9);
      for (var t = 0; t < 8 && ok; t++) {
        var x = Math.random() * 9, y = Math.random() * 12, r2 = 1 + Math.random() * 6;
        if (w.pick(n, x, y, r2) !== j.pick(n, x, y, r2)) ok = false;
        var a = w.within(n, x, y, r2), b = j.within(n, x, y, r2);
        if (a !== b) ok = false;
        // 视图开局取一次就一直用：本引擎的 mem.buffer 零拷贝指向线性内存，模块不 grow 就不会搬家、不会 detach
        for (var q = 0; q < a && ok; q++) if (w.i32[OUT / 4 + q] !== j.i32[OUT / 4 + q]) ok = false;
        if (Math.abs(w.dmg(20 + t, 6, 0) - j.dmg(20 + t, 6, 0)) > 1e-4 || w.dmg(3, 9, 0) !== 0.75 || w.dmg(7, 9, 1) !== 7) ok = false;
      }
      return ok;
    } catch (e) { note = '自检出错：' + e; return false; }
  }

  // 每个模拟步开头重建桶（不分配：数组清长度）
  function rebuild(enemies) {
    for (var b = 0; b < buckets.length; b++) buckets[b].length = 0;
    for (var i = 0; i < enemies.length; i++) {
      var e = enemies[i];
      if (!e.alive) continue;
      var c = Math.max(0, Math.min(COLS - 1, Math.floor(e.x))), r = Math.max(0, Math.min(ROWS - 1, Math.floor(e.y)));
      buckets[r * COLS + c].push(e);
    }
  }
  // 方框里的桶 → 候选写进内存；filter(e) 为假的跳过（炮塔不打飞行）
  function gather(x, y, range, filter) {
    var n = 0, f = impl.f32;
    var c0 = Math.max(0, Math.floor(x - range)), c1 = Math.min(COLS - 1, Math.floor(x + range));
    var r0 = Math.max(0, Math.floor(y - range)), r1 = Math.min(ROWS - 1, Math.floor(y + range));
    for (var r = r0; r <= r1; r++) for (var c = c0; c <= c1; c++) {
      var bk = buckets[r * COLS + c];
      for (var k = 0; k < bk.length && n < MAX; k++) {
        var e = bk[k];
        if (!e.alive || (filter && !filter(e))) continue;
        f[n * 4] = e.x; f[n * 4 + 1] = e.y; f[n * 4 + 2] = e.d;
        cand[n++] = e;
      }
    }
    return n;
  }
  function pick(x, y, range, filter) {
    var n = gather(x, y, range, filter);
    if (!n) return null;
    calls++;
    var i = impl.pick(n, x, y, range * range);
    return i >= 0 ? cand[i] : null;
  }
  // 圆内的敌人依次交给 fn(e)
  function each(x, y, range, filter, fn) {
    var n = gather(x, y, range, filter);
    if (!n) return 0;
    calls++;
    var cnt = impl.within(n, x, y, range * range);
    // 先把命中的敌人抄出来再回调：fn 里可以再 pick（会覆盖内存与 cand），但不能嵌套 each（hits 只有一份）
    var ix = impl.i32;
    for (var k = 0; k < cnt; k++) hits[k] = cand[ix[OUT / 4 + k]];
    for (var h = 0; h < cnt; h++) { var e = hits[h]; hits[h] = null; fn(e); }
    return cnt;
  }
  function dmg(base, armor, pierce) { calls++; return impl.dmg(base, armor, pierce ? 1 : 0); }

  return { init: init, rebuild: rebuild, pick: pick, each: each, dmg: dmg,
    info: function () { return { mode: mode, calls: calls, note: note }; } };
}

module.exports = { make: make };
