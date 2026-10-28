// 屏幕按键：左半屏是浮动摇杆（按哪儿哪儿就是中心，8 方向），右下是 4 个攻击键 + 2 个必杀键 + 超必杀键。多指同时按（边走边打）。
// PC 上：方向 WASD / 方向键，J 轻拳 K 轻脚 U 重拳 I 重脚，L 必杀一 O 必杀二，空格 超必杀。
//
// 为什么按下就记、不等抬手：格斗要的是「按下这一帧就出招」，等 tap 会慢 100ms 以上。
// 按键事件来的时刻与逻辑帧不同步，所以先攒在 pending 里，下一次 sample() 一起交给逻辑（不会丢键，也不会一次按键算两帧）。
var B = require('./base.js');

var W = B.W, H = B.H;
var R = (B.safe.right || W) - 10, L = (B.safe.left || 0) + 10;
var STICK_R = 54, KNOB_R = 24;
var stick = { id: null, cx: L + 92, cy: H - 88, x: 0, y: 0, home: [L + 92, H - 88] };

// 键位：b = 逻辑键名，t = 主标，s = 小字，r = 半径
var BTNS = [
  { b: 'lp', t: 'A', s: '轻拳', x: R - 198, y: H - 56, r: 30, c: '#ff9a3c' },
  { b: 'lk', t: 'B', s: '轻脚', x: R - 128, y: H - 44, r: 30, c: '#4fd06a' },
  { b: 'hp', t: 'C', s: '重拳', x: R - 214, y: H - 128, r: 30, c: '#ff4a4a' },
  { b: 'hk', t: 'D', s: '重脚', x: R - 144, y: H - 114, r: 30, c: '#4aa8ff' },
  { b: 's1', t: '波', s: '必杀', x: R - 54, y: H - 50, r: 27, c: '#ffc83a' },
  { b: 's2', t: '升', s: '必杀', x: R - 72, y: H - 120, r: 27, c: '#ffc83a' },
  { b: 'su', t: '超', s: '超必杀', x: R - 118, y: H - 186, r: 28, c: '#ff3cbe' }
];
var touches = {};       // 触点 id → 按钮键名
var held = {}, pending = {}, keyHeld = {}, glow = {};
var enabled = false;

function setEnabled(v) {
  enabled = v;
  if (!v) { touches = {}; held = {}; pending = {}; stick.id = null; stick.x = stick.y = 0; resetStick(); }
}
function resetStick() { stick.cx = stick.home[0]; stick.cy = stick.home[1]; }

function btnAt(x, y) {
  var best = null, bd = 1e9;
  for (var i = 0; i < BTNS.length; i++) {
    var b = BTNS[i], dx = x - b.x, dy = y - b.y, d = dx * dx + dy * dy, rr = b.r * 1.35;
    if (d < rr * rr && d < bd) { bd = d; best = b; }
  }
  return best;
}
function press(name) { pending[name] = true; held[name] = true; glow[name] = 1; }

B.on('down', function (x, y, id) {
  if (!enabled) return;
  var b = btnAt(x, y);
  if (b) { touches[id] = b.b; press(b.b); return; }
  if (x < W * 0.48 && y > H * 0.28 && stick.id === null) {
    stick.id = id;
    stick.cx = B.clamp(x, L + STICK_R, W * 0.48 - STICK_R);
    stick.cy = B.clamp(y, H * 0.35, H - STICK_R - 4);
    moveStick(x, y);
  }
});
B.on('move', function (x, y, id) {
  if (!enabled) return;
  if (id === stick.id) { moveStick(x, y); return; }
  // 手指从一个键滑到另一个键：松开旧的、按下新的（连打时手指不抬起来也能换键）
  if (touches[id] !== undefined) {
    var b = btnAt(x, y);
    if (b && b.b !== touches[id]) { held[touches[id]] = false; touches[id] = b.b; press(b.b); }
  }
});
B.on('up', function (x, y, id) {
  if (id === stick.id) { stick.id = null; stick.x = stick.y = 0; resetStick(); }
  if (touches[id] !== undefined) { held[touches[id]] = false; delete touches[id]; }
});
function moveStick(x, y) {
  var dx = x - stick.cx, dy = y - stick.cy, d = Math.sqrt(dx * dx + dy * dy);
  // 手指拖出圈外：中心跟着手指走（不用回头找中心），这是手游摇杆的通行做法
  if (d > STICK_R * 1.4) { var k = (d - STICK_R * 1.4) / d; stick.cx += dx * k; stick.cy += dy * k; dx -= dx * k; dy -= dy * k; d = STICK_R * 1.4; }
  stick.x = dx / STICK_R; stick.y = dy / STICK_R;
}

var KEYMAP = { KeyJ: 'lp', j: 'lp', KeyK: 'lk', k: 'lk', KeyU: 'hp', u: 'hp', KeyI: 'hk', i: 'hk', KeyL: 's1', l: 's1', KeyO: 's2', o: 's2', Space: 'su', ' ': 'su' };
var DIRMAP = { ArrowLeft: 'L', KeyA: 'L', a: 'L', ArrowRight: 'R', KeyD: 'R', d: 'R', ArrowUp: 'U', KeyW: 'U', w: 'U', ArrowDown: 'D', KeyS: 'D', s: 'D' };
B.on('keydown', function (code) {
  if (!enabled) return;
  if (KEYMAP[code]) { if (!keyHeld[code]) press(KEYMAP[code]); keyHeld[code] = true; }
  if (DIRMAP[code]) keyHeld[DIRMAP[code]] = true;
});
B.on('keyup', function (code) {
  if (KEYMAP[code]) { keyHeld[code] = false; held[KEYMAP[code]] = false; }
  if (DIRMAP[code]) keyHeld[DIRMAP[code]] = false;
});

// 本逻辑帧的输入：x / y ∈ {-1, 0, 1}（屏幕方向，y 正 = 下），press = 这一帧新按下的键
function sample() {
  var x = 0, y = 0;
  if (stick.id !== null) {
    var m = Math.sqrt(stick.x * stick.x + stick.y * stick.y);
    if (m > 0.32) {
      // 8 方向：按角度分扇区；上下的扇区开得宽一点（跳和蹲比斜向更常用，斜向只在搓招时用到）
      var a = Math.atan2(stick.y, stick.x) * 180 / Math.PI;
      x = Math.abs(a) < 67.5 ? 1 : Math.abs(a) > 112.5 ? -1 : 0;
      y = a > 22.5 && a < 157.5 ? 1 : a < -22.5 && a > -157.5 ? -1 : 0;
    }
  }
  if (keyHeld.L) x = -1; if (keyHeld.R) x = 1; if (keyHeld.U) y = -1; if (keyHeld.D) y = 1;
  var p = pending; pending = {};
  return { x: x, y: y, press: p };
}

function step(dt) { for (var k in glow) glow[k] = Math.max(0, glow[k] - dt * 4); }

// 画控件（meter：玩家的超必杀能不能放、几格）
function draw(g, meterStocks, alpha) {
  if (alpha <= 0) return;
  g.save(); g.setAlpha(alpha);
  // 摇杆底盘：两圈 + 四个方向的小三角 + 发光的摇杆头
  var cx = stick.cx, cy = stick.cy, act = stick.id !== null;
  g.circle(cx, cy, STICK_R + 8, act ? 'rgba(10,14,30,0.45)' : 'rgba(10,14,30,0.3)');
  g.strokeCircle(cx, cy, STICK_R + 8, 2, 'rgba(255,255,255,0.35)');
  g.strokeCircle(cx, cy, STICK_R - 12, 1.2, 'rgba(255,255,255,0.15)');
  for (var i = 0; i < 4; i++) {
    var a = i * Math.PI / 2, ux = Math.cos(a), uy = Math.sin(a), d = STICK_R - 2, s = 7;
    g.poly([[cx + ux * (d + s), cy + uy * (d + s)], [cx + ux * d - uy * s, cy + uy * d + ux * s], [cx + ux * d + uy * s, cy + uy * d - ux * s]], 'rgba(255,255,255,0.45)');
  }
  var kx = cx + B.clamp(stick.x, -1, 1) * STICK_R * 0.62, ky = cy + B.clamp(stick.y, -1, 1) * STICK_R * 0.62;
  if (act) { g.setBlend('add'); g.glow(kx, ky, KNOB_R + 22, 'rgba(120,180,255,0.5)'); g.setBlend(); }
  g.circle(kx, ky + 2, KNOB_R, 'rgba(0,0,0,0.35)');
  g.circle(kx, ky, KNOB_R, '#dfe6f3');
  g.circle(kx, ky - 3, KNOB_R - 5, '#ffffff');
  g.circle(kx, ky, KNOB_R - 11, '#c3cde0');
  // 按钮
  for (var j = 0; j < BTNS.length; j++) {
    var b = BTNS[j], on = held[b.b], gl = glow[b.b] || 0;
    var dim = b.b === 'su' && meterStocks < 1;
    var col = g.color(b.c);
    if (gl > 0 || on) { g.setBlend('add'); g.glow(b.x, b.y, b.r + 22 + gl * 10, [col[0], col[1], col[2], 0.7 * Math.max(gl, on ? 0.6 : 0)]); g.setBlend(); }
    var r = on ? b.r * 0.94 : b.r;
    g.circle(b.x, b.y + 3, r, 'rgba(0,0,0,0.22)');
    g.circle(b.x, b.y, r, dim ? 'rgba(60,60,70,0.4)' : [col[0] * 0.3, col[1] * 0.3, col[2] * 0.3, on ? 0.7 : 0.45]);
    g.strokeCircle(b.x, b.y, r - 1, 2.5, dim ? 'rgba(160,160,170,0.45)' : [col[0], col[1], col[2], 0.85]);
    g.ellipse(b.x, b.y - r * 0.42, r * 0.62, r * 0.3, 0, 'rgba(255,255,255,0.16)');
    g.text(b.t, b.x, b.y - 4, Math.round(r * 0.72), dim ? '#8a8a94' : '#ffffff', 'center', 'bold', ['rgba(0,0,0,0.6)', 1.5]);
    g.text(b.s, b.x, b.y + r * 0.52, 9, dim ? '#77777f' : 'rgba(255,255,255,0.85)', 'center', 'bold');
    if (b.b === 'su' && meterStocks >= 1) {
      // 能放超必杀：外圈转动的光点
      var t = Date.now() / 300;
      g.setBlend('add');
      for (var q = 0; q < 3; q++) { var aa = t + q * Math.PI * 2 / 3; g.circle(b.x + Math.cos(aa) * (r + 5), b.y + Math.sin(aa) * (r + 5), 3, 'rgba(255,120,220,0.9)'); }
      g.setBlend();
    }
  }
  g.restore();
}

module.exports = { sample: sample, step: step, draw: draw, setEnabled: setEnabled, BTNS: BTNS, stick: stick };
