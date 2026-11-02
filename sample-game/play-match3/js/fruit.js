// 六种水果 + 特效块的画法。都画在原点、半径 r，由 R.sprite 录成精灵（录一次，之后每帧一次 drawArrays，位置 / 缩放 / 透明度走 uniform）。
var COLORS = ['#e53935', '#fb8c00', '#8e24aa', '#fdd835', '#7cb342', '#1e88e5'];

function star(R, cx, cy, ro, ri, n, rot, color) {
  // 星形 = 中心扇形三角化（凹多边形不能直接 poly），每个尖一个三角 + 中间正多边形
  var pts = [];
  for (var i = 0; i < n * 2; i++) {
    var a = rot + Math.PI * i / n, rr = i % 2 ? ri : ro;
    pts.push([cx + Math.cos(a) * rr, cy + Math.sin(a) * rr]);
  }
  for (var k = 0; k < n * 2; k++) R.poly([[cx, cy], pts[k], pts[(k + 1) % (n * 2)]], color);
}

function apple(R, r) {
  R.circle(-r * 0.32, r * 0.05, r * 0.66, '#c62828');
  R.circle(r * 0.32, r * 0.05, r * 0.66, '#c62828');
  R.circle(-r * 0.3, 0, r * 0.6, '#e53935');
  R.circle(r * 0.3, 0, r * 0.6, '#e53935');
  R.circle(-r * 0.35, -r * 0.28, r * 0.18, 'rgba(255,255,255,0.55)');
  R.rect(-r * 0.05, -r * 0.92, r * 0.1, r * 0.4, '#5d4037');
  R.ellipse(r * 0.24, -r * 0.78, r * 0.26, r * 0.12, -0.5, '#43a047');
}
function orange(R, r) {
  R.circle(0, r * 0.06, r * 0.86, '#ef6c00');
  R.circle(0, 0, r * 0.82, '#fb8c00');
  for (var i = 0; i < 7; i++) {
    var a = i * 0.9 + 0.4;
    R.circle(Math.cos(a) * r * 0.48, Math.sin(a) * r * 0.48 + r * 0.05, r * 0.05, 'rgba(230,81,0,0.6)');
  }
  R.circle(-r * 0.32, -r * 0.34, r * 0.2, 'rgba(255,255,255,0.5)');
  R.ellipse(r * 0.12, -r * 0.82, r * 0.26, r * 0.11, -0.3, '#2e7d32');
}
function grape(R, r) {
  var b = r * 0.3, pts = [[-0.42, -0.32], [0, -0.38], [0.42, -0.32], [-0.22, 0.06], [0.22, 0.06], [-0.42, 0.42], [0.42, 0.42], [0, 0.46], [0, 0.82]];
  pts.slice(0, 8).concat([pts[8]]).forEach(function (p, i) {
    if (i === 5 || i === 6) return;                                 // 两边下角空出来，成一串的样子
    R.circle(p[0] * r, p[1] * r + r * 0.03, b, '#6a1b9a');
    R.circle(p[0] * r, p[1] * r, b * 0.94, '#8e24aa');
    R.circle(p[0] * r - b * 0.35, p[1] * r - b * 0.35, b * 0.25, 'rgba(255,255,255,0.45)');
  });
  R.rect(-r * 0.04, -r * 0.95, r * 0.08, r * 0.3, '#5d4037');
  R.ellipse(-r * 0.26, -r * 0.78, r * 0.28, r * 0.12, 0.4, '#43a047');
}
function lemon(R, r) {
  R.ellipse(0, r * 0.05, r * 0.9, r * 0.66, -0.35, '#f9a825');
  R.ellipse(0, 0, r * 0.86, r * 0.62, -0.35, '#fdd835');
  R.circle(Math.cos(-0.35) * r * 0.86, Math.sin(-0.35) * r * 0.86, r * 0.14, '#fdd835');
  R.circle(-Math.cos(-0.35) * r * 0.86, -Math.sin(-0.35) * r * 0.86, r * 0.14, '#fdd835');
  R.ellipse(-r * 0.25, -r * 0.18, r * 0.28, r * 0.12, -0.35, 'rgba(255,255,255,0.55)');
}
function pear(R, r) {
  R.circle(0, r * 0.3, r * 0.62, '#689f38');
  R.circle(0, -r * 0.22, r * 0.4, '#689f38');
  R.circle(0, r * 0.26, r * 0.58, '#7cb342');
  R.circle(0, -r * 0.24, r * 0.36, '#7cb342');
  R.rect(-r * 0.32, -r * 0.24, r * 0.64, r * 0.5, '#7cb342');
  R.circle(-r * 0.22, r * 0.12, r * 0.15, 'rgba(255,255,255,0.45)');
  R.rect(-r * 0.04, -r * 0.92, r * 0.08, r * 0.34, '#5d4037');
  R.ellipse(r * 0.22, -r * 0.8, r * 0.24, r * 0.1, -0.4, '#33691e');
}
function blueberry(R, r) {
  R.circle(0, r * 0.06, r * 0.82, '#1565c0');
  R.circle(0, 0, r * 0.78, '#1e88e5');
  star(R, 0, -r * 0.42, r * 0.3, r * 0.12, 5, -Math.PI / 2, '#0d47a1');
  R.circle(-r * 0.36, r * 0.1, r * 0.18, 'rgba(255,255,255,0.5)');
}
var DRAW = [apple, orange, grape, lemon, pear, blueberry];

function fruit(R, t, r) { DRAW[t](R, r); }

// 条纹：白色三道（横 / 竖），叠在水果上面
function stripes(R, r, dir) {
  for (var i = -1; i <= 1; i++) {
    if (dir === 'h') R.rrect(-r * 0.86, i * r * 0.36 - r * 0.07, r * 1.72, r * 0.14, r * 0.07, 'rgba(255,255,255,0.85)');
    else R.rrect(i * r * 0.36 - r * 0.07, -r * 0.86, r * 0.14, r * 1.72, r * 0.07, 'rgba(255,255,255,0.85)');
  }
}
// 炸弹：外圈光环 + 中间小火花
function bombRing(R, r) {
  R.strokeCircle(0, 0, r * 0.98, r * 0.14, 'rgba(255,241,118,0.95)');
  star(R, 0, 0, r * 0.36, r * 0.15, 4, 0, 'rgba(255,255,255,0.9)');
}
// 彩虹球：六色扇面 + 白心
function rainbow(R, r) {
  for (var i = 0; i < 6; i++) R.ellipse(0, 0, r * 0.82, r * 0.82, 0, COLORS[i], i * Math.PI / 3, (i + 1) * Math.PI / 3);
  R.strokeCircle(0, 0, r * 0.82, r * 0.08, '#ffffff');
  R.circle(0, 0, r * 0.3, '#ffffff');
  R.circle(-r * 0.3, -r * 0.34, r * 0.14, 'rgba(255,255,255,0.7)');
}

module.exports = { COLORS: COLORS, fruit: fruit, stripes: stripes, bombRing: bombRing, rainbow: rainbow, star: star };
