// 贪吃蛇（横屏）：滑动换方向（PC 方向键 / WASD），吃苹果变长变快，每 5 个苹果出一个限时金苹果（+5）。
// 撞墙或撞自己结束。逻辑按固定步长走格子，画面在两步之间插值，低帧率下也是平滑移动。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var W = B.W, H = B.H;
// 画面走 WebGL（js/gl2d.js：形状攒成一批三角形画，文字是图集纹理）
var R = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2);

var ROWS = 15;
var top = Math.max(B.hudTop, 44) + 8;
var left = Math.max(B.safe.left, 12), right = Math.max(W - B.safe.right, 12);
var CELL = Math.floor(Math.min((H - top - 10) / ROWS, 40));
var COLS = Math.floor((W - left - right) / CELL);
var OX = left + Math.floor((W - left - right - COLS * CELL) / 2), OY = top;
var DIR = { up: [0, -1], down: [0, 1], left: [-1, 0], right: [1, 0] };

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';

var state, snake, prev, dir, queue, food, gold, eaten, score, interval, acc, overAt = 0, clock = 0;
var best = Number(B.load('snake.best', 0)) || 0, userPaused = false, newBest = false;

function reset() {
  var cx = Math.floor(COLS / 3), cy = Math.floor(ROWS / 2);
  snake = [[cx, cy], [cx - 1, cy], [cx - 2, cy]];
  prev = snake.map(function (p) { return p.slice(); });
  dir = 'right'; queue = []; eaten = 0; score = 0; interval = 0.14; acc = 0;
  gold = null; newBest = false;
  food = place();
  state = 'ready';
}
function occupied(x, y) { return snake.some(function (p) { return p[0] === x && p[1] === y; }); }
function place() {
  var free = [];
  for (var x = 0; x < COLS; x++) for (var y = 0; y < ROWS; y++)
    if (!occupied(x, y) && !(food && food[0] === x && food[1] === y)) free.push([x, y]);
  return free.length ? free[Math.floor(Math.random() * free.length)] : null;
}

function turn(d) {
  if (state === 'over') { if (clock - overAt > 0.5) { reset(); } return; }
  if (userPaused) { userPaused = false; return; }
  if (state === 'ready') state = 'play';
  // 最多排两次转向：快速连划「上、左」要都生效；与当前方向相反的丢掉
  var last = queue.length ? queue[queue.length - 1] : dir;
  if (!DIR[d] || d === last) return;
  if (DIR[d][0] === -DIR[last][0] && DIR[d][1] === -DIR[last][1]) return;
  if (queue.length < 2) queue.push(d);
}
B.on('swipe', turn);
B.on('tap', function () {
  if (state === 'over') turn('right');
  else if (state === 'ready') state = 'play';
  else if (userPaused) userPaused = false;
});
B.on('key', function (k) {
  if (k === 'pause') { if (state === 'play') userPaused = !userPaused; return; }
  if (k === 'ok') { if (state === 'over') turn(dir); else if (state === 'ready') state = 'play'; else userPaused = false; return; }
  turn(k);
});
B.onHide(function () { if (state === 'play') userPaused = true; });

function step() {
  if (AUTO) think();
  if (queue.length) dir = queue.shift();
  var d = DIR[dir], h = snake[0], nx = h[0] + d[0], ny = h[1] + d[1];
  var grow = food && nx === food[0] && ny === food[1];
  var golden = gold && nx === gold.x && ny === gold.y;
  // 尾巴这一步会挪走，所以撞到当前尾巴不算（除非这一步要长）
  var body = grow ? snake : snake.slice(0, -1);
  if (nx < 0 || ny < 0 || nx >= COLS || ny >= ROWS || body.some(function (p) { return p[0] === nx && p[1] === ny; })) return die();
  prev = snake.map(function (p) { return p.slice(); });
  snake.unshift([nx, ny]);
  if (grow || golden) {
    if (grow) {
      score += 1; eaten++;
      interval = Math.max(0.065, interval - 0.003);
      food = place();
      if (eaten % 5 === 0 && !gold) { var g = place(); if (g) gold = { x: g[0], y: g[1], t: 6 }; }
      B.beep(660, 0.06, 'square', 0.08, 1.5);
    }
    if (golden) {
      score += 5; gold = null;
      [880, 1175, 1568].forEach(function (f, i) { setTimeout(function () { B.beep(f, 0.07, 'triangle', 0.1); }, i * 50); });
    }
    if (!grow) snake.pop();
    prev.push(prev[prev.length - 1].slice());     // 新长出的那一节从尾巴原地长出来
  } else snake.pop();
  if (!food) die();                               // 铺满了：也算结束
}
function die() {
  state = 'over'; overAt = clock;
  B.beep(200, 0.4, 'sawtooth', 0.14, 0.4); B.buzz();
  if (score > best) { best = score; newBest = true; B.save('snake.best', best); }
}
// 自动驾驶：在不立刻送死的方向里选离食物最近的；都不安全就随便走
function think() {
  var target = gold ? [gold.x, gold.y] : food; if (!target) return;
  var h = snake[0], opts = [];
  Object.keys(DIR).forEach(function (k) {
    var d = DIR[k], cur = DIR[dir];
    if (d[0] === -cur[0] && d[1] === -cur[1]) return;
    var nx = h[0] + d[0], ny = h[1] + d[1];
    if (nx < 0 || ny < 0 || nx >= COLS || ny >= ROWS) return;
    if (snake.slice(0, -1).some(function (p) { return p[0] === nx && p[1] === ny; })) return;
    opts.push([Math.abs(nx - target[0]) + Math.abs(ny - target[1]) + Math.random() * 0.1, k]);
  });
  opts.sort(function (a, b) { return a[0] - b[0]; });
  if (opts.length) { queue = [opts[0][1]]; }
}

function update(dt) {
  clock += dt;
  if (AUTO && state === 'over' && clock - overAt > 1) { reset(); state = 'play'; }
  if (state !== 'play' || userPaused) return;
  if (gold) { gold.t -= dt; if (gold.t <= 0) gold = null; }
  acc += dt;
  while (acc >= interval && state === 'play') { acc -= interval; step(); }
}

// ───────── 绘制 ─────────
function px(x) { return OX + x * CELL + CELL / 2; }
function py(y) { return OY + y * CELL + CELL / 2; }
function render() {
  R.begin('#1e3a2b');
  // 棋盘格不会变：录成图层，之后每帧一次 drawArrays（每帧现拼 ~1500 个顶点在真机上要 ~4ms）
  R.layer('board', 1, function () {
    R.rect(OX, OY, COLS * CELL, ROWS * CELL, '#8ecc39');
    for (var x = 0; x < COLS; x++) for (var y = 0; y < ROWS; y++) {
      if ((x + y) % 2) R.rect(OX + x * CELL, OY + y * CELL, CELL, CELL, '#a7d948');
    }
  });
  if (food) drawApple(px(food[0]), py(food[1]), '#e53935', 1);
  if (gold) {
    var blink = gold.t < 2 ? (Math.floor(gold.t * 6) % 2 ? 0.4 : 1) : 1;
    drawApple(px(gold.x), py(gold.y), '#ffc107', blink);
  }
  drawSnake();
  drawHud();
  R.end();
}
function drawApple(x, y, color, alpha) {
  var r = CELL * 0.36, s = 1 + Math.sin(clock * 6) * 0.05;
  R.setAlpha(alpha);
  R.circle(x, y + 1, r * s, color);
  R.circle(x - r * 0.35, y - r * 0.25, r * 0.25, 'rgba(255,255,255,0.45)');
  R.rect(x - 1, y - r - 4, 2, 6, '#4e342e');
  R.ellipse(x + 4, y - r - 2, 4, 2, -0.5, '#43a047');
  R.setAlpha(1);
}
function drawSnake() {
  var t = state === 'play' && !userPaused ? Math.min(1, acc / interval) : 1;
  if (state !== 'play') t = 1;
  var pts = snake.map(function (p, i) {
    var a = prev[i] || p;
    return [px(a[0] + (p[0] - a[0]) * t), py(a[1] + (p[1] - a[1]) * t)];
  });
  // 画的是「上一步 → 这一步」的插值：显示比逻辑晚最多一步（一百多毫秒内），换来匀速平滑，
  // 不用去预测下一步往哪拐。粗线 + 每个拐点补圆 = lineJoin / lineCap round
  R.line(pts, CELL * 0.78, '#3f51b5', true);
  R.line(pts, CELL * 0.4, '#5c6bc0', true);
  // 头
  var h = pts[0], d = DIR[dir];
  R.circle(h[0], h[1], CELL * 0.45, '#3f51b5');
  var ex = -d[1], ey = d[0];                      // 垂直于前进方向的两只眼
  [-1, 1].forEach(function (s) {
    var cx = h[0] + d[0] * CELL * 0.15 + ex * s * CELL * 0.2, cy = h[1] + d[1] * CELL * 0.15 + ey * s * CELL * 0.2;
    R.circle(cx, cy, CELL * 0.13, '#ffffff');
    R.circle(cx + d[0] * 2, cy + d[1] * 2, CELL * 0.06, '#000000');
  });
}
function drawHud() {
  var y = OY / 2 + 2;
  R.text('得分 ' + score, OX, y, 18, '#ffffff', 'left', 'bold');
  R.text('最高 ' + best, OX + 110, y, 16, '#c8e6c9', 'left');
  if (gold) R.text('金苹果 ' + Math.ceil(gold.t) + 's', W / 2, y, 16, '#ffc107', 'center', 'bold');
  var cx = OX + COLS * CELL / 2, cy = OY + ROWS * CELL / 2;
  if (state === 'ready') {
    panel(cx, cy, 300, 120);
    R.text('贪吃蛇', cx, cy - 22, 30, '#ffffff', 'center', 'bold');
    R.text('滑动屏幕控制方向，点一下开始', cx, cy + 22, 15, '#e0e0e0', 'center');
  } else if (state === 'over') {
    panel(cx, cy, 300, 150);
    R.text(newBest ? '新纪录！' : '游戏结束', cx, cy - 40, 30, newBest ? '#ffc107' : '#ffffff', 'center', 'bold');
    R.text('得分 ' + score + '    最高 ' + best, cx, cy + 4, 18, '#ffffff', 'center');
    if (clock - overAt > 0.5) R.text('点击或滑动重新开始', cx, cy + 44, 15, '#e0e0e0', 'center');
  } else if (userPaused) {
    panel(cx, cy, 220, 90);
    R.text('已暂停', cx, cy - 10, 26, '#ffffff', 'center', 'bold');
    R.text('点击继续', cx, cy + 22, 14, '#e0e0e0', 'center');
  }
}
function panel(cx, cy, w, h) { R.rrect(cx - w / 2, cy - h / 2, w, h, 14, 'rgba(0,0,0,0.6)'); }

GameGlobal.__play = function () { return { state: state, score: score, len: snake.length, cols: COLS, rows: ROWS, cell: CELL }; };

reset();
if (AUTO) state = 'play';
B.loop(update, render);
