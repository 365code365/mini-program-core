// Worker 入口（game.json 的 "workers": "workers"）：收主线程的 { cmd: 'think', id, moves, side, level }，
// 算完回 { cmd: 'move', id, move, score, nodes, ms }。id 原样带回：主线程悔棋 / 新局之后，迟到的结果按 id 丢掉
var ai = require('./ai.js');
worker.onMessage(function (m) {
  if (!m || m.cmd !== 'think') return;
  var r = ai.think(m.moves || [], m.side, m.level);
  worker.postMessage({ cmd: 'move', id: m.id, move: r.move, score: r.score, nodes: r.nodes, ms: r.ms });
});
worker.postMessage({ cmd: 'ready' });
