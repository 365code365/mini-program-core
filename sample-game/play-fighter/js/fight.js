// 一场对战：两名角色 + 飞行道具 + 判定 + 回合 / 计时 + 镜头。逻辑按固定 60 步 / 秒走（step），画面在 render.js。
// 打击感三件套都在这里：定格（hitstop：打中那一下双方停几帧）、镜头震动、火花；KO 时慢放 + 闪白。
var D = require('./data.js');
var Fighter = require('./fighter.js');
var AI = require('./ai.js');
var FX = require('./fx.js');

var STAGE_W = 1600, VH = 640;            // 场景图在世界里 1600 宽；镜头竖直方向看 640 个单位
var ROUND_FRAMES = 60 * 60;              // 一回合 60 秒

function Fight(opts) {
  // opts: { p1, p2, art, stage, diff, auto, sfx(name), onEnd(result) }
  this.o = opts;
  this.art = opts.art;
  this.stage = opts.stage;
  this.VW = VH * opts.W / opts.H;
  var d = D.DIFFS[opts.diff];
  this.p = [new Fighter(opts.p1, opts.art.chars[opts.p1], opts.auto ? new AI(D.DIFFS[2]) : null),
            new Fighter(opts.p2, opts.art.chars[opts.p2], new AI(d))];
  this.wins = [0, 0]; this.round = 0;
  this.fx = new FX(this);
  this.stats = { maxCombo: 0, frames: 0, perfect: 0 };
  this.cam = { x: STAGE_W / 2, zoom: 1, shake: 0, sx: 0, sy: 0 };
  this.nextRound();
}

Fight.prototype.nextRound = function () {
  this.round++;
  this.p[0].reset(STAGE_W / 2 - 210, 1);
  this.p[1].reset(STAGE_W / 2 + 210, -1);
  this.proj = []; this.hitstop = 0; this.freeze = 0; this.superBy = null;
  this.timer = ROUND_FRAMES; this.phase = 'intro'; this.pt = 0;
  this.slow = 1; this.flash = 0; this.dark = 0; this.announce = null; this.callouts = [];
  this.comboShow = [{ n: 0, t: 0 }, { n: 0, t: 0 }];
  this.cam.x = STAGE_W / 2; this.cam.zoom = 1;
  this.fx.clear();
};

// ───────── 给 fighter.js 用的回调 ─────────
Fight.prototype.sfx = function (n) { this.o.sfx(n); };
Fight.prototype.footstep = function (f) { this.fx.dust(f.x - f.facing * 20, 0.25); this.sfx('step'); };
Fight.prototype.land = function (f, hard) { this.fx.dust(f.x, hard ? 1.6 : 0.8); if (hard) { this.cam.shake = Math.max(this.cam.shake, 4); this.sfx('thud'); } };
Fight.prototype.hasProj = function (f) { for (var i = 0; i < this.proj.length; i++) if (this.proj[i].owner === f && !this.proj[i].big) return true; return false; };
Fight.prototype.spawn = function (f, m) {
  var pj = m.proj;
  this.proj.push({ owner: f, m: m, x: f.x + f.facing * 95, y: pj.y, vx: pj.v * f.facing, r: pj.r, hits: pj.hits, every: pj.every || 0,
    cd: 0, life: pj.life, big: !!pj.big, t: 0, kind: f.id, dead: false });
  this.sfx(m.sfx);
  if (pj.big) this.cam.shake = 10;
};
Fight.prototype.burst = function (f, m) { this.fx.rise(f); this.sfx(m.sfx); };
Fight.prototype.superFlash = function (f) {
  this.freeze = 46; this.superBy = f; this.flash = 0.6; this.sfx('superStart');
  this.fx.superAura(f);
};
Fight.prototype.shout = function (f, m) { if (m.name) this.callouts.push({ f: f, text: m.name, t: 0, big: m.key === 'su' }); };

// ───────── 一逻辑帧 ─────────
Fight.prototype.step = function (inp1) {
  var p = this.p;
  this.pt++;
  this.fx.step();
  if (this.flash > 0) this.flash = Math.max(0, this.flash - 0.06);
  this.callouts = this.callouts.filter(function (c) { return ++c.t < 60; });
  for (var ci = 0; ci < 2; ci++) if (this.comboShow[ci].t > 0) this.comboShow[ci].t--;
  // 超必杀的定格：整个世界停住，只放演出
  if (this.freeze > 0) { this.freeze--; this.dark = Math.min(1, this.dark + 0.12); return; }
  this.dark = Math.max(0, this.dark - 0.06);
  if (this.hitstop > 0) { this.hitstop--; this.cam.shake *= 0.9; return; }

  var fighting = this.phase === 'fight';
  var none = { x: 0, y: 0, press: {} };
  var i0 = fighting ? (p[0].ai ? p[0].ai.think(p[0], p[1], this) : inp1) : none;
  var i1 = fighting && !this.o.cpuOff ? p[1].ai.think(p[1], p[0], this) : none;
  p[0].update(i0, p[1], this);
  p[1].update(i1, p[0], this);
  this.bodies();
  this.hits();
  this.projectiles();
  this.phaseStep();
  for (var k = 0; k < 2; k++) {
    var f = p[k];
    f.shown += (f.hp - f.shown) * 0.35;
    if (f.state !== 'hit' && f.state !== 'airhit') f.trail += (f.hp - f.trail) * 0.06;
    for (var gi = f.ghosts.length - 1; gi >= 0; gi--) { f.ghosts[gi].life -= 0.08; if (f.ghosts[gi].life <= 0) f.ghosts.splice(gi, 1); }
  }
  this.camera();
};

// 两人身体不重叠、不出场景、不出镜头（镜头能看的宽度有限，两人最远只能拉开这么多）
Fight.prototype.bodies = function () {
  var a = this.p[0], b = this.p[1], minD = 74;
  var dx = b.x - a.x;
  if (Math.abs(dx) < minD && Math.abs(a.y - b.y) < 150 && a.state !== 'down' && b.state !== 'down') {
    var push = (minD - Math.abs(dx)) / 2, s = dx >= 0 ? 1 : -1;
    if (dx === 0) s = a.facing;
    a.x -= push * s; b.x += push * s;
  }
  var lo = 70, hi = STAGE_W - 70, span = this.VW - 110;
  [a, b].forEach(function (f) { f.x = Math.max(lo, Math.min(hi, f.x)); });
  if (Math.abs(b.x - a.x) > span) {
    var mid = (a.x + b.x) / 2, h = span / 2, sgn = b.x > a.x ? 1 : -1;
    a.x = mid - h * sgn; b.x = mid + h * sgn;
  }
};

function overlap(a, b) { return a && b && a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]; }

Fight.prototype.hits = function () {
  for (var i = 0; i < 2; i++) {
    var a = this.p[i], d = this.p[1 - i], hb = a.hitbox();
    if (!hb || a.connected) continue;
    var hu = d.hurt();
    if (!overlap(hb, hu)) continue;
    var cx = (Math.max(hb[0], hu[0]) + Math.min(hb[2], hu[2])) / 2, cy = (Math.max(hb[1], hu[1]) + Math.min(hb[3], hu[3])) / 2;
    if (a.facing > 0) cx = Math.min(hb[2], hu[2]) - 8; else cx = Math.max(hb[0], hu[0]) + 8;
    this.resolve(a, d, a.move, cx, cy, !!a.move.kd || !!a.move.launch);
    a.connected = true;
  }
};

Fight.prototype.canBlock = function (d, m) {
  var s = d.state;
  if (!(s === 'idle' || s === 'walk' || s === 'crouch' || s === 'block' || s === 'land')) return false;
  if (!d.backHeld || d.airborne()) return false;
  if (m.lv === 'low') return d.downHeld;
  if (m.lv === 'high') return !d.downHeld;
  return true;
};

Fight.prototype.resolve = function (a, d, m, x, y, kd) {
  var side = this.p.indexOf(a);
  if (this.canBlock(d, m)) {
    d.crouching = d.downHeld;
    d.takeBlock(m, m.chip || 0);
    if (Math.abs(d.x - (side ? 70 : STAGE_W - 70)) < 4) a.x -= a.facing * m.push * 0.5;    // 对手贴墙：反推自己
    this.hitstop = Math.max(4, Math.round(m.stop * 0.6));
    this.fx.guard(x, y, a.facing);
    a.gain(m.meter * 0.5); d.gain(m.meter * 0.4);
    this.sfx('block');
    return;
  }
  var juggling = d.state === 'hit' || d.state === 'airhit';
  a.combo = juggling ? a.combo + 1 : 1;
  var dmg = Math.round(m.dmg * Math.max(0.35, 1 - 0.12 * (a.combo - 1)));
  d.takeHit(m, dmg, a, kd);
  if (d.x <= 72 || d.x >= STAGE_W - 72) a.x -= a.facing * m.push * 0.4;
  if (a.combo >= 2) { this.comboShow[side] = { n: a.combo, t: 70 }; if (side === 0) this.stats.maxCombo = Math.max(this.stats.maxCombo, a.combo); }
  this.hitstop = m.stop;
  this.cam.shake = Math.max(this.cam.shake, m.shake);
  var col = m.fire || m.proj ? a.def.spark : null;
  this.fx.spark(x, y, m.spark, col, a.facing);
  a.gain(m.meter); d.gain(m.meter * 0.6);
  // 打击声按招式轻重；必杀 / 飞行道具命中在重击上再叠一层自己的属性音（火 / 冰电），超必杀每段都响
  this.sfx(m.sfx === 'light' ? 'hitL' : 'hitH', m.proj && m.proj.big ? { vol: 0.8 } : null);
  if (m.fire) this.sfx(a.id === 'yan' ? 'fire' : 'ice', { vol: 0.5 });
  if (d.hp <= 0) this.ko(d);
};

Fight.prototype.projectiles = function () {
  var ps = this.proj;
  for (var i = 0; i < ps.length; i++) {
    var q = ps[i];
    if (q.dead) continue;
    q.t++; q.x += q.vx; q.life--; if (q.cd > 0) q.cd--;
    this.fx.trail(q);
    if (q.life <= 0 || q.x < -200 || q.x > STAGE_W + 200) { q.dead = true; continue; }
    // 两个波相撞：各掉一段，掉光的消失（超必杀的大波能吃掉普通波）
    for (var j = i + 1; j < ps.length; j++) {
      var o = ps[j];
      if (o.dead || o.owner === q.owner || Math.abs(o.x - q.x) > q.r + o.r || Math.abs(o.y - q.y) > q.r + o.r) continue;
      q.hits--; o.hits--;
      this.fx.clash((q.x + o.x) / 2, (q.y + o.y) / 2);
      this.sfx('clash');
      if (q.hits <= 0) q.dead = true; if (o.hits <= 0) o.dead = true;
    }
    if (q.dead || q.cd > 0) continue;
    var d = this.p[0] === q.owner ? this.p[1] : this.p[0], hu = d.hurt();
    var box = [q.x - q.r * 0.8, q.y - q.r * 0.8, q.x + q.r * 0.8, q.y + q.r * 0.8];
    if (!overlap(box, hu)) continue;
    q.hits--; q.cd = q.every;
    var last = q.hits <= 0;
    q.owner.connected = true;
    this.resolve(q.owner, d, q.m, q.x + (q.vx > 0 ? q.r * 0.4 : -q.r * 0.4), q.y, q.big && last);
    if (q.big) this.cam.shake = Math.max(this.cam.shake, 8);
    if (last) { q.dead = true; this.fx.pop(q); }
  }
  this.proj = ps.filter(function (q) { return !q.dead; });
};

// ───────── 回合流程 ─────────
Fight.prototype.ko = function (d) {
  if (this.phase !== 'fight') return;
  this.phase = 'ko'; this.pt = 0; this.loser = d;
  this.slow = 0.25; this.flash = 1; this.hitstop = 18;
  this.cam.shake = 14;
  this.sfx('ko');
};
Fight.prototype.phaseStep = function () {
  var p = this.p, t = this.pt;
  if (this.phase === 'intro') {
    // 第三回合起叫「Final round」（与街机一致）；解说比字晚几帧出来，字先砸下来、声音跟上
    if (t === 1) this.announce = { text: '第 ' + this.round + ' 回合', sub: 'ROUND ' + this.round, t: 0 };
    if (t === 6) this.sfx(this.round >= 3 ? 'v_round3' : 'v_round' + this.round);
    if (t === 64) { this.announce = { text: '开战', sub: 'FIGHT!', t: 0, hot: true }; this.sfx('fight'); }
    if (t >= 100) { this.phase = 'fight'; this.pt = 0; this.announce = null; p[0].set('idle'); p[1].set('idle'); }
  } else if (this.phase === 'fight') {
    this.timer--; this.stats.frames++;
    if (this.timer <= 0) {
      this.phase = 'timeover'; this.pt = 0;
      this.loser = p[0].hp === p[1].hp ? null : p[0].hp < p[1].hp ? p[0] : p[1];
      this.announce = { text: '时间到', sub: 'TIME OVER', t: 0 }; this.sfx('v_timeover');
    }
  } else if (this.phase === 'ko' || this.phase === 'timeover') {
    if (this.phase === 'ko') {
      if (t > 50) this.slow = Math.min(1, this.slow + 0.03);
      if (t === 20) { this.announce = { text: 'K.O.', sub: '', t: 0, ko: true }; this.sfx('v_ko'); }
    }
    var winner = this.loser ? p[1 - p.indexOf(this.loser)] : null;
    if (t === 130 && winner) {
      winner.set('win'); winner.move = null;
      if (this.loser.state !== 'down' && this.loser.state !== 'airhit') this.loser.set('lose');
      this.announce = { text: winner === p[0] ? '你赢了' : '你输了', sub: winner.def.full + ' 胜', t: 0 };
      var perfect = winner.hp >= D.HP;
      if (perfect) { this.announce.text = '完美'; this.announce.sub = 'PERFECT'; if (winner === p[0]) this.stats.perfect++; }
      this.wins[p.indexOf(winner)]++;
      this.sfx(perfect ? 'v_perfect' : winner === p[0] ? 'win' : 'lose');
    }
    if (t === 130 && !winner) { this.announce = { text: '平局', sub: 'DRAW', t: 0 }; this.wins[0]++; this.wins[1]++; this.sfx('v_draw'); }
    if (t >= 240) {
      if (this.wins[0] >= 2 || this.wins[1] >= 2) {
        this.phase = 'end';
        var r = this.wins[0] > this.wins[1] ? 'win' : this.wins[0] < this.wins[1] ? 'lose' : 'draw';
        this.o.onEnd({ result: r, wins: this.wins.slice(), stats: this.stats });
      } else this.nextRound();
    }
  }
  if (this.announce) this.announce.t++;
};

Fight.prototype.camera = function () {
  var c = this.cam, a = this.p[0], b = this.p[1];
  var tx = Math.max(this.VW / 2, Math.min(STAGE_W - this.VW / 2, (a.x + b.x) / 2));
  c.x += (tx - c.x) * 0.18;
  var tz = this.phase === 'ko' && this.pt < 90 ? 1.1 : 1;
  c.zoom += (tz - c.zoom) * 0.08;
  c.shake *= 0.86;
  c.sx = (Math.random() - 0.5) * c.shake * 2; c.sy = (Math.random() - 0.5) * c.shake * 1.4;
};

Fight.STAGE_W = STAGE_W; Fight.VH = VH;
module.exports = Fight;
