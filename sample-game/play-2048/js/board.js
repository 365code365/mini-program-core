// 2048 的纯逻辑（不碰画布）：4×4 棋盘、滑动合并、出新块、判输赢。
// 每个方块有稳定的 id，渲染层靠 id 把方块从旧位置补间到新位置。
var N = 4;
var nextId = 1;

function create() { return { tiles: [], score: 0, won: false, keepGoing: false }; }

function at(b, r, c) {
  for (var i = 0; i < b.tiles.length; i++) { var t = b.tiles[i]; if (t.r === r && t.c === c && !t.gone) return t; }
  return null;
}
function empties(b) {
  var out = [];
  for (var r = 0; r < N; r++) for (var c = 0; c < N; c++) if (!at(b, r, c)) out.push([r, c]);
  return out;
}
function spawn(b, rnd) {
  var e = empties(b); if (!e.length) return null;
  var p = e[Math.floor(rnd() * e.length)];
  var t = { id: nextId++, r: p[0], c: p[1], v: rnd() < 0.9 ? 2 : 4, fromR: p[0], fromC: p[1], born: true, merged: false };
  b.tiles.push(t);
  return t;
}

// 沿 dir 滑动。返回 { moved, gained }；被合并掉的方块标 gone，渲染完补间后由 settle() 清掉
var DIRS = { left: [0, -1], right: [0, 1], up: [-1, 0], down: [1, 0] };
function move(b, dir) {
  settle(b);
  var d = DIRS[dir]; if (!d) return { moved: false, gained: 0 };
  b.tiles.forEach(function (t) { t.fromR = t.r; t.fromC = t.c; t.born = false; t.merged = false; });
  var order = [0, 1, 2, 3];
  if (d[0] === 1 || d[1] === 1) order.reverse();     // 从移动方向的那一头开始推
  var moved = false, gained = 0;
  for (var a = 0; a < N; a++) {
    for (var bi = 0; bi < N; bi++) {
      var r = d[0] ? order[bi] : a, c = d[1] ? order[bi] : a;
      var t = at(b, r, c); if (!t) continue;
      var nr = r, nc = c;
      while (true) {
        var tr = nr + d[0], tc = nc + d[1];
        if (tr < 0 || tr >= N || tc < 0 || tc >= N) break;
        var o = at(b, tr, tc);
        if (!o) { nr = tr; nc = tc; continue; }
        if (o.v === t.v && !o.merged) {
          // 合并：t 滑进 o 的格子后消失，o 翻倍（同一次滑动里每块最多合并一次）
          t.r = tr; t.c = tc; t.gone = true;
          o.v *= 2; o.merged = true; gained += o.v;
          if (o.v === 2048 && !b.won) b.won = true;
          moved = true;
          nr = -1;
        }
        break;
      }
      if (nr >= 0 && (nr !== r || nc !== c)) { t.r = nr; t.c = nc; moved = true; }
    }
  }
  b.score += gained;
  return { moved: moved, gained: gained };
}
function settle(b) { b.tiles = b.tiles.filter(function (t) { return !t.gone; }); }

function canMove(b) {
  if (empties(b).length) return true;
  for (var r = 0; r < N; r++) for (var c = 0; c < N; c++) {
    var t = at(b, r, c);
    var right = c + 1 < N && at(b, r, c + 1), down = r + 1 < N && at(b, r + 1, c);
    if ((right && right.v === t.v) || (down && down.v === t.v)) return true;
  }
  return false;
}
function maxTile(b) { return b.tiles.reduce(function (m, t) { return t.gone ? m : Math.max(m, t.v); }, 0); }

// 存档只存值，读回来重新编 id
function serialize(b) {
  settle(b);
  return { score: b.score, won: b.won, keepGoing: b.keepGoing, cells: b.tiles.map(function (t) { return [t.r, t.c, t.v]; }) };
}
function restore(s) {
  if (!s || !s.cells || !s.cells.length) return null;
  var b = create();
  b.score = Number(s.score) || 0; b.won = !!s.won; b.keepGoing = !!s.keepGoing;
  s.cells.forEach(function (x) {
    if (x[0] >= 0 && x[0] < N && x[1] >= 0 && x[1] < N && x[2] > 0)
      b.tiles.push({ id: nextId++, r: x[0], c: x[1], v: x[2], fromR: x[0], fromC: x[1], born: false, merged: false });
  });
  return b.tiles.length ? b : null;
}

module.exports = { N: N, create: create, spawn: spawn, move: move, settle: settle, canMove: canMove,
  maxTile: maxTile, serialize: serialize, restore: restore };
