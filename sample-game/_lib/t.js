// 夹具共用的小工具：机器可读日志 + 确定性随机数 + 断言。只依赖 wx 与 console。
var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
function line(s) { console.log(s); }
function j(o) { try { return JSON.stringify(o); } catch (e) { return '"<unserializable>"'; } }
var seed = (Number(q.seed) || 1) >>> 0;
function rand() { // mulberry32：跨平台位精确，Chrome 参考实现跑出同一串
  seed = (seed + 0x6D2B79F5) >>> 0;
  var t = seed;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
}
module.exports = {
  q: q,
  ready: function (name) { line('T:READY ' + name); },
  frame: function (n, o) { line('T:FRAME ' + n + ' ' + j(o)); },
  ev: function (name, o) { line('T:EV ' + name + ' ' + j(o)); },
  ok: function (name) { line('T:OK ' + name); },
  fail: function (name, why) { line('T:FAIL ' + name + ' ' + why); },
  check: function (name, cond, why) { cond ? this.ok(name) : this.fail(name, why || 'assert'); return !!cond; },
  done: function (o) { line('T:DONE ' + j(o || {})); },
  rand: rand,
  j: j
};
