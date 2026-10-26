// g-hello：运行模式骨架（GT-R1、R5、S4；A1.1）。期望见 README.md
var T = require('_lib/t.js');
console.log('hello', 1, true, null, undefined, { a: [1, 2] });
console.warn('warn-line');
console.error(new Error('boom-for-test'));
wx.onError(function (e) { T.ev('onError', { msg: String(e.message).slice(0, 40) }); });
wx.onUnhandledRejection(function (e) { T.ev('unhandled', { reason: String(e.reason) }); });
Promise.reject('nope');
// 晚一行就挂上处理者的拒绝不算未处理（浏览器 / 微信的微任务检查点语义）
Promise.reject('handled-late').catch(function () {});
setTimeout(function () { throw new Error('in-timer'); }, 0);
T.check('GameGlobal', typeof GameGlobal === 'object' && GameGlobal === globalThis);
T.check('no-page', typeof Page === 'undefined' && typeof App === 'undefined');
T.ready('g-hello');
setTimeout(function () { T.done({}); }, 50);
