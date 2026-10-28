// 3D 保龄球（十局制）：手指左右拖动摆球的位置，往前一划把球扔出去（划得越快球越快，斜着划就斜着走，划的时候往一边弯就带弧线）。
// 物理是 cannon-es 刚体（球、十个瓶、边沟、坑），计分按标准规则（全中 / 补中 / 第十局加投，js/score.js）。
// three.js r160 + cannon-es 0.20（libs/，`bash tools/fetch-play-libs.sh` 拉）经 js/adapter.js 跑；界面文字走 js/hud.js。
// PC：左右键摆位，空格扔。启动参数 auto=1：自动驾驶（演示 / 无头长稳）。
var B = require('js/base.js');
var canvas = require('js/adapter.js');
var THREE = require('libs/three.js');
var CANNON = require('libs/cannon-es.js');
var Hud = require('js/hud.js');
var Lane = require('js/lane.js');
var Score = require('js/score.js');
var W = B.W, H = B.H, PR = B.sys.pixelRatio || 2;

var renderer = new THREE.WebGLRenderer({ canvas: canvas, antialias: true });
renderer.setPixelRatio(PR);
renderer.setSize(W, H, false);
var hud = Hud.make(THREE, renderer, W, H, PR);
var lane = Lane.make(THREE, CANNON, W / H);

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';
var MAX_X = lane.LANE_W / 2 - lane.BALL_R - 0.02;

var state, phase, rolls, aimX, rollT, clock = 0, message = null, best = Number(B.load('bowling.best', 0)) || 0, newBest = false;
var drag = null, againBtn = null, autoT = 0;

function reset() {
  rolls = []; aimX = 0; newBest = false; message = null;
  lane.rack();
  newBall();
  state = 'ready';
}
function newBall() { phase = 'aim'; rollT = 0; lane.holdBall(aimX); }

// ───────── 输入：拖动摆位，向前划扔球 ─────────
B.on('down', function (x, y, id) {
  if (state === 'over' || phase !== 'aim') return;
  if (state === 'ready') state = 'play';
  drag = { id: id, x0: x, y0: y, t0: clock, pts: [[x, y, clock]], ax: aimX };
});
B.on('move', function (x, y, id) {
  if (!drag || drag.id !== id) return;
  drag.pts.push([x, y, clock]);
  if (drag.pts.length > 30) drag.pts.shift();
  // 还没往上划：左右拖就是摆位（整屏宽 ≈ 整条道宽）
  if (drag.y0 - y < 30) { aimX = Math.max(-MAX_X, Math.min(MAX_X, drag.ax + (x - drag.x0) / W * lane.LANE_W * 1.2)); lane.holdBall(aimX); }
});
B.on('up', function (x, y, id) {
  if (!drag || drag.id !== id) return;
  var d = drag; drag = null;
  // 用最后 0.15 秒的那段算速度（整段算的话「先拖再划」会被拖的那段拉低）
  var recent = d.pts.filter(function (p) { return clock - p[2] <= 0.15; });
  var p0 = recent.length > 1 ? recent[0] : d.pts[0], dt = Math.max(0.03, clock - p0[2]);
  var dy = p0[1] - y, dx = x - p0[0];
  if (dy < 40) return;                                             // 没往前划：只是摆位
  var speed = Math.max(4.5, Math.min(10.5, dy / dt / H * 9));     // 每秒划过一屏 ≈ 9 m/s
  var angle = Math.max(-0.09, Math.min(0.09, Math.atan2(dx, dy) * 0.35));
  // 弧线：整段轨迹的中点偏离首尾连线多少（往左鼓 = 往右弯回来）
  var mid = d.pts[d.pts.length >> 1], hook = 0;
  if (d.pts.length > 4) { var lx = (d.pts[0][0] + x) / 2; hook = Math.max(-1, Math.min(1, (lx - mid[0]) / 40)); }
  bowl(speed, angle, hook);
});
B.on('tap', function (x, y) { if (state === 'over' && B.hit(againBtn, x, y)) reset(); });
B.on('key', function (k) {
  if (state === 'over') { if (k === 'ok') reset(); return; }
  if (phase !== 'aim') return;
  if (state === 'ready') state = 'play';
  if (k === 'left' || k === 'right') { aimX = Math.max(-MAX_X, Math.min(MAX_X, aimX + (k === 'left' ? -0.05 : 0.05))); lane.holdBall(aimX); }
  if (k === 'ok' || k === 'up') bowl(8, -aimX * 0.02, 0);
});

function bowl(speed, angle, hook) {
  phase = 'roll'; rollT = 0; message = null;
  lane.throwBall(Math.sin(angle) * speed, -Math.cos(angle) * speed, hook * 0.12);
  B.beep(180, 0.25, 'triangle', 0.08, 0.7);
}
// 一投结束：数倒了几个、记分、决定重摆还是只扫倒瓶
function settle() {
  var before = Score.cursor(rolls), down = lane.sweep();
  rolls.push(down);
  var cur = Score.cursor(rolls);
  if (before.ball === 0 && down === 10 || before.frame === 9 && before.fresh && down === 10) { message = '全中！'; B.beep(880, 0.2, 'triangle', 0.14); setTimeout(function () { B.beep(1320, 0.3, 'triangle', 0.12); }, 120); B.buzz(); }
  else if (!before.fresh && lane.standing() === 0) { message = '补中！'; B.beep(990, 0.25, 'triangle', 0.12); }
  else if (down === 0) message = before.fresh ? '洗沟…' : '没打到';
  else message = '倒了 ' + down + ' 个';
  if (cur.done) {
    state = 'over';
    var t = Score.total(rolls);
    if (t > best) { best = t; newBest = true; B.save('bowling.best', best); }
    return;
  }
  if (cur.fresh) lane.rack();
  newBall();
}

function update(dt) {
  clock += dt;
  if (AUTO) autopilot(dt);
  if (phase === 'roll') {
    rollT += dt;
    var b = lane.ball().position;
    // 球进坑 / 停了 / 滚了太久，再等瓶子都停稳（最多 3.5 秒）
    if (b.z < lane.HEAD_Z - 1 || b.y < -0.2 || rollT > 8) phase = 'pins';
  }
  if (phase === 'pins') {
    rollT += dt;
    if ((lane.pinsSettled() && rollT > 1.6) || rollT > 9) settle();
  }
  lane.step(dt, phase === 'roll');
  lane.aim(aimX, 0, phase === 'aim' && state !== 'over');
  lane.camera(dt, phase);
}
function autopilot(dt) {
  if (state === 'over') { autoT += dt; if (autoT > 2) { autoT = 0; reset(); } return; }
  if (phase !== 'aim') return;
  autoT += dt;
  if (autoT < 0.8) return;
  autoT = 0;
  if (state === 'ready') state = 'play';
  // 第一投瞄右口袋（头瓶右侧），补球瞄剩下的瓶的平均位置；都带点误差
  aimX = Math.max(-MAX_X, Math.min(MAX_X, 0.07 + (Math.random() - 0.5) * 0.2));
  lane.holdBall(aimX);
  bowl(7.5 + Math.random() * 2, -aimX * 0.004 + (Math.random() - 0.5) * 0.01, 0);
}

// ───────── 界面：计分板（十格）、提示、结算 ─────────
function render() {
  renderer.render(lane.scene, lane.cam);
  hud.begin();
  var cur = Score.cursor(rolls), y0 = B.hudTop;
  // 计分板：一整块面板，只在多投了一球 / 局数变时重画（拆成 40 个格子与字时，界面层每帧 40 次 draw call）
  hud.panel('board', rolls.join(',') + '|' + (state === 'over' ? -1 : cur.frame), 6, y0 - 4, W - 12, 52, function (c, pw, ph) {
    var fs = Score.frames(rolls), cw = (pw - 4) / 10;
    c.fillStyle = 'rgba(0,0,0,0.45)'; c.fillRect(0, 0, pw, ph);
    c.textAlign = 'center'; c.textBaseline = 'middle';
    for (var i = 0; i < 10; i++) {
      var x = 2 + i * cw, f = fs[i], on = i === cur.frame && state !== 'over';
      c.fillStyle = on ? 'rgba(255,209,102,0.92)' : 'rgba(255,255,255,0.86)'; c.fillRect(x + 1, 4, cw - 2, 44);
      c.font = '9px sans-serif'; c.fillStyle = '#666666'; c.fillText(String(i + 1), x + cw / 2, 11);
      c.font = 'bold 11px sans-serif'; c.fillStyle = '#d62828'; c.fillText(f ? f.marks.join(' ') : '', x + cw / 2, 23);
      c.font = 'bold 13px sans-serif'; c.fillStyle = '#222222'; c.fillText(f && f.score !== null ? String(f.score) : '', x + cw / 2, 38);
    }
  });
  if (state === 'ready') {
    hud.rect('mask0', 0, H * 0.3, W, 120, '#000000', 0.45);
    hud.text('t1', '3D 保龄球', W / 2, H * 0.3 + 34, 34, '#ffffff', 'center', 'bold');
    hud.text('t2', '左右拖动摆好位置，往前一划把球扔出去', W / 2, H * 0.3 + 74, 14, '#ffffff', 'center');
    hud.text('t3', '划得越快球越快 · 划个弧线球会拐弯', W / 2, H * 0.3 + 98, 12, 'rgba(255,255,255,0.75)', 'center');
  }
  if (message && phase === 'aim' && state === 'play') hud.text('msg', message, W / 2, H * 0.42, message.length <= 3 ? 40 : 26, '#ffffff', 'center', 'bold', { outline: ['rgba(0,0,0,0.5)', 2] });
  if (state === 'play' && phase === 'aim' && !message) hud.text('hint', '第 ' + (cur.frame + 1) + ' 局 · 第 ' + (cur.ball + 1) + ' 投', W / 2, H - 60, 14, 'rgba(255,255,255,0.8)', 'center', 'bold');
  if (state === 'over') {
    hud.rect('mask', 0, 0, W, H, '#000000', 0.55);
    hud.text('o1', '总分', W / 2, H * 0.32, 18, '#dddddd', 'center');
    hud.text('o2', Score.total(rolls), W / 2, H * 0.32 + 56, 64, '#ffffff', 'center', 'bold');
    hud.text('o3', newBest ? '新纪录！' : '最高 ' + best, W / 2, H * 0.32 + 110, 16, newBest ? '#ffd166' : '#dddddd', 'center', 'bold');
    againBtn = hud.button('again', '再来一场', W / 2, H * 0.72, 180, 50, '#ffd166', '#222222');
  }
  hud.render();
}

GameGlobal.__play = function () { return { state: state, phase: phase, rolls: rolls.slice(), total: Score.total(rolls), frame: Score.cursor(rolls).frame + 1, calls: renderer.info.render.calls }; };

reset();
if (AUTO) state = 'play';
B.loop(update, render);
