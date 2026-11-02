// 棋盘画面：格子画在一块离屏 2D 画布上（只重画变了的格子），每帧再按镜头（缩放 + 平移）把可见那块
// drawImage 到主画布上。为什么不每帧把格子直接画到主画布：高级 480 格，每格几次 fillRect / fillText，
// 捏合缩放时每帧都要重画一遍，一帧上千次原生调用；离屏之后缩放平移只是一次 drawImage。
var CELL = 32;                       // 世界坐标里一格的边长（镜头缩放 s 之后才是屏上的逻辑像素）
var NUM_COLORS = ['', '#1c64f2', '#2f9e44', '#e03131', '#5f3dc4', '#a61e4d', '#0c8599', '#343a40', '#868e96'];

function View(board, area, dpr) {
  this.b = board; this.area = area;
  // 离屏每格的像素：按宽铺满时（一格 22~39 逻辑像素、@2x 44~78 物理像素）够清楚，又别让画布太大
  // （高级 16×30 格 → 1088×2040，约 8.9MB；放大到最大时略糊，换来每次上屏少拷一半像素）
  this.tex = Math.floor(Math.min(72, 2048 / Math.max(board.cols, board.rows)));
  this.cv = wx.createCanvas();
  this.cv.width = board.cols * this.tex; this.cv.height = board.rows * this.tex;
  this.g = this.cv.getContext('2d');
  this.g.fillStyle = '#1d2a40'; this.g.fillRect(0, 0, this.cv.width, this.cv.height);
  for (var i = 0; i < board.n; i++) this.cell(i);
  board.changed.length = 0;
  this.bw = board.cols * CELL; this.bh = board.rows * CELL;
  var fitW = area.w / this.bw, fitAll = Math.min(fitW, area.h / this.bh);
  this.minS = fitAll;
  this.maxS = Math.max(fitAll * 1.2, 2);
  // 初始按宽铺满（高级竖着放不下也先这样：一格 ~22 逻辑像素还点得准，上下拖着看），镜头对准中间
  this.s = Math.min(fitW, this.maxS);
  this.ox = area.x; this.oy = area.y + (area.h - this.bh * this.s) / 2;
  this.clamp();
}

// ───────── 镜头 ─────────
View.prototype.clamp = function () {
  var a = this.area, w = this.bw * this.s, h = this.bh * this.s;
  this.ox = w <= a.w ? a.x + (a.w - w) / 2 : Math.min(a.x, Math.max(a.x + a.w - w, this.ox));
  this.oy = h <= a.h ? a.y + (a.h - h) / 2 : Math.min(a.y, Math.max(a.y + a.h - h, this.oy));
};
View.prototype.pan = function (dx, dy) { this.ox += dx; this.oy += dy; this.clamp(); };
// 以屏上 (px, py) 为中心缩放 k 倍：那一点下面的格子不动
View.prototype.zoomAt = function (k, px, py) {
  var wx0 = (px - this.ox) / this.s, wy0 = (py - this.oy) / this.s;
  this.s = Math.max(this.minS, Math.min(this.maxS, this.s * k));
  this.ox = px - wx0 * this.s; this.oy = py - wy0 * this.s;
  this.clamp();
};
View.prototype.zoomed = function () { return this.s > this.minS * 1.01; };
// 底栏「全览 / 放大」：放大着就缩回整盘可见；整盘可见就放大到 1.8 倍（至少按宽铺满），以画面中心为准
View.prototype.fit = function () {
  var a = this.area, k = this.zoomed() ? this.minS / this.s
    : Math.min(this.maxS, Math.max(a.w / this.bw, this.minS * 1.8)) / this.s;
  this.zoomAt(k, a.x + a.w / 2, a.y + a.h / 2);
};
// 把某格挪进视野（自动试玩在放大的高级棋盘上跟着走）
View.prototype.follow = function (i, k) {
  var a = this.area, x = (i % this.b.cols + 0.5) * CELL * this.s, y = (((i / this.b.cols) | 0) + 0.5) * CELL * this.s;
  this.ox += (a.x + a.w / 2 - x - this.ox) * k; this.oy += (a.y + a.h / 2 - y - this.oy) * k;
  this.clamp();
};
View.prototype.cellAt = function (px, py) {
  var a = this.area;
  if (px < a.x || py < a.y || px > a.x + a.w || py > a.y + a.h) return -1;
  var cx = Math.floor((px - this.ox) / (CELL * this.s)), cy = Math.floor((py - this.oy) / (CELL * this.s));
  if (cx < 0 || cy < 0 || cx >= this.b.cols || cy >= this.b.rows) return -1;
  return cy * this.b.cols + cx;
};
View.prototype.cellRect = function (i) {
  var c = CELL * this.s;
  return { x: this.ox + (i % this.b.cols) * c, y: this.oy + ((i / this.b.cols) | 0) * c, s: c };
};

// ───────── 离屏格子 ─────────
View.prototype.flush = function () {
  var ch = this.b.changed;
  if (!ch.length) return false;
  for (var k = 0; k < ch.length; k++) this.cell(ch[k]);
  ch.length = 0;
  return true;
};
// 一格是哪种样子：0 没翻开、1 旗、2 插错的旗、3 空白、4~11 数字 1~8、12 雷、13 踩到的雷
View.prototype.kind = function (i) {
  var b = this.b, lost = b.state === 'lost';
  if (!b.open[i] && !(lost && b.mine[i] && !b.flag[i])) return b.flag[i] ? (lost && !b.mine[i] ? 2 : 1) : 0;
  if (b.mine[i]) return i === b.boom ? 13 : 12;
  return b.num[i] ? 3 + b.num[i] : 3;
};
// 重画一格 = 从图块表拷一块。为什么不现画：一次展开上百格，每格 fillText 一个大号数字，
// 无头 CPU 后端实测一帧 90~170ms（中级一片空白展开时）；图块表每种样子只画一次
View.prototype.cell = function (i) {
  var s = this.tex, x = (i % this.b.cols) * s, y = ((i / this.b.cols) | 0) * s;
  this.g.drawImage(sheet(s), this.kind(i) * s, 0, s, s, x, y, s, s);
};
var sheets = {};
function sheet(s) {
  if (sheets[s]) return sheets[s];
  var cv = wx.createCanvas();
  cv.width = s * 14; cv.height = s;
  var g = cv.getContext('2d'), ins = Math.max(1, s * 0.04);
  for (var k = 0; k < 14; k++) {
    var x = k * s, y = 0;
    g.fillStyle = '#1d2a40'; g.fillRect(x, y, s, s);
    if (k <= 2) {
      // 没翻开：凸起的方块（上亮下暗两层）
      roundRect(g, x + ins, y + ins, s - ins * 2, s - ins * 2, s * 0.14); g.fillStyle = '#35507e'; g.fill();
      roundRect(g, x + ins, y + ins, s - ins * 2, s - ins * 2 - s * 0.07, s * 0.14); g.fillStyle = '#5b82c4'; g.fill();
      if (k >= 1) flagIcon(g, x, y, s);
      if (k === 2) cross(g, x, y, s);
      continue;
    }
    g.fillStyle = k === 13 ? '#ff6b6b' : '#e3e9f1';
    g.fillRect(x + ins * 0.5, y + ins * 0.5, s - ins, s - ins);
    if (k >= 12) { mineIcon(g, x, y, s); continue; }
    if (k > 3) {
      g.font = 'bold ' + Math.round(s * 0.62) + 'px sans-serif';
      g.textAlign = 'center'; g.textBaseline = 'middle';
      g.fillStyle = NUM_COLORS[k - 3];
      g.fillText(String(k - 3), x + s / 2, y + s * 0.53);
    }
  }
  return (sheets[s] = cv);
}
function roundRect(g, x, y, w, h, r) {
  g.beginPath();
  g.moveTo(x + r, y); g.arcTo(x + w, y, x + w, y + h, r); g.arcTo(x + w, y + h, x, y + h, r);
  g.arcTo(x, y + h, x, y, r); g.arcTo(x, y, x + w, y, r); g.closePath();
}
function flagIcon(g, x, y, s) {
  g.fillStyle = '#263238';
  g.fillRect(x + s * 0.40, y + s * 0.2, s * 0.07, s * 0.56);
  g.fillRect(x + s * 0.28, y + s * 0.72, s * 0.38, s * 0.08);
  g.beginPath();
  g.moveTo(x + s * 0.47, y + s * 0.2); g.lineTo(x + s * 0.78, y + s * 0.35); g.lineTo(x + s * 0.47, y + s * 0.5);
  g.closePath(); g.fillStyle = '#ff4d4f'; g.fill();
}
function mineIcon(g, x, y, s) {
  var cx = x + s / 2, cy = y + s / 2, r = s * 0.22;
  g.strokeStyle = '#212529'; g.lineWidth = s * 0.07; g.lineCap = 'round';
  g.beginPath();
  for (var k = 0; k < 4; k++) {
    var a = k * Math.PI / 4, dx = Math.cos(a) * r * 1.5, dy = Math.sin(a) * r * 1.5;
    g.moveTo(cx - dx, cy - dy); g.lineTo(cx + dx, cy + dy);
  }
  g.stroke();
  g.beginPath(); g.arc(cx, cy, r, 0, Math.PI * 2); g.fillStyle = '#212529'; g.fill();
  g.beginPath(); g.arc(cx - r * 0.35, cy - r * 0.35, r * 0.28, 0, Math.PI * 2); g.fillStyle = '#ffffff'; g.fill();
}
function cross(g, x, y, s) {
  g.strokeStyle = '#d6336c'; g.lineWidth = s * 0.08; g.lineCap = 'round';
  g.beginPath();
  g.moveTo(x + s * 0.22, y + s * 0.22); g.lineTo(x + s * 0.78, y + s * 0.78);
  g.moveTo(x + s * 0.78, y + s * 0.22); g.lineTo(x + s * 0.22, y + s * 0.78);
  g.stroke();
}

// ───────── 上屏：只拷可见的那一块 ─────────
View.prototype.draw = function (ctx) {
  var a = this.area, s = this.s, k = this.tex / CELL;
  var x0 = Math.max(a.x, this.ox), y0 = Math.max(a.y, this.oy);
  var x1 = Math.min(a.x + a.w, this.ox + this.bw * s), y1 = Math.min(a.y + a.h, this.oy + this.bh * s);
  if (x1 <= x0 || y1 <= y0) return;
  var sx = (x0 - this.ox) / s * k, sy = (y0 - this.oy) / s * k, sw = (x1 - x0) / s * k, sh = (y1 - y0) / s * k;
  ctx.drawImage(this.cv, sx, sy, sw, sh, x0, y0, x1 - x0, y1 - y0);
};

module.exports = View;
