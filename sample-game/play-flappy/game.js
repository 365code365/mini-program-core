// 飞翔小鸟：点屏幕（或空格）往上扑，穿过水管得分，碰到水管或地面结束。最高分存在本地。
// 画面走 WebGL（js/gl2d.js：所有形状攒成一批三角形画，文字是缓存的纹理）。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var W = B.W, H = B.H;
var g = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2);

var GROUND = 90;                 // 地面高度
var GRAVITY = 1500, FLAP = -430, MAX_FALL = 620;
var PIPE_W = 62, GAP = 152, SPACING = 220, SPEED = 150;
var BIRD_R = 15;

var state = 'ready';             // ready → play → over
var bird, pipes, score, best = Number(B.load('flappy.best', 0)) || 0;
var scroll = 0, overAt = 0, clock = 0, flash = 0, newBest = false;
var clouds = [];
for (var i = 0; i < 6; i++) clouds.push({ x: B.rand(0, W), y: B.rand(B.hudTop + 40, H * 0.45), s: B.rand(0.6, 1.3) });

function reset() {
  bird = { x: W * 0.3, y: H * 0.42, vy: 0, rot: 0, wing: 0 };
  pipes = [];
  score = 0; newBest = false;
  for (var i = 0; i < 4; i++) addPipe(W + 80 + i * SPACING);
}
function addPipe(x) {
  var top = B.hudTop + 60, bottom = H - GROUND - 60 - GAP;
  pipes.push({ x: x, gapY: B.rand(top, Math.max(top + 1, bottom)), passed: false });
}

function flap() {
  if (state === 'ready') { state = 'play'; }
  if (state === 'play') {
    bird.vy = FLAP; bird.wing = 1;
    B.beep(620, 0.08, 'square', 0.08, 1.6);
  } else if (state === 'over' && clock - overAt > 0.6) {
    reset(); state = 'ready';
  }
}
B.on('down', flap);              // 按下即扑，不等抬手（等 tap 会慢一拍，手感发黏）
B.on('key', function (k) { if (k === 'ok' || k === 'up') flap(); });

function die() {
  if (state !== 'play') return;
  state = 'over'; overAt = clock; flash = 1;
  B.beep(180, 0.35, 'sawtooth', 0.15, 0.4); B.buzz();
  if (score > best) { best = score; newBest = true; B.save('flappy.best', best); }
}

function update(dt) {
  clock += dt;
  flash = Math.max(0, flash - dt * 3);
  if (state !== 'over') scroll += SPEED * dt;
  clouds.forEach(function (c) { if (state !== 'over') { c.x -= 12 * c.s * dt; if (c.x < -80) { c.x = W + 60; c.y = B.rand(B.hudTop + 40, H * 0.45); } } });
  bird.wing = Math.max(0, bird.wing - dt * 5);

  if (state === 'ready') {
    bird.y = H * 0.42 + Math.sin(clock * 5) * 8;
    return;
  }
  // over 状态下小鸟继续掉到地上
  bird.vy = Math.min(MAX_FALL, bird.vy + GRAVITY * dt);
  bird.y += bird.vy * dt;
  bird.rot = B.clamp(bird.vy / 600, -0.5, 1.3);
  if (bird.y > H - GROUND - BIRD_R) { bird.y = H - GROUND - BIRD_R; bird.vy = 0; die(); }
  if (bird.y < BIRD_R) { bird.y = BIRD_R; bird.vy = 0; }
  if (state !== 'play') return;

  for (var i = 0; i < pipes.length; i++) {
    var p = pipes[i];
    p.x -= SPEED * dt;
    if (!p.passed && p.x + PIPE_W < bird.x) {
      p.passed = true; score++;
      B.beep(880, 0.06, 'triangle', 0.12); setTimeout(function () { B.beep(1320, 0.08, 'triangle', 0.1); }, 60);
    }
    if (collide(p)) { die(); return; }
  }
  if (pipes[0].x < -PIPE_W) { pipes.shift(); addPipe(pipes[pipes.length - 1].x + SPACING); }
}
// 圆与两段水管（矩形）求最近点距离
function collide(p) {
  return circleRect(bird.x, bird.y, BIRD_R - 2, p.x, -1000, PIPE_W, p.gapY + 1000) ||
         circleRect(bird.x, bird.y, BIRD_R - 2, p.x, p.gapY + GAP, PIPE_W, H);
}
function circleRect(cx, cy, r, x, y, w, h) {
  var nx = B.clamp(cx, x, x + w), ny = B.clamp(cy, y, y + h);
  var dx = cx - nx, dy = cy - ny;
  return dx * dx + dy * dy < r * r;
}

// ───────── 绘制 ─────────
var OUTLINE = ['#3b2a0a', 3];
function render() {
  g.begin('#bfeef0');
  // 天空、地面底色不变：一个图层
  g.layer('sky', 1, function () { g.gradRect(0, 0, W, H, [[0, '#4ec0ca'], [1, '#bfeef0']], true); });
  clouds.forEach(function (c) { g.sprite('cloud', 1, drawCloud, c.x, c.y, c.s); });
  drawCity();
  pipes.forEach(drawPipe);
  drawGround();
  drawBird();
  if (flash > 0) g.rect(0, 0, W, H, [1, 1, 1, flash]);
  drawHud();
  g.end();
}
// 下面这些都按「录一次、每帧平移」画（gl2d 的 sprite / layer）：真机上每帧的时间几乎都花在 JS 拼顶点上，
// 一帧现拼 2000 个顶点要 ~5ms；录好的只是一次 drawArrays
// 云：原点处、比例 1 录一朵，每朵按 (x, y, s) 摆。不透明的浅色（半透明的话重叠处会更白）
function drawCloud() {
  var col = '#e6f7f9';
  g.circle(0, 0, 18, col); g.circle(20, -8, 22, col); g.circle(44, 0, 17, col);
}
// 楼：高度按楼号 k 算（37k mod 50，50 栋一个周期），所以录 50 栋 + 盖满一屏的那几栋成一条，按卷动量平移
var CITY_PERIOD = 50 * 60;
function drawCity() {
  var base = H - GROUND, off = (scroll * 0.25) % CITY_PERIOD;
  g.layer('city', 1, function () {
    for (var k = 0; k < 50 + Math.ceil(W / 60) + 2; k++) {
      var h = 40 + ((k * 37) % 50 + 50) % 50;
      g.rect(k * 60, base - h, 44, h, '#a3dfc8');
    }
  }, -off - 60, 0);
  // 山坡：sin 的周期是 2π·30 ≈ 188.5，录一条比屏宽多一个周期的，按卷动量平移
  var period = Math.PI * 2 * 30, off2 = (scroll * 0.5) % period;
  g.layer('hills', 1, function () {
    for (var x2 = 0; x2 < W + period + 10; x2 += 10) {
      var y0 = base - 14 - Math.sin(x2 / 30) * 6, y1 = base - 14 - Math.sin((x2 + 10) / 30) * 6;
      g.poly([[x2, base], [x2, y0], [x2 + 10, y1], [x2 + 10, base]], '#7fd06b');
    }
  }, -off2, 0);
}
var PIPE_GRAD = [[0, '#5fa83a'], [0.45, '#9be15d'], [1, '#4a8a2c']];
// 水管：上下两种各录一根「整屏高」的（管口在缺口那一端），按 (x, 缺口位置) 摆；露出地面的部分被随后画的地面盖住
function drawPipe(p) {
  g.sprite('pipe-top', 1, function () {
    var e = '#2f5d1c';
    g.gradRect(0, -H, PIPE_W, H, PIPE_GRAD, false);
    g.gradRect(-5, -26, PIPE_W + 10, 26, PIPE_GRAD, false);
    g.strokeRect(0, -H - 2, PIPE_W, H - 24, 2, e);
    g.strokeRect(-5, -26, PIPE_W + 10, 26, 2, e);
  }, p.x, p.gapY, 1);
  g.sprite('pipe-bottom', 1, function () {
    var e = '#2f5d1c';
    g.gradRect(0, 0, PIPE_W, H, PIPE_GRAD, false);
    g.gradRect(-5, 0, PIPE_W + 10, 26, PIPE_GRAD, false);
    g.strokeRect(-5, 0, PIPE_W + 10, 26, 2, e);
    g.strokeRect(0, 26, PIPE_W, H, 2, e);
  }, p.x, p.gapY + GAP, 1);
}
function drawGround() {
  var y = H - GROUND;
  g.layer('ground', 1, function () {
    g.rect(0, y, W, GROUND, '#ded895');
    g.rect(0, y, W, 14, '#73bf2e');
  });
  // 草尖：24 一个周期，录一条比屏宽多一个周期的，按卷动量平移
  g.layer('grass', 1, function () {
    for (var x = 0; x < W + 48; x += 24) g.poly([[x, y + 14], [x + 12, y + 14], [x + 6, y + 4]], '#5a9e1f');
  }, -(scroll % 24), 0);
  g.rect(0, y + 14, W, 3, '#c9c27a');
}
function drawBird() {
  var ink = '#5a3d0a';
  g.save();
  g.translate(bird.x, bird.y);
  g.rotate(state === 'ready' ? 0 : bird.rot);
  g.ellipse(0, 0, BIRD_R + 3, BIRD_R, 0, '#f8d531');
  g.strokeEllipse(0, 0, BIRD_R + 3, BIRD_R, 0, 2, ink);
  // 翅膀：扑的时候往上翻
  var wy = 3 - bird.wing * 9 + (state === 'ready' ? Math.sin(clock * 18) * 3 : 0);
  g.ellipse(-7, wy, 9, 6, -0.3, '#fff6c8');
  g.strokeEllipse(-7, wy, 9, 6, -0.3, 2, ink);
  // 眼睛与嘴
  g.circle(7, -6, 6, '#ffffff'); g.strokeCircle(7, -6, 6, 2, ink);
  g.circle(9, -6, 2.5, '#000000');
  g.poly([[10, 1], [24, 4], [10, 9]], '#f36b21');
  g.line([[10, 1], [24, 4], [10, 9], [10, 1]], 2, ink, true);
  g.restore();
}
function outlined(s, x, y, size) { g.text(s, x, y, size, '#ffffff', 'center', 'bold', ['#3b2a0a', Math.max(2, size / 14)]); }
function drawHud() {
  if (state === 'play') outlined(score, W / 2, B.hudTop + 30, 44);
  if (state === 'ready') {
    outlined('飞翔小鸟', W / 2, H * 0.22, 40);
    g.text('点击屏幕起飞', W / 2, H * 0.58, 20, '#ffffff', 'center', 'bold');
    if (best) g.text('最高分 ' + best, W / 2, H * 0.58 + 32, 16, '#ffffff', 'center');
  }
  if (state === 'over') {
    var t = Math.min(1, (clock - overAt) / 0.3);
    var pw = 260, ph = 190, px = (W - pw) / 2, py = H * 0.28 + (1 - t) * 40;
    g.setAlpha(t);
    outlined('游戏结束', W / 2, py - 36, 36);
    g.rrect(px - 1.5, py - 1.5, pw + 3, ph + 3, 15.5, '#5a3d0a');
    g.rrect(px + 1.5, py + 1.5, pw - 3, ph - 3, 12.5, '#ded895');
    g.text('得分', px + 70, py + 40, 18, '#c0592b', 'center', 'bold');
    g.text(score, px + 70, py + 80, 40, '#ffffff', 'center', 'bold');
    g.text('最高', px + pw - 70, py + 40, 18, '#c0592b', 'center', 'bold');
    g.text(best, px + pw - 70, py + 80, 40, '#ffffff', 'center', 'bold');
    if (newBest) g.text('新纪录！', W / 2, py + 120, 18, '#e74c3c', 'center', 'bold');
    drawMedal(W / 2, py + 120, score);
    if (clock - overAt > 0.6) g.text('点击重新开始', W / 2, py + ph - 24, 18, '#5a3d0a', 'center', 'bold');
    g.setAlpha(1);
  }
}
function drawMedal(x, y, s) {
  if (s < 10 || newBest) return;
  var c = s >= 40 ? '#e5e4e2' : s >= 20 ? '#ffd700' : '#cd7f32';
  g.circle(x, y, 16, c); g.strokeCircle(x, y, 16, 2, '#5a3d0a');
}

// 启动参数 auto=1：自动驾驶（低于下一个缺口中线且在往下掉就扑），死了一秒后自动重来，无头长稳 / 演示用
var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
if (q.auto === '1') {
  var upd = update;
  update = function (dt) {
    upd(dt);
    if (state === 'over' && clock - overAt > 1) { flap(); return; }
    var p = pipes.filter(function (x) { return x.x + PIPE_W > bird.x - BIRD_R; })[0];
    var aim = p ? p.gapY + GAP * 0.62 : H * 0.5;
    if ((state === 'ready' || bird.y > aim) && bird.vy >= 0) flap();
  };
}
GameGlobal.__play = function () { return { state: state, score: score, best: best }; };

reset();
B.loop(function (dt) { update(dt); }, render);
