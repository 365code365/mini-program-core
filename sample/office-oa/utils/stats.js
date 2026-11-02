// 考勤统计入口：优先放到 wx.createWorker('workers/stats.js') 里算；没有 worker 能力 / 出错 / 800ms 没回包时在主线程算（同一份 stats-core）。
const core = require('../workers/stats-core.js');

let w = null, seq = 0;
const pending = {};

function ensureWorker() {
  if (w || !wx.createWorker) return w;
  try {
    w = wx.createWorker('workers/stats.js');
    w.onMessage(res => {
      const p = pending[res.id];
      if (!p) return;
      delete pending[res.id];
      clearTimeout(p.timer);
      if (res.error) p.fallback('worker 报错：' + res.error); else p.resolve({ stats: res.stats, via: 'worker' });
    });
    w.onError && w.onError(err => console.warn('[stats] worker 出错，改主线程', err && err.message));
  } catch (e) {
    console.warn('[stats] createWorker 失败，改主线程', e && e.message);
    w = null;
  }
  return w;
}

function compute(month, records, today) {
  return new Promise(resolve => {
    const local = why => resolve({ stats: core.monthStats(month, records, today), via: 'main', why });
    const wk = ensureWorker();
    if (!wk) return local('不支持 createWorker');
    const id = ++seq;
    pending[id] = {
      resolve,
      fallback: local,
      timer: setTimeout(() => { if (pending[id]) { delete pending[id]; local('worker 超时'); } }, 800)
    };
    wk.postMessage({ id, month, records, today });
  });
}

function stop() { if (w) { w.terminate(); w = null; } }

module.exports = { compute, stop };
