// g-plugin（game-doc/11 GT-PL）：引擎插件 eng（pluginDir 优先、包内 local-eng 兜底）+ 没装的 ghost
var T = require('_lib/t.js');
var eng = requirePlugin('eng');
T.ev('eng', eng);
T.ev('sub', { v: requirePlugin('eng/util').v });
['ghost', 'nope'].forEach(function (n) {
  try { requirePlugin(n); T.ev('miss', { n: n, threw: false }); } catch (e) { T.ev('miss', { n: n, threw: true, msg: e.message }); }
});
wx.onError(function (e) { T.ev('onError', { message: e.message }); });
setTimeout(function () { requirePlugin('ghost'); }, 0);
T.ready('g-plugin');
