// 特效：消行碎块粒子 + 飘字（「四消！」「连击 ×2」）。对象池预先建好，帧里不分配（真机 jemalloc 带锁，分配会顿）
var MAXP = 96, MAXT = 4;
function Fx() {
  this.p = []; this.t = [];
  for (var i = 0; i < MAXP; i++) this.p.push({ on: false, x: 0, y: 0, vx: 0, vy: 0, life: 0, max: 1, s: 4, c: '#fff' });
  for (var j = 0; j < MAXT; j++) this.t.push({ on: false, s: '', x: 0, y: 0, life: 0, c: '#fff', size: 20 });
}
Fx.prototype.burst = function (x, y, color, n) {
  for (var i = 0, k = 0; i < MAXP && k < n; i++) {
    var q = this.p[i]; if (q.on) continue;
    var a = Math.random() * Math.PI * 2, v = 60 + Math.random() * 180;
    q.on = true; q.x = x; q.y = y; q.vx = Math.cos(a) * v; q.vy = Math.sin(a) * v - 120;
    q.max = q.life = 0.45 + Math.random() * 0.35; q.s = 3 + Math.random() * 4; q.c = color; k++;
  }
};
Fx.prototype.say = function (s, x, y, color, size) {
  var slot = this.t[0];
  for (var i = 0; i < MAXT; i++) { if (!this.t[i].on) { slot = this.t[i]; break; } if (this.t[i].life < slot.life) slot = this.t[i]; }
  slot.on = true; slot.s = s; slot.x = x; slot.y = y; slot.life = 1.1; slot.c = color; slot.size = size || 22;
};
Fx.prototype.update = function (dt) {
  for (var i = 0; i < MAXP; i++) {
    var q = this.p[i]; if (!q.on) continue;
    q.life -= dt; if (q.life <= 0) { q.on = false; continue; }
    q.vy += 520 * dt; q.x += q.vx * dt; q.y += q.vy * dt;
  }
  for (var j = 0; j < MAXT; j++) { var t = this.t[j]; if (t.on) { t.life -= dt; t.y -= 30 * dt; if (t.life <= 0) t.on = false; } }
};
Fx.prototype.draw = function (R) {
  for (var i = 0; i < MAXP; i++) {
    var q = this.p[i]; if (!q.on) continue;
    R.setAlpha(Math.min(1, q.life / q.max * 1.5));
    R.rect(q.x - q.s / 2, q.y - q.s / 2, q.s, q.s, q.c);
  }
  for (var j = 0; j < MAXT; j++) {
    var t = this.t[j]; if (!t.on) continue;
    R.setAlpha(Math.min(1, t.life * 2));
    R.text(t.s, t.x, t.y, t.size, t.c, 'center', 'bold', ['#1a1f4a', 2]);
  }
  R.setAlpha(1);
};
Fx.prototype.clear = function () { for (var i = 0; i < MAXP; i++) this.p[i].on = false; for (var j = 0; j < MAXT; j++) this.t[j].on = false; };
module.exports = { Fx: Fx };
