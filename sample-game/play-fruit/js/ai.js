// 自动试玩（?auto=1）：挑一个（能串上就挑两个挨着的）飞到高处的水果，算一条穿过它、离炸弹够远的直线，
// 分 5 帧把这条线当成手指划出来 —— 走的是和真手指同一条刀光 / 切割路径（blade.move → world.slice）。不分配。
var F = require('./fruits.js');
var st = { on: false, i: 0, n: 5, x0: 0, y0: 0, x1: 0, y1: 0, cool: 0.3 };
var ID = 9999;                                      // 假手指的 identifier

function clearOfBombs(Wd, x0, y0, x1, y1) {
  for (var i = 0; i < Wd.fruits.length; i++) {
    var b = Wd.fruits[i];
    if (b.on && b.k < 0 && Wd.segHit(x0, y0, x1, y1, b.x + b.vx * 0.05, b.y + b.vy * 0.05, F.BOMB_R + 22)) return false;
  }
  return true;
}
function plan(ax, ay, ux, uy, ext) { st.x0 = ax - ux * ext; st.y0 = ay - uy * ext; st.x1 = ax + ux * ext; st.y1 = ay + uy * ext; }

function think(S, Wd, W, H, dt, hand) {
  if (st.on) {
    st.i++;
    var t = st.i / st.n;
    hand.move(st.x0 + (st.x1 - st.x0) * t, st.y0 + (st.y1 - st.y0) * t, ID);
    if (st.i >= st.n) { hand.up(ID); st.on = false; st.cool = 0.12 + Math.random() * 0.25; }
    return;
  }
  if ((st.cool -= dt) > 0) return;
  if (S.state === 'ready' && !S.menuGo) {           // 菜单：轮流切两种模式
    var m = S.menu[S.aiNext === 'timed' ? 1 : 0];
    plan(m.x, m.y, 0.8, -0.6, 90);
    return go(hand);
  }
  if (S.state !== 'play') return;
  var fr = Wd.fruits, best = null, bestY = 1e9, i;
  for (i = 0; i < fr.length; i++) {
    var f = fr[i];
    if (!f.on || f.k < 0 || f.y > H - 70 || f.y < 60 || f.vy < -650) continue;
    if (f.y < bestY) { bestY = f.y; best = f; }
  }
  if (!best) return;
  var px = best.x + best.vx * 0.05, py = best.y + best.vy * 0.05, mate = null;
  for (i = 0; i < fr.length; i++) {                  // 找个挨着的一起切，凑连切
    var g = fr[i];
    if (g === best || !g.on || g.k < 0 || g.y > H - 70) continue;
    var dx = g.x - best.x, dy = g.y - best.y;
    if (dx * dx + dy * dy < 150 * 150) { mate = g; break; }
  }
  var tries = 0;
  while (tries++ < 5) {
    var ux, uy, ax = px, ay = py, ext = 70;
    if (mate && tries === 1) {
      var mx = mate.x + mate.vx * 0.05, my = mate.y + mate.vy * 0.05, l = Math.sqrt((mx - px) * (mx - px) + (my - py) * (my - py)) || 1;
      ux = (mx - px) / l; uy = (my - py) / l; ax = (px + mx) / 2; ay = (py + my) / 2; ext = l / 2 + 60;
    } else { var a = (Math.random() - 0.5) * 2.4; ux = Math.cos(a); uy = Math.sin(a); if (Math.random() < 0.5) { ux = -ux; uy = -uy; } }
    plan(ax, ay, ux, uy, ext);
    if (clearOfBombs(Wd, st.x0, st.y0, st.x1, st.y1)) return go(hand);
  }
  st.cool = 0.1;                                    // 这一刀躲不开炸弹：等一会儿再说
}
function go(hand) { st.on = true; st.i = 0; hand.down(st.x0, st.y0, ID); }

module.exports = { think: think };
