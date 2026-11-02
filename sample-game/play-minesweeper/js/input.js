// 手势识别：单指点按 / 长按（自己算时长）/ 拖动平移，双指捏合缩放（按 e.touches 里前两根算距离与中点）。
// PC：右键 = 长按（插旗），滚轮 = 以指针为中心缩放。
// 回调：tap(x, y)、long(x, y)、pan(dx, dy)、pinch(k, cx, cy, dx, dy)（k = 这一下的缩放倍数）、down(x, y)、up()。
// 为什么不用 base.js 那种「抬手时归一成 tap / swipe」：长按要在手指还按着时就触发（到点就插旗、震一下），
// 双指要看 touches 里同时在屏上的全部触点，归一之后这些都没了。
var LONG_MS = 380, SLOP = 10;
var cb = {}, mode = 'none', one = null, pinch = null, suppressUntil = 0, stats = { taps: 0, longs: 0, pinches: 0, pans: 0 };

function now() { return Date.now(); }
function emit(name, a, b, c, d, e) { var f = cb[name]; if (f) f(a, b, c, d, e); }
function two(ts) {
  var a = ts[0], b = ts[1];
  var dx = b.clientX - a.clientX, dy = b.clientY - a.clientY;
  return { d: Math.max(1, Math.sqrt(dx * dx + dy * dy)), x: (a.clientX + b.clientX) / 2, y: (a.clientY + b.clientY) / 2 };
}

wx.onTouchStart(function (e) {
  var ts = e.touches || [];
  if (ts.length >= 2) {
    // 第二根手指落下：单指的点按 / 长按作废，转成捏合
    if (one) emit('up');
    one = null; mode = 'pinch';
    pinch = two(ts); stats.pinches++;
    return;
  }
  var p = e.changedTouches[0];
  if (!p || mode === 'pinch') return;
  mode = 'one';
  one = { id: p.identifier, sx: p.clientX, sy: p.clientY, x: p.clientX, y: p.clientY, t: now(), moved: false, fired: false };
  emit('down', p.clientX, p.clientY);
});
wx.onTouchMove(function (e) {
  var ts = e.touches || [];
  if (mode === 'pinch') {
    if (ts.length < 2) return;
    var q = two(ts);
    emit('pinch', q.d / pinch.d, q.x, q.y, q.x - pinch.x, q.y - pinch.y);
    pinch = q;
    return;
  }
  if (mode !== 'one' || !one) return;
  var p = null;
  for (var i = 0; i < e.changedTouches.length; i++) if (e.changedTouches[i].identifier === one.id) p = e.changedTouches[i];
  if (!p) return;
  if (!one.moved) {
    var dx = p.clientX - one.sx, dy = p.clientY - one.sy;
    if (dx * dx + dy * dy > SLOP * SLOP) { one.moved = true; stats.pans++; emit('up'); }
  }
  if (one.moved && !one.fired) emit('pan', p.clientX - one.x, p.clientY - one.y);
  one.x = p.clientX; one.y = p.clientY;
});
function ended(e, cancelled) {
  var ts = e.touches || [];
  if (mode === 'pinch') {
    // 捏合松开一根：剩下那根不接着当点按（否则松手那一下会误翻格子），全部抬起才回到空闲
    if (!ts.length) { mode = 'none'; pinch = null; }
    else if (ts.length >= 2) pinch = two(ts);
    return;
  }
  if (mode !== 'one' || !one) return;
  var o = one; one = null; mode = 'none';
  emit('up');
  if (cancelled || o.moved || o.fired || now() < suppressUntil) return;
  stats.taps++;
  emit('tap', o.x, o.y);
}
wx.onTouchEnd(function (e) { ended(e, false); });
wx.onTouchCancel(function (e) { ended(e, true); });

if (wx.onMouseDown) wx.onMouseDown(function (e) {
  if (e.button !== 2) return;
  suppressUntil = now() + 300;
  stats.longs++;
  emit('long', e.x, e.y);
});
if (wx.onWheel) wx.onWheel(function (e) {
  if (!e.deltaY) return;
  emit('pinch', Math.exp(-e.deltaY * 0.0015), e.x, e.y, 0, 0);
});

// 每帧调一次：手指按住不动到点就算长按（不等抬手）
function update() {
  if (mode !== 'one' || !one || one.moved || one.fired) return;
  if (now() - one.t >= LONG_MS) {
    one.fired = true; stats.longs++;
    emit('up');
    emit('long', one.x, one.y);
  }
}
// 按住了多久（0~1，画长按进度环用）；没在按返回 -1
function holdProgress() {
  if (mode !== 'one' || !one || one.moved || one.fired) return -1;
  return Math.min(1, (now() - one.t) / LONG_MS);
}

module.exports = {
  on: function (name, f) { cb[name] = f; }, update: update, holdProgress: holdProgress,
  holding: function () { return mode === 'one' && one && !one.moved && !one.fired ? one : null; },
  stats: stats
};
