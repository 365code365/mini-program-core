// 独立分包：从这里启动时主包还没执行；__GAME__ 加载后主包 game.js 才跑。
// 独立分包里 require 不到主包的模块（包括 _lib/t.js，与微信一致），日志行自己拼
function ev(name, o) { console.log('T:EV ' + name + ' ' + JSON.stringify(o)); }
ev('run', { who: 'indep', mainRan: !!GameGlobal.__runs });
var err = '';
try { require('/stage1/util.js'); } catch (e) { err = e.message; }
var errLib = '';
try { require('/_lib/t.js'); } catch (e) { errLib = e.message; }
ev('indepPre', { err: err, errLib: errLib });
globalThis.goMain = function () {
  wx.loadSubpackage({ name: '__GAME__', success: function (r) { ev('main', { ok: true, runs: GameGlobal.__runs, errMsg: r.errMsg }); },
    fail: function (r) { ev('main', { ok: false, errMsg: r.errMsg }); } });
};
console.log('T:READY g-subpkg-indep');
