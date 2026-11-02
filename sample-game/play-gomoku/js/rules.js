// 主线程这边的规则：连五判定（并给出整条连线用来高亮）、坐标换算。AI 在 workers/ai.js。
var N = 15;
var DIRS = [[1, 0], [0, 1], [1, 1], [1, -1]];
// board：长 225 的数组（0 空 / 1 黑 / 2 白）。返回从 i 出发连成 ≥5 的那条线（下标数组），没有返回 null
function fiveAt(board, i) {
  var p = board[i]; if (!p) return null;
  var x0 = i % N, y0 = (i - x0) / N;
  for (var d = 0; d < 4; d++) {
    var dx = DIRS[d][0], dy = DIRS[d][1], line = [i];
    for (var s = -1; s <= 1; s += 2) {
      var x = x0 + dx * s, y = y0 + dy * s;
      while (x >= 0 && y >= 0 && x < N && y < N && board[y * N + x] === p) {
        if (s < 0) line.unshift(y * N + x); else line.push(y * N + x);
        x += dx * s; y += dy * s;
      }
    }
    if (line.length >= 5) return line;
  }
  return null;
}
// 坐标名：横 A~O，竖 15~1（棋谱习惯从下往上数），例：天元 H8
function coordName(i) { var x = i % N, y = (i - x) / N; return 'ABCDEFGHIJKLMNO'[x] + (N - y); }
module.exports = { N: N, fiveAt: fiveAt, coordName: coordName };
