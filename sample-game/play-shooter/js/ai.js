// 自动试玩（?auto=1）：每帧在战机周围 21 个候选点里挑「未来 0.12 / 0.28 秒最不危险、又离瞄准点近」的那个飞过去，
// 躲不开就扔炸弹。先把战机附近的敌弹筛进 near[]，候选点 × 子弹的计算量才够小；不分配（结果写进同一个 out）。
var near = new Int16Array(512);
var DX = [-90, -45, -20, 0, 20, 45, 90], DY = [-40, 0, 40], TS = [0.12, 0.28], WT = [2, 1];
var out = { x: 0, y: 0, bomb: false };
var bombCd = 0, planT = 0;

function danger(Wd, n, cx, cy) {
  var eb = Wd.eb, en = Wd.en, d = 0;
  for (var k = 0; k < 2; k++) {
    var t = TS[k];
    for (var i = 0; i < n; i++) {
      var b = eb[near[i]], px = b.x + b.vx * t - cx, py = b.y + b.vy * t - cy, R = b.r + 14, R2 = R * R * 3;
      var d2 = px * px + py * py;
      if (d2 < R2) d += (1 - d2 / R2) * WT[k];
    }
    for (var j = 0; j < en.length; j++) {
      var e = en[j]; if (!e.on) continue;
      var ex = e.x - cx, ey = e.y + (e.type === 0 ? e.vy * t : 0) - cy, ER = e.r + 22;
      var e2 = ex * ex + ey * ey;
      if (e2 < ER * ER) d += (1 - e2 / (ER * ER)) * 2 * WT[k];
    }
  }
  return d;
}

// 每 50ms 重新规划一次（20Hz 的反应对躲弹足够；每帧都算在无头 CPU 上要多 1~2ms）
function think(S, Wd, W, H, dt) {
  bombCd -= dt; planT -= dt;
  if (planT > 0) { out.bomb = false; return out; }
  planT = 0.05;
  var eb = Wd.eb, n = 0, i;
  for (i = 0; i < eb.length && n < near.length; i++) {
    var b = eb[i];
    if (b.on && Math.abs(b.x - S.sx) < 200 && Math.abs(b.y - S.sy) < 220) near[n++] = i;
  }
  // 瞄准：首领 > 最靠下的敌机；道具在可达范围就先去捡
  var ax = W / 2, low = -1e9;
  if (S.boss.on) ax = S.boss.x;
  else for (i = 0; i < Wd.en.length; i++) {
    var e = Wd.en[i];
    if (e.on && e.y > 20 && e.y < S.sy - 60 && e.y > low) { low = e.y; ax = e.x; }
  }
  for (i = 0; i < Wd.pu.length; i++) {
    var p = Wd.pu[i];
    if (p.on && p.y < S.sy + 10 && p.y > S.sy - 240) { ax = p.x; break; }
  }
  var homeY = H - 130, best = 1e18, bd = 0;
  for (var a = 0; a < DX.length; a++) for (var c = 0; c < DY.length; c++) {
    var cx = Math.max(24, Math.min(W - 24, S.sx + DX[a])), cy = Math.max(H * 0.45, Math.min(H - 70, S.sy + DY[c]));
    var d = danger(Wd, n, cx, cy);
    var cost = d * 1000 + Math.abs(cx - ax) * 0.6 + Math.abs(cy - homeY) * 0.3 + (Math.abs(DX[a]) + Math.abs(DY[c])) * 0.05;
    if (cost < best) { best = cost; out.x = cx; out.y = cy; bd = d; }
  }
  out.bomb = bd > 0.9 && S.bombs > 0 && S.invT <= 0 && bombCd <= 0;
  if (out.bomb) bombCd = 3;
  return out;
}

module.exports = { think: think };
