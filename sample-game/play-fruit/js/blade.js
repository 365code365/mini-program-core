// 刀光：wx.onTouchMove 来的每个点进环形缓冲（带时刻），画成从新到旧逐渐变细、变淡的带子；每来一个点就拿「上一个点 → 这个点」
// 这一段去切（world.slice）。触摸事件比帧密（真机 120Hz 采样），所以判定按事件做、不按帧做：快划时一帧里的几段都算。
var N = 32, LIFE = 0.16;
var px = new Float32Array(N), py = new Float32Array(N), pt = new Float32Array(N);
var head = 0, len = 0, id = -1, lastX = 0, lastY = 0, speed = 0;
var Q = [[0, 0], [0, 0], [0, 0], [0, 0]];                  // 每段四边形复用同一个数组

function push(x, y, t) { head = (head + 1) % N; px[head] = x; py[head] = y; pt[head] = t; if (len < N) len++; }
// 返回 true = 这根手指现在握着刀
function down(x, y, ident, t) {
  if (id !== -1) return false;
  id = ident; len = 0; lastX = x; lastY = y; speed = 0; push(x, y, t);
  return true;
}
// 返回这一段的长度（0 = 不是握刀的那根手指）；调用方拿 (x0, y0) → (x, y) 去切
var seg = { x0: 0, y0: 0, x1: 0, y1: 0 };
function move(x, y, ident, t) {
  if (ident !== id) return null;
  seg.x0 = lastX; seg.y0 = lastY; seg.x1 = x; seg.y1 = y;
  var dx = x - lastX, dy = y - lastY;
  speed = Math.sqrt(dx * dx + dy * dy);
  lastX = x; lastY = y; push(x, y, t);
  return seg;
}
function up(ident) { if (ident === id) id = -1; }

function draw(R, now) {
  if (len < 2) return;
  pass(R, now, 2.4, 'rgba(160,220,255,0.35)');
  pass(R, now, 1, '#ffffff');
}
function pass(R, now, k, color) {
  var i = head, prev = -1, wPrev = 0;
  for (var n = 0; n < len; n++, i = (i - 1 + N) % N) {
    var age = now - pt[i];
    if (age > LIFE) break;
    var w = 4.5 * k * (1 - age / LIFE);
    if (prev >= 0) {
      var dx = px[prev] - px[i], dy = py[prev] - py[i], l = Math.sqrt(dx * dx + dy * dy);
      if (l > 0.5) {
        var nx = -dy / l, ny = dx / l;
        Q[0][0] = px[prev] + nx * wPrev; Q[0][1] = py[prev] + ny * wPrev;
        Q[1][0] = px[i] + nx * w; Q[1][1] = py[i] + ny * w;
        Q[2][0] = px[i] - nx * w; Q[2][1] = py[i] - ny * w;
        Q[3][0] = px[prev] - nx * wPrev; Q[3][1] = py[prev] - ny * wPrev;
        R.poly(Q, color);
      }
    } else if (age < LIFE * 0.5) R.circle(px[i], py[i], w, color);   // 刀尖圆头
    prev = i; wPrev = w;
  }
}
function active() { return id !== -1; }
function count(now) { var c = 0, i = head; for (var n = 0; n < len; n++, i = (i - 1 + N) % N) { if (now - pt[i] > LIFE) break; c++; } return c; }

module.exports = { down: down, move: move, up: up, draw: draw, active: active, count: count, speed: function () { return speed; } };
