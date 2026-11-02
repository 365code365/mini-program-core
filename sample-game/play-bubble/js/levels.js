// 关卡：前 12 关手调参数（行数、颜色数、几发不消除就下压一行、摆法），之后按公式无限往后加。
// 摆法用固定种子的随机数生成，同一关每次一样；生成时四成的格子照抄左边或右上的颜色，摆出成片的同色，开局就有得消。
var NAMES = ['初来乍到', '彩虹糖', '三色堇', '夏日汽水', '果冻山', '星光小径', '漩涡', '棉花糖塔', '极光', '深海', '火山口', '龙之巢'];
var TABLE = [
  // rows 初始行数, colors 颜色数, push 几发不消除就下压, shape 摆法
  { rows: 5, colors: 3, push: 8, shape: 'full' },
  { rows: 6, colors: 3, push: 7, shape: 'stripes' },
  { rows: 6, colors: 4, push: 7, shape: 'full' },
  { rows: 7, colors: 4, push: 6, shape: 'diamond' },
  { rows: 7, colors: 4, push: 6, shape: 'stripes' },
  { rows: 7, colors: 5, push: 6, shape: 'full' },
  { rows: 8, colors: 5, push: 5, shape: 'zigzag' },
  { rows: 8, colors: 5, push: 5, shape: 'diamond' },
  { rows: 8, colors: 5, push: 5, shape: 'full' },
  { rows: 8, colors: 6, push: 5, shape: 'stripes' },
  { rows: 9, colors: 6, push: 4, shape: 'zigzag' },
  { rows: 9, colors: 6, push: 4, shape: 'full' }
];

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
function info(n) {
  var t = TABLE[Math.min(n, TABLE.length) - 1];
  if (n > TABLE.length) t = { rows: 9, colors: 6, push: Math.max(3, 4 - Math.floor((n - TABLE.length) / 4)), shape: ['full', 'stripes', 'diamond', 'zigzag'][n % 4] };
  return { n: n, name: n <= NAMES.length ? NAMES[n - 1] : '无尽 ' + (n - NAMES.length), rows: t.rows, colors: t.colors, push: t.push, shape: t.shape };
}
// 摆法决定哪些格子有泡泡（true = 放）
function keep(shape, i, c, cols) {
  if (shape === 'diamond') return Math.abs(c - (cols - 1) / 2) <= cols / 2 - Math.abs(i - 2) * 0.8 || i < 2;
  if (shape === 'zigzag') return i < 2 || ((c + i) % 6) !== 0;
  return true;
}
function build(grid, n) {
  var L = info(n), rand = rng(n * 7919 + 13);
  grid.rows = []; grid.wide0 = true; grid.version++;
  for (var i = 0; i < L.rows; i++) {
    grid.ensure(i);
    for (var c = 0; c < grid.len(i); c++) {
      if (!keep(L.shape, i, c, grid.cols)) continue;
      var k;
      if (L.shape === 'stripes' && rand() < 0.7) k = Math.floor(c / 2) % L.colors;
      else {
        var left = grid.get(i, c - 1), up = grid.get(i - 1, c);
        k = rand() < 0.4 && (left >= 0 || up >= 0) ? (left >= 0 && (up < 0 || rand() < 0.5) ? left : up) : Math.floor(rand() * L.colors);
      }
      grid.rows[i][c] = k;
    }
  }
  grid.floating().forEach(function (p) { grid.rows[p[0]][p[1]] = -1; });   // 摆法挖出的孤岛去掉
  grid.version++;
  return L;
}
module.exports = { info: info, build: build, rng: rng };
