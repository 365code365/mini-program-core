// 数独求解器：位掩码 + 最少候选优先（MRV）回溯。
// count(grid, limit)：数解的个数，数到 limit 就停（出题时 limit = 2：只关心「唯一」还是「不唯一」）；
// fill(grid)：把空格随机填成一个完整解（出题第一步：随机终盘）。
// grid 是长 81 的 Uint8Array，0 = 空。为什么不用「舞蹈链」：QuickJS 里对象 / 链表操作慢，
// 27 个 Uint16 掩码 + 定长数组的回溯在这里更快，而且整个搜索不分配。
var ROW = new Uint8Array(81), COL = new Uint8Array(81), BOX = new Uint8Array(81);
for (var i = 0; i < 81; i++) { ROW[i] = (i / 9) | 0; COL[i] = i % 9; BOX[i] = ((ROW[i] / 3) | 0) * 3 + ((COL[i] / 3) | 0); }
var POP = new Uint8Array(512), LOW = new Uint8Array(512);   // 掩码里 1 的个数；最低位是数字几（1~9）
for (var m = 1; m < 512; m++) { POP[m] = POP[m >> 1] + (m & 1); var d = 0; while (!(m >> d & 1)) d++; LOW[m] = d + 1; }

var r = new Uint16Array(9), c = new Uint16Array(9), b = new Uint16Array(9), cells = new Uint8Array(81);
var order = new Uint8Array(82 * 9);              // 第 k 层的候选顺序放在 order[k*9 ..]
var n = 0, found = 0, limit = 1, nodes = 0, rnd = false, out = null;

function setup(grid) {
  r.fill(0); c.fill(0); b.fill(0); n = 0;
  for (var i = 0; i < 81; i++) {
    var v = grid[i];
    if (!v) { cells[n++] = i; continue; }
    var bit = 1 << (v - 1);
    if ((r[ROW[i]] | c[COL[i]] | b[BOX[i]]) & bit) return false;   // 题面自己就冲突
    r[ROW[i]] |= bit; c[COL[i]] |= bit; b[BOX[i]] |= bit;
  }
  return true;
}

function rec(k) {
  if (k === n) { found++; return found >= limit; }
  nodes++;
  var best = k, mask = 0, cnt = 10;
  for (var j = k; j < n; j++) {
    var ci = cells[j], mm = ~(r[ROW[ci]] | c[COL[ci]] | b[BOX[ci]]) & 511, pc = POP[mm];
    if (pc < cnt) { cnt = pc; best = j; mask = mm; if (pc <= 1) break; }
  }
  if (!cnt) return false;
  var t = cells[k]; cells[k] = cells[best]; cells[best] = t;
  var i = cells[k], ri = ROW[i], cc = COL[i], bi = BOX[i], o = k * 9, q = 0;
  while (mask) { order[o + q++] = LOW[mask]; mask &= mask - 1; }
  if (rnd) for (var x = cnt - 1; x > 0; x--) {    // 出终盘时候选顺序打乱，每次出的盘都不一样
    var y = Math.floor(Math.random() * (x + 1)), s = order[o + x]; order[o + x] = order[o + y]; order[o + y] = s;
  }
  for (q = 0; q < cnt; q++) {
    var dv = order[o + q], bit = 1 << (dv - 1);
    r[ri] |= bit; c[cc] |= bit; b[bi] |= bit;
    if (out) out[i] = dv;
    var done = rec(k + 1);
    r[ri] &= ~bit; c[cc] &= ~bit; b[bi] &= ~bit;
    if (done) return true;
  }
  if (out) out[i] = 0;
  return false;
}

function count(grid, lim) {
  if (!setup(grid)) return 0;
  found = 0; limit = lim || 2; rnd = false; out = null;
  rec(0);
  return found;
}
function fill(grid) {
  if (!setup(grid)) return false;
  found = 0; limit = 1; rnd = true; out = grid;
  var ok = rec(0);
  out = null; rnd = false;
  return ok;
}
// 某格当前能填哪些数（掩码）：自动试玩挑「候选最少」的格子、提示挑格子用
function candidates(grid, i) {
  var m = 0;
  for (var j = 0; j < 81; j++) {
    if (!grid[j] || j === i) continue;
    if (ROW[j] === ROW[i] || COL[j] === COL[i] || BOX[j] === BOX[i]) m |= 1 << (grid[j] - 1);
  }
  return ~m & 511;
}

module.exports = { count: count, fill: fill, candidates: candidates, nodes: function () { return nodes; }, ROW: ROW, COL: COL, BOX: BOX, POP: POP };
