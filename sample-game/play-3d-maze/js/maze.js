// 迷宫数据：DFS（回溯法）挖出 n×n 个格子的完美迷宫，存成 (2n+1)×(2n+1) 的砖块图（1 = 墙，0 = 路），
// 再随机打通几面墙让迷宫带环（完美迷宫只有一条路，宝石多了来回走太闷）。
// 砖块图同时给碰撞、合并墙体几何、小地图、自动试玩的 BFS 寻路用，一份数据四处读。

// 可复现的伪随机（mulberry32）：同一个种子出同一张图，方便复现 bug
function rng(seed) {
  var s = seed >>> 0;
  return function () {
    s = (s + 0x6D2B79F5) >>> 0;
    var t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

var DIRS = [[1, 0], [-1, 0], [0, 1], [0, -1]];

function generate(n, seed) {
  var T = 2 * n + 1, g = new Uint8Array(T * T), r = rng(seed);
  g.fill(1);
  var seen = new Uint8Array(n * n), stack = [0];
  seen[0] = 1; g[T + 1] = 0;
  // 显式栈，不递归：31×31 的图递归深度能到几百层
  while (stack.length) {
    var c = stack[stack.length - 1], cx = c % n, cy = (c / n) | 0, opts = [];
    for (var d = 0; d < 4; d++) {
      var nx = cx + DIRS[d][0], ny = cy + DIRS[d][1];
      if (nx >= 0 && ny >= 0 && nx < n && ny < n && !seen[ny * n + nx]) opts.push(d);
    }
    if (!opts.length) { stack.pop(); continue; }
    var k = opts[(r() * opts.length) | 0], ax = cx + DIRS[k][0], ay = cy + DIRS[k][1];
    seen[ay * n + ax] = 1;
    g[(2 * cy + 1 + DIRS[k][1]) * T + 2 * cx + 1 + DIRS[k][0]] = 0;    // 两格之间那面墙
    g[(2 * ay + 1) * T + 2 * ax + 1] = 0;
    stack.push(ay * n + ax);
  }
  // 打通 n/2 面内墙：只挑两侧都是路、另两侧是墙的那种（不会打出大空场）
  var loops = Math.floor(n / 2), tries = 0;
  while (loops > 0 && tries++ < 500) {
    var i = 1 + ((r() * (T - 2)) | 0), j = 1 + ((r() * (T - 2)) | 0);
    if (g[j * T + i] !== 1 || (i + j) % 2 === 0) continue;
    var h = !g[j * T + i - 1] && !g[j * T + i + 1] && g[(j - 1) * T + i] && g[(j + 1) * T + i];
    var v = !g[(j - 1) * T + i] && !g[(j + 1) * T + i] && g[j * T + i - 1] && g[j * T + i + 1];
    if (h || v) { g[j * T + i] = 0; loops--; }
  }
  return { n: n, T: T, g: g, rand: r, start: { i: 1, j: 1 }, exit: { i: T - 2, j: T - 2 } };
}

function wall(m, i, j) { return i < 0 || j < 0 || i >= m.T || j >= m.T || m.g[j * m.T + i] === 1; }

// BFS：从 (si, sj) 到每块路砖的步数（-1 = 到不了）与前驱
function bfs(m, si, sj) {
  var T = m.T, dist = new Int16Array(T * T), prev = new Int32Array(T * T), q = new Int32Array(T * T), h = 0, t = 0;
  dist.fill(-1);
  dist[sj * T + si] = 0; prev[sj * T + si] = -1; q[t++] = sj * T + si;
  while (h < t) {
    var c = q[h++], ci = c % T, cj = (c / T) | 0;
    for (var d = 0; d < 4; d++) {
      var ni = ci + DIRS[d][0], nj = cj + DIRS[d][1], k = nj * T + ni;
      if (wall(m, ni, nj) || dist[k] >= 0) continue;
      dist[k] = dist[c] + 1; prev[k] = c; q[t++] = k;
    }
  }
  return { dist: dist, prev: prev };
}

// 从 from 走到 to 的砖块序列（含两端）；到不了返回 []
function path(m, from, to) {
  var b = bfs(m, from.i, from.j), k = to.j * m.T + to.i, out = [];
  if (b.dist[k] < 0) return out;
  while (k >= 0) { out.push({ i: k % m.T, j: (k / m.T) | 0 }); k = b.prev[k]; }
  return out.reverse();
}

// 宝石放在格子中心（奇数坐标）上，优先死胡同（三面是墙）：要拐进去拿，才有「探索」的意思。
// 离起点、出口与已放的宝石都要隔开，避免扎堆
function placeGems(m, count) {
  var T = m.T, dead = [], open = [], r = m.rand, out = [];
  for (var j = 1; j < T - 1; j += 2) for (var i = 1; i < T - 1; i += 2) {
    if ((i === m.start.i && j === m.start.j) || (i === m.exit.i && j === m.exit.j)) continue;
    var walls = 0;
    for (var d = 0; d < 4; d++) if (wall(m, i + DIRS[d][0], j + DIRS[d][1])) walls++;
    (walls === 3 ? dead : open).push({ i: i, j: j });
  }
  function shuffle(a) { for (var x = a.length - 1; x > 0; x--) { var y = (r() * (x + 1)) | 0, t = a[x]; a[x] = a[y]; a[y] = t; } return a; }
  var pool = shuffle(dead).concat(shuffle(open));
  for (var minGap = 6; minGap >= 0 && out.length < count; minGap -= 2) {
    for (var p = 0; p < pool.length && out.length < count; p++) {
      var c = pool[p], ok = Math.abs(c.i - m.start.i) + Math.abs(c.j - m.start.j) > minGap;
      for (var q = 0; q < out.length && ok; q++) if (out[q] === c || Math.abs(out[q].i - c.i) + Math.abs(out[q].j - c.j) <= minGap) ok = false;
      if (ok) out.push(c);
    }
  }
  return out;
}

// 贪心路线：从起点每次去 BFS 最近的宝石，最后去出口。返回总步数（定时限用）
function tourLength(m, gems) {
  var cur = m.start, left = gems.slice(), total = 0;
  while (left.length) {
    var b = bfs(m, cur.i, cur.j), best = 0;
    for (var k = 1; k < left.length; k++) if (b.dist[left[k].j * m.T + left[k].i] < b.dist[left[best].j * m.T + left[best].i]) best = k;
    total += b.dist[left[best].j * m.T + left[best].i];
    cur = left.splice(best, 1)[0];
  }
  return total + bfs(m, cur.i, cur.j).dist[m.exit.j * m.T + m.exit.i];
}

module.exports = { generate: generate, wall: wall, bfs: bfs, path: path, placeGems: placeGems, tourLength: tourLength, rng: rng };
