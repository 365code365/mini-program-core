// 自动试玩（?auto=1）：每次给出一步 { type: 'open' | 'flag' | 'chord', i }，走的是和手指一样的入口
// （翻开 / 长按插旗 / 双击数字快速开周围），所以无头跑一遍就把这几条路径都走到了。
// 推理只用两条单格规则：① 数字 == 周围旗数 → 周围剩下的都安全（chord）；② 数字 − 旗数 == 周围未翻格数 → 都是雷（插旗）。
// 推不出来就猜：前沿格按「(数字 − 旗) / 未翻格」取各邻居里最大的那个当雷概率，非前沿格按剩余雷密度，挑最小的。
function next(b) {
  if (!b.planted) return { type: 'open', i: ((b.rows / 2) | 0) * b.cols + ((b.cols / 2) | 0) };
  var n = b.n, i, j, best = -1, bestP = 2;
  var risk = new Float32Array(n);                // 0 = 不在前沿
  for (i = 0; i < n; i++) {
    if (!b.open[i] || !b.num[i]) continue;
    var f = 0, h = 0, firstHidden = -1;
    for (j = i * 9; b.nb[j] >= 0; j++) {
      var m = b.nb[j];
      if (b.flag[m]) f++;
      else if (!b.open[m]) { h++; if (firstHidden < 0) firstHidden = m; }
    }
    if (!h) continue;
    if (f === b.num[i]) return { type: 'chord', i: i };
    if (b.num[i] - f === h) return { type: 'flag', i: firstHidden };
    var p = (b.num[i] - f) / h;
    for (j = i * 9; b.nb[j] >= 0; j++) {
      var q = b.nb[j];
      if (!b.open[q] && !b.flag[q] && p > risk[q]) risk[q] = p;
    }
  }
  var unknown = 0;
  for (i = 0; i < n; i++) if (!b.open[i] && !b.flag[i]) unknown++;
  var density = unknown ? (b.mines - b.flags) / unknown : 1;
  for (i = 0; i < n; i++) {
    if (b.open[i] || b.flag[i]) continue;
    var r = risk[i] || density;
    r += Math.random() * 0.01;                   // 同样危险时随机挑，别总从左上角猜
    if (r < bestP) { bestP = r; best = i; }
  }
  return best >= 0 ? { type: 'open', i: best } : null;
}

module.exports = { next: next };
