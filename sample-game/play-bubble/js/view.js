// 画面：只读状态（命中框写回 S.buttons）。
// - 网格里的泡泡整片录成一个图层，网格没变（grid.version 不变）就不重录，每帧一次 drawArrays；
//   下压一行时图层整体从上面滑下来（只改图层的 dy）。飞行中 / 消除 / 掉落的泡泡用按颜色录好的精灵画（位置缩放透明度走 uniform）。
// - 无头 CPU 后端上一帧的时间主要花在填充像素：底色交给清屏色，弹层只画面板，不铺整屏遮罩（见 play-piano 的 README）。
var B, W, H;
// 颜色顺序按「前几关只用前几种」排：前三种（红 / 蓝 / 黄）色相拉得最开，第一关不会出现分不清的橙红
var BASE = ['#ef5350', '#42a5f5', '#ffd54f', '#66bb6a', '#ab47bc', '#ffa726'];
var DARK = ['#b71c1c', '#1565c0', '#f9a825', '#2e7d32', '#6a1b9a', '#e65100'];
function setup(b) { B = b; W = b.W; H = b.H; }

// 一颗泡泡 = 实心底色 + 外圈深色环 + 左上高光。外圈用环（strokeCircle）而不是「先铺一个深色大圆再盖一个浅色圆」：
// 满屏泡泡时少填一整层像素（无头 CPU 后端的时间主要花在填充上）
function bubble(R, x, y, r, k) {
  R.circle(x, y, r * 0.87, BASE[k]);
  R.strokeCircle(x, y, r * 0.935 - 0.25, r * 0.13, DARK[k]);
  R.ellipse(x - r * 0.32, y - r * 0.36, r * 0.3, r * 0.17, -0.6, 'rgba(255,255,255,0.75)');
}
function sprite(R, k, x, y, r, sc, a) { R.sprite('b' + k, 1, function () { bubble(R, 0, 0, r, k); }, x, y, sc, a); }

function draw(R, S, grid, aimSim, nextBox) {
  R.begin('#1b2a49');
  var g = S.geo, r = g.RAD;
  if (S.scene === 'menu') { menu(R, S, r); return R.end(); }
  // 场地：两侧墙、顶边、底线（静态图层）
  R.layer('field', 1, function () {
    R.gradRect(0, 0, W, g.FT, [[0, '#13203a'], [1, '#1b2a49']], true);
    R.rect(g.LEFT - 3, g.FT, 3, g.DY - g.FT + 30, '#2f4677'); R.rect(W - g.LEFT, g.FT, 3, g.DY - g.FT + 30, '#2f4677');
    R.rect(g.LEFT - 3, g.FT - 3, W - 2 * g.LEFT + 6, 3, '#2f4677');
    for (var x = g.LEFT; x < W - g.LEFT; x += 14) R.rect(x, g.DY, 8, 2, 'rgba(255,82,82,0.55)');
    R.circle(g.SX, g.SY + 6, 40, '#22365f'); R.strokeCircle(g.SX, g.SY + 6, 40, 3, '#3b5793');
  });
  var push = Math.min(1, (S.clock - S.pushAt) / 0.22);
  R.layer('grid', grid.version, function () { grid.each(function (i, c, k) { bubble(R, grid.x(i, c), grid.y(i), r, k); }); }, 0, (push - 1) * grid.rowH);
  // 消除：放大淡出；掉落：精灵
  for (var i = 0; i < S.pops.length; i++) { var p = S.pops[i]; if (p.t >= 0) sprite(R, p.k, p.x, p.y, r, 1 + p.t * 2.2, 1 - p.t / 0.22); }
  for (i = 0; i < S.falls.length; i++) { var f = S.falls[i]; if (f.t >= 0) sprite(R, f.k, f.x, f.y, r, 1, Math.max(0.2, 1 - f.t * 0.8)); }
  if (S.scene === 'play' && !S.fly && S.aim.on) aimLine(R, S, grid, aimSim(), r);
  if (S.fly) sprite(R, S.fly.color, S.fly.pos[0], S.fly.pos[1], r, 1, 1);
  shooter(R, S, g, r, nextBox);
  hud(R, S, g);
  if (S.scene === 'over') over(R, S);
  else if (S.scene === 'clear') clear(R, S);
  if (S.paused) {
    R.rrect(30, H / 2 - 60, W - 60, 120, 18, 'rgba(10,16,30,0.92)');
    R.text('已暂停', W / 2, H / 2 - 14, 28, '#ffffff', 'center', 'bold');
    R.text('点一下继续', W / 2, H / 2 + 24, 14, '#b8c4e0');
  }
  return R.end();
}

// 瞄准线：沿模拟出的折线每 18px 一个点，越远越淡；终点画一个空心圈 = 泡泡会停的那一格
function aimLine(R, S, grid, sim, r) {
  if (!sim) return;
  var pts = sim.pts, acc = 9, total = 0;
  for (var i = 0; i + 1 < pts.length; i++) {
    var a = pts[i], b = pts[i + 1], dx = b[0] - a[0], dy = b[1] - a[1], l = Math.sqrt(dx * dx + dy * dy);
    while (acc < l) {
      var k = acc / l;
      R.setAlpha(Math.max(0.25, 1 - total / 700));
      R.circle(a[0] + dx * k, a[1] + dy * k, 3, '#ffffff');
      acc += 18; total += 18;
    }
    acc -= l;
  }
  R.setAlpha(1);
  if (sim.cell) R.strokeCircle(grid.x(sim.cell[0], sim.cell[1]), grid.y(sim.cell[0]), r - 2, 2, '#ffffff');
}

function shooter(R, S, g, r, nb) {
  var a = S.aim.ang, cx = g.SX, cy = g.SY;
  R.line([[cx + Math.cos(a) * (r + 4), cy - Math.sin(a) * (r + 4)], [cx + Math.cos(a) * (r + 26), cy - Math.sin(a) * (r + 26)]], 6, 'rgba(255,255,255,0.55)', true);
  if (S.scene === 'play') sprite(R, S.cur, cx, cy, r, 1, 1);           // 发出去的那颗已经换成下一颗了：炮口一直有球
  var nx = cx - 74, ny = cy + 22;
  R.rrect(nb.x, nb.y, nb.w, nb.h, 10, 'rgba(255,255,255,0.08)');
  sprite(R, S.nxt, nx, ny - 4, r, 0.72, 1);
  R.text('换一个', nx, ny + r * 0.72 + 4, 11, '#b8c4e0');
  // 下压倒计时：还剩几发没消除就压下来
  var n = S.L ? S.L.push : 0;
  R.text('下压', cx + 66, cy + 4, 12, '#b8c4e0', 'right');
  for (var i = 0; i < n; i++) R.circle(cx + 76 + i * 11, cy + 4, 4, i < S.pushLeft ? '#ffd54f' : 'rgba(255,255,255,0.18)');
}

function hud(R, S, g) {
  var y = B.hudTop + 14;
  R.text('第 ' + S.level + ' 关', 14, y, 17, '#ffffff', 'left', 'bold');
  R.text(S.L ? S.L.name : '', 14, y + 22, 12, '#9fb0d6', 'left');
  R.text(String(S.score), W / 2, y + 6, 28, '#ffd54f', 'center', 'bold');
  R.text('最高 ' + S.best, W - 14, y + 6, 13, '#9fb0d6', 'right');
  if (S.msg && S.clock - S.msgAt < 1.1) {
    var k = (S.clock - S.msgAt) / 1.1;
    R.setAlpha(1 - k * k);
    R.text(S.msg, W / 2, g.DY - 24 - k * 12, 16, '#ffffff', 'center', 'bold', ['#1b2a49', 2]);
    R.setAlpha(1);
  }
}

function panel(R, h) {
  var pw = Math.min(320, W - 36), top = H / 2 - h / 2;
  R.rrect(W / 2 - pw / 2 - 3, top - 3, pw + 6, h + 6, 21, 'rgba(0,0,0,0.35)');
  R.rrect(W / 2 - pw / 2, top, pw, h, 18, '#ffffff');
  return { top: top, pw: pw };
}
function over(R, S) {
  var p = panel(R, 330), cx = W / 2, t = p.top, bw = p.pw - 56;
  R.text(S.newBest ? '新纪录！' : '泡泡压到底线了', cx, t + 40, 24, S.newBest ? '#ff8f00' : '#1b2a49', 'center', 'bold');
  R.text('第 ' + S.level + ' 关 · ' + S.L.name + ' · 发射 ' + S.shots + ' 次', cx, t + 72, 13, '#7a86a6');
  R.text(String(S.score), cx, t + 112, 42, '#1b2a49', 'center', 'bold');
  var y = t + 166;
  S.buttons.revive = null;
  if (!S.revived) { S.buttons.revive = R.button(S.reviving ? '正在加载广告…' : reviveText(S), cx, y, bw, 42, '#ff7043', '#ffffff'); y += 52; }
  S.buttons.retry = R.button('重新挑战本关', cx, y, bw, 40, '#1b2a49', '#ffffff'); y += 50;
  S.buttons.share = R.button('炫耀一下', cx - bw / 4 - 4, y, bw / 2 - 8, 36, '#eef1f7', '#1b2a49');
  S.buttons.home = R.button('回主页', cx + bw / 4 + 4, y, bw / 2 - 8, 36, '#eef1f7', '#1b2a49');
}
function reviveText(S) { return S.reviveLabel ? S.reviveLabel() : '复活'; }
function clear(R, S) {
  var p = panel(R, 300), cx = W / 2, t = p.top, bw = p.pw - 56;
  R.text('第 ' + S.level + ' 关通关！', cx, t + 40, 24, '#1b2a49', 'center', 'bold');
  R.text(S.L.name + ' · 发射 ' + S.shots + ' 次 · 奖励 +' + S.bonus, cx, t + 72, 13, '#7a86a6');
  for (var i = 0; i < 3; i++) R.text(i < S.stars ? '★' : '☆', cx + (i - 1) * 46, t + 116, 40, i < S.stars ? '#ffb300' : '#c9d0e0');
  R.text('总分 ' + S.score + '  ·  最高 ' + S.best, cx, t + 160, 15, '#1b2a49');
  S.buttons.next = R.button('下一关', cx, t + 210, bw, 42, '#42a5f5', '#ffffff');
  S.buttons.share = R.button('炫耀一下', cx, t + 262, bw, 36, '#eef1f7', '#1b2a49');
}

function menu(R, S, r) {
  R.layer('menu-deco', 1, function () {
    R.gradRect(0, 0, W, 260, [[0, '#13203a'], [1, '#1b2a49']], true);
    var rows = [[0, 1, 2, 3, 4, 5, 0, 1], [3, 4, 5, 0, 1, 2, 3]];
    rows.forEach(function (row, i) { row.forEach(function (k, c) { bubble(R, W / 2 - (row.length - 1) * r + c * 2 * r, H - 150 + i * r * 1.75, r, k); }); });
  });
  var y0 = B.hudTop + 40, cx = W / 2, bw = Math.min(280, W - 60);
  R.text('泡泡龙', cx, y0 + 20, 44, '#ffffff', 'center', 'bold', ['#42a5f5', 3]);
  R.text('瞄准 · 反弹 · 三连消除', cx, y0 + 66, 14, '#9fb0d6');
  R.text('最高分 ' + S.best + '   ·   已解锁第 ' + S.unlocked + ' 关', cx, y0 + 100, 13, '#c5d0ea');
  var y = y0 + 160;
  S.buttons.invite = null; S.buttons.cont = null;
  if (S.invite) {
    R.rrect(cx - bw / 2, y - 26, bw, 52, 14, 'rgba(255,213,79,0.16)');
    R.text('好友邀请你挑战第 ' + S.invite + ' 关', cx, y - 6, 14, '#ffd54f', 'center', 'bold');
    R.text(S.invite <= S.unlocked ? '点这里接受挑战' : '先打到第 ' + S.invite + ' 关才能接受', cx, y + 13, 11, '#e8dca8');
    if (S.invite <= S.unlocked) S.buttons.invite = { x: cx - bw / 2, y: y - 26, w: bw, h: 52 };
    y += 70;
  }
  if (S.unlocked > 1) { S.buttons.cont = R.button('继续第 ' + S.unlocked + ' 关', cx, y, bw, 50, '#42a5f5', '#ffffff'); y += 66; }
  S.buttons.fresh = R.button(S.unlocked > 1 ? '从第 1 关开始' : '开始游戏', cx, y, bw, S.unlocked > 1 ? 44 : 50, S.unlocked > 1 ? 'rgba(255,255,255,0.14)' : '#42a5f5', '#ffffff'); y += 60;
  S.buttons.share = R.button('转发给好友', cx, y, bw * 0.6, 38, 'rgba(255,255,255,0.1)', '#c5d0ea');
  R.text('按住瞄准、松手发射，点「换一个」换球', cx, H - 62, 12, '#7f8fb5');
}

module.exports = { setup: setup, draw: draw, BASE: BASE };
