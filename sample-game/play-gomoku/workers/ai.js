// 五子棋 AI：五元组评分 + α-β 剪枝（负极大值写法）。跑在 Worker 里（workers/index.js），主线程不卡。
//
// 评分：棋盘上所有「连续 5 格」的窗口（横竖斜共 572 个）。一个窗口里只有一方的子时，按子数给分（1~4 子 1/12/120/1500），
// 两方都有就作废。活四 = 两个 4 子窗口，冲四 = 一个；活三、眠三同理落在窗口数上，不用写棋型表。
// 下子 / 提子只更新经过那一点的至多 20 个窗口（增量），所以搜索里每个节点的估值是 O(1) 的。
//
// 剪枝：候选点只取已有棋子周围两格内的空点，按「进攻增益 + 防守增益」排序后只留前 K 个；
// 自己能连五只走那一步，对方有冲四只考虑堵点（必应手）；搜到底时对方还有冲四就顺着堵下去（不算层数），
// 连续冲四的杀棋不会藏在搜索深度之外。
var N = 15, NN = N * N;
var SC = [0, 1, 12, 120, 1500, 0];
var WIN_SCORE = 10000000, INF = 1e9, MAX_PLY = 16;

var WINS = [], CW = [];
(function build() {
  for (var i = 0; i < NN; i++) CW.push([]);
  var D = [[1, 0], [0, 1], [1, 1], [1, -1]];
  for (var y = 0; y < N; y++) for (var x = 0; x < N; x++) for (var d = 0; d < 4; d++) {
    var ex = x + D[d][0] * 4, ey = y + D[d][1] * 4;
    if (ex < 0 || ex >= N || ey < 0 || ey >= N) continue;
    var w = [];
    for (var k = 0; k < 5; k++) w.push((y + D[d][1] * k) * N + x + D[d][0] * k);
    var id = WINS.length;
    WINS.push(w);
    for (var j = 0; j < 5; j++) CW[w[j]].push(id);
  }
})();
var NW = WINS.length;

function Engine() {
  this.b = new Int8Array(NN);
  this.c = [null, new Int8Array(NW), new Int8Array(NW)];   // 每个窗口里黑 / 白各几子
  this.s = [0, 0, 0];                                       // 黑 / 白的窗口总分
  this.four = [0, 0, 0];                                    // 黑 / 白「4 子 + 对方 0 子」的窗口数（下一手能连五）
  this.near = new Int16Array(NN);                           // 周围两格内有几颗子（候选点）
  this.count = 0; this.nodes = 0; this.deadline = Infinity; this.aborted = false;
  this.buf = [];                                            // 每层一块候选点缓冲（搜索里不分配）
  for (var i = 0; i <= MAX_PLY + 2; i++) this.buf.push(new Int16Array(NN));
}
var E = Engine.prototype;
E.place = function (i, p) {
  var o = 3 - p, cp = this.c[p], co = this.c[o], ws = CW[i], won = false;
  for (var k = 0; k < ws.length; k++) {
    var w = ws[k], m = cp[w], n = co[w];
    if (n === 0) {
      this.s[p] += SC[m + 1] - SC[m];
      if (m === 3) this.four[p]++; else if (m === 4) { this.four[p]--; won = true; }
    } else if (m === 0) {                                   // 这个窗口原来只有对方的子：被我堵死
      this.s[o] -= SC[n];
      if (n === 4) this.four[o]--;
    }
    cp[w] = m + 1;
  }
  this.b[i] = p; this.count++;
  this.mark(i, 1);
  return won;
};
E.remove = function (i, p) {
  var o = 3 - p, cp = this.c[p], co = this.c[o], ws = CW[i];
  for (var k = 0; k < ws.length; k++) {
    var w = ws[k], m = cp[w] - 1, n = co[w];
    cp[w] = m;
    if (n === 0) {
      this.s[p] -= SC[m + 1] - SC[m];
      if (m === 3) this.four[p]--; else if (m === 4) this.four[p]++;
    } else if (m === 0) {
      this.s[o] += SC[n];
      if (n === 4) this.four[o]++;
    }
  }
  this.b[i] = 0; this.count--;
  this.mark(i, -1);
};
E.mark = function (i, d) {
  var x0 = i % N, y0 = (i - x0) / N;
  for (var y = Math.max(0, y0 - 2); y <= Math.min(N - 1, y0 + 2); y++)
    for (var x = Math.max(0, x0 - 2); x <= Math.min(N - 1, x0 + 2); x++) this.near[y * N + x] += d;
};
// 站在 p 这边的局面分：轮到 p 走，所以 p 有冲四就是赢；对方两个以上冲四基本堵不住
E.evaluate = function (p) {
  var o = 3 - p;
  if (this.four[p] > 0) return WIN_SCORE / 2;
  if (this.four[o] > 1) return -WIN_SCORE / 2;
  return this.s[p] * 1.2 - this.s[o];
};
// 候选点：按分数从高到低写进 out（Int16Array），返回个数（最多 k 个）。能赢只给那一步；对方有冲四只给堵点。
// 整个搜索里不分配对象：打分放共用的 SCORE / CAND，按层各用一块 out，前 k 名用选择排序挑。
// 不用「[分数, 下标] 数组 + sort(比较函数)」：那样每个节点要建几十个小数组，一步棋上万个节点就是几十万个垃圾对象
var SCORE = new Float64Array(NN), CAND = new Int16Array(NN);
E.moves = function (p, k, out) {
  var o = 3 - p, cp = this.c[p], co = this.c[o], n = 0, nb = 0;
  if (this.count === 0) { out[0] = 7 * N + 7; return 1; }
  for (var i = 0; i < NN; i++) {
    if (this.b[i] || !this.near[i]) continue;
    var ws = CW[i], atk = 0, def = 0, win = false, block = false;
    for (var j = 0; j < ws.length; j++) {
      var w = ws[j], m = cp[w], c = co[w];
      if (c === 0) { atk += SC[m + 1] - SC[m]; if (m === 4) win = true; }
      if (m === 0 && c > 0) { def += SC[c]; if (c === 4) block = true; }
    }
    if (win) { out[0] = i; return 1; }
    if (block) out[nb++] = i;
    SCORE[n] = atk + def * 0.9; CAND[n] = i; n++;
  }
  if (nb) return nb;
  var top = n < k ? n : k;
  for (var q = 0; q < top; q++) {
    var bi = q;
    for (var r = q + 1; r < n; r++) if (SCORE[r] > SCORE[bi]) bi = r;
    var ts = SCORE[q], tc = CAND[q];
    SCORE[q] = SCORE[bi]; CAND[q] = CAND[bi]; SCORE[bi] = ts; CAND[bi] = tc;
    out[q] = CAND[q];
  }
  return top;
};
E.search = function (depth, alpha, beta, p, ply, width) {
  this.nodes++;
  if (this.four[p] > 0) return WIN_SCORE - ply - 1;         // 轮到我且我有冲四：下一手就连五
  // 搜到底时对方有冲四：这一手只能堵，不算深度继续往下看（冲四连冲的杀棋不会被「地平线」藏起来）
  if (depth === 0 && !(this.four[3 - p] > 0 && ply < MAX_PLY)) return this.evaluate(p);
  if ((this.nodes & 255) === 0 && Date.now() > this.deadline) this.aborted = true;
  if (this.aborted) return 0;                               // 超时：这一层的结果整个作废（见 think）
  if (depth === 0) depth = 1;
  var ms = this.buf[ply], cnt = this.moves(p, width, ms);
  if (!cnt) return 0;                                       // 下满了：和棋
  var best = -INF;
  for (var i = 0; i < cnt; i++) {
    var mv = ms[i], won = this.place(mv, p);
    var v = won ? WIN_SCORE - ply : -this.search(depth - 1, -beta, -alpha, 3 - p, ply + 1, width);
    this.remove(mv, p);
    if (v > best) best = v;
    if (v > alpha) alpha = v;
    if (alpha >= beta) break;
  }
  return best;
};

// 难度：1 初级（不搜索，前几名里随机挑，偶尔漏看对方的活三）、2 中级（2 层）、3 高级（迭代加深 2 → 4 → 6 层，限时）。
// 高级的宽度 / 深度是自对弈定的（Node 里各 12 盘，开局第二手随机）：固定 4 层 × 宽 9 对中级只赢 6/10，6 层 × 宽 8 赢 12/12。
// 但 QuickJS 比 Node 慢约 80 倍（同一局面 12k 节点：Node ~12ms、本引擎 977ms），个别局面 6 层要算几十秒，
// 所以高级按时间走：每加深一层前看表，超时就停在已经算完的那一层（budget 毫秒）
var LEVELS = { 1: { depth: 0, width: 4 }, 2: { depth: 2, width: 12 }, 3: { depth: 6, width: 8, rootWidth: 14, budget: 1500 } };

function root(e, ms, side, depth, width) {
  var best = ms[0], bestV = -INF, alpha = -INF, beta = INF;
  for (var k = 0; k < ms.length; k++) {
    var won = e.place(ms[k], side);
    var v = won ? WIN_SCORE : -e.search(depth - 1, -beta, -alpha, 3 - side, 1, width);
    e.remove(ms[k], side);
    if (e.aborted) return null;
    if (v > bestV) { bestV = v; best = ms[k]; }
    if (v > alpha) alpha = v;
  }
  return { move: best, score: bestV };
}

// moves：落子序列（下标 = y*15+x），第一手黑（1）。side：轮到谁（1 / 2）。返回 { move, score, nodes, ms, depth }
function think(moves, side, level) {
  var t0 = Date.now(), e = new Engine(), cfg = LEVELS[level] || LEVELS[2];
  for (var i = 0; i < moves.length; i++) e.place(moves[i], i % 2 === 0 ? 1 : 2);
  var cnt = e.moves(side, cfg.rootWidth || cfg.width, e.buf[0]), ms = [];
  for (var c = 0; c < cnt; c++) ms.push(e.buf[0][c]);
  if (!ms.length) return { move: -1, score: 0, nodes: 0, ms: 0, depth: 0 };
  if (cfg.depth === 0 || ms.length === 1) {
    // 初级：前几名按名次加权随机（第一名 50%），必胜 / 必堵仍然照走
    var best = ms[0];
    if (ms.length > 1) {
      var r = Math.random(), pick = r < 0.5 ? 0 : r < 0.75 ? 1 : r < 0.9 ? 2 : 3;
      best = ms[Math.min(pick, ms.length - 1)];
    }
    return { move: best, score: 0, nodes: 1, ms: Date.now() - t0, depth: 0 };
  }
  e.deadline = cfg.budget ? t0 + cfg.budget : Infinity; e.aborted = false;
  var res = null, done = 0;
  for (var d = cfg.budget ? 2 : cfg.depth; d <= cfg.depth; d += 2) {
    var r2 = root(e, ms, side, d, cfg.width);
    if (!r2) break;
    res = r2; done = d;
    if (r2.score >= WIN_SCORE / 2 || r2.score <= -WIN_SCORE / 2) break;   // 已经算出必胜 / 必败，不用再深
    var k = ms.indexOf(r2.move); ms.splice(k, 1); ms.unshift(r2.move);    // 上一层的最好着法放最前，下一层剪得多
  }
  if (!res) res = { move: ms[0], score: 0 };                // 连 2 层都没算完（极慢的机器）：按排序第一名走
  return { move: res.move, score: res.score, nodes: e.nodes, ms: Date.now() - t0, depth: done };
}

module.exports = { think: think, N: N };
