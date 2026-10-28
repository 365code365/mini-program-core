// 跳一跳：按住屏幕蓄力（小人和脚下的方块一起往下压），松开起跳，按得越久跳得越远。
// 落在下一个方块上 +1；正中心 +2，连续正中心 +4、+6……（最多 +32）；没落上去就掉下去，游戏结束。
// 方块越往后越小、越远。PC：按住空格蓄力。启动参数 auto=1：自动驾驶（演示 / 无头长稳）。
// 画面走 WebGL（js/scene.js：正交等距相机 + 光照 + 深度测试），文字是离屏 2D 画布传成的纹理（js/labels.js）。
// 2D 版（等距投影、画家算法排序、每帧现画渐变）在真机上每帧 9~30ms；WebGL 版把这些都交给 GPU。
var B = require('js/base.js');
var Scene = require('js/scene.js');
var Labels = require('js/labels.js');
var W = B.W, H = B.H;

var S = Math.min(W, H * 0.6) / 8.5;          // 一格多少逻辑像素
var PR = B.sys.pixelRatio || 2;
// 主画布按设备像素：小游戏的主画布默认是逻辑尺寸，WebGL 要自己乘 pixelRatio 才清楚（各引擎的适配层都这么做）
B.canvas.width = Math.round(W * PR); B.canvas.height = Math.round(H * PR);
var scene = Scene.make(B.canvas, W, H, S, 0.6);
if (!scene) throw new Error('这台设备拿不到 WebGL 上下文');
var labels = Labels.make(scene.gl, PR);
var camFocus = { x: 0, y: 0, z: 0 };

var MAX_HOLD = 1.6, SPEED = 4.2;             // 最多蓄 1.6 秒；每秒蓄力换 4.2 格距离
var JUMP_T = 0.5;                            // 空中时间（秒），与距离无关：远跳看起来更「用力」
var PERFECT = 0.22;                          // 离中心多近算正中心（格）
var PALETTE = [
  ['#f4f4f4', '#d9d9d9', '#bdbdbd'], ['#ffd7a8', '#f0b27a', '#d6925a'], ['#b8e0d2', '#8cc5b0', '#6aa893'],
  ['#f6b5c0', '#e58c9c', '#c96d7f'], ['#c3c8f5', '#9aa1e6', '#7a82cc'], ['#fbe38e', '#ecc95a', '#d1ab3a'],
  ['#9ad0f5', '#6db3e6', '#4f96cc'], ['#d9c2a7', '#bfa385', '#a08566']
];

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';

var state, blocks, cur, next, player, score, combo, best = Number(B.load('jump.best', 0)) || 0;
var hold = 0, holding = false, jump = null, fall = null, overAt = 0, clock = 0;
var fx = [], dust = [], ripple = null, spring = null, againBtn = null, newBest = false, autoHoldFor = 0;

function makeBlock(x, z, n) {
  var shrink = Math.max(0.55, 1 - n * 0.006);
  var colors = PALETTE[Math.floor(Math.random() * PALETTE.length)];
  return { x: x, z: z, w: B.rand(0.62, 1.0) * shrink, h: B.rand(0.7, 1.05), shape: Math.random() < 0.45 ? 'cyl' : 'cube',
    colors: colors, stripe: Math.random() < 0.3 ? 'rgba(255,255,255,0.35)' : null, comp: 0 };
}
function addNext() {
  // 下一个往 +x 或 +z 走（屏幕上是右上或左上），距离随分数慢慢拉开
  var dirX = Math.random() < 0.5;
  var d = B.rand(2.3, 3.4) + Math.min(1.2, score * 0.02);
  var nb = makeBlock(cur.x + (dirX ? d : 0), cur.z + (dirX ? 0 : d), score);
  // 两块不能挨着：中心距至少比两个半宽之和大 0.6
  var gap = d - cur.w - nb.w;
  if (gap < 0.6) { if (dirX) nb.x += 0.6 - gap; else nb.z += 0.6 - gap; }
  blocks.push(nb);
  if (blocks.length > 8) blocks.shift();
  next = nb;
}
function reset() {
  score = 0; combo = 0; newBest = false; fx = []; dust = []; ripple = null; spring = null;
  var first = makeBlock(0, 0, 0); first.w = 1.0; first.shape = 'cube'; first.colors = PALETTE[0];
  blocks = [first]; cur = first;
  addNext();
  player = { x: 0, y: first.h, z: 0, spin: 0 };
  focusCamera(true);
  state = 'ready';
}

// ───────── 输入：按下开始蓄力，抬起起跳 ─────────
function press() {
  if (state === 'over') return;
  if (state === 'ready') state = 'play';
  if (state === 'play' && !jump && !fall) { holding = true; hold = 0; }
}
function release() {
  if (!holding) return;
  holding = false;
  launch(Math.min(hold, MAX_HOLD));
}
B.on('down', function () { press(); });
B.on('up', function () { release(); });
B.on('tap', function (x, y) {
  if (state === 'over' && clock - overAt > 0.6 && B.hit(againBtn, x, y)) reset();
});
var keyWas = false;
function pollKeys() {
  var k = !!B.held.ok;
  if (k && !keyWas) { if (state === 'over' && clock - overAt > 0.6) reset(); else press(); }
  if (!k && keyWas) release();
  keyWas = k;
}
B.onHide(function () { if (holding) { holding = false; hold = 0; } });

// ───────── 起跳：算出落点、落到哪块上，按固定时长走一条抛物线 ─────────
function launch(t) {
  var dist = t * SPEED;
  var dx = next.x - player.x, dz = next.z - player.z, len = Math.sqrt(dx * dx + dz * dz) || 1;
  var tx = player.x + dx / len * dist, tz = player.z + dz / len * dist;
  var on = inside(next, tx, tz) ? next : inside(cur, tx, tz) ? cur : null;
  var y1 = on ? on.h : next.h;
  // 空翻轴 = 竖直向上 × 前进方向：绕它正着转，头朝前翻
  jump = { t: 0, x0: player.x, z0: player.z, y0: player.y, x1: tx, z1: tz, y1: y1, peak: 1.4 + dist * 0.18,
    on: on, ax: dz / len, az: -dx / len };
  // 方块弹回：从压下去的地方回弹，衰减振荡
  spring = { b: cur, c0: cur.comp, t: 0 };
  cur.comp = 0;
  B.beep(260 + t * 200, 0.12, 'triangle', 0.1, 1.8);
}
function inside(b, x, z) {
  if (b.shape === 'cyl') { var dx = x - b.x, dz = z - b.z; return dx * dx + dz * dz <= b.w * b.w; }
  return Math.abs(x - b.x) <= b.w && Math.abs(z - b.z) <= b.w;
}
function land() {
  var on = jump.on;
  player.x = jump.x1; player.z = jump.z1; player.y = jump.y1; player.spin = 0;
  jump = null;
  if (!on) {                                   // 没落在任何块上：往下掉
    fall = { v: 0 };
    B.beep(150, 0.4, 'sawtooth', 0.12, 0.4); B.buzz();
    return;
  }
  puff(player.x, player.y, player.z);
  if (on === cur) { B.beep(330, 0.05, 'sine', 0.06); return; }   // 原地小跳：不得分
  var off = Math.sqrt(Math.pow(player.x - on.x, 2) + Math.pow(player.z - on.z, 2));
  var gain;
  if (off <= PERFECT) {
    combo++;
    gain = Math.min(32, combo * 2);
    ripple = { b: on, t: 0 };
    var notes = [523, 659, 784, 1046, 1318];
    B.beep(notes[Math.min(combo - 1, notes.length - 1)], 0.18, 'triangle', 0.13);
  } else {
    combo = 0; gain = 1;
    B.beep(440, 0.06, 'square', 0.07);
  }
  score += gain;
  fx.push({ text: '+' + gain, x: player.x, y: player.y + 1.6, z: player.z, t: 0, perfect: gain > 1 });
  cur = on;
  addNext();
}
function puff(x, y, z) {
  for (var i = 0; i < 8; i++) {
    var a = i / 8 * Math.PI * 2;
    dust.push({ x: x, y: y, z: z, vx: Math.cos(a) * 1.2, vz: Math.sin(a) * 1.2, life: 0.35 });
  }
}

// ───────── 更新 ─────────
function update(dt) {
  clock += dt;
  pollKeys();
  if (AUTO) autopilot(dt);
  if (holding) {
    hold = Math.min(MAX_HOLD, hold + dt);
    cur.comp = hold / MAX_HOLD;
    player.y = cur.h * (1 - cur.comp * 0.25);
    // 蓄力时四周往小人身上聚的小点
    if (Math.random() < dt * 30) {
      var a = Math.random() * Math.PI * 2, r = B.rand(0.8, 1.4);
      dust.push({ x: player.x + Math.cos(a) * r, y: player.y + B.rand(0.2, 1.2), z: player.z + Math.sin(a) * r,
        vx: -Math.cos(a) * r * 2, vz: -Math.sin(a) * r * 2, life: 0.4, gather: true });
    }
  }
  if (spring) {
    spring.t += dt;
    spring.b.comp = spring.c0 * Math.exp(-7 * spring.t) * Math.cos(22 * spring.t);
    if (spring.t > 0.8) { spring.b.comp = 0; spring = null; }
  }
  if (jump) {
    jump.t = Math.min(1, jump.t + dt / JUMP_T);
    var t = jump.t;
    player.x = jump.x0 + (jump.x1 - jump.x0) * t;
    player.z = jump.z0 + (jump.z1 - jump.z0) * t;
    player.y = jump.y0 + (jump.y1 - jump.y0) * t + 4 * jump.peak * t * (1 - t);
    player.spin = t * Math.PI * 2;
    if (t >= 1) land();
  }
  if (fall) {
    fall.v += 30 * dt;
    player.y = Math.max(0, player.y - fall.v * dt);
    if (player.y <= 0 && state === 'play') {
      state = 'over'; overAt = clock;
      if (score > best) { best = score; newBest = true; B.save('jump.best', best); }
    }
  }
  if (ripple) { ripple.t += dt * 1.6; if (ripple.t >= 1) ripple = null; }
  fx.forEach(function (f) { f.t += dt; });
  fx = fx.filter(function (f) { return f.t < 0.9; });
  dust.forEach(function (d) { d.x += d.vx * dt; d.z += d.vz * dt; d.life -= dt; });
  dust = dust.filter(function (d) { return d.life > 0; });
  focusCamera(false, dt);
}
// 镜头对准当前块与下一块顶面的中点，平滑跟过去
function focusCamera(snap, dt) {
  var tx = (cur.x + next.x) / 2, ty = (cur.h + next.h) / 2, tz = (cur.z + next.z) / 2;
  var k = snap ? 1 : 1 - Math.exp(-(dt || 0) * 5);
  camFocus.x += (tx - camFocus.x) * k; camFocus.y += (ty - camFocus.y) * k; camFocus.z += (tz - camFocus.z) * k;
  scene.setCamera(camFocus.x, camFocus.y, camFocus.z);
}
// 自动驾驶：算出正中心需要按多久，加一点误差；落地停一下再按
function autopilot(dt) {
  if (state === 'over') { if (clock - overAt > 1.2) reset(); return; }
  if (jump || fall) return;
  if (holding) { if (hold >= autoHoldFor) release(); return; }
  autoHoldFor -= dt;
  if (autoHoldFor > -0.4) return;               // 落地后歇 0.4 秒
  var d = Math.sqrt(Math.pow(next.x - player.x, 2) + Math.pow(next.z - player.z, 2));
  press();
  autoHoldFor = d / SPEED + B.rand(-0.07, 0.07);     // ±0.3 格：有正中心、有普通落点，块小了偶尔会掉
}

// ───────── 绘制 ─────────
function render() {
  var hue = Math.round((200 + score * 3) % 360);
  scene.begin(hsl(hue, 0.45, 0.88), hsl((hue + 30) % 360, 0.4, 0.72));
  // 不透明：方块、小人（深度测试管遮挡，不用排序）
  blocks.forEach(function (b) { scene.block(b, b.h * (1 - b.comp * 0.25)); });
  var sq = holding ? hold / MAX_HOLD : 0;
  scene.piece(player.x, player.y, player.z, sq, player.spin, jump ? jump.ax : 0, jump ? jump.az : 1);
  // 半透明：影子（空中不画，不给落点提示）、涟漪、粒子
  scene.beginTransparent();
  if (!jump && !fall) scene.shadow(player.x, player.y, player.z);
  if (ripple) scene.ripple(ripple.b, ripple.b.h * (1 - ripple.b.comp * 0.25), ripple.t);
  scene.dust(dust);
  // 屏幕层：飘字、HUD、结算
  scene.beginScreen();
  fx.forEach(function (f, i) {
    var p = scene.project(f.x, f.y + f.t * 0.8, f.z);
    label('fx' + i, f.text, f.perfect ? 22 : 18, f.perfect ? '#ffffff' : 'rgba(60,60,80,0.9)', 'bold', p[0], p[1], 'center', 1 - f.t / 0.9);
  });
  drawHud();
}
// 文字：align 'left' 时 x 是左边，'center' 时是中心；y 是中线
function label(key, text, size, color, weight, x, y, align, alpha, bg) {
  var e = labels.get(key, text, size, color, weight, bg);
  var lx = align === 'left' ? x : x - e.w / 2;
  scene.image(e.tex, lx, y - e.h / 2, e.w, e.h, alpha === undefined ? 1 : alpha);
  return { x: lx, y: y - e.h / 2, w: e.w, h: e.h };
}
// 本引擎的画布颜色解析暂时不认 hsl()（会变黑）；这里直接算成 0~1 的 rgb 给着色器
function hsl(h, s, l) {
  var c = (1 - Math.abs(2 * l - 1)) * s, x = c * (1 - Math.abs((h / 60) % 2 - 1)), m = l - c / 2;
  var r = 0, g = 0, b = 0, k = Math.floor(h / 60) % 6;
  if (k === 0) { r = c; g = x; } else if (k === 1) { r = x; g = c; } else if (k === 2) { g = c; b = x; }
  else if (k === 3) { g = x; b = c; } else if (k === 4) { r = x; b = c; } else { r = c; b = x; }
  return [r + m, g + m, b + m, 1];
}
function drawHud() {
  var y = B.hudTop + 18;
  if (state !== 'over') {                      // 结算面板上有大号分数，左上角就不重复了
    label('score', score, 40, '#333344', 'bold', 22, y + 8, 'left');
    label('best', '最高 ' + best, 14, 'rgba(50,50,70,0.7)', 'normal', 24, y + 42, 'left');
  }
  if (combo >= 2 && state === 'play') label('combo', '连续命中中心 ×' + combo, 15, 'rgba(50,50,70,0.75)', 'normal', W / 2, y + 70, 'center');
  if (state === 'ready') {
    label('t1', '跳一跳', 40, '#333344', 'bold', W / 2, H * 0.22, 'center');
    label('t2', '按住屏幕蓄力，松开跳到下一个方块', 15, 'rgba(50,50,70,0.8)', 'normal', W / 2, H * 0.22 + 44, 'center');
    label('t3', '落在正中心有连击加分', 13, 'rgba(50,50,70,0.6)', 'normal', W / 2, H * 0.22 + 68, 'center');
  }
  if (state === 'over') {
    var a = Math.min(1, (clock - overAt) / 0.3);
    scene.fill([0, 0, 0, 0.55 * a], 0, 0, W, H);
    label('o1', '本局得分', 18, '#dddddd', 'normal', W / 2, H * 0.3, 'center', a);
    label('o2', score, 64, '#ffffff', 'bold', W / 2, H * 0.3 + 56, 'center', a);
    label('o3', newBest ? '新纪录！' : '历史最高 ' + best, 16, newBest ? '#ffd166' : '#dddddd', 'bold', W / 2, H * 0.3 + 112, 'center', a);
    againBtn = label('again', '再玩一局', 21, '#ffffff', 'bold', W / 2, H * 0.3 + 190, 'center', a, ['#07c160', 0, 48, 13]);
  }
}

GameGlobal.__play = function () { return { state: state, score: score, best: best, combo: combo, blocks: blocks.length }; };

reset();
if (AUTO) state = 'play';
B.loop(update, render);
