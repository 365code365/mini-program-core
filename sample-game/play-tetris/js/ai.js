// 自动试玩（?auto=1）的落点选择：枚举当前块（以及暂存块）的 4 个朝向 × 每一列，直落到底，
// 按「总高度、消行数、空洞、凹凸度」四项打分（Yiyuan Lee 的经典权重）选最好的一处。
// 只给出目标（朝向、列、要不要先暂存），由 game.js 每隔几十毫秒按一下「旋转 / 左右 / 硬降」去执行，
// 所以自动试玩走的是和手指 / 键盘同一套操作（踢墙、锁定、消行动画都照常发生）。
var T = require('./tetris.js');
var COLS = T.COLS, ROWS = T.ROWS, CELLS = T.CELLS;
var W_HEIGHT = -0.510066, W_LINES = 0.760666, W_HOLES = -0.35663, W_BUMP = -0.184483;

function evaluate(board) {
  var heights = [], holes = 0, agg = 0, bump = 0;
  for (var x = 0; x < COLS; x++) {
    var h = 0, seen = false;
    for (var y = 0; y < ROWS; y++) {
      if (board[y * COLS + x]) { if (!seen) { h = ROWS - y; seen = true; } }
      else if (seen) holes++;
    }
    heights.push(h); agg += h;
  }
  for (var i = 0; i + 1 < COLS; i++) bump += Math.abs(heights[i] - heights[i + 1]);
  return { agg: agg, holes: holes, bump: bump };
}
// 把 t 朝向 r 放在第 x 列直落，返回打分（放不下返回 null）
function place(board, t, r, x) {
  var cells = CELLS[t][r];
  function fits(y) {
    for (var i = 0; i < 4; i++) {
      var cx = x + cells[i][0], cy = y + cells[i][1];
      if (cx < 0 || cx >= COLS || cy >= ROWS) return false;
      if (cy >= 0 && board[cy * COLS + cx]) return false;
    }
    return true;
  }
  if (!fits(0)) return null;
  var y = 0;
  while (fits(y + 1)) y++;
  var b = board.slice();
  for (var i = 0; i < 4; i++) { var py = y + cells[i][1]; if (py >= 0) b[py * COLS + x + cells[i][0]] = t; }
  var lines = 0;
  for (var yy = ROWS - 1; yy >= 0; yy--) {
    var full = true;
    for (var xx = 0; xx < COLS; xx++) if (!b[yy * COLS + xx]) { full = false; break; }
    if (full) { b.splice(yy * COLS, COLS); for (var k = 0; k < COLS; k++) b.unshift(0); lines++; yy++; }
  }
  var e = evaluate(b);
  return W_HEIGHT * e.agg + W_LINES * lines + W_HOLES * e.holes + W_BUMP * e.bump;
}
function bestFor(board, t) {
  var best = null, rots = t === 'O' ? 1 : 4;
  for (var r = 0; r < rots; r++) for (var x = -2; x < COLS; x++) {
    var s = place(board, t, r, x);
    if (s !== null && (!best || s > best.score)) best = { r: r, x: x, score: s };
  }
  return best;
}
// game：tetris.Game。返回 { hold, r, x }：hold=true 表示先暂存，换出来的块再重新规划
function plan(game) {
  var c = game.cur; if (!c) return null;
  var a = bestFor(game.board, c.t);
  if (game.canHold) {
    var other = game.hold || game.queue[0];
    var b = other && other !== c.t ? bestFor(game.board, other) : null;
    if (b && (!a || b.score > a.score + 0.3)) return { hold: true };
  }
  return a ? { hold: false, r: a.r, x: a.x } : { hold: false, r: c.r, x: c.x };
}

module.exports = { plan: plan };
