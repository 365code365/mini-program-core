// 打砖块：手指左右拖挡板（不用按在挡板上，整屏相对拖动），点一下发球；PC 上方向键移动、空格发球、P 暂停。
// 道具：W 加长挡板、M 分出三个球、S 球减速、+ 加一条命。五关循环，每轮球速加快。
var B = require('js/base.js');
var LEVELS = require('js/levels.js');
var G2 = require('js/gl2d.js');
var W = B.W, H = B.H;
// 画面走 WebGL（js/gl2d.js：形状攒成一批三角形画，文字是图集纹理）
var R = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2);

var COLS = 10, GAP = 4, BH = 18, SIDE = 8;
var BW = (W - SIDE * 2 - GAP * (COLS - 1)) / COLS;
var TOP = B.hudTop + 44;
var PADDLE_Y = H - Math.max(70, H - B.safe.bottom + 50), PADDLE_H = 12;
var BALL_R = 7;
var BRICK_COLORS = { 1: '#ef476f', 2: '#f78c6b', 3: '#ffd166', 4: '#06d6a0', 5: '#118ab2', H: '#8d99ae', '#': '#4a4e69' };
var POWERS = [['W', '#06d6a0'], ['M', '#ffd166'], ['S', '#118ab2'], ['+', '#ef476f']];

var state, level, round, score, lives, best = Number(B.load('breakout.best', 0)) || 0;
var bricks, balls, drops, parts, paddle, banner = 0, userPaused = false;
var drag = null;

function speed() { return Math.min(720, 360 + level * 18 + round * 50); }
function newGame() { level = 0; round = 0; score = 0; lives = 3; loadLevel(); }
function loadLevel() {
  bricks = []; brickRev++;
  LEVELS[level].forEach(function (row, r) {
    for (var c = 0; c < COLS; c++) {
      var ch = row.charAt(c); if (!ch || ch === '.') continue;
      bricks.push({ x: SIDE + c * (BW + GAP), y: TOP + r * (BH + GAP), kind: ch,
        hp: ch === 'H' ? 2 : ch === '#' ? Infinity : 1, flash: 0 });
    }
  });
  drops = []; parts = [];
  paddle = { x: W / 2, w: 84, wideT: 0 };
  serve();
  state = 'serve'; banner = 1.4;
}
function serve() { balls = [{ x: paddle.x, y: PADDLE_Y - BALL_R - 1, vx: 0, vy: 0, stuck: true, slowT: 0 }]; state = 'serve'; }
function launch() {
  if (state !== 'serve') return;
  var a = B.rand(-0.35, 0.35), s = speed();
  balls.forEach(function (b) { b.stuck = false; b.vx = Math.sin(a) * s; b.vy = -Math.cos(a) * s; });
  state = 'play';
  B.beep(520, 0.06, 'square', 0.08);
}

// ───────── 输入 ─────────
B.on('down', function (x, y, id) {
  if (drag === null) drag = { id: id, sx: x, px: paddle.x, moved: 0 };
});
B.on('move', function (x, y, id) {
  if (!drag || drag.id !== id || state === 'over') return;
  drag.moved = Math.max(drag.moved, Math.abs(x - drag.sx));
  paddle.x = B.clamp(drag.px + (x - drag.sx) * 1.2, paddle.w / 2, W - paddle.w / 2);
});
B.on('up', function (x, y, id) { if (drag && drag.id === id) drag = null; });
B.on('tap', function (x, y) {
  if (state === 'over') { if (B.hit(againBtn, x, y)) newGame(); return; }
  if (userPaused) { userPaused = false; return; }
  launch();
});
B.on('key', function (k) {
  if (k === 'pause') userPaused = !userPaused;
  else if (k === 'ok') { if (state === 'over') newGame(); else if (userPaused) userPaused = false; else launch(); }
});
B.onHide(function () { userPaused = true; });
var againBtn = null;

// ───────── 更新 ─────────
function update(dt) {
  banner = Math.max(0, banner - dt);
  updateParts(dt);
  if (userPaused || state === 'over') return;
  if (AUTO) autopilot();
  if (B.held.left) paddle.x -= 520 * dt;
  if (B.held.right) paddle.x += 520 * dt;
  paddle.wideT = Math.max(0, paddle.wideT - dt);
  paddle.w += ((paddle.wideT > 0 ? 130 : 84) - paddle.w) * Math.min(1, dt * 10);
  paddle.x = B.clamp(paddle.x, paddle.w / 2, W - paddle.w / 2);
  bricks.forEach(function (b) { b.flash = Math.max(0, b.flash - dt * 4); });

  if (state === 'serve') { balls[0].x = paddle.x; balls[0].y = PADDLE_Y - BALL_R - 1; return; }
  if (state === 'clear') {
    if (banner <= 0) { level = (level + 1) % LEVELS.length; if (level === 0) round++; loadLevel(); }
    return;
  }
  // 子步长：每步最多走 4px，快球不会穿砖
  balls.forEach(function (b) {
    b.slowT = Math.max(0, b.slowT - dt);
    var k = b.slowT > 0 ? 0.65 : 1;
    var dist = Math.sqrt(b.vx * b.vx + b.vy * b.vy) * k * dt, n = Math.max(1, Math.ceil(dist / 4));
    for (var i = 0; i < n && !b.dead; i++) step(b, dt * k / n);
  });
  balls = balls.filter(function (b) { return !b.dead; });
  updateDrops(dt);
  if (!balls.length) {
    lives--; B.beep(160, 0.4, 'sawtooth', 0.12, 0.5); B.buzz();
    if (lives <= 0) gameOver(); else serve();
  } else if (!bricks.some(function (b) { return b.hp !== Infinity; })) {
    state = 'clear'; banner = 1.6; score += 100 * (level + 1);
    [523, 659, 784, 1046].forEach(function (f, i) { setTimeout(function () { B.beep(f, 0.12, 'triangle', 0.12); }, i * 90); });
  }
}
function step(b, dt) {
  b.x += b.vx * dt; b.y += b.vy * dt;
  if (b.x < BALL_R) { b.x = BALL_R; b.vx = Math.abs(b.vx); B.beep(300, 0.03, 'sine', 0.05); }
  if (b.x > W - BALL_R) { b.x = W - BALL_R; b.vx = -Math.abs(b.vx); B.beep(300, 0.03, 'sine', 0.05); }
  if (b.y < TOP - 30 + BALL_R) { b.y = TOP - 30 + BALL_R; b.vy = Math.abs(b.vy); B.beep(300, 0.03, 'sine', 0.05); }
  if (b.y > H + BALL_R) { b.dead = true; return; }
  // 挡板：按击中位置决定出射角（中间直上，两端最多偏 60°）
  if (b.vy > 0 && b.y + BALL_R >= PADDLE_Y && b.y - BALL_R <= PADDLE_Y + PADDLE_H &&
      b.x >= paddle.x - paddle.w / 2 - BALL_R && b.x <= paddle.x + paddle.w / 2 + BALL_R) {
    var t = B.clamp((b.x - paddle.x) / (paddle.w / 2), -1, 1), a = t * Math.PI / 3, s = speed();
    b.vx = Math.sin(a) * s; b.vy = -Math.cos(a) * s; b.y = PADDLE_Y - BALL_R;
    B.beep(440, 0.04, 'square', 0.07);
    return;
  }
  for (var i = 0; i < bricks.length; i++) {
    var br = bricks[i];
    var nx = B.clamp(b.x, br.x, br.x + BW), ny = B.clamp(b.y, br.y, br.y + BH);
    var dx = b.x - nx, dy = b.y - ny;
    if (dx * dx + dy * dy >= BALL_R * BALL_R) continue;
    // 按穿进去的深浅决定从哪条边弹开
    var ox = Math.min(b.x + BALL_R - br.x, br.x + BW - (b.x - BALL_R));
    var oy = Math.min(b.y + BALL_R - br.y, br.y + BH - (b.y - BALL_R));
    if (ox < oy) { b.vx = b.x < br.x + BW / 2 ? -Math.abs(b.vx) : Math.abs(b.vx); b.x += b.vx > 0 ? ox : -ox; }
    else { b.vy = b.y < br.y + BH / 2 ? -Math.abs(b.vy) : Math.abs(b.vy); b.y += b.vy > 0 ? oy : -oy; }
    hitBrick(br, i);
    return;                                    // 一个子步只碰一块
  }
}
var brickRev = 0;                              // 砖块有变化就 +1（图层据此重录）
function hitBrick(br, i) {
  br.flash = 1; brickRev++;
  if (br.hp === Infinity) { B.beep(900, 0.03, 'square', 0.05); return; }
  br.hp--;
  if (br.hp > 0) { score += 5; B.beep(700, 0.04, 'square', 0.07); return; }
  bricks.splice(i, 1);
  score += 10;
  B.beep(600 + Math.random() * 300, 0.05, 'triangle', 0.1);
  burst(br.x + BW / 2, br.y + BH / 2, BRICK_COLORS[br.kind]);
  if (Math.random() < 0.14) {
    var p = POWERS[Math.floor(Math.random() * POWERS.length)];
    drops.push({ x: br.x + BW / 2, y: br.y + BH / 2, type: p[0], color: p[1] });
  }
}
function updateDrops(dt) {
  drops.forEach(function (d) {
    d.y += 130 * dt;
    if (d.y + 9 >= PADDLE_Y && d.y - 9 <= PADDLE_Y + PADDLE_H && Math.abs(d.x - paddle.x) <= paddle.w / 2 + 12) {
      d.dead = true; power(d.type);
    }
    if (d.y > H + 20) d.dead = true;
  });
  drops = drops.filter(function (d) { return !d.dead; });
}
function power(t) {
  B.beep(880, 0.1, 'triangle', 0.12, 1.5); score += 20;
  if (t === 'W') paddle.wideT = 12;
  if (t === '+') lives = Math.min(6, lives + 1);
  if (t === 'S') balls.forEach(function (b) { b.slowT = 8; });
  if (t === 'M' && balls.length) {
    var src = balls[0], s = Math.sqrt(src.vx * src.vx + src.vy * src.vy), base = Math.atan2(src.vx, -src.vy);
    [-0.5, 0.5].forEach(function (off) {
      if (balls.length < 12) balls.push({ x: src.x, y: src.y, vx: Math.sin(base + off) * s, vy: -Math.abs(Math.cos(base + off) * s), slowT: src.slowT });
    });
  }
}
function burst(x, y, color) {
  for (var i = 0; i < 10; i++) {
    var a = Math.random() * Math.PI * 2, v = B.rand(60, 220);
    parts.push({ x: x, y: y, vx: Math.cos(a) * v, vy: Math.sin(a) * v - 60, life: B.rand(0.35, 0.7), color: color });
  }
  if (parts.length > 300) parts.splice(0, parts.length - 300);
}
function updateParts(dt) {
  parts.forEach(function (p) { p.x += p.vx * dt; p.y += p.vy * dt; p.vy += 500 * dt; p.life -= dt; });
  parts = parts.filter(function (p) { return p.life > 0; });
}
function gameOver() {
  state = 'over';
  if (score > best) { best = score; B.save('breakout.best', best); }
}

// ───────── 绘制 ─────────
function render() {
  R.begin('#2d1e4a');
  R.gradRect(0, 0, W, H, [[0, '#1b1b3a'], [1, '#2d1e4a']], true);
  drawHud();
  // 砖块只在被打中时变：按 brickRev 录成图层；正在闪白的几块另外盖在上面
  R.layer('bricks', brickRev, function () { bricks.forEach(drawBrick); });
  bricks.forEach(function (b) { if (b.flash > 0) R.rrect(b.x, b.y, BW, BH, 3, [1, 1, 1, b.flash]); });
  parts.forEach(function (p) { R.setAlpha(Math.min(1, p.life * 2)); R.rect(p.x - 2, p.y - 2, 4, 4, p.color); });
  R.setAlpha(1);
  drops.forEach(function (d) {
    R.rrect(d.x - 16, d.y - 9, 32, 18, 9, d.color);
    R.text(d.type, d.x, d.y + 1, 13, '#1b1b3a', 'center', 'bold');
  });
  // 挡板：上浅下深的竖直渐变（圆角挡板 = 圆角矩形底色 + 中段渐变盖上去）
  var px = paddle.x - paddle.w / 2;
  R.rrect(px, PADDLE_Y, paddle.w, PADDLE_H, 6, '#b7bbf7');
  R.gradRect(px + 6, PADDLE_Y, paddle.w - 12, PADDLE_H, [[0, '#e0e1ff'], [1, '#8e94f2']], true);
  balls.forEach(function (b) { R.circle(b.x, b.y, BALL_R, b.slowT > 0 ? '#9ad1ff' : '#ffffff'); });
  if (state === 'serve' && banner <= 0) R.text('点击屏幕发球', W / 2, PADDLE_Y - 60, 18, 'rgba(255,255,255,0.8)', 'center');
  if (banner > 0 && state !== 'over') {
    R.setAlpha(Math.min(1, banner * 2));
    R.text(state === 'clear' ? '过关！' : '第 ' + (level + 1 + round * LEVELS.length) + ' 关', W / 2, H * 0.55, 34, '#ffd166', 'center', 'bold');
    R.setAlpha(1);
  }
  if (userPaused && state !== 'over') {
    R.rect(0, 0, W, H, 'rgba(0,0,0,0.5)');
    R.text('已暂停', W / 2, H / 2 - 10, 32, '#ffffff', 'center', 'bold');
    R.text('点击继续', W / 2, H / 2 + 30, 16, '#cccccc', 'center');
  }
  if (state === 'over') drawOver();
  R.end();
}
function drawBrick(b) {
  R.rrect(b.x, b.y, BW, BH, 3, BRICK_COLORS[b.kind]);
  R.rect(b.x + 2, b.y + 2, BW - 4, 4, 'rgba(255,255,255,0.25)');
  if (b.kind === 'H' && b.hp === 1)              // 打过一下的硬砖画裂纹
    R.line([[b.x + BW * 0.3, b.y], [b.x + BW * 0.5, b.y + BH * 0.5], [b.x + BW * 0.4, b.y + BH]], 1.5, 'rgba(0,0,0,0.45)');
}
function drawHud() {
  var y = B.hudTop + 8;
  R.text('分数 ' + score, SIDE + 4, y, 16, '#ffffff', 'left', 'bold');
  R.text('第 ' + (level + 1 + round * LEVELS.length) + ' 关', W / 2, y, 16, '#ffffff', 'center', 'bold');
  for (var i = 0; i < lives; i++) R.circle(W - SIDE - 10 - i * 20, y, 6, '#ef476f');
}
function drawOver() {
  R.rect(0, 0, W, H, 'rgba(0,0,0,0.6)');
  R.text('游戏结束', W / 2, H * 0.36, 38, '#ffffff', 'center', 'bold');
  R.text('得分 ' + score, W / 2, H * 0.36 + 56, 22, '#ffd166', 'center', 'bold');
  R.text('最高 ' + best, W / 2, H * 0.36 + 88, 16, '#cccccc', 'center');
  againBtn = R.button('再来一局', W / 2, H * 0.36 + 150, 160, 46, '#ffd166', '#1b1b3a');
}

// 启动参数 auto=1：自动驾驶（挡板追最低那个往下走的球、自动发球），无头跑长稳 / 演示用
var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';
function autopilot() {
  if (state === 'serve' && banner <= 0) launch();
  var t = null;
  balls.forEach(function (b) { if (b.vy > 0 && (!t || b.y > t.y)) t = b; });
  if (t) paddle.x = t.x + Math.sin(t.y) * paddle.w * 0.3;   // 故意偏一点，出射角才会变
}
// 给无头 --eval 看进度
GameGlobal.__play = function () { return { state: state, level: level, round: round, score: score, lives: lives, bricks: bricks.length, balls: balls.length }; };

newGame();
B.loop(update, render);
