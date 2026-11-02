// 特效：果汁碎屑（粒子）、飘字（+分数 / 连击）、条纹光束、炸弹光圈。全部预分配进池子，帧里不 new。
var PMAX = 260, FMAX = 12, BMAX = 16;
var parts = [], floats = [], beams = [];
for (var i = 0; i < PMAX; i++) parts.push({ on: false, x: 0, y: 0, vx: 0, vy: 0, life: 0, max: 1, color: '#fff', size: 4 });
for (var j = 0; j < FMAX; j++) floats.push({ on: false, x: 0, y: 0, life: 0, max: 1, text: '', color: '#fff', size: 18 });
for (var k = 0; k < BMAX; k++) beams.push({ on: false, kind: 'h', x: 0, y: 0, life: 0, max: 1 });
var pi = 0, fi = 0, bi = 0;

// 一个水果碎成 n 片：随机方向往外崩，带重力
function burst(x, y, color, n) {
  for (var q = 0; q < n; q++) {
    var p = parts[pi]; pi = (pi + 1) % PMAX;
    var a = Math.random() * Math.PI * 2, sp = 60 + Math.random() * 160;
    p.on = true; p.x = x; p.y = y; p.vx = Math.cos(a) * sp; p.vy = Math.sin(a) * sp - 80;
    p.life = p.max = 0.45 + Math.random() * 0.3; p.color = color; p.size = 3 + Math.random() * 4;
  }
}
function float(text, x, y, color, size) {
  var f = floats[fi]; fi = (fi + 1) % FMAX;
  f.on = true; f.text = text; f.x = x; f.y = y; f.life = f.max = 0.9; f.color = color || '#ffffff'; f.size = size || 20;
}
// kind：'h' 一整行的光带、'v' 一整列、'ring' 炸弹光圈、'rainbow' 全屏彩光
function beam(kind, x, y) {
  var b = beams[bi]; bi = (bi + 1) % BMAX;
  b.on = true; b.kind = kind; b.x = x; b.y = y; b.life = b.max = kind === 'rainbow' ? 0.6 : 0.35;
}
function update(dt) {
  var q;
  for (q = 0; q < PMAX; q++) {
    var p = parts[q]; if (!p.on) continue;
    p.life -= dt; if (p.life <= 0) { p.on = false; continue; }
    p.vy += 520 * dt; p.x += p.vx * dt; p.y += p.vy * dt;
  }
  for (q = 0; q < FMAX; q++) { var f = floats[q]; if (f.on) { f.life -= dt; f.y -= 40 * dt; if (f.life <= 0) f.on = false; } }
  for (q = 0; q < BMAX; q++) { var b = beams[q]; if (b.on) { b.life -= dt; if (b.life <= 0) b.on = false; } }
}
// 光束与碎屑画在水果上面、HUD 下面；box = 棋盘 { x, y, w, h }
function draw(R, box) {
  var q;
  for (q = 0; q < BMAX; q++) {
    var b = beams[q]; if (!b.on) continue;
    var t = b.life / b.max;
    R.setAlpha(t);
    if (b.kind === 'h') { R.rect(box.x, b.y - 14 * t, box.w, 28 * t, 'rgba(255,255,255,0.85)'); R.rect(box.x, b.y - 4, box.w, 8, '#fff59d'); }
    else if (b.kind === 'v') { R.rect(b.x - 14 * t, box.y, 28 * t, box.h, 'rgba(255,255,255,0.85)'); R.rect(b.x - 4, box.y, 8, box.h, '#fff59d'); }
    else if (b.kind === 'ring') R.strokeCircle(b.x, b.y, 20 + (1 - t) * 70, 10 * t + 2, '#ffeb3b');
    else R.rect(box.x, box.y, box.w, box.h, 'rgba(255,255,255,0.5)');
  }
  R.setAlpha(1);
  for (q = 0; q < PMAX; q++) {
    var p = parts[q]; if (!p.on) continue;
    R.setAlpha(Math.min(1, p.life / p.max * 1.6));
    R.rect(p.x - p.size / 2, p.y - p.size / 2, p.size, p.size, p.color);
  }
  R.setAlpha(1);
}
// 飘字画在最上层
function drawFloats(R) {
  for (var q = 0; q < FMAX; q++) {
    var f = floats[q]; if (!f.on) continue;
    R.setAlpha(Math.min(1, f.life / f.max * 2));
    R.text(f.text, f.x, f.y, f.size, f.color, 'center', 'bold', ['#6d2c00', 2]);
  }
  R.setAlpha(1);
}
function clear() {
  parts.forEach(function (p) { p.on = false; }); floats.forEach(function (f) { f.on = false; }); beams.forEach(function (b) { b.on = false; });
}
module.exports = { burst: burst, float: float, beam: beam, update: update, draw: draw, drawFloats: drawFloats, clear: clear };
