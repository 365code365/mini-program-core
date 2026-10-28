// g-subpkg（game-doc/05 GT-P）：主包 + 目录分包 stage1 + 单文件分包 one + 独立分包 indep + 入口抛错的 broken。
// 每个入口执行时记一行 T:EV run {who}；go(step) 由测试驱动。
var T = require('_lib/t.js');
GameGlobal.__runs = (GameGlobal.__runs || 0) + 1;
T.ev('run', { who: 'main', n: GameGlobal.__runs });
wx.onError(function (e) { T.ev('onError', { message: e.message }); });
function load(name, tag) {
  var prog = [];
  var task = wx.loadSubpackage({
    name: name,
    success: function (r) { T.ev('load', { tag: tag, ok: true, errMsg: r.errMsg, prog: prog, ran: GameGlobal.__ran || [] }); },
    fail: function (r) { T.ev('load', { tag: tag, ok: false, errMsg: r.errMsg, prog: prog }); }
  });
  task.onProgressUpdate(function (p) { prog.push(p.progress); });
}
globalThis.go = function (step) {
  if (step === 'pre') {
    var err = '';
    try { require('stage1/util.js'); } catch (e) { err = e.message; }
    T.ev('pre', { err: err });
  }
  if (step === 'a') load('stage1', 'byName');
  if (step === 'b') load('one.js', 'byRoot');
  if (step === 'c') load('stage1', 'again');
  if (step === 'd') load('nope', 'missing');
  if (step === 'e') load('broken', 'broken');
};
T.ready('g-subpkg');
