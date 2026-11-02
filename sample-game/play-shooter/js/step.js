// 每帧推进：开火、出场表、敌机与首领的走位和弹幕、子弹与碰撞、道具、粒子。只读写 world.js 的池子，不分配。
var B = require('./base.js');
var Wd = require('./world.js');
var S = Wd.S, W = B.W, H = B.H, TAU = Math.PI * 2;
var pb = Wd.pb, eb = Wd.eb, en = Wd.en, pu = Wd.pu, pt = Wd.pt, rings = Wd.rings, texts = Wd.texts, sched = Wd.sched;

function pshot(x, y, ang) {
  var b = Wd.take(pb); if (!b) return;
  b.x = x; b.y = y; b.vx = Math.sin(ang) * 760; b.vy = -Math.cos(ang) * 760;
}
function shoot() {
  var x = S.sx, y = S.sy - 24;
  if (S.power === 1) pshot(x, y, 0);
  else { pshot(x - 7, y, 0); pshot(x + 7, y, 0); }
  if (S.power >= 3) { pshot(x - 14, y + 8, -0.18); pshot(x + 14, y + 8, 0.18); }
  S.shots++;
  if (S.shots % 2 === 0) B.beep(1400, 0.03, 'square', 0.025, 0.5);
}

function enemies(dt) {
  for (var i = 0; i < en.length; i++) {
    var e = en[i]; if (!e.on) continue;
    e.t += dt; if (e.flash > -1) e.flash -= dt;
    if (e.type === 0) {
      e.y += e.vy * dt;
      if (e.mode === 0) e.x += e.vx * dt; else e.x = e.x0 + Math.sin(e.t * e.freq) * e.amp;
      if (S.wave >= 4 && !e.shot && e.y > 120 && e.y < S.sy - 160 && Math.random() < dt * 0.5) {
        e.shot = 1; Wd.fire(e.x, e.y + 10, Wd.aim(e.x, e.y), 170, 0);
      }
    } else if (e.type === 1) {
      if (e.mode === 0) {                                 // 缓入到悬停高度
        e.y += (e.ty - e.y) * Math.min(1, dt * 2.6);
        if (e.ty - e.y < 3) { e.mode = 1; e.t = 0; }
      } else if (e.mode === 1) {
        e.x = e.x0 + Math.sin(e.t * 1.3) * 36;
        e.fire -= dt;
        if (e.fire <= 0) {
          e.fire = 1.6 - Math.min(0.6, S.wave * 0.04);
          var a = Wd.aim(e.x, e.y);
          for (var k = -1; k <= 1; k++) Wd.fire(e.x, e.y + 14, a + k * 0.2, 160, 0);
        }
        if (e.t > e.stay) e.mode = 2;
      } else e.y += 160 * dt;                              // 待够了就俯冲离场
    } else {
      e.y += e.vy * dt; e.x = e.x0 + Math.sin(e.t * 0.7) * 20;
      e.fire -= dt;
      if (e.fire <= 0 && e.y > 30) {
        e.fire = 2.3;
        for (var j = 0; j < 10; j++) Wd.fire(e.x, e.y, e.t + j * TAU / 10, 110, 1);
      }
    }
    if (e.y > H + 60 || e.x < -80 || e.x > W + 80) { e.on = false; continue; }
    // 撞上战机：战机受伤，敌机吃 10 点
    var dx = e.x - S.sx, dy = e.y - S.sy, rr = e.r + 10;
    if (dx * dx + dy * dy < rr * rr) {
      if (S.shieldT > 0) Wd.damage(e, 99);
      else if (S.invT <= 0) { Wd.hurt(); Wd.damage(e, 10); }
    }
  }
}

function boss(dt) {
  var b = S.boss;
  if (S.bossPending && S.waveT > 2.2) {
    S.bossPending = false;
    b.on = true; b.tier = S.wave / 5; b.max = b.hp = 260 + (b.tier - 1) * 170;
    b.x = W / 2; b.y = -70; b.t = 0; b.mt = 0; b.pat = 0; b.patT = 0; b.fire = 1; b.ang = 0; b.burst = 0; b.flash = -1;
  }
  if (!b.on) return;
  b.t += dt; if (b.flash > -1) b.flash -= dt;
  if (b.y < 190) { b.y += 80 * dt; return; }     // 悬停在血条下面
  b.mt += dt * 0.7;
  b.x = W / 2 + Math.sin(b.mt) * (W / 2 - 82);
  b.patT += dt;
  if (b.patT > 5.5) { b.patT = 0; b.pat = (b.pat + 1) % 3; b.fire = 0.6; b.burst = 0; }
  b.fire -= dt * (b.hp < b.max / 2 ? 1.4 : 1);
  if (b.fire > 0) return;
  if (b.pat === 0) {                                      // 扇形 13 发
    b.fire = 0.9;
    for (var i = 0; i < 13; i++) Wd.fire(b.x, b.y + 30, Math.PI / 2 + (i - 6) * 0.13, 150, 1);
  } else if (b.pat === 1) {                               // 双臂螺旋
    b.fire = 0.08; b.ang += 0.27;
    Wd.fire(b.x, b.y + 10, b.ang, 130, 2); Wd.fire(b.x, b.y + 10, b.ang + Math.PI, 130, 2);
  } else {                                                // 五连发瞄准
    if (b.burst < 5) { b.fire = 0.11; b.burst++; Wd.fire(b.x - 24, b.y + 40, Wd.aim(b.x - 24, b.y + 40), 240, 0); Wd.fire(b.x + 24, b.y + 40, Wd.aim(b.x + 24, b.y + 40), 240, 0); }
    else { b.fire = 0.9; b.burst = 0; }
  }
}
function bossHit(x, y) {
  var b = S.boss; if (!b.on || b.y < -20) return false;
  var dx = (x - b.x) / 74, dy = (y - b.y) / 36;
  return dx * dx + dy * dy < 1;
}

function bullets(dt) {
  for (var i = 0; i < pb.length; i++) {
    var b = pb[i]; if (!b.on) continue;
    b.x += b.vx * dt; b.y += b.vy * dt;
    if (b.y < -30 || b.x < -20 || b.x > W + 20) { b.on = false; continue; }
    for (var j = 0; j < en.length; j++) {
      var e = en[j]; if (!e.on) continue;
      var dx = e.x - b.x, dy = e.y - b.y, rr = e.r + 5;
      if (dx * dx + dy * dy < rr * rr) { b.on = false; Wd.spark(b.x, b.y, Wd.PLASMA); Wd.damage(e, 1); break; }
    }
    if (b.on && bossHit(b.x, b.y)) { b.on = false; Wd.spark(b.x, b.y, Wd.FIRE); Wd.bossDamage(1); }
  }
  var shield = S.shieldT > 0, hr = shield ? 30 : 4;
  for (var k = 0; k < eb.length; k++) {
    var q = eb[k]; if (!q.on) continue;
    q.x += q.vx * dt; q.y += q.vy * dt;
    if (q.y < -30 || q.y > H + 30 || q.x < -30 || q.x > W + 30) { q.on = false; continue; }
    if (S.state !== 'play') continue;
    var ex = q.x - S.sx, ey = q.y - S.sy, r2 = q.r + hr;
    if (ex * ex + ey * ey < r2 * r2) {
      if (shield) { q.on = false; Wd.spark(q.x, q.y, Wd.PLASMA); }
      else if (S.invT <= 0) { q.on = false; Wd.hurt(); }
    }
  }
  // 首领本体撞战机
  if (S.boss.on && S.state === 'play' && S.invT <= 0 && S.shieldT <= 0 && bossHit(S.sx, S.sy - 8)) Wd.hurt();
}

function powerups(dt) {
  for (var i = 0; i < pu.length; i++) {
    var p = pu[i]; if (!p.on) continue;
    p.t += dt; p.x += p.vx * dt; p.y += p.vy * dt;
    if (p.x < 20 || p.x > W - 20) p.vx = -p.vx;
    if (p.y > H + 30) { p.on = false; continue; }
    var dx = p.x - S.sx, dy = p.y - S.sy;
    if (S.state === 'play' && dx * dx + dy * dy < 30 * 30) Wd.collect(p);
  }
}

function effects(dt) {
  for (var i = 0; i < pt.length; i++) {
    var p = pt[i]; if (!p.on) continue;
    p.life -= dt; if (p.life <= 0) { p.on = false; continue; }
    var d = Math.max(0, 1 - p.drag * dt);
    p.vx *= d; p.vy *= d; p.x += p.vx * dt; p.y += p.vy * dt;
  }
  for (var j = 0; j < rings.length; j++) { var g = rings[j]; if (g.on && (g.t += dt) >= g.max) g.on = false; }
  for (var k = 0; k < texts.length; k++) { var t = texts[k]; if (t.on) { t.t += dt; t.y -= 40 * dt; if (t.t > 0.9) t.on = false; } }
}

function step(dt) {
  S.clock += dt; S.scroll += dt;
  if (S.shake > 0) S.shake = Math.max(0, S.shake - dt * 30);
  if (S.flashT > 0) S.flashT -= dt;
  if (S.bannerT > 0) S.bannerT -= dt;
  effects(dt);
  if (S.state !== 'play') { bullets(dt); powerups(dt); return; }
  S.waveT += dt;
  if (S.invT > 0) S.invT -= dt;
  if (S.shieldT > 0) S.shieldT -= dt;
  if (S.powerT > 0 && (S.powerT -= dt) <= 0 && S.power > 1) { S.power--; S.powerT = S.power > 1 ? 14 : 0; }
  S.fireT -= dt;
  if (S.fireT <= 0) { S.fireT += 0.1; shoot(); }
  var pending = false;
  for (var i = 0; i < sched.length; i++) {
    var s = sched[i]; if (!s.on) continue;
    if (S.waveT >= s.t) { s.on = false; Wd.spawn(s.type, s.x, s.mode); } else pending = true;
  }
  enemies(dt); boss(dt); bullets(dt); powerups(dt);
  if (S.state !== 'play') return;
  // 这一波出完、打完：歇 1.2 秒进下一波
  if (!pending && !S.bossPending && !S.boss.on && !Wd.count(en)) {
    if (!S.nextWaveAt) { S.nextWaveAt = S.clock + 1.2; if (S.wave % 5 === 0) { S.banner = '首领击破！'; S.bannerT = 1.4; } }
    else if (S.clock >= S.nextWaveAt) Wd.startWave(S.wave + 1);
  }
}

module.exports = { step: step };
