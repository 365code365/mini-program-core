// g-worker 的 Worker 端：按 cmd 干活，结果 postMessage 回去
var util = require('./util.js');
function fnv(u8) { var h = 2166136261; for (var i = 0; i < u8.length; i++) { h ^= u8[i]; h = Math.imul(h, 16777619) >>> 0; } return h; }
worker.onMessage(function (m) {
  if (m.cmd === 'busy') {
    var t0 = Date.now(), n = 0;
    while (Date.now() - t0 < m.ms) n++;
    worker.postMessage({ cmd: 'busyDone', n: n > 0 });
  } else if (m.cmd === 'echo') {
    var d = m.data;
    var info = { sum: d.buf ? fnv(new Uint8Array(d.buf)) : 0, len: d.buf ? d.buf.byteLength : 0, sameBuf: d.view ? d.view.buffer === d.buf : null };
    worker.postMessage({ cmd: 'echoed', data: d, info: info }, m.transferBack && d.buf ? [d.buf] : undefined);
    if (m.transferBack && d.buf) worker.postMessage({ cmd: 'afterTransfer', len: d.buf.byteLength });
  } else if (m.cmd === 'env') {
    worker.postMessage({ cmd: 'env', wx: typeof wx, worker: typeof worker, console: typeof console, setTimeout: typeof setTimeout,
      require: typeof require, util: util.name, canvas: typeof wx !== 'undefined' && typeof wx.createCanvas });
  } else if (m.cmd === 'timer') {
    setTimeout(function () { worker.postMessage({ cmd: 'timer', ok: true }); }, 30);
  } else if (m.cmd === 'throw') {
    throw new Error('worker 里故意抛错');
  } else if (m.cmd === 'kill') {
    worker.testOnProcessKilled();
  } else if (m.cmd === 'log') {
    console.log('T:EV workerLog {"hi":1}');
  }
});
worker.postMessage({ cmd: 'ready' });
