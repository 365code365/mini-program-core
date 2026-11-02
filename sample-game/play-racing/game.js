// 极速狂飙（横屏）：伪 3D 公路上一直往前开，换道躲开慢车，吃金币，压加速带，捡油桶续油；油耗尽滑停就结束。
// 转向：倾斜手机（wx.startAccelerometer + onAccelerometerChange）；传感器不可用时按住屏幕左 / 右半边；PC 方向键 / A D。
// 撞车 wx.vibrateLong + 掉速扣油。画面走 WebGL（js/gl2d.js 批量三角形），公路投影与绘制在 js/scene.js，赛道数据在 js/track.js。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var T = require('js/track.js');
var S = require('js/scene.js');
var W = B.W, H = B.H;
var R = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2);
var SC = S.make(R, W, H);
var SEG = T.SEG, LANE_X = T.LANE_X, PLAYER_Z = S.PLAYER_Z;
var MAX = SEG * 60, ACCEL = MAX / 5, CENTRI = 0.12, OFF_MAX = MAX / 4;

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';

// 安全区：横屏时刘海在左或右；右上角胶囊按钮不让 HUD 压住
var safeL = Math.max(B.safe.left || 0, 12) + 4, safeR = Math.min(B.safe.right || W, W - 12) - 4;
var hudY = Math.max(B.safe.top || 0, 8) + 18, hudRight = safeR;
try {
  var menu = wx.getMenuButtonBoundingClientRect && wx.getMenuButtonBoundingClientRect();
  if (menu && menu.left > W / 2 && menu.bottom < H / 3) { hudRight = Math.min(hudRight, menu.left - 12); hudY = Math.max(hudY, menu.top + menu.height / 2); }
} catch (e) {}
var PAUSE_BTN = { x: hudRight - 34, y: hudY - 16, w: 34, h: 32 };

var segs = T.build(), N = segs.length, LEN = N * SEG;
var state = 'ready', clock = 0, overAt = 0, userPaused = false;
var pos, speed, playerX, totalZ, coins, fuel, crashT, boostT, lap, cars, dry, skyX, lastSeg, shake, crashes, boosts, newBest;
var best = Number(B.load('racing.best', 0)) || 0;
var steerNow = 0, aiSteer = 0, lastDraws = 0, lastVerts = 0, flash = '', flashT = 0;

function reset() {
  pos = 0; speed = 0; playerX = 0; totalZ = 0; coins = 0; fuel = 100; crashT = 0; boostT = 0; lap = 0;
  dry = false; skyX = 0; lastSeg = 0; shake = 0; crashes = 0; boosts = 0; newBest = false; flashT = 0;
  T.scatter(segs, 0);
  cars = T.makeCars(14, LEN, MAX);
  SC.setPalette(0);
  state = 'ready';
}
function score() { return Math.floor(totalZ / 1960) + coins * 50; }

// ───────── 输入：倾斜 / 左右半屏按住 / 键盘 ─────────
var touches = {}, tilt = 0, tiltOn = false, tiltSign = 1, tiltErr = '';
function startTilt() {
  if (!wx.startAccelerometer) { tiltErr = '没有 startAccelerometer'; return; }
  wx.startAccelerometer({
    interval: 'game',
    success: function () { tiltOn = true; },
    fail: function (e) { tiltOn = false; tiltErr = (e && e.errMsg) || 'fail'; }
  });
}
// 横屏时设备 y 轴指向屏幕左边：右边往下压 → 重力落在 -y → y 变负 → 往右打。landscapeReverse（反着拿）符号反过来
if (wx.onAccelerometerChange) wx.onAccelerometerChange(function (r) {
  tiltOn = true;
  var v = -Number(r.y || 0) * tiltSign * 3;
  tilt = Math.abs(v) < 0.12 ? 0 : B.clamp(v, -1, 1);
});
if (wx.onDeviceOrientationChange) wx.onDeviceOrientationChange(function (r) { tiltSign = r && r.value === 'landscapeReverse' ? -1 : 1; });
startTilt();

function sideOf(x) { return x < W / 2 ? -1 : 1; }
B.on('down', function (x, y, id) {
  if (state === 'ready') { state = 'play'; return; }
  if (state === 'over') { if (clock - overAt > 0.8) { reset(); state = 'play'; } return; }
  if (userPaused) { userPaused = false; return; }
  if (B.hit(PAUSE_BTN, x, y)) { userPaused = true; return; }
  touches[id] = sideOf(x);
});
B.on('move', function (x, y, id) { if (touches[id] !== undefined) touches[id] = sideOf(x); });
B.on('up', function (x, y, id) { delete touches[id]; });
B.on('key', function (k) {
  if (k === 'pause') { if (state === 'play') userPaused = !userPaused; return; }
  if (k === 'ok' || k === 'up') {
    if (state === 'ready') state = 'play';
    else if (state === 'over' && clock - overAt > 0.8) { reset(); state = 'play'; }
    else userPaused = false;
  }
});
B.onHide(function () {
  if (state === 'play') userPaused = true;
  touches = {};
  try { wx.stopAccelerometer && wx.stopAccelerometer({}); } catch (e) {}
});
B.onShow(function () { startTilt(); });

function steerInput() {
  if (AUTO) return aiSteer;
  var t = 0;
  for (var id in touches) t += touches[id];
  if (B.held.left) t -= 1;
  if (B.held.right) t += 1;
  if (t) return B.clamp(t, -1, 1);
  return tiltOn ? tilt : 0;
}

// ───────── 自动驾驶：哪条车道前方的慢车最远就去哪条，同样空时去有金币 / 油桶 / 加速带的车道 ─────────
var free = [0, 0, 0], gain = [0, 0, 0];
function laneOf(x) { return x < -1 / 3 ? 0 : x > 1 / 3 ? 2 : 1; }
function think() {
  var pz = pos + PLAYER_Z, i, l;
  for (l = 0; l < 3; l++) { free[l] = SEG * 45; gain[l] = 0; }
  for (i = 0; i < cars.length; i++) {
    var c = cars[i], dz = wrapDz(c.z - pz);
    if (dz > -SEG * 1.5 && dz < SEG * 45 && c.speed < speed + MAX * 0.05) {
      l = laneOf(c.x); free[l] = Math.min(free[l], Math.max(0, dz));
      if (Math.abs(c.x - LANE_X[l]) > 0.15) { var l2 = c.x < LANE_X[l] ? l - 1 : l + 1; if (l2 >= 0 && l2 < 3) free[l2] = Math.min(free[l2], Math.max(0, dz)); }
    }
  }
  var si = Math.floor(pz / SEG);
  for (i = 1; i < 30; i++) {
    var s = segs[(si + i) % N];
    if (s.coin >= 0 && !s.got) gain[s.coin] += SEG * 1.5;
    if (s.fuel >= 0 && s.fuelOn) gain[s.fuel] += fuel < 85 ? SEG * 14 : 0;
    if (s.boost >= 0) gain[s.boost] += SEG * 2;
  }
  var cur = laneOf(playerX), bestL = cur, bestV = -1e9;
  for (l = 0; l < 3; l++) {
    var v = free[l] + gain[l] - Math.abs(l - cur) * SEG * 4;
    // 跨两条道要经过中间那条：中间那条马上有车就别横穿
    if (Math.abs(l - cur) === 2 && free[1] < SEG * 3) v -= SEG * 30;
    if (v > bestV) { bestV = v; bestL = l; }
  }
  aiSteer = B.clamp((LANE_X[bestL] - playerX) * 6, -1, 1);
}
function wrapDz(dz) { return dz > LEN / 2 ? dz - LEN : dz < -LEN / 2 ? dz + LEN : dz; }

// ───────── 更新 ─────────
function update(dt) {
  clock += dt;
  if (flashT > 0) flashT -= dt;
  if (AUTO && state === 'over' && clock - overAt > 1.5) { reset(); state = 'play'; }
  if (AUTO && state === 'ready') state = 'play';
  moveCars(state === 'play' && !userPaused ? dt : 0);
  if (state !== 'play' || userPaused) return;
  if (AUTO) think();
  var pct = speed / MAX, dxs = dt * 2 * Math.min(1, pct * 1.2 + 0.15);
  var pSeg = segs[Math.floor((pos + PLAYER_Z) / SEG) % N];
  steerNow = steerInput();
  playerX += dxs * steerNow;
  playerX -= dt * 2 * pct * pct * pSeg.curve * CENTRI;     // 过弯离心：越快甩得越狠
  skyX += pSeg.curve * (speed * dt / SEG) * 0.8;

  if (crashT > 0) crashT -= dt;
  if (boostT > 0) boostT -= dt;
  var top = boostT > 0 ? MAX * 1.35 : MAX;
  if (dry) speed -= MAX * 0.3 * dt;
  else if (speed < top) speed = Math.min(top, speed + ACCEL * dt * (boostT > 0 ? 2.5 : 1));
  else speed = Math.max(top, speed - MAX * 0.4 * dt);
  if (Math.abs(playerX) > 1.05) {                          // 冲出路面：草地上开不快，车身抖
    if (speed > OFF_MAX) speed = Math.max(OFF_MAX, speed - MAX * 0.9 * dt);
    shake = Math.max(shake, 0.08);
  }
  speed = B.clamp(speed, 0, MAX * 1.35);
  playerX = B.clamp(playerX, -2.2, 2.2);
  pos += speed * dt; totalZ += speed * dt;
  if (pos >= LEN) { pos -= LEN; onLap(); }
  if (shake > 0) shake -= dt;

  fuel = Math.max(0, fuel - (0.6 + 3.4 * pct) * dt);
  if (fuel <= 0 && !dry) { fuel = 0; dry = true; say('油耗尽了'); B.beep(330, 0.5, 'triangle', 0.12, 0.5); }
  pickups();
  collide();
  if (dry && speed < MAX * 0.02) over();
}
function onLap() {
  lap++;
  T.scatter(segs, lap);
  for (var i = 0; i < 2 && cars.length < 30; i++) cars.push(T.newCar(LEN, MAX));
  SC.setPalette(lap % SC.palettes.length);
  say('第 ' + (lap + 1) + ' 圈 · ' + SC.palettes[lap % SC.palettes.length].name);
}
function say(s) { flash = s; flashT = 1.6; }

// 这一帧跨过的每一段都查一遍（快的时候一帧过 2~3 段）
function pickups() {
  var si = Math.floor((pos + PLAYER_Z) / SEG) % N, guard = 0;
  while (lastSeg !== si && guard++ < 8) {
    lastSeg = (lastSeg + 1) % N;
    var s = segs[lastSeg];
    if (s.coin >= 0 && !s.got && Math.abs(LANE_X[s.coin] - playerX) < 0.3) {
      s.got = true; coins++;
      B.beep(1320 + (coins % 4) * 110, 0.08, 'triangle', 0.08);
    }
    if (s.fuel >= 0 && s.fuelOn && Math.abs(LANE_X[s.fuel] - playerX) < 0.32) {
      s.fuelOn = false; fuel = Math.min(100, fuel + 30); dry = false; say('油量 +30');
      B.beep(660, 0.08, 'square', 0.07); setTimeout(function () { B.beep(990, 0.12, 'square', 0.07); }, 70);
    }
    if (s.boost >= 0 && Math.abs(LANE_X[s.boost] - playerX) < 0.32 && boostT < 1.8) {
      boostT = 2.2; boosts++; speed = Math.max(speed, MAX * 1.05); say('加速！');
      B.beep(220, 0.45, 'sawtooth', 0.08, 3);
    }
  }
  lastSeg = si;
}
function moveCars(dt) {
  for (var i = 0; i < cars.length; i++) {
    var c = cars[i];
    c.z += c.speed * dt;
    if (c.z >= LEN) c.z -= LEN;
    c.think -= dt;
    if (c.think < 0) {                                     // 偶尔并线：只并相邻车道
      c.think = 3 + Math.random() * 6;
      var nl = c.lane + (Math.random() < 0.5 ? -1 : 1);
      if (nl >= 0 && nl < 3) c.lane = nl;
    }
    c.x += (LANE_X[c.lane] - c.x) * Math.min(1, dt * 1.2);
  }
}
function collide() {
  if (crashT > 0) return;
  var pz = pos + PLAYER_Z;
  for (var i = 0; i < cars.length; i++) {
    var c = cars[i], dz = wrapDz(c.z - pz);
    if (dz > -SEG * 0.6 && dz < SEG * 1.6 && Math.abs(c.x - playerX) < 0.27 && speed > c.speed) {
      crashes++; crashT = 0.8; shake = 0.45;
      speed = c.speed * 0.55; fuel = Math.max(0, fuel - 8);
      playerX += playerX < c.x ? -0.12 : 0.12;
      say('撞车！油量 -8');
      B.beep(110, 0.35, 'sawtooth', 0.16, 0.5);
      try { wx.vibrateLong && wx.vibrateLong({}); } catch (e) {}
      return;
    }
  }
}
function over() {
  state = 'over'; overAt = clock; speed = 0;
  var sc = score();
  if (sc > best) { best = sc; newBest = true; B.save('racing.best', best); }
  B.beep(196, 0.6, 'triangle', 0.12, 0.6);
}

// ───────── 绘制 ─────────
function render() {
  var pz = pos + PLAYER_Z, ps = segs[Math.floor(pz / SEG) % N], pp = (pz % SEG) / SEG;
  var playerY = ps.y1 + (ps.y2 - ps.y1) * pp;
  R.begin(SC.clearColor());
  var base = SC.project(segs, pos, playerX, playerY);
  // 车挂到它所在的段上（只清视野里的段，复用数组不分配）
  for (var k = 0; k <= S.DRAW; k++) segs[(base + k) % N].cars.length = 0;
  for (var i = 0; i < cars.length; i++) {
    var si = Math.floor(cars[i].z / SEG) % N;
    if ((si - base + N) % N < S.DRAW) segs[si].cars.push(cars[i]);
  }
  var ox = shake > 0 ? Math.sin(clock * 70) * shake * 14 : 0, oy = shake > 0 ? Math.cos(clock * 53) * shake * 8 : 0;
  R.save();
  R.translate(ox, oy);
  SC.background(skyX);
  SC.render(segs, base, clock, ox, oy);
  var bounce = speed > 0 ? Math.sin(clock * 30) * Math.min(1.5, speed / MAX * 1.5) : 0;
  SC.player(W / 2, SC.playerY() - 4 + bounce, steerNow, crashT > 0 || dry, 0);
  R.restore();
  if (crashT > 0.5) R.rect(0, 0, W, H, 'rgba(255,40,40,' + ((crashT - 0.5) * 0.8).toFixed(2) + ')');
  drawHud();
  lastDraws = R.end(); lastVerts = R.stats().verts;
}
function drawHud() {
  var kmh = Math.round(speed / MAX * 220);
  R.rrect(safeL - 6, hudY - 17, 190, 34, 17, 'rgba(0,0,0,0.35)');
  R.text('得分 ' + score(), safeL + 6, hudY - 1, 16, '#ffffff', 'left', 'bold');
  R.circle(safeL + 122, hudY - 1, 7, '#ffca28');
  R.text('' + coins, safeL + 134, hudY - 1, 15, '#ffe082', 'left', 'bold');
  R.text('最高 ' + best, safeL + 4, hudY + 30, 13, 'rgba(255,255,255,0.85)', 'left');
  // 右边从胶囊往左排：暂停键、油量条、速度表
  var fw = 110, fx = PAUSE_BTN.x - 14 - fw, low = fuel < 25;
  var sx = fx - 34 - 10 - 112;
  R.rrect(sx, hudY - 17, 112, 34, 17, 'rgba(0,0,0,0.35)');
  R.text(kmh + '', sx + 62, hudY - 1, 20, boostT > 0 ? '#ffb300' : '#ffffff', 'right', 'bold');
  R.text('km/h', sx + 68, hudY + 1, 12, '#e0e0e0', 'left');
  R.rrect(fx - 34, hudY - 17, fw + 44, 34, 17, 'rgba(0,0,0,0.35)');
  R.text('油', fx - 16, hudY - 1, 14, '#ffffff', 'center', 'bold');
  R.rrect(fx, hudY - 6, fw, 12, 6, 'rgba(255,255,255,0.25)');
  if (fuel > 0 && (!low || Math.floor(clock * 4) % 2)) R.rrect(fx, hudY - 6, Math.max(12, fw * fuel / 100), 12, 6, low ? '#ff5252' : '#66bb6a');
  // 暂停键
  R.rrect(PAUSE_BTN.x, PAUSE_BTN.y, PAUSE_BTN.w, PAUSE_BTN.h, 16, 'rgba(0,0,0,0.35)');
  R.rect(PAUSE_BTN.x + 12, PAUSE_BTN.y + 9, 3, 14, '#ffffff'); R.rect(PAUSE_BTN.x + 19, PAUSE_BTN.y + 9, 3, 14, '#ffffff');
  if (flashT > 0 && state === 'play') {
    R.setAlpha(Math.min(1, flashT * 2));
    R.text(flash, W / 2, H * 0.3, 24, '#ffffff', 'center', 'bold', ['rgba(0,0,0,0.6)', 2]);
    R.setAlpha(1);
  }
  if (!tiltOn && state === 'play' && !AUTO) arrows();
  var cx = W / 2, cy = H / 2;
  if (state === 'ready') {
    panel(cx, cy, 340, 150);
    R.text('极速狂飙', cx, cy - 40, 32, '#ffffff', 'center', 'bold');
    R.text(tiltOn ? '左右倾斜手机转向' : '按住屏幕左 / 右半边转向', cx, cy + 6, 16, '#e0e0e0', 'center');
    R.text('躲开慢车 · 吃金币 · 压加速带 · 捡油桶', cx, cy + 32, 14, '#ffe082', 'center');
    R.text('点一下出发', cx, cy + 58, 14, '#bdbdbd', 'center');
  } else if (state === 'over') {
    panel(cx, cy, 340, 170);
    R.text(newBest ? '新纪录！' : '油耗尽 · 比赛结束', cx, cy - 50, 26, newBest ? '#ffca28' : '#ffffff', 'center', 'bold');
    R.text('里程 ' + (totalZ / 196000).toFixed(2) + ' km    金币 ' + coins + '    撞车 ' + crashes, cx, cy - 8, 15, '#e0e0e0', 'center');
    R.text('得分 ' + score() + '    最高 ' + best, cx, cy + 22, 18, '#ffffff', 'center', 'bold');
    if (clock - overAt > 0.8) R.text('点击屏幕再来一局', cx, cy + 56, 14, '#bdbdbd', 'center');
  } else if (userPaused) {
    panel(cx, cy, 220, 100);
    R.text('已暂停', cx, cy - 12, 26, '#ffffff', 'center', 'bold');
    R.text('点击继续', cx, cy + 24, 14, '#e0e0e0', 'center');
  }
}
// 触屏模式：左右下角两个半透明箭头，按着的那边亮
function arrows() {
  var y = H - 52, l = 0, r = 0;
  for (var id in touches) { if (touches[id] < 0) l = 1; else r = 1; }
  var xl = safeL + 40, xr = safeR - 40;
  R.circle(xl, y, 30, l ? 'rgba(255,255,255,0.4)' : 'rgba(255,255,255,0.16)');
  R.circle(xr, y, 30, r ? 'rgba(255,255,255,0.4)' : 'rgba(255,255,255,0.16)');
  R.quad(xl - 12, y, xl + 8, y - 13, xl + 8, y + 13, xl - 12, y, '#ffffff');
  R.quad(xr + 12, y, xr - 8, y - 13, xr - 8, y + 13, xr + 12, y, '#ffffff');
}
function panel(cx, cy, w, h) { R.rrect(cx - w / 2, cy - h / 2, w, h, 16, 'rgba(0,0,0,0.62)'); }

GameGlobal.__play = function () {
  return {
    state: userPaused ? 'paused' : state, score: score(), best: best, km: +(totalZ / 196000).toFixed(3), kmh: Math.round(speed / MAX * 220),
    coins: coins, fuel: +fuel.toFixed(1), lap: lap, crashes: crashes, boosts: boosts, x: +playerX.toFixed(3),
    steer: +steerNow.toFixed(2), tiltOn: tiltOn, tilt: +tilt.toFixed(2), tiltErr: tiltErr, cars: cars.length,
    segs: N, draws: lastDraws, verts: lastVerts, w: W, h: H
  };
};
// 无头调试：按固定步长推进 sec 秒（--eval 里直接推进再取 __play()）
// 无头调试：在正前方放一辆慢车（验撞车：掉速、扣油、vibrateLong）
GameGlobal.__carAhead = function () {
  var c = T.newCar(LEN, MAX); c.z = (pos + PLAYER_Z + SEG * 3) % LEN; c.x = playerX; c.lane = laneOf(playerX); c.think = 99; c.speed = MAX * 0.2;
  cars.push(c); return cars.length;
};
GameGlobal.__sim = function (sec) { for (var i = 0; i < Math.round(sec * 60); i++) update(1 / 60); return GameGlobal.__play(); };

reset();
if (AUTO) state = 'play';
B.loop(update, render);
