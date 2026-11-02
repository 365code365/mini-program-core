// 3D 迷宫（竖屏）：每关随机生成一张迷宫（DFS，js/maze.js），俯视追尾相机跟着小球走；收集全部宝石打开出口，
// 赶在倒计时结束前走到出口过关。10 关，迷宫从 6×6 长到 15×15；按剩余时间给 1~3 星，进度与每关最佳存 storage。
// 操作：屏幕下半部按住拖动 = 虚拟摇杆；「重力」按钮开 wx.startDeviceMotionListening 倾斜控制（不支持时提示并留在摇杆）；
// PC 方向键 / WASD，P 暂停。启动参数 auto=1：自动试玩（BFS 找最近的宝石，再去出口；过关自动下一关）。
// three.js r160（libs/three.js，`bash tools/fetch-play-libs.sh` 拉）经 js/adapter.js 跑；界面文字走 js/hud.js。
var B = require('js/base.js');
var canvas = require('js/adapter.js');
var THREE = require('libs/three.js');
var Hud = require('js/hud.js');
var M = require('js/maze.js');
var World = require('js/world.js');
var Ctl = require('js/control.js');
var Ui = require('js/ui.js');
var W = B.W, H = B.H, PR = B.sys.pixelRatio || 2;

// 不开 MSAA：画面是 2 倍像素比，锯齿看不太出来；无头 SwiftShader 上开着每帧中位 17.4ms、关掉 9.4ms（墙和地面铺满全屏，填充率是大头）
var renderer = new THREE.WebGLRenderer({ canvas: canvas, antialias: false });
renderer.setPixelRatio(PR);
renderer.setSize(W, H, false);
var hud = Hud.make(THREE, renderer, W, H, PR);
var ui = Ui.make(hud);
var ctl = Ctl.make();

var SPEED = 3.0, RADIUS = 0.28, LEVELS = 10;
var scene = new THREE.Scene();
scene.background = new THREE.Color(0x1b2338);
scene.fog = new THREE.Fog(0x1b2338, 10, 26);
var cam = new THREE.PerspectiveCamera(55, W / H, 0.1, 120);
scene.add(new THREE.HemisphereLight(0xffffff, 0x445566, 0.95));
var sun = new THREE.DirectionalLight(0xffffff, 1.25); sun.position.set(-6, 12, 8); scene.add(sun);
var world = World.make(THREE, scene);

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';

// 进度：{ unlocked: 已解锁到第几关, best: { 关号: { score, time, stars } } }；读坏了从头来
var P = B.load('maze.progress', null);
if (!P || typeof P !== 'object' || !P.best) P = { unlocked: 1, best: {} };
P.unlocked = Math.max(1, Math.min(LEVELS, Number(P.unlocked) || 1));

var G = { state: 'title', pick: P.unlocked, level: 1, clock: 0, time: 0, limit: 60, view: 0, paused: false, open: false, openAt: 0,
  endAt: 0, px: 1.5, pz: 1.5, vx: 0, vz: 0, roll: { x: 0, z: 0 }, gems: [], maze: null, seed: 0, score: 0, stars: 0, newBest: false,
  tris: 0, calls: 0, picked: 0, cleared: 0, auto: null };
var camPos = new THREE.Vector3(0, 20, 10), camLook = new THREE.Vector3(), tmp = new THREE.Vector3();

function build(level) {
  var n = 5 + level;
  G.level = level;
  G.seed = (Date.now() ^ (level * 7919)) >>> 0;
  G.maze = M.generate(n, G.seed);
  G.gems = M.placeGems(G.maze, 3 + Math.ceil(level / 2)).map(function (c) { return { i: c.i, j: c.j, taken: false, fade: 0 }; });
  // 时限按「贪心走一遍」的步数定：保证一定走得完，又不至于随便逛
  G.limit = Math.ceil(M.tourLength(G.maze, G.gems) / SPEED * 1.7 + 12);
  G.tris = world.build(G.maze, G.gems, (0.55 + level * 0.083) % 1).tris;
  G.px = G.maze.start.i + 0.5; G.pz = G.maze.start.j + 0.5; G.vx = G.vz = 0;
  G.time = 0; G.open = false; G.paused = false; G.newBest = false; G.auto = null;
}
function startLevel(level) {
  build(level);
  G.state = 'play';
  B.beep(523, 0.12, 'triangle', 0.1);
  if (ctl.tilt.on) ctl.recalibrate();
}
function toTitle() { G.state = 'title'; G.pick = Math.min(G.pick, P.unlocked); build(G.pick); G.state = 'title'; }

function clearLevel() {
  G.state = 'clear'; G.endAt = G.clock; G.cleared++;
  var remain = Math.max(0, G.limit - G.time), ratio = remain / G.limit;
  G.stars = ratio >= 0.45 ? 3 : ratio >= 0.2 ? 2 : 1;
  G.score = 300 + G.gems.length * 100 + Math.floor(remain * 10);
  var b = P.best[G.level];
  G.newBest = !b || G.score > b.score;
  P.best[G.level] = { score: Math.max(G.score, b ? b.score : 0), time: Math.min(G.time, b ? b.time : 1e9), stars: Math.max(G.stars, b ? b.stars : 0) };
  P.unlocked = Math.min(LEVELS, Math.max(P.unlocked, G.level + 1));
  B.save('maze.progress', P);
  [659, 784, 1047].forEach(function (f, i) { setTimeout(function () { B.beep(f, 0.14, 'triangle', 0.12); }, i * 90); });
}
function timeUp() {
  G.state = 'over'; G.endAt = G.clock;
  B.beep(196, 0.45, 'sawtooth', 0.12, 0.5);
  try { wx.vibrateLong && wx.vibrateLong({}); } catch (e) {}
}

// ───────── 输入 ─────────
function action(k) {
  if (k === 'pause' && G.state === 'play') G.paused = true;
  else if (k === 'resume') G.paused = false;
  else if (k === 'quit') toTitle();
  else if (k === 'view' && G.state === 'play') G.view = 1 - G.view;
  else if (k === 'tilt' && G.state === 'play') ctl.toggleTilt(false);
  else if (k === 'prev' && G.pick > 1) { G.pick--; build(G.pick); G.state = 'title'; }
  else if (k === 'next' && G.pick < P.unlocked) { G.pick++; build(G.pick); G.state = 'title'; }
  else if (k === 'start') startLevel(G.pick);
  else if (k === 'nextLv') { G.pick = Math.min(LEVELS, G.level + 1); startLevel(G.pick); }
  else if (k === 'retry') startLevel(G.level);
  else if (k === 'menu') toTitle();
  else return false;
  B.beep(880, 0.04, 'square', 0.05);
  return true;
}
function playOnly(k) { return k === 'pause' || k === 'view' || k === 'tilt'; }
B.on('down', function (x, y, id) {
  for (var k in ui.btns) {
    if (!B.hit(ui.btns[k], x, y)) continue;
    if (G.state !== 'play' && playOnly(k)) continue;        // 结算页下面还画着游戏中的按钮，点不到它们
    if (G.paused && playOnly(k)) continue;
    if (action(k)) return;
  }
  if (G.state === 'play' && !G.paused && y > H * 0.4) ctl.down(x, y, id);
});
B.on('move', function (x, y, id) { ctl.move(x, y, id); });
B.on('up', function (x, y, id) { ctl.up(x, y, id); });
B.on('key', function (k) {
  if (k === 'pause') { if (G.state === 'play') G.paused = !G.paused; return; }
  if (k !== 'ok') return;
  if (G.state === 'title') action('start');
  else if (G.paused) action('resume');
  else if (G.state === 'clear') action(G.level < LEVELS ? 'nextLv' : 'retry');
  else if (G.state === 'over') action('retry');
});
B.onHide(function () { if (G.state === 'play') G.paused = true; });

// ───────── 物理：圆与墙砖（轴对齐单位方块）求最近点推开；每帧位移 ≤ 0.15 < 半径，不会穿墙 ─────────
function collide() {
  for (var pass = 0; pass < 2; pass++) {
    var ci = Math.floor(G.px), cj = Math.floor(G.pz);
    for (var j = cj - 1; j <= cj + 1; j++) for (var i = ci - 1; i <= ci + 1; i++) {
      if (!M.wall(G.maze, i, j)) continue;
      var nx = Math.max(i, Math.min(i + 1, G.px)), nz = Math.max(j, Math.min(j + 1, G.pz));
      var dx = G.px - nx, dz = G.pz - nz, d2 = dx * dx + dz * dz;
      if (d2 >= RADIUS * RADIUS || d2 < 1e-12) continue;
      var d = Math.sqrt(d2), push = RADIUS - d;
      G.px += dx / d * push; G.pz += dz / d * push;
      if (Math.abs(dx) > Math.abs(dz)) G.vx *= 0.3; else G.vz *= 0.3;   // 顶墙时这一向的速度卸掉，贴墙滑
    }
  }
}

// 自动试玩：BFS 到最近的宝石（都拿了就去出口），沿砖块中心走
function autopilot() {
  var a = G.auto, ti = Math.floor(G.px), tj = Math.floor(G.pz);
  if (!a || a.k >= a.path.length) {
    var target = null, b = M.bfs(G.maze, ti, tj), best = 1e9;
    G.gems.forEach(function (g) { var d = b.dist[g.j * G.maze.T + g.i]; if (!g.taken && d >= 0 && d < best) { best = d; target = g; } });
    a = G.auto = { path: M.path(G.maze, { i: ti, j: tj }, target || G.maze.exit), k: 1 };
    if (a.path.length < 2) return { x: G.maze.exit.i + 0.5 - G.px, y: G.maze.exit.j + 0.5 - G.pz };
  }
  var p = a.path[a.k], dx = p.i + 0.5 - G.px, dz = p.j + 0.5 - G.pz, d = Math.sqrt(dx * dx + dz * dz);
  if (d < 0.12) { a.k++; return autopilot(); }
  return { x: dx / d, y: dz / d };
}

function update(dt) {
  G.clock += dt;
  if (AUTO) autoMeta();
  if (G.state === 'play' && !G.paused) {
    G.time += dt;
    var v = AUTO ? autopilot() : ctl.vector(), k = 1 - Math.exp(-dt * 10);
    G.vx += (v.x * SPEED - G.vx) * k; G.vz += (v.y * SPEED - G.vz) * k;
    var ox = G.px, oz = G.pz;
    G.px += G.vx * dt; G.pz += G.vz * dt;
    collide();
    // 滚动：走过的距离 / 半径 = 转过的弧度
    G.roll.x += (G.pz - oz) / RADIUS; G.roll.z -= (G.px - ox) / RADIUS;
    G.gems.forEach(function (g) {
      if (g.taken) { g.fade = Math.min(1, g.fade + dt); return; }
      var dx = g.i + 0.5 - G.px, dz = g.j + 0.5 - G.pz;
      if (dx * dx + dz * dz > 0.2) return;
      g.taken = true; G.picked++;
      var got = G.gems.filter(function (x) { return x.taken; }).length;
      B.beep(660 * Math.pow(1.12, got), 0.12, 'triangle', 0.12); B.buzz();
      if (got === G.gems.length) { G.open = true; G.openAt = G.clock; G.auto = null; }
    });
    var ex = G.maze.exit.i + 0.5 - G.px, ez = G.maze.exit.j + 0.5 - G.pz;
    if (G.open && ex * ex + ez * ez < 0.16) clearLevel();
    else if (G.time >= G.limit) timeUp();
  }
  world.update(G.clock, G.gems, G.open, G.px, G.pz, G.roll);
  camera(dt);
}
// 自动试玩的「人」：标题页直接开始；结算页停一会儿点下一关 / 再试；开局试一次重力感应（不支持就回摇杆，不弹提示）
function autoMeta() {
  if (G.state === 'title' && G.clock > 0.5) { if (!G.triedTilt) { G.triedTilt = true; ctl.toggleTilt(true); } startLevel(G.pick); }
  else if ((G.state === 'clear' || G.state === 'over') && G.clock - G.endAt > 1.5) action(G.state === 'clear' && G.level < LEVELS ? 'nextLv' : 'retry');
}
// 相机：游戏中在小球后上方追尾（近景 / 俯视两档），标题页绕迷宫中心慢慢转
function camera(dt) {
  var m = G.maze, k = 1 - Math.exp(-dt * 5);
  if (G.state === 'title') {
    var c = m.T / 2, a = G.clock * 0.25, r = m.T * 0.75;
    camPos.set(c + Math.cos(a) * r, m.T * 0.9, c + Math.sin(a) * r);
    camLook.set(c, 0, c);
    scene.fog.far = 200;
  } else {
    var off = G.view ? [0, 16, 2.4] : [0, 10.5, 3.0];
    camPos.lerp(tmp.set(G.px + off[0], off[1], G.pz + off[2]), k);
    camLook.lerp(tmp.set(G.px, 0, G.pz - 0.5), k);
    scene.fog.far += ((G.view ? 34 : 22) - scene.fog.far) * k;
  }
  scene.fog.near = scene.fog.far * 0.45;
  cam.position.copy(camPos);
  cam.lookAt(camLook);
}

function render() {
  renderer.render(scene, cam);
  G.calls = renderer.info.render.calls;              // 只算 3D 这一遍（HUD 那一遍另算）
  hud.begin();
  ui.draw(G, ctl, P);
  hud.render();
}

GameGlobal.__play = function () {
  return { state: G.state, level: G.level, unlocked: P.unlocked, gems: G.gems.filter(function (g) { return g.taken; }).length, need: G.gems.length,
    time: Math.round(G.time * 10) / 10, limit: G.limit, picked: G.picked, cleared: G.cleared, score: G.score, total: ui.total(P),
    pos: [Math.round(G.px * 100) / 100, Math.round(G.pz * 100) / 100], maze: G.maze.T, tilt: ctl.tilt.state, calls3d: G.calls, tris: G.tris };
};

build(G.pick);
B.loop(update, render);
