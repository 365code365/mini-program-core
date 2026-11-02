// 补间动画：把对象的数值属性在 dur 秒里按缓动曲线从当前值走到目标值。
// 消消乐的交换 / 回弹 / 消除缩放 / 下落 / 特效块弹出都是它；一帧里 update(dt) 推进全部补间。
// 不在帧里分配：补间对象放进池子复用（一次连锁几十个补间，每帧 new 会让真机 GC 抖一下）。
var EASE = {
  linear: function (t) { return t; },
  inQuad: function (t) { return t * t; },
  outQuad: function (t) { return t * (2 - t); },
  inOutQuad: function (t) { return t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t; },
  // 回弹：先冲过头再回来（交换不成立时弹回原位、新特效块「啵」地冒出来）
  outBack: function (t) { var s = 1.70158; t -= 1; return t * t * ((s + 1) * t + s) + 1; },
  inBack: function (t) { var s = 1.70158; return t * t * ((s + 1) * t - s); },
  // 落地小弹两下（下落）
  outBounce: function (t) {
    if (t < 1 / 2.75) return 7.5625 * t * t;
    if (t < 2 / 2.75) { t -= 1.5 / 2.75; return 7.5625 * t * t + 0.75; }
    if (t < 2.5 / 2.75) { t -= 2.25 / 2.75; return 7.5625 * t * t + 0.9375; }
    t -= 2.625 / 2.75; return 7.5625 * t * t + 0.984375;
  }
};

var active = [], pool = [];
// props：{ x: 目标, y: 目标, … }；delay 秒后开始；done 在走完那一帧调用
function to(obj, props, dur, ease, delay, done) {
  var tw = pool.pop() || { keys: [], from: [], dest: [] };
  tw.obj = obj; tw.dur = Math.max(0.001, dur); tw.t = -(delay || 0);
  tw.ease = EASE[ease] || EASE.outQuad; tw.done = done || null; tw.started = false;
  tw.keys.length = 0; tw.from.length = 0; tw.dest.length = 0;
  for (var k in props) { tw.keys.push(k); tw.dest.push(props[k]); tw.from.push(0); }
  active.push(tw);
  return tw;
}
// 同一个对象上还没走完的补间全部停掉（停在当前值）
function kill(obj) {
  for (var i = active.length - 1; i >= 0; i--) if (active[i].obj === obj) { pool.push(active[i]); active.splice(i, 1); }
}
function update(dt) {
  for (var i = 0; i < active.length; i++) {
    var tw = active[i];
    tw.t += dt;
    if (tw.t < 0) continue;
    var j;
    if (!tw.started) {                              // 延迟走完才取起点：排队的补间接着上一个的终点走
      tw.started = true;
      for (j = 0; j < tw.keys.length; j++) tw.from[j] = tw.obj[tw.keys[j]];
    }
    var k = Math.min(1, tw.t / tw.dur), e = tw.ease(k);
    for (j = 0; j < tw.keys.length; j++) tw.obj[tw.keys[j]] = tw.from[j] + (tw.dest[j] - tw.from[j]) * e;
    if (k >= 1) {
      active.splice(i, 1); i--;
      var cb = tw.done, o = tw.obj;                 // 先取出来再回池：done 里新建的补间可能正好复用这一个
      tw.done = null; tw.obj = null;
      pool.push(tw);
      if (cb) cb(o);
    }
  }
}
function busy() { return active.length > 0; }
function clear() { while (active.length) pool.push(active.pop()); }

module.exports = { to: to, kill: kill, update: update, busy: busy, clear: clear, EASE: EASE };
