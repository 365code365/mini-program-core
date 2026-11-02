// 战场模拟：固定步长（game.js 每帧按倍速走 1~3 步，每步 1/60 秒），敌人沿路走、塔冷却好了就索敌开火、
// 子弹飞到了结算伤害。敌人 / 子弹 / 特效都是对象池（死了标 alive=false，下次复用），模拟步里不 new 对象。
var D = require('data.js');
var Maps = require('maps.js');
var B = require('base.js');

function make(C) {
  var S = { state: 'idle', map: null, mapIdx: 0, gold: 0, lives: 0, wave: 0, queue: [], qi: 0, qt: 0, countdown: -1, bonusAt: 0,
    enemies: [], towers: [], shots: [], fx: [], kills: 0, leaked: 0, time: 0, score: 0, built: 0 };
  var zapStamp = 0, sfxAt = {};
  function ground(e) { return !e.air; }
  function unzapped(e) { return e.zap !== zapStamp; }

  function slot(pool) { for (var i = 0; i < pool.length; i++) if (!pool[i].alive) return pool[i]; var o = { alive: false }; pool.push(o); return o; }
  function sfx(name, f, dur, type, vol) {
    if (S.time - (sfxAt[name] || -1) < 0.07) return;       // 同一种音 70ms 内只响一次：几十座塔一起开火别糊成一片
    sfxAt[name] = S.time; B.beep(f, dur, type, vol);
  }
  function fx(kind, x, y, T, r, text) {
    var f = slot(S.fx); f.alive = true; f.kind = kind; f.x = x; f.y = y; f.t = 0; f.T = T; f.r = r || 0; f.text = text || '';
    if (!f.pts) f.pts = []; f.pts.length = 0;
    return f;
  }

  function reset(idx) {
    S.mapIdx = idx; S.map = Maps.list[idx];
    S.gold = D.START_GOLD; S.lives = D.LIVES; S.wave = 0; S.queue.length = 0; S.qi = 0; S.qt = 0; S.countdown = -1;
    S.enemies.forEach(function (e) { e.alive = false; }); S.shots.forEach(function (s) { s.alive = false; }); S.fx.forEach(function (f) { f.alive = false; });
    S.towers.length = 0; S.kills = 0; S.leaked = 0; S.time = 0; S.score = 0; S.built = 0;
    S.state = 'play';
  }

  // ───────── 波次 ─────────
  function canCall() { return S.state === 'play' && S.wave < D.WAVES.length && S.qi >= S.queue.length; }
  function callWave() {
    if (!canCall()) return false;
    // 提前出兵有奖励：倒计时还剩几秒给几×3 金
    var early = S.countdown > 0 ? Math.floor(S.countdown) * 3 : 0, bonus = S.wave ? 20 + S.wave * 4 : 0;
    S.gold += early + bonus;
    if (early + bonus) fx('gold', S.map.E.c + 0.5, S.map.E.r - 0.2, 1.2, 0, '+' + (early + bonus));
    S.queue.length = 0; S.qi = 0; S.qt = 0;
    var t = 0;
    D.WAVES[S.wave].forEach(function (g) { for (var k = 0; k < g[1]; k++) { S.queue.push({ type: g[0], at: t }); t += g[2]; } t += 1.2; });
    S.wave++; S.countdown = -1;
    sfx('wave', 392, 0.25, 'triangle', 0.12);
    return true;
  }
  function spawn(type) {
    var def = D.ENEMIES[type], e = slot(S.enemies), hp = Math.round(def.hp * D.hpScale(S.map, S.wave - 1));
    e.alive = true; e.type = type; e.def = def; e.hp = e.max = hp; e.d = 0; e.seg = 0; e.air = !!def.air;
    e.slowT = 0; e.slowF = 0; e.hitT = 0; e.zap = -1; e.wob = Math.random() * 6;
    place(e);
  }
  var tmp = { x: 0, y: 0 };
  function place(e) {
    if (e.air) {
      var f = S.map.fly, L = Math.sqrt((f.x1 - f.x0) * (f.x1 - f.x0) + (f.y1 - f.y0) * (f.y1 - f.y0)), t = Math.min(1, e.d / L);
      e.x = f.x0 + (f.x1 - f.x0) * t; e.y = f.y0 + (f.y1 - f.y0) * t; e.len = L;
    } else { e.seg = Maps.along(S.map, e.d, tmp, e.seg); e.x = tmp.x; e.y = tmp.y; e.len = S.map.len; }
  }

  // ───────── 塔 ─────────
  function towerAt(c, r) { for (var i = 0; i < S.towers.length; i++) if (S.towers[i].c === c && S.towers[i].r === r) return S.towers[i]; return null; }
  function build(c, r, type) {
    var def = D.TOWERS[type];
    if (!Maps.buildable(S.map, c, r) || towerAt(c, r) || S.gold < def.cost || S.state !== 'play') return null;
    S.gold -= def.cost; S.built++;
    var t = { c: c, r: r, x: c + 0.5, y: r + 0.5, type: type, def: def, lvl: 0, cd: 0, angle: -Math.PI / 2, spent: def.cost, flash: 0 };
    S.towers.push(t);
    sfx('build', 523, 0.1, 'square', 0.08);
    return t;
  }
  function upgradeCost(t) { var n = t.def.levels[t.lvl + 1]; return n ? n.upg : 0; }
  function upgrade(t) {
    var cost = upgradeCost(t);
    if (!cost || S.gold < cost) return false;
    S.gold -= cost; t.spent += cost; t.lvl++;
    fx('ring', t.x, t.y, 0.4, 0.6);
    sfx('upg', 784, 0.14, 'triangle', 0.1);
    return true;
  }
  function refund(t) { return Math.floor(t.spent * D.SELL); }
  function sell(t) {
    var i = S.towers.indexOf(t); if (i < 0) return 0;
    var g = refund(t); S.gold += g; S.towers.splice(i, 1);
    fx('gold', t.x, t.y - 0.3, 1, 0, '+' + g);
    sfx('sell', 330, 0.1, 'square', 0.07);
    return g;
  }

  // ───────── 伤害 ─────────
  function hurt(e, base, pierce) {
    if (!e.alive) return;
    e.hp -= C.dmg(base, e.def.armor, pierce);
    e.hitT = 0.08;
    if (e.hp > 0) return;
    e.alive = false; S.kills++; S.gold += e.def.gold;
    fx('puff', e.x, e.y, 0.35, e.def.r * 1.6);
    fx('gold', e.x, e.y - 0.3, 0.8, 0, '+' + e.def.gold);
    sfx('die', e.def.boss ? 150 : 260, e.def.boss ? 0.4 : 0.08, 'square', 0.08);
  }
  var splashDmg = 0, splashSlow = 0;
  function hitSplash(e) { hurt(e, splashDmg, false); }
  function hitFrost(e) { if (e.alive) { e.slowT = 1.5; e.slowF = Math.max(e.slowF, splashSlow); } hurt(e, splashDmg, false); }

  function fire(t, e) {
    var L = t.def.levels[t.lvl];
    t.angle = Math.atan2(e.y - t.y, e.x - t.x); t.flash = 0.08;
    if (t.type === 'tesla') {
      zapStamp++;
      var z = fx('zap', t.x, t.y, 0.14), cur = e;
      z.pts.push(t.x, t.y - 0.25);
      for (var k = 0; k < L.chain && cur; k++) {
        cur.zap = zapStamp; z.pts.push(cur.x, cur.y);
        hurt(cur, L.dmg * Math.pow(0.85, k), true);
        cur = C.pick(cur.x, cur.y, 1.6, unzapped);
      }
      sfx('zap', 1200, 0.07, 'sawtooth', 0.06);
      return;
    }
    var s = slot(S.shots);
    s.alive = true; s.kind = t.type; s.x = t.x; s.y = t.y; s.target = e; s.tx = e.x; s.ty = e.y; s.dmg = L.dmg; s.splash = L.splash || 0; s.slow = L.slow || 0;
    s.t = 0; s.sx = t.x; s.sy = t.y;
    s.speed = t.type === 'arrow' ? 10 : t.type === 'ice' ? 7 : 0;
    if (t.type === 'cannon') sfx('boom0', 140, 0.12, 'square', 0.1);
    else sfx(t.type, t.type === 'arrow' ? 880 : 1046, 0.04, 'triangle', 0.05);
  }
  function stepShot(s, dt) {
    if (s.kind === 'cannon') {                         // 炮弹：抛物线飞 0.45s 落到开火时目标所在处
      s.t += dt;
      var k = Math.min(1, s.t / 0.45);
      s.x = s.sx + (s.tx - s.sx) * k; s.y = s.sy + (s.ty - s.sy) * k; s.h = Math.sin(k * Math.PI) * 0.8;
      if (k < 1) return;
      s.alive = false; splashDmg = s.dmg;
      C.each(s.tx, s.ty, s.splash, ground, hitSplash);
      fx('boom', s.tx, s.ty, 0.3, s.splash);
      sfx('boom', 90, 0.18, 'sawtooth', 0.1);
      return;
    }
    if (s.target && s.target.alive) { s.tx = s.target.x; s.ty = s.target.y; }
    var dx = s.tx - s.x, dy = s.ty - s.y, d = Math.sqrt(dx * dx + dy * dy), mv = s.speed * dt;
    if (d > mv + 0.05) { s.x += dx / d * mv; s.y += dy / d * mv; return; }
    s.alive = false;
    if (s.kind === 'arrow') { if (s.target && s.target.alive) hurt(s.target, s.dmg, false); return; }
    splashDmg = s.dmg; splashSlow = s.slow;              // 冰：落点一圈都减速
    C.each(s.tx, s.ty, s.splash, null, hitFrost);
    fx('frost', s.tx, s.ty, 0.35, s.splash);
  }

  function step(dt) {
    if (S.state !== 'play') return;
    S.time += dt;
    // 出兵
    if (S.qi < S.queue.length) {
      S.qt += dt;
      while (S.qi < S.queue.length && S.queue[S.qi].at <= S.qt) spawn(S.queue[S.qi++].type);
      if (S.qi >= S.queue.length && S.wave < D.WAVES.length) S.countdown = 12;
    } else if (S.countdown > 0) { S.countdown -= dt; if (S.countdown <= 0) callWave(); }
    // 敌人走路
    var alive = 0;
    for (var i = 0; i < S.enemies.length; i++) {
      var e = S.enemies[i];
      if (!e.alive) continue;
      if (e.slowT > 0) { e.slowT -= dt; if (e.slowT <= 0) e.slowF = 0; }
      e.hitT = Math.max(0, e.hitT - dt);
      e.d += e.def.speed * (1 - e.slowF) * dt;
      place(e);
      if (e.d >= e.len) {
        e.alive = false; S.leaked++; S.lives -= e.def.boss ? 5 : 1;
        fx('leak', S.map.E.c + 0.5, S.map.E.r + 0.5, 0.4, 0.7);
        sfx('leak', 220, 0.2, 'sawtooth', 0.1); B.buzz();
        continue;
      }
      alive++;
    }
    C.rebuild(S.enemies);
    // 塔开火：冷却好了才索敌；没目标 0.1s 后再看（不每步都查）
    for (var k = 0; k < S.towers.length; k++) {
      var t = S.towers[k];
      t.flash = Math.max(0, t.flash - dt);
      t.cd -= dt;
      if (t.cd > 0) continue;
      var L = t.def.levels[t.lvl], e2 = C.pick(t.x, t.y, L.range, t.def.air ? null : ground);
      if (e2) { fire(t, e2); t.cd = 1 / L.rate; } else t.cd = 0.1;
    }
    for (var s = 0; s < S.shots.length; s++) if (S.shots[s].alive) stepShot(S.shots[s], dt);
    for (var f = 0; f < S.fx.length; f++) { var x = S.fx[f]; if (x.alive) { x.t += dt; if (x.t >= x.T) x.alive = false; } }
    // 结束判定
    if (S.lives <= 0) { S.lives = 0; end(false); }
    else if (S.wave >= D.WAVES.length && S.qi >= S.queue.length && !alive && !S.enemies.some(function (q) { return q.alive; })) end(true);
  }
  function end(win) {
    S.state = win ? 'win' : 'lose';
    S.score = S.kills * 10 + S.lives * 50 + (win ? S.gold + 500 : 0);
    S.stars = win ? (S.lives >= 18 ? 3 : S.lives >= 10 ? 2 : 1) : 0;
    if (win) [523, 659, 784, 1047].forEach(function (fq, i) { setTimeout(function () { B.beep(fq, 0.16, 'triangle', 0.12); }, i * 110); });
    else B.beep(160, 0.6, 'sawtooth', 0.12, 0.5);
  }
  function aliveCount() { var n = 0; for (var i = 0; i < S.enemies.length; i++) if (S.enemies[i].alive) n++; return n; }

  return { S: S, reset: reset, step: step, build: build, upgrade: upgrade, upgradeCost: upgradeCost, sell: sell, refund: refund,
    towerAt: towerAt, callWave: callWave, canCall: canCall, aliveCount: aliveCount };
}

module.exports = { make: make };
