// 考勤统计 worker：主线程 postMessage({id, month, records, today})，算完回 {id, stats}。
const core = require('./stats-core.js');

worker.onMessage(function (msg) {
  try {
    const stats = core.monthStats(msg.month, msg.records || {}, msg.today);
    worker.postMessage({ id: msg.id, stats });
  } catch (e) {
    worker.postMessage({ id: msg.id, error: String(e && e.message || e) });
  }
});
