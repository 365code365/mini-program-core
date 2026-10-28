// 3D 跑酷：三条跑道，左右滑换道、上滑跳、下滑铲；躲开路障（矮的跳、高的铲）与火车（只能换道），吃金币。越跑越快。
// three.js r160（libs/three.js，`bash tools/fetch-play-libs.sh` 拉）经 js/adapter.js 跑；界面文字走 js/hud.js。
// PC：方向键。启动参数 auto=1：自动驾驶（演示 / 无头长稳）。
var B = require('js/base.js');
var canvas = require('js/adapter.js');
var THREE = require('libs/three.js');
var Hud = require('js/hud.js');
var World = require('js/world.js');
var W = B.W, H = B.H, PR = B.sys.pixelRatio || 2;

var renderer = new THREE.WebGLRenderer({ canvas: canvas, antialias: true });
renderer.setPixelRatio(PR);
renderer.setSize(W, H, false);
var hud = Hud.make(THREE, renderer, W, H, PR);
var world = World.make(THREE, W / H);

var LANE = 2.2, GRAV = 40, JUMP_V = 13, SLIDE_T = 0.7;
var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';

var state, p, speed, dist, coins, best = Number(B.load('runner.best', 0)) || 0, clock = 0, overAt = 0, newBest = false, againBtn = null;
function reset() {
  world.reset();
  p = { lane: 0, x: 0, y: 0, vy: 0, slide: 0, run: 0 };
  speed = 14; dist = 0; coins = 0; newBest = false;
  state = 'ready';
}
function score() { return Math.floor(dist / 2) + coins * 10; }

// ───────── 输入 ─────────
function act(k) {
  if (state === 'over') { if (clock - overAt > 0.8) reset(); return; }
  if (state === 'ready') { state = 'play'; if (k === 'tap') return; }
  if (k === 'left' && p.lane > -1) { p.lane--; B.beep(500, 0.05, 'sine', 0.07); }
  if (k === 'right' && p.lane < 1) { p.lane++; B.beep(500, 0.05, 'sine', 0.07); }
  if (k === 'up' && p.y <= 0.001) { p.vy = JUMP_V; p.slide = 0; B.beep(620, 0.08, 'triangle', 0.08, 1.5); }
  if (k === 'down') { if (p.y > 0) p.vy = -JUMP_V * 1.5; p.slide = SLIDE_T; B.beep(260, 0.08, 'sine', 0.07); }
}
B.on('swipe', act);
B.on('tap', function () { act('tap'); });
B.on('key', function (k) { act(k === 'ok' ? 'up' : k); });
B.onHide(function () { if (state === 'play') state = 'paused'; });
B.onShow(function () { if (state === 'paused') state = 'play'; });

// ───────── 更新 ─────────
function update(dt) {
  clock += dt;
  if (AUTO) autopilot();
  if (state === 'play') {
    speed = Math.min(34, speed + dt * 0.35);
    dist += speed * dt;
    p.x += (p.lane * LANE - p.x) * (1 - Math.exp(-dt * 14));
    p.vy -= GRAV * dt; p.y = Math.max(0, p.y + p.vy * dt); if (p.y === 0) p.vy = 0;
    p.slide = Math.max(0, p.slide - dt);
    p.run += dt * speed * 0.9;
    var hit = world.step(dt, speed, p, function () { coins++; B.beep(1200, 0.06, 'triangle', 0.08); });
    if (hit) die();
  } else if (state === 'over') {
    world.step(dt, 0, p, null);
  }
  world.pose(p, state, clock);
  world.camera(p, dt, state);
}
function die() {
  state = 'over'; overAt = clock;
  B.beep(140, 0.5, 'sawtooth', 0.14, 0.4); B.buzz();
  var s = score();
  if (s > best) { best = s; newBest = true; B.save('runner.best', best); }
}
// 自动驾驶：看前方 12 格内本道的障碍：火车就换到空的道，矮栏跳，高栏铲
function autopilot() {
  if (state === 'over') { if (clock - overAt > 1.5) reset(); return; }
  if (state === 'ready') { state = 'play'; return; }
  var look = world.ahead(p.lane, speed * 0.55 + 3);
  if (!look) return;
  if (look.kind === 'train') {
    var free = [-1, 0, 1].filter(function (l) { return Math.abs(l - p.lane) === 1 && !world.ahead(l, speed * 0.55 + 6); });
    if (free.length) act(free[0] < p.lane ? 'left' : 'right');
  } else if (look.kind === 'low' && p.y === 0 && look.z > -speed * 0.35) act('up');
  else if (look.kind === 'high' && !p.slide && look.z > -speed * 0.3) act('down');
}

function render() {
  renderer.render(world.scene, world.cam);
  hud.begin();
  var y = B.hudTop + 14;
  if (state !== 'over') {
    hud.text('score', score(), 20, y, 32, '#ffffff', 'left', 'bold', { outline: ['rgba(0,0,0,0.35)', 2] });
    hud.text('coins', '金币 ' + coins, 22, y + 34, 15, '#ffd84d', 'left', 'bold', { outline: ['rgba(0,0,0,0.35)', 1.5] });
    hud.text('speed', Math.round(speed * 3.6) + ' km/h', W - 18, y, 14, '#ffffff', 'right', 'bold', { outline: ['rgba(0,0,0,0.35)', 1.5] });
  }
  if (state === 'ready') {
    hud.rect('mask0', 0, H * 0.14, W, 130, '#000000', 0.25);
    hud.text('t1', '3D 跑酷', W / 2, H * 0.14 + 38, 36, '#ffffff', 'center', 'bold');
    hud.text('t2', '左右滑换道 · 上滑跳 · 下滑铲', W / 2, H * 0.14 + 80, 15, '#ffffff', 'center');
    hud.text('t3', '点一下开始', W / 2, H * 0.14 + 106, 13, 'rgba(255,255,255,0.8)', 'center');
  }
  if (state === 'over') {
    var a = Math.min(1, (clock - overAt) / 0.4);
    hud.rect('mask', 0, 0, W, H, '#000000', 0.5 * a);
    hud.text('o1', '撞上了！', W / 2, H * 0.24, 34, '#ffffff', 'center', 'bold', { alpha: a });
    hud.text('o2', score(), W / 2, H * 0.24 + 64, 60, '#ffffff', 'center', 'bold', { alpha: a });
    hud.text('o3', Math.floor(dist) + ' 米 · 金币 ' + coins, W / 2, H * 0.24 + 112, 16, '#dddddd', 'center', 'normal', { alpha: a });
    hud.text('o4', newBest ? '新纪录！' : '最高 ' + best, W / 2, H * 0.24 + 140, 16, newBest ? '#ffd166' : '#dddddd', 'center', 'bold', { alpha: a });
    if (clock - overAt > 0.8) againBtn = hud.button('again', '再跑一次', W / 2, H * 0.72, 180, 50, '#ffd84d', '#333333');
  }
  hud.render();
}

GameGlobal.__play = function () { return { state: state, score: score(), dist: Math.floor(dist), coins: coins, speed: +speed.toFixed(1), objects: world.count() }; };

reset();
if (AUTO) state = 'play';
B.loop(update, render);
