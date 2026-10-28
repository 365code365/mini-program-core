// g-worker（game-doc/05 GT-W）：真线程 Worker。测试经 go(cmd, …) 驱动，Worker 的回复记成 T:EV w {...}
var T = require('_lib/t.js');
var w = null;
function fnv(u8) { var h = 2166136261; for (var i = 0; i < u8.length; i++) { h ^= u8[i]; h = Math.imul(h, 16777619) >>> 0; } return h; }
function hook(x, tag) {
  x.onMessage(function (m) {
    var o = { tag: tag, cmd: m.cmd };
    if (m.cmd === 'echoed') {
      var d = m.data;
      o.info = m.info;
      o.back = { sum: d.buf ? fnv(new Uint8Array(d.buf)) : 0, len: d.buf ? d.buf.byteLength : 0, sameBuf: d.view ? d.view.buffer === d.buf : null,
        nan: d.nan !== d.nan, negZero: 1 / d.negZero === -Infinity, undef: 'u' in d && d.u === undefined, big: typeof d.big === 'bigint' && String(d.big),
        date: d.date instanceof Date && d.date.getTime(), map: d.map instanceof Map && d.map.get('k'), set: d.set instanceof Set && d.set.has(3),
        cyc: d.self === d, re: d.re instanceof RegExp && d.re.flags, viewType: d.view && d.view.constructor.name };
    } else { for (var k in m) if (k !== 'cmd') o[k] = m[k]; }
    T.ev('w', o);
    // auto=1（真机无人值守）：ready → 3s 后 Worker 死算 2s（量主线程顿挫）→ 1MB 往返。
    // 不用 16MB：主线程上逐字节算校验和在手机的 QuickJS 里要 ~2s，量到的是夹具自己的开销（16MB 留给无头 W2）
    if (T.q.auto) {
      if (m.cmd === 'ready') setTimeout(function () { go('busy', 2000); }, 3000);
      if (m.cmd === 'busyDone') go('echo', { mb: 1, transfer: true, back: true });
    }
  });
  x.onError(function (e) { T.ev('w', { tag: tag, cmd: 'onError', message: e.message }); });
  x.onProcessKilled(function () { T.ev('w', { tag: tag, cmd: 'killed' }); });
}
globalThis.go = function (cmd, arg) {
  if (cmd === 'create') { w = wx.createWorker('workers/index.js'); hook(w, arg || 'a'); return; }
  if (cmd === 'badPath') { try { wx.createWorker('nope/index.js'); } catch (e) { T.ev('w', { cmd: 'badPath', msg: e.message }); } return; }
  if (cmd === 'terminate') { w.terminate(); return; }
  if (cmd === 'echo') {
    var n = arg.mb * 1024 * 1024, buf = new ArrayBuffer(n), u = new Uint8Array(buf);
    for (var i = 0; i < n; i += 4096) u[i] = (i / 4096) & 255;
    u[n - 1] = 7;
    var data = { buf: buf, view: new Float32Array(buf, 8, 4), nan: NaN, negZero: -0, u: undefined, big: 12345678901234567890n,
      date: new Date(1700000000000), map: new Map([['k', 'v']]), set: new Set([1, 3]), re: /a+/gi };
    data.self = data;
    var sum = fnv(u);
    w.postMessage({ cmd: 'echo', data: data, transferBack: !!arg.back }, arg.transfer ? [buf] : undefined);
    T.ev('w', { cmd: 'sent', sum: sum, lenAfter: buf.byteLength });
    return;
  }
  if (cmd === 'fn') { try { w.postMessage({ f: function () {} }); } catch (e) { T.ev('w', { cmd: 'fnErr', name: e.name }); } return; }
  w.postMessage({ cmd: cmd, ms: arg });
};
var frames = 0, cv = wx.createCanvas(), g = cv.getContext('2d');
// 每帧画一个移动的方块：真机量「Worker 死算期间主线程画面是否照常」用（android-game-jank.sh）
(function tick() {
  frames++;
  g.fillStyle = '#102030'; g.fillRect(0, 0, cv.width, cv.height);
  g.fillStyle = '#4fc3f7'; g.fillRect((frames * 4) % (cv.width - 40), 200, 40, 40);
  requestAnimationFrame(tick);
})();
globalThis.frames = function () { return frames; };
if (T.q.auto) go('create', 'auto');
T.ready('g-worker');
