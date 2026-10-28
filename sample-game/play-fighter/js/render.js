// 画一场对战（HUD 在 hud.js）：场景 → 超必杀压暗 → 倒影 → 影子 → 角色（出招的那个画在前面）→ 飞行道具 → 特效 → 氛围 → 闪白。
// 世界坐标 → 屏幕：镜头中心在屏幕中线，脚底（y = 0）在屏幕 H - (900 - floor) × s；y 向上为正，所以画的时候是 -y。
// 角色是单张关键帧（AI 画的），动起来的「肉」靠这里的程序化处理：待机呼吸、出招前的蓄力后缩、受击抖动与闪白、残影、浮空翻转。
var NOW = typeof performance !== 'undefined' && performance.now ? function () { return performance.now(); } : Date.now;   // 计时用（Date.now 只到毫秒）
var Fight = require('./fight.js');

function worldBegin(g, fight, W, H) {
  var c = fight.cam, s = H / Fight.VH * c.zoom, floorY = H - (900 - fight.stage.floor) * (H / Fight.VH);
  g.save();
  g.translate(W / 2 + c.sx, floorY + c.sy - (c.zoom - 1) * 120);
  g.scale(s, s);
  g.translate(-c.x, 0);
  return s;
}

function drawStage(g, fight, tex) {
  var st = fight.stage, fl = st.floor;
  if (tex) g.image(tex, 0, 0, tex.w, tex.h, 0, -fl, Fight.STAGE_W, 900);
  else {
    g.gradRect(0, -fl, Fight.STAGE_W, fl, [[0, '#0b1030'], [1, '#3a2050']], true);
    g.gradRect(0, 0, Fight.STAGE_W, 110, [[0, '#2a2234'], [1, '#100c18']], true);
  }
  // 地面往镜头这边压暗一点（场景图是平光，角色站上去才有「地面」的纵深）
  g.gradRect(0, 30, Fight.STAGE_W, 80, [[0, 'rgba(0,0,0,0)'], [1, 'rgba(0,0,0,0.45)']], true);
}

// 画一个角色的一张图；mode：null 正常、'mirror' 地面倒影、'ghost' 残影、'flash' 受击闪白
function sprite(g, f, art, pose, x, y, facing, tint, extra) {
  if (!art || !art.poses[pose]) {
    // 没有美术：画个剪影胶囊，至少能玩
    var c = tint || [f.def.color === '#ff6a2a' ? 0.9 : 0.3, 0.4, f.def.color === '#ff6a2a' ? 0.2 : 0.9, 1];
    g.rrect(x - 30, -y - 240, 60, 240, 26, c);
    return;
  }
  var p = art.poses[pose], k = art.k, w = p[2] * k, h = p[3] * k;
  var flip = facing < 0, ax = (flip ? p[2] - p[4] : p[4]) * k, ay = p[5] * k;
  g.save();
  g.translate(x, -y);
  if (extra) {
    if (extra.lean) g.rotate(extra.lean);
    if (extra.rot) { g.translate(0, -h * 0.5); g.rotate(extra.rot); g.translate(0, h * 0.5); }
    if (extra.sx || extra.sy) g.scale(extra.sx || 1, extra.sy || 1);
  }
  g.image(art.tex, p[0], p[1], p[2], p[3], -ax, -ay, w, h, flip, tint);
  g.restore();
}

function poseExtra(f, fight) {
  var e = { sx: 1, sy: 1, rot: 0, dx: 0, dy: 0 };
  var s = f.state;
  if (s === 'idle' || s === 'intro') { e.sy = 1 + Math.sin(f.clock * 0.09) * 0.012; e.sx = 1 - Math.sin(f.clock * 0.09) * 0.006; }
  if (s === 'walk' || s === 'dash') {
    // 身体随步伐起伏：两腿分开那两帧（1、3）重心最低，并脚那两帧（2、4）最高；帧之间按步内进度插值，不是一跳一跳的
    var ph = (((f.stride % 136) + 136) % 136) / 136 * Math.PI * 4;
    e.dy = -Math.cos(ph) * 4 - 2;
    e.lean = f.walkDir < 0 ? -f.facing * 0.03 : f.facing * 0.025;  // 前进微微前倾、后退微微后仰（绕脚底转，脚不滑）
  }
  if (s === 'jumpsq' || s === 'land') { e.sy = 0.96; e.sx = 1.03; }
  if (s === 'attack' && f.move && !f.move.air) {
    var m = f.move;
    if (f.t < m.s) e.dx = -f.facing * 10 * (f.t / m.s);              // 蓄力：先往后缩
    else if (f.t < m.s + m.a) { e.sx = 1.04; e.dx = f.facing * 4; }   // 打出去的那几帧略微拉长
  }
  if (s === 'airhit') e.rot = -f.facing * Math.min(1.25, f.t * 0.07);
  if (s === 'hit' && fight.hitstop > 0) e.dx = (f.clock % 2 ? 3 : -3);
  if (s === 'getup') e.sy = 0.8 + Math.min(1, f.t / 16) * 0.2;
  return e;
}

function drawFighter(g, f, art, fight, mode) {
  var pose = f.pose(), e = poseExtra(f, fight), x = f.x + e.dx, y = f.y + (e.dy || 0);
  if (mode === 'mirror') {
    g.save(); g.scale(1, -1);
    sprite(g, f, art, pose, x, y, f.facing, [0.55, 0.55, 0.75, fight.stage.reflect], e);
    g.restore();
    return;
  }
  // 残影：叠加混合、角色主色
  if (f.ghosts.length) {
    g.setBlend('add');
    var gc = f.def.glow;
    for (var i = 0; i < f.ghosts.length; i++) {
      var gh = f.ghosts[i];
      sprite(g, f, art, gh.pose, gh.x, gh.y, gh.facing, [gc[0], gc[1], gc[2], gh.life * 0.45], null);
    }
    g.setBlend();
  }
  sprite(g, f, art, pose, x, y, f.facing, null, e);
  if (f.flash > 0 || (fight.superBy === f && fight.freeze > 0)) {
    g.setBlend('add');
    var a = f.flash > 0 ? 0.42 : 0.25 + 0.2 * Math.sin(fight.pt * 0.6), c = f.flash > 0 ? [1, 1, 1] : f.def.glow;
    sprite(g, f, art, pose, x, y, f.facing, [c[0], c[1], c[2], a], e);
    g.setBlend();
  }
}

function drawShadow(g, f) {
  var k = Math.max(0.35, 1 - f.y / 320), w = (f.state === 'down' ? 120 : 64) * k;
  g.ellipse(f.x, 2, w, 12 * k, 0, [0, 0, 0, 0.42 * k]);
}

function render(g, fight, W, H, opts) {
  var art = fight.art, P = opts && opts.perf, t0 = NOW();
  worldBegin(g, fight, W, H);
  drawStage(g, fight, art.stages[fight.stage.id]);
  if (fight.dark > 0) g.rect(fight.cam.x - 2000, -1200, 4000, 2400, [0.02, 0.0, 0.06, fight.dark * 0.72]);
  var order = fight.p.slice();
  // 出招 / 超必杀的那个人画在前面
  if (order[0].state === 'attack' && order[1].state !== 'attack' || fight.superBy === order[0] && fight.freeze > 0) order.reverse();
  if (fight.stage.reflect > 0.15) order.forEach(function (f) { drawFighter(g, f, art.chars[f.id], fight, 'mirror'); });
  order.forEach(drawShadow.bind(null, g));
  for (var j = 0; j < fight.proj.length; j++) {
    var q = fight.proj[j];
    g.ellipse(q.x, 2, q.r * 0.9, 9, 0, [0, 0, 0, 0.3]);
  }
  order.forEach(function (f) { drawFighter(g, f, art.chars[f.id], fight, null); });
  var t1 = NOW();
  for (var i = 0; i < fight.proj.length; i++) fight.fx.drawProj(g, fight.proj[i]);
  fight.fx.draw(g);
  var t2 = NOW();
  if (opts && opts.boxes) drawBoxes(g, fight);
  g.restore();
  var cdx = (fight.cam.x - (fight.lastCamX || fight.cam.x)) * H / Fight.VH;
  fight.lastCamX = fight.cam.x;
  fight.fx.ambient(g, fight.stage.fx, W, H, cdx);
  if (P) { P.world = t1 - t0; P.fx = t2 - t1; P.amb = NOW() - t2; }
  // 画面四角压暗（暗角）：上下两条渐变即可，省顶点
  g.gradRect(0, 0, W, H * 0.22, [[0, 'rgba(0,0,0,0.45)'], [1, 'rgba(0,0,0,0)']], true);
  if (fight.flash > 0) {
    g.setBlend('add'); g.rect(0, 0, W, H, [1, 1, 1, fight.flash * 0.5]); g.setBlend();
  }
}

// 调试：?boxes=1 画受击框（绿）与攻击框（红）
function drawBoxes(g, fight) {
  fight.p.forEach(function (f) {
    var h = f.hurt(), b = f.hitbox();
    if (h) g.strokeRect(h[0], -h[3], h[2] - h[0], h[3] - h[1], 2, 'rgba(0,255,0,0.8)');
    if (b) g.strokeRect(b[0], -b[3], b[2] - b[0], b[3] - b[1], 2, 'rgba(255,0,0,0.9)');
  });
}

module.exports = { render: render, sprite: sprite };
