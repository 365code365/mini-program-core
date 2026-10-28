// 3D 叠方块（Stack）：方块在塔顶来回滑，点一下放下；伸出塔外的部分被切掉、掉下去，塔顶越来越小。
// 正好对齐（误差 < 0.1）不切、连击，连击 3 次起每次往回长一点。完全没压上就结束。
// three.js r160（libs/three.js，`bash tools/fetch-play-libs.sh` 拉）经 js/adapter.js 跑；界面文字走 js/hud.js。
// PC：空格放下。启动参数 auto=1：自动驾驶（演示 / 无头长稳）。
var B = require('js/base.js');
var canvas = require('js/adapter.js');
var THREE = require('libs/three.js');
var Hud = require('js/hud.js');
var W = B.W, H = B.H, PR = B.sys.pixelRatio || 2;

var renderer = new THREE.WebGLRenderer({ canvas: canvas, antialias: true });
renderer.setPixelRatio(PR);
renderer.setSize(W, H, false);
var hud = Hud.make(THREE, renderer, W, H, PR);

var BASE = 3, SLAB = 0.5, PERFECT = 0.1, RANGE = 5.5;
var scene = new THREE.Scene();
scene.background = new THREE.Color();
var aspect = W / H, VIEW = 12;                         // 正交相机竖向看 12 格
var cam = new THREE.OrthographicCamera(-VIEW * aspect / 2, VIEW * aspect / 2, VIEW / 2, -VIEW / 2, 0.1, 200);
scene.add(new THREE.AmbientLight(0xffffff, 0.55));
var sun = new THREE.DirectionalLight(0xffffff, 1.1); sun.position.set(6, 12, 4); scene.add(sun);
scene.add(sun.target);
var box = new THREE.BoxGeometry(1, 1, 1);

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';

var state, tower, moving, chunks, score, combo, best = Number(B.load('stack.best', 0)) || 0, camY = 0, clock = 0;
var hueBase = Math.random() * 360, overAt = 0, playAt = 0, againBtn = null, flash = 0, newBest = false, autoWait = 0;

function color(level) { return new THREE.Color().setHSL(((hueBase + level * 5) % 360) / 360, 0.55, 0.6); }
// 材质池：方块的材质用完放回池里、下次改个颜色接着用，**不 dispose**。
// 为什么不每块 new 一个、用完 dispose：three.js 在最后一个用某着色器的材质 dispose 时会把那个 program 删掉，
// 重开一局 clear() 把所有方块的材质都 dispose 了，下一帧就要重新编译 Lambert 着色器（模拟器上每次重开 25~200ms 一顿）；
// 新材质第一次画也要走一遍 initMaterial。池子的大小就是同时在场的方块数（塔 ≤ 60 层 + 正在掉的碎块）
var matPool = [];
function mat(level) {
  var m = matPool.pop() || new THREE.MeshLambertMaterial();
  m.color.copy(color(level));
  return m;
}
function release(mesh) { scene.remove(mesh); matPool.push(mesh.material); }
function slab(x, z, w, d, level) {
  var m = new THREE.Mesh(box, mat(level));
  m.scale.set(w, SLAB, d);
  m.position.set(x, level * SLAB, z);
  scene.add(m);
  return { mesh: m, x: x, z: z, w: w, d: d, level: level };
}
function clear() {
  (tower || []).concat(chunks || []).forEach(function (s) { release(s.mesh); });
  if (moving) release(moving.mesh);
}
function reset() {
  clear();
  hueBase = Math.random() * 360;
  tower = [];
  // 底座：一根高柱子（只露顶上几层），看起来塔是从下面长上来的
  var base = slab(0, 0, BASE, BASE, 0);
  base.mesh.scale.y = SLAB * 20; base.mesh.position.y = -SLAB * 9.5;
  tower.push(base);
  chunks = []; moving = null; score = 0; combo = 0; flash = 0; newBest = false;
  state = 'ready';
  spawn();
}
// 新的一块从塔顶的一侧滑进来：奇数层沿 x、偶数层沿 z
function spawn() {
  var top = tower[tower.length - 1], level = top.level + 1, axis = level % 2 ? 'x' : 'z';
  var s = slab(top.x, top.z, top.w, top.d, level);
  s.axis = axis; s.t = -1; s.dir = 1;
  s[axis] = top[axis] - RANGE;
  s.mesh.position[axis] = s[axis];
  s.speed = Math.min(9, 3.2 + level * 0.08);
  moving = s;
}

function drop() {
  // 「再来一局」直接开局（方块马上滑起来），不回标题页再等一下点
  if (state === 'over') { if (clock - overAt > 0.6) { reset(); start(); } return; }
  // 开局那一下只开始滑，不放：方块这时还停在塔外 RANGE 处，当场放下必然整块落空（从前每局都是 0 分就结束）
  if (state === 'ready') { start(); return; }
  // 开局后 0.35s 内的点按不算：连点两下（点「再来一局」/ 开局时手指多点了一下）的第二下会落在方块还没滑到塔上的时候，
  // 一下就整块落空结束，看起来就是「还没开始就结束了」
  if (clock - playAt < 0.35) return;
  if (!moving) return;
  var top = tower[tower.length - 1], s = moving, ax = s.axis, size = ax === 'x' ? 'w' : 'd';
  var delta = s[ax] - top[ax], overlap = top[size] - Math.abs(delta);
  if (overlap <= 0) {                                    // 完全没压上：整块掉下去
    chunks.push({ mesh: s.mesh, vy: 0, spin: new THREE.Vector3(Math.random(), 0, Math.random()).normalize(), w: 3 });
    moving = null;
    gameOver();
    return;
  }
  if (Math.abs(delta) < PERFECT) {
    // 正好对齐：吸到塔顶上；连击 3 次起每次往回长 0.15（最多长回底座大小）
    combo++;
    s[ax] = top[ax];
    if (combo >= 3) s[size] = Math.min(BASE, s[size] + 0.15);
    flash = 1;
    B.beep(440 * Math.pow(1.06, Math.min(combo, 24)), 0.15, 'triangle', 0.12);
  } else {
    combo = 0;
    // 切：留下重叠部分，伸出去的那截变成往下掉的碎块
    var cut = Math.abs(delta), keepC = top[ax] + delta / 2, side = delta > 0 ? 1 : -1;
    s[size] = overlap; s[ax] = keepC;
    var cx = ax === 'x' ? keepC + side * (overlap / 2 + cut / 2) : s.x, cz = ax === 'z' ? keepC + side * (overlap / 2 + cut / 2) : s.z;
    var piece = new THREE.Mesh(box, mat(s.level));
    piece.scale.set(ax === 'x' ? cut : s.w, SLAB, ax === 'z' ? cut : s.d);
    piece.position.set(cx, s.mesh.position.y, cz);
    scene.add(piece);
    chunks.push({ mesh: piece, vy: 0, spin: new THREE.Vector3(ax === 'z' ? side : 0, 0, ax === 'x' ? -side : 0), w: 2.5 });
    B.beep(300, 0.08, 'square', 0.08);
  }
  s.mesh.scale.set(s.w, SLAB, s.d);
  s.mesh.position.set(s.x, s.level * SLAB, s.z);
  tower.push(s);
  moving = null;
  score++;
  if (tower.length > 60) { var old = tower.splice(1, 1)[0]; release(old.mesh); }   // 远在镜头外的老层回收
  spawn();
}
function start() { state = 'play'; playAt = clock; }
function gameOver() {
  state = 'over'; overAt = clock;
  B.beep(160, 0.4, 'sawtooth', 0.12, 0.5); B.buzz();
  if (score > best) { best = score; newBest = true; B.save('stack.best', best); }
}
B.on('down', drop);
B.on('key', function (k) { if (k === 'ok') drop(); });

function update(dt) {
  clock += dt;
  flash = Math.max(0, flash - dt * 3);
  if (moving && state !== 'over') {
    var s = moving, ax = s.axis, top = tower[tower.length - 1];
    if (state === 'play') {
      s[ax] += s.dir * s.speed * dt;
      if (s[ax] > top[ax] + RANGE) { s[ax] = top[ax] + RANGE; s.dir = -1; }
      if (s[ax] < top[ax] - RANGE) { s[ax] = top[ax] - RANGE; s.dir = 1; }
    }
    s.mesh.position[ax] = s[ax];
  }
  if (AUTO) autopilot(dt);
  chunks.forEach(function (c) {
    c.vy -= 18 * dt;
    c.mesh.position.y += c.vy * dt;
    c.mesh.rotateOnWorldAxis(c.spin, c.w * dt);
  });
  chunks = chunks.filter(function (c) {
    if (c.mesh.position.y > camY - 30) return true;
    release(c.mesh); return false;
  });
  // 镜头跟塔顶往上走；结束时往后拉远，看整座塔
  var topY = tower[tower.length - 1].level * SLAB;
  camY += (topY - camY) * (1 - Math.exp(-dt * 4));
  var zoom = state === 'over' ? Math.max(1, Math.min(4, (topY + 6) / 10)) : 1;
  cam.zoom += (1 / zoom - cam.zoom) * (1 - Math.exp(-dt * 3));
  cam.updateProjectionMatrix();
  cam.position.set(10, camY + 10, 10);
  cam.lookAt(0, camY - 1.5, 0);
  sun.position.set(6, camY + 12, 4); sun.target.position.set(0, camY, 0);
  scene.background.setHSL(((hueBase + score * 5 + 180) % 360) / 360, 0.35, 0.82);
}
// 自动驾驶：滑到离对齐点很近时放（带一点误差，偶尔会切、会连击）
function autopilot(dt) {
  if (state === 'over') { if (clock - overAt > 1.5) reset(); return; }
  if (state === 'ready') { state = 'play'; return; }
  if (!moving) return;
  autoWait -= dt;
  var top = tower[tower.length - 1], ax = moving.axis;
  if (autoWait <= 0 && Math.abs(moving[ax] - top[ax]) < 0.12 + Math.random() * 0.1) { drop(); autoWait = 0.25; }
}

function render() {
  renderer.render(scene, cam);
  hud.begin();
  var y = B.hudTop + 30;
  if (flash > 0) hud.rect('flash', 0, 0, W, H, '#ffffff', flash * 0.25);
  if (state !== 'over') hud.text('score', score, W / 2, y, 56, '#ffffff', 'center', 'bold', { outline: ['rgba(0,0,0,0.25)', 2] });
  if (combo >= 2 && state === 'play') hud.text('combo', '完美 ×' + combo, W / 2, y + 48, 18, '#ffffff', 'center', 'bold', { outline: ['rgba(0,0,0,0.25)', 1.5] });
  if (state === 'ready') {
    hud.text('t1', '叠方块', W / 2, H * 0.2, 40, '#333344', 'center', 'bold');
    hud.text('t2', '点击屏幕放下方块，对齐越准塔越高', W / 2, H * 0.2 + 42, 15, 'rgba(40,40,60,0.8)', 'center');
    if (best) hud.text('t3', '最高 ' + best, W / 2, H * 0.2 + 68, 14, 'rgba(40,40,60,0.6)', 'center');
  }
  if (state === 'over') {
    var a = Math.min(1, (clock - overAt) / 0.4);
    hud.rect('mask', 0, 0, W, H, '#000000', 0.35 * a);
    hud.text('o1', '本局', W / 2, H * 0.18, 18, '#ffffff', 'center', 'normal', { alpha: a });
    hud.text('o2', score, W / 2, H * 0.18 + 52, 64, '#ffffff', 'center', 'bold', { alpha: a });
    hud.text('o3', newBest ? '新纪录！' : '最高 ' + best, W / 2, H * 0.18 + 104, 16, newBest ? '#ffd166' : '#eeeeee', 'center', 'bold', { alpha: a });
    if (clock - overAt > 0.6) againBtn = hud.button('again', '再来一局', W / 2, H * 0.82, 180, 50, '#ffffff', '#333344');
  }
  hud.render();
}

GameGlobal.__play = function () { return { state: state, score: score, best: best, combo: combo, layers: tower.length, chunks: chunks.length, calls: renderer.info.render.calls }; };

reset();
if (AUTO) state = 'play';
B.loop(update, render);
