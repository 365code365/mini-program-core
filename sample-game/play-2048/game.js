// 2048：上下左右滑（或方向键 / WASD），相同数字相撞合并，凑出 2048 获胜。
// 每步自动存档，退出再进接着玩；最高分单独存。
var B = require('js/base.js');
var Board = require('js/board.js');
var G2 = require('js/gl2d.js');
var W = B.W, H = B.H;
// 画面走 WebGL（js/gl2d.js：形状攒成一批三角形画，文字是缓存的纹理）
var R = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2);

var SLIDE = 0.1, POP = 0.15;           // 补间时长（秒）
var COLORS = {
  2: ['#eee4da', '#776e65'], 4: ['#ede0c8', '#776e65'], 8: ['#f2b179', '#fff'], 16: ['#f59563', '#fff'],
  32: ['#f67c5f', '#fff'], 64: ['#f65e3b', '#fff'], 128: ['#edcf72', '#fff'], 256: ['#edcc61', '#fff'],
  512: ['#edc850', '#fff'], 1024: ['#edc53f', '#fff'], 2048: ['#edc22e', '#fff']
};

var board, best = Number(B.load('2048.best', 0)) || 0;
var anim = 1;                          // 自上次滑动起过了多久（秒）
var gainFx = [];                       // 「+N」飘字
var buttons = {};

var size = Math.min(W - 32, 420, (H - B.hudTop - 200));
var gap = Math.round(size * 0.03), cell = (size - gap * 5) / 4;
var bx = (W - size) / 2, by = Math.max(B.hudTop + 150, (H - size) / 2 + 40);

function newGame() {
  board = Board.create();
  Board.spawn(board, Math.random); Board.spawn(board, Math.random);
  anim = 0; persist();
}
function persist() { B.save('2048.game', Board.serialize(board)); }

function slide(dir) {
  if (overlay()) return;
  var r = Board.move(board, dir);
  if (!r.moved) { B.beep(140, 0.05, 'sine', 0.05); return; }
  Board.spawn(board, Math.random);
  anim = 0;
  if (r.gained) {
    gainFx.push({ v: r.gained, t: 0 });
    B.beep(300 + Math.min(900, Math.log2(r.gained) * 60), 0.09, 'triangle', 0.12);
  } else B.beep(220, 0.04, 'sine', 0.06);
  if (board.score > best) { best = board.score; B.save('2048.best', best); }
  if (board.won && !board.keepGoing) B.buzz();
  persist();
}
function overlay() {
  if (board.won && !board.keepGoing) return 'win';
  if (!Board.canMove(board)) return 'lose';
  return null;
}

B.on('swipe', slide);
B.on('key', function (k) {
  if (k === 'ok' && overlay()) return press(overlay() === 'win' ? 'keep' : 'again');
  slide(k);
});
B.on('tap', function (x, y) {
  for (var name in buttons) if (B.hit(buttons[name], x, y)) return press(name);
});
function press(name) {
  B.beep(500, 0.05, 'square', 0.06);
  if (name === 'new' || name === 'again') { Board.settle(board); newGame(); }
  if (name === 'keep') { board.keepGoing = true; persist(); }
  if (name === 'sound') B.setMuted(!B.isMuted());
}

function update(dt) {
  anim += dt;
  gainFx.forEach(function (g) { g.t += dt; });
  gainFx = gainFx.filter(function (g) { return g.t < 0.8; });
  if (anim > SLIDE + POP) Board.settle(board);
}

function render() {
  R.begin('#faf8ef');
  buttons = {};
  drawHeader();
  drawGrid();
  var o = overlay();
  if (o && anim > SLIDE + POP) drawOverlay(o);
  R.end();
}
function drawHeader() {
  var y = B.hudTop;
  // 标题、两个按钮、底部提示只在静音开关变时变：录成图层；按钮命中框录的时候记下，之后每帧照用
  R.layer('header', B.isMuted() ? 'm' : 's', function () {
    R.text('2048', bx, y + 34, 48, '#776e65', 'left', 'bold');
    headerBtns['new'] = R.button('新游戏', bx + size - 52, y + 90, 104, 34, '#8f7a66', '#f9f6f2');
    headerBtns.sound = R.button(B.isMuted() ? '静音' : '音效', bx + size - 150, y + 90, 72, 34, '#bbada0', '#f9f6f2');
    R.text('滑动屏幕，合并相同数字，凑出 2048！', W / 2, by + size + 30, 15, '#776e65', 'center');
  });
  buttons['new'] = headerBtns['new']; buttons.sound = headerBtns.sound;
  scoreBox('分数', board.score, bx + size - 150, y + 10);
  scoreBox('最高', best, bx + size - 72, y + 10);
  gainFx.forEach(function (g) {
    // 从分数框顶上往上飘，不压住框里的字
    R.setAlpha(1 - g.t / 0.8);
    R.text('+' + g.v, bx + size - 114, y - 4 - g.t * 30, 18, '#776e65', 'center', 'bold');
    R.setAlpha(1);
  });
}
var headerBtns = {};
function scoreBox(label, v, x, y) {
  R.rrect(x, y, 72, 50, 6, '#bbada0');
  R.text(label, x + 36, y + 14, 12, '#eee4da', 'center', 'bold');
  R.text(v, x + 36, y + 34, v >= 100000 ? 14 : 18, '#ffffff', 'center', 'bold');
}
function cellPos(r, c) { return [bx + gap + c * (cell + gap), by + gap + r * (cell + gap)]; }
function ease(t) { return 1 - (1 - t) * (1 - t); }
function drawGrid() {
  // 底板与 16 个空格不会变：录成图层
  R.layer('grid', 1, function () {
    R.rrect(bx, by, size, size, 8, '#bbada0');
    for (var r = 0; r < 4; r++) for (var c = 0; c < 4; c++) {
      var p = cellPos(r, c);
      R.rrect(p[0], p[1], cell, cell, 5, 'rgba(238,228,218,0.35)');
    }
  });
  var s = ease(Math.min(1, anim / SLIDE));
  // 先画要消失的，再画留下的：合并时留下的那块盖在上面
  board.tiles.slice().sort(function (a, b) { return (b.gone ? 1 : 0) - (a.gone ? 1 : 0); }).forEach(function (t) {
    if (t.gone && anim >= SLIDE) return;
    if (t.born && anim < SLIDE) return;          // 新块等滑动结束再出现
    var a = cellPos(t.fromR, t.fromC), b = cellPos(t.r, t.c);
    var x = a[0] + (b[0] - a[0]) * s, y = a[1] + (b[1] - a[1]) * s;
    var k = 1, pt = (anim - SLIDE) / POP;
    if (t.born && pt < 1) k = ease(Math.max(0, pt));
    else if (t.merged && pt >= 0 && pt < 1) k = 1 + 0.18 * Math.sin(pt * Math.PI);
    // 合并中的块在滑动阶段还显示旧值
    var v = t.merged && anim < SLIDE ? t.v / 2 : t.v;
    drawTile(x + cell / 2, y + cell / 2, cell * k, v);
  });
}
// 方块：每个数值录一个精灵（原点为中心、边长 cell 的圆角方块 + 数字），滑动 / 弹出只改位置与缩放（gl2d sprite）
function drawTile(cx, cy, w, v) {
  if (w <= 1) return;
  R.sprite('tile' + v, cell, function () {
    var col = COLORS[v] || ['#3c3a32', '#fff'];
    R.rrect(-cell / 2, -cell / 2, cell, cell, 5, col[0]);
    var digits = String(v).length;
    var fs = cell * (digits <= 2 ? 0.48 : digits === 3 ? 0.4 : digits === 4 ? 0.32 : 0.26);
    R.text(v, 0, 1, Math.round(fs), col[1], 'center', 'bold');
  }, cx, cy, w / cell);
}
function drawOverlay(kind) {
  R.rrect(bx, by, size, size, 8, kind === 'win' ? 'rgba(237,194,46,0.55)' : 'rgba(238,228,218,0.75)');
  var cy = by + size / 2;
  R.text(kind === 'win' ? '你赢了！' : '游戏结束', W / 2, cy - 40, 40, kind === 'win' ? '#f9f6f2' : '#776e65', 'center', 'bold');
  if (kind === 'win') {
    buttons.keep = R.button('继续挑战', W / 2 - 70, cy + 30, 120, 40, '#8f7a66', '#f9f6f2');
    buttons.again = R.button('重新开始', W / 2 + 70, cy + 30, 120, 40, '#8f7a66', '#f9f6f2');
  } else {
    R.text('最大方块 ' + Board.maxTile(board), W / 2, cy, 18, '#776e65', 'center');
    buttons.again = R.button('再来一局', W / 2, cy + 50, 140, 42, '#8f7a66', '#f9f6f2');
  }
}

// 启动参数 auto=1：每 0.2 秒按「下、左、右、上」的优先级滑一次（够撑很久的笨办法），输了自动重开
var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var autoT = 0;
function autopilot(dt) {
  autoT += dt; if (autoT < 0.2) return; autoT = 0;
  var o = overlay();
  if (o) return press(o === 'win' ? 'keep' : 'again');
  ['down', 'left', 'right', 'up'].some(function (d) {
    var before = JSON.stringify(Board.serialize(board).cells);
    slide(d);
    return JSON.stringify(Board.serialize(board).cells) !== before;
  });
}
GameGlobal.__play = function () { return { score: board.score, best: best, max: Board.maxTile(board), over: overlay() }; };

board = Board.restore(B.load('2048.game', null));
if (!board) newGame();
B.loop(function (dt) { update(dt); if (q.auto === '1') autopilot(dt); }, render);
