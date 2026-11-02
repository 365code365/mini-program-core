// 三张关卡图（9 列 × 12 行）：'.' 草地可建塔、'#' 路、'S' 出兵口、'E' 城堡、'x' 石头、't' 树（后两种不能建）。
// 路线不手写拐点：从 S 沿路砖 BFS 到 E 得到逐格的路径，再并成拐点折线（寻路）。飞行敌人不走路，直线从 S 飞向 E。
var COLS = 9, ROWS = 12;

var MAPS = [
  { name: '青石关', hp: 1, tip: '蛇形长路，适合新手', rows: [
    '.S.....t.',
    '.#..x....',
    '.#######.',
    't......#.',
    '..x....#.',
    '.#######.',
    '.#.....t.',
    '.#..x....',
    '.#######.',
    '.t.....#.',
    '....x..#.',
    '.......E.'] },
  { name: '河谷道', hp: 1.3, tip: '回字形长路，城堡在谷底', rows: [
    '..t....x.',
    'S#######.',
    '.......#.',
    '.x.....#.',
    '.####..#.',
    '.#..#..#.',
    '.#..#.t#.',
    '.#..E..#.',
    '.#.....#.',
    '.#..x..#.',
    '.#######.',
    't......x.'] },
  { name: '烽火台', hp: 1.6, tip: '路短敌强，精打细算', rows: [
    '....S..t.',
    '.x..#....',
    '.####....',
    '.#.....x.',
    '.#..t....',
    '.#.......',
    '.#######.',
    '.......#.',
    '..x....#.',
    '...#####.',
    '...#..t..',
    '...E.....'] }
];

function parse(def) {
  var g = [], S = null, E = null;
  for (var r = 0; r < ROWS; r++) for (var c = 0; c < COLS; c++) {
    var ch = def.rows[r][c];
    g.push(ch);
    if (ch === 'S') S = { c: c, r: r };
    if (ch === 'E') E = { c: c, r: r };
  }
  function road(c, r) { if (c < 0 || r < 0 || c >= COLS || r >= ROWS) return false; var ch = g[r * COLS + c]; return ch === '#' || ch === 'S' || ch === 'E'; }
  // BFS：路砖上的最短路（图上只有一条路，BFS 顺便验证 S 与 E 连通）
  var prev = new Int32Array(COLS * ROWS).fill(-2), q = [S.r * COLS + S.c];
  prev[q[0]] = -1;
  for (var h = 0; h < q.length; h++) {
    var k = q[h], kc = k % COLS, kr = (k / COLS) | 0;
    [[1, 0], [-1, 0], [0, 1], [0, -1]].forEach(function (d) {
      var nc = kc + d[0], nr = kr + d[1], nk = nr * COLS + nc;
      if (road(nc, nr) && prev[nk] === -2) { prev[nk] = k; q.push(nk); }
    });
  }
  var tiles = [], e = E.r * COLS + E.c;
  if (prev[e] === -2) throw new Error('关卡「' + def.name + '」的路不通');
  for (; e >= 0; e = prev[e]) tiles.push({ c: e % COLS, r: (e / COLS) | 0 });
  tiles.reverse();
  // 逐格 → 拐点：同方向的中间格去掉。出兵口往外再延半格，敌人从图边走进来
  var pts = [{ x: S.c + 0.5, y: S.r + 0.5 }];
  for (var i = 1; i < tiles.length - 1; i++) {
    var a = tiles[i - 1], b = tiles[i], c2 = tiles[i + 1];
    if ((b.c - a.c) !== (c2.c - b.c) || (b.r - a.r) !== (c2.r - b.r)) pts.push({ x: b.c + 0.5, y: b.r + 0.5 });
  }
  pts.push({ x: E.c + 0.5, y: E.r + 0.5 });
  var first = tiles[1] || tiles[0], dx = S.c - first.c, dy = S.r - first.r;
  pts[0] = { x: S.c + 0.5 + dx * 0.6, y: S.r + 0.5 + dy * 0.6 };
  var segs = [], total = 0;
  for (var s = 0; s + 1 < pts.length; s++) {
    var l = Math.abs(pts[s + 1].x - pts[s].x) + Math.abs(pts[s + 1].y - pts[s].y);
    segs.push({ x0: pts[s].x, y0: pts[s].y, x1: pts[s + 1].x, y1: pts[s + 1].y, len: l, at: total });
    total += l;
  }
  // 每块空地覆盖多少路砖（射程 2.5 格内）：自动试玩挑塔位用
  var cover = [];
  for (var cc = 0; cc < COLS * ROWS; cc++) {
    if (g[cc] !== '.') { cover.push(-1); continue; }
    var x = cc % COLS, y = (cc / COLS) | 0, n = 0;
    tiles.forEach(function (t) { var ddx = t.c - x, ddy = t.r - y; if (ddx * ddx + ddy * ddy <= 6.25) n++; });
    cover.push(n);
  }
  return { name: def.name, hp: def.hp, tip: def.tip, g: g, S: S, E: E, tiles: tiles, pts: pts, segs: segs, len: total, cover: cover,
    fly: { x0: pts[0].x, y0: pts[0].y, x1: E.c + 0.5, y1: E.r + 0.5 } };
}

// 沿路走了 d 格时的位置（写进 out，不分配）；seg 是上次所在段的提示，沿路只会往前走
function along(m, d, out, hint) {
  var k = hint || 0, segs = m.segs;
  while (k + 1 < segs.length && d > segs[k].at + segs[k].len) k++;
  var s = segs[k], t = s.len ? Math.min(1, Math.max(0, (d - s.at) / s.len)) : 0;
  out.x = s.x0 + (s.x1 - s.x0) * t; out.y = s.y0 + (s.y1 - s.y0) * t;
  return k;
}
function buildable(m, c, r) { return c >= 0 && r >= 0 && c < COLS && r < ROWS && m.g[r * COLS + c] === '.'; }

module.exports = { COLS: COLS, ROWS: ROWS, list: MAPS.map(parse), along: along, buildable: buildable };
