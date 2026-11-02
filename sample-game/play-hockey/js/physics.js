// 桌上冰球的物理：固定步长（240Hz）推进，渲染帧再快再慢，碰撞结果都一样。
// 球（puck）是会动的圆，两只球拍（mallet）是质量无穷大的圆：按目标点追过去（限速），把自己的速度带给球。
// 碰撞：圆与圆（球拍、门柱）、圆与墙（四边，两端墙中间开球门）。球速有上限，冰面有一点点摩擦。
var STEP = 1 / 240;
var PUCK_R = 13, MALLET_R = 24, MAX_PUCK = 1500, MAX_MALLET = 2200;
var WALL_E = 0.88, MALLET_E = 0.92, FRICTION = 0.32;

function makeRink(L, T, R, B) {
  var h = B - T;
  return { L: L, T: T, R: R, B: B, W: R - L, H: h, mid: (L + R) / 2, cy: (T + B) / 2, goalH: Math.round(h * 0.38) };
}

function makeWorld(rink, onEvent) {
  var puck = { x: rink.mid, y: rink.cy, vx: 0, vy: 0 };
  // side：-1 左（红），1 右（蓝）
  var mallets = [-1, 1].map(function (side) {
    var hx = side < 0 ? rink.L + rink.W * 0.12 : rink.R - rink.W * 0.12;
    return { side: side, x: hx, y: rink.cy, vx: 0, vy: 0, tx: hx, ty: rink.cy, hx: hx, touch: null };
  });
  var acc = 0, frozen = 0, idle = 0;

  function clampMallet(m) {
    var r = MALLET_R;
    var x0 = m.side < 0 ? rink.L + r : rink.mid + r, x1 = m.side < 0 ? rink.mid - r : rink.R - r;
    m.tx = Math.max(x0, Math.min(x1, m.tx)); m.ty = Math.max(rink.T + r, Math.min(rink.B - r, m.ty));
  }
  function moveMallet(m, dt) {
    clampMallet(m);
    var dx = m.tx - m.x, dy = m.ty - m.y, d = Math.sqrt(dx * dx + dy * dy), maxd = (m.maxSpeed || MAX_MALLET) * dt;
    if (d > maxd) { dx *= maxd / d; dy *= maxd / d; }
    m.vx = dx / dt; m.vy = dy / dt;
    m.x += dx; m.y += dy;
  }
  // 圆 vs 固定圆（或球拍）：把球推到刚好相切，再按相对法向速度反弹
  function bounceOff(cx, cy, cr, cvx, cvy, e) {
    var dx = puck.x - cx, dy = puck.y - cy, rr = cr + PUCK_R, d2 = dx * dx + dy * dy;
    if (d2 >= rr * rr) return 0;
    var d = Math.sqrt(d2) || 0.0001, nx = dx / d, ny = dy / d;
    puck.x = cx + nx * rr; puck.y = cy + ny * rr;
    var rvx = puck.vx - cvx, rvy = puck.vy - cvy, vn = rvx * nx + rvy * ny;
    if (vn >= 0) return 0;
    puck.vx -= (1 + e) * vn * nx; puck.vy -= (1 + e) * vn * ny;
    return -vn;
  }
  function step(dt) {
    for (var i = 0; i < 2; i++) moveMallet(mallets[i], dt);
    if (frozen > 0) { frozen -= dt; return; }
    var k = Math.exp(-FRICTION * dt);
    puck.vx *= k; puck.vy *= k;
    puck.x += puck.vx * dt; puck.y += puck.vy * dt;
    for (i = 0; i < 2; i++) {
      var m = mallets[i], hit = bounceOff(m.x, m.y, MALLET_R, m.vx, m.vy, MALLET_E);
      if (hit > 0) onEvent('mallet', hit, puck.x, puck.y, m.side);
    }
    var r = PUCK_R, g0 = rink.cy - rink.goalH / 2, g1 = rink.cy + rink.goalH / 2;
    var inMouth = puck.y > g0 && puck.y < g1;
    // 上下墙
    if (puck.y < rink.T + r && !(puck.x < rink.L || puck.x > rink.R)) { puck.y = rink.T + r; if (puck.vy < 0) { onEvent('wall', -puck.vy); puck.vy = -puck.vy * WALL_E; } }
    if (puck.y > rink.B - r && !(puck.x < rink.L || puck.x > rink.R)) { puck.y = rink.B - r; if (puck.vy > 0) { onEvent('wall', puck.vy); puck.vy = -puck.vy * WALL_E; } }
    // 两端墙：门口那一段是空的；门柱是两个点（半径 0 的圆）
    for (var s = -1; s <= 1; s += 2) {
      var wx = s < 0 ? rink.L : rink.R, beyond = s < 0 ? puck.x - r < wx : puck.x + r > wx;
      if (!beyond) continue;
      if (!inMouth) {
        if ((s < 0 ? puck.x : -puck.x) > (s < 0 ? wx : -wx) - r * 0.5) {
          puck.x = wx - s * r;
          if (puck.vx * s > 0) { onEvent('wall', Math.abs(puck.vx)); puck.vx = -puck.vx * WALL_E; }
        }
      }
      var p0 = bounceOff(wx, g0, 0, 0, 0, WALL_E), p1 = bounceOff(wx, g1, 0, 0, 0, WALL_E);
      if (p0 + p1 > 0) onEvent('post', p0 + p1);
      // 进了门洞：上下贴着门框走，过了门线一个球身就算进
      if ((s < 0 ? puck.x < wx : puck.x > wx)) {
        puck.y = Math.max(g0 + r * 0.6, Math.min(g1 - r * 0.6, puck.y));
        if (s < 0 ? puck.x < wx - r * 1.6 : puck.x > wx + r * 1.6) { onEvent('goal', s > 0 ? -1 : 1); return; }
      }
    }
    var sp = Math.sqrt(puck.vx * puck.vx + puck.vy * puck.vy);
    if (sp > MAX_PUCK) { puck.vx *= MAX_PUCK / sp; puck.vy *= MAX_PUCK / sp; }
    idle = sp < 12 ? idle + dt : 0;
  }
  // 推进 dt 秒（渲染帧）：按固定步长走若干步，余量留到下一帧
  function advance(dt) {
    acc += Math.min(dt, 0.1);
    var n = 0;
    while (acc >= STEP && n < 40) { step(STEP); acc -= STEP; n++; }
    return n;
  }
  // 开球：球放在 side 那一半（丢球的一方先发），停一会儿再动
  function serve(side, wait) {
    puck.x = side < 0 ? rink.L + rink.W * 0.3 : rink.R - rink.W * 0.3;
    if (!side) puck.x = rink.mid;
    puck.y = rink.cy; puck.vx = 0; puck.vy = 0; frozen = wait || 0; idle = 0;
  }
  function resetMallets() {
    mallets.forEach(function (m) { m.x = m.tx = m.hx; m.y = m.ty = rink.cy; m.vx = m.vy = 0; });
  }
  return {
    puck: puck, mallets: mallets, rink: rink, advance: advance, serve: serve, resetMallets: resetMallets,
    frozen: function () { return frozen > 0; }, idle: function () { return idle; }
  };
}

module.exports = { makeRink: makeRink, makeWorld: makeWorld, PUCK_R: PUCK_R, MALLET_R: MALLET_R, MAX_PUCK: MAX_PUCK, STEP: STEP };
