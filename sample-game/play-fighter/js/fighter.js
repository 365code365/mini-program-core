// 一个格斗角色：状态机 + 物理 + 搓招识别。只管「自己」：打没打中、挡没挡住由 fight.js 判（它同时看两个人）。
// 坐标：x 是世界横坐标（脚底中心），y 是离地高度（≥ 0，向上为正）；facing 1 = 朝右，-1 = 朝左。
// 方向用小键盘记法（相对朝向）：6 前、4 后、2 下、8 上、3 前下、1 后下 …… 格斗游戏的通用写法，搓招表就照它写。
//
// 状态：idle walk crouch jumpsq jump land attack block hit airhit down getup dash back intro win lose
var D = require('./data.js');

var GRAV = 1.0;
var BUF = 7;            // 按键缓冲帧：收招最后几帧按下的键，收完马上出（不用卡点按）

function Fighter(id, art, ai) {
  this.id = id;
  this.def = D.CHARS[id];
  this.art = art;               // art.js 的 chars[id]（可能是 null）
  this.ai = ai;
  this.meter = 0;               // 能量跨回合保留（与 KOF 一致）
  this.moves = {};
  var self = this;
  Object.keys(D.NORMALS).forEach(function (k) { self.moves[k] = self.prep(k, D.NORMALS[k]); });
  ['s1', 's2', 'su'].forEach(function (k) { self.moves[k] = self.prep(k, self.def[k]); });
}

// 判定框前沿 = 该动作的图里身体伸得最远的地方（图集算出来的），这样「看着打到了」就是打到了
Fighter.prototype.prep = function (key, m) {
  var o = {}; for (var k in m) o[k] = m[k];
  o.key = key;
  var reach = 120;
  if (this.art && this.art.poses[m.pose]) { var p = this.art.poses[m.pose]; reach = (p[2] - p[4]) * this.art.k - 6; }
  o.reach = reach;
  return o;
};

Fighter.prototype.reset = function (x, facing) {
  this.x = x; this.y = 0; this.vx = 0; this.vy = 0; this.facing = facing;
  this.hp = D.HP; this.shown = D.HP; this.trail = D.HP;     // shown：血条动画用；trail：白色掉血残影
  this.state = 'intro'; this.t = 0; this.move = null; this.stun = 0; this.push = 0;
  this.hist = []; this.buf = {}; this.connected = false; this.airAtk = false; this.inv = 0;
  this.combo = 0; this.flash = 0; this.guard = false; this.crouching = false; this.ko = false;
  this.lastFwd = -99; this.lastBack = -99; this.fwdReleased = true; this.backReleased = true; this.clock = 0;
  this.ghosts = [];
  this.stride = 0;              // 走过的距离（世界单位），走路循环按它取帧：走快帧换得快，停下就停在当前帧
};
Fighter.prototype.set = function (s) { this.state = s; this.t = 0; };
Fighter.prototype.stocks = function () { return Math.floor(this.meter / D.STOCK); };
Fighter.prototype.gain = function (v) { this.meter = Math.min(D.METER_MAX, this.meter + v); };
Fighter.prototype.airborne = function () { return this.y > 0 || this.state === 'jump' || this.state === 'airhit'; };

// 搓招：在最近 n 帧的方向记录里按顺序找到 seq 的每一项（中间可以夹别的方向）
Fighter.prototype.motion = function (seq, n) {
  var h = this.hist, i = h.length - 1, j = seq.length - 1, lim = Math.max(0, h.length - n);
  for (; i >= lim && j >= 0; i--) if (h[i] === seq[j]) j--;
  return j < 0;
};
Fighter.prototype.pressed = function (k) { return (this.buf[k] || 0) > 0; };
Fighter.prototype.eat = function (k) { this.buf[k] = 0; };

Fighter.prototype.neutral = function () {
  var s = this.state;
  return s === 'idle' || s === 'walk' || s === 'crouch' || s === 'intro_done';
};

// 每逻辑帧调一次（定格 hitstop 时 fight.js 不调）。inp = controls.sample() 或 ai 的同格式输出
Fighter.prototype.update = function (inp, opp, world) {
  this.clock++;
  // 挡不挡得住由 fight.js 在被打那一刻判：记下这一帧有没有按住「后」与「下」
  this.backHeld = inp.x * this.facing < 0; this.downHeld = inp.y > 0;
  var fx = inp.x * this.facing, num = (inp.y > 0 ? 1 : inp.y < 0 ? 7 : 4) + fx + 1;
  this.hist.push(num); if (this.hist.length > 30) this.hist.shift();
  for (var k in this.buf) if (this.buf[k] > 0) this.buf[k]--;
  for (var p in inp.press) if (inp.press[p]) this.buf[p] = BUF;
  // 双击前 / 后：冲刺、后撤（记「松开过」再按下）
  var dash = null;
  if (fx > 0 && inp.y === 0) { if (this.fwdReleased && this.clock - this.lastFwd < 12) dash = 'dash'; if (this.fwdReleased) this.lastFwd = this.clock; this.fwdReleased = false; } else this.fwdReleased = true;
  if (fx < 0 && inp.y === 0) { if (this.backReleased && this.clock - this.lastBack < 12) dash = 'back'; if (this.backReleased) this.lastBack = this.clock; this.backReleased = false; } else this.backReleased = true;

  if (this.inv > 0) this.inv--;
  if (this.flash > 0) this.flash--;
  this.t++;
  var s = this.state, d = this.def;
  this.guard = false;

  if (s === 'intro' || s === 'win' || s === 'lose') { this.vx = 0; this.physics(world); return; }

  if (s === 'attack') { this.attackStep(inp, opp, world); this.physics(world); return; }
  if (s === 'hit' || s === 'block') {
    this.x += this.push * -this.facing; this.push *= 0.82;
    if (--this.stun <= 0) this.set(inp.y > 0 ? 'crouch' : 'idle');
    this.crouching = this.state === 'crouch' || (s === 'block' && this.crouching);
    this.physics(world); return;
  }
  if (s === 'airhit') {
    this.physics(world);
    if (this.y <= 0 && this.vy <= 0 && this.t > 2) { this.y = 0; this.vx = 0; this.set('down'); world.land(this, true); }
    return;
  }
  if (s === 'down') { if (!this.ko && this.t >= 40) { this.set('getup'); this.inv = 20; } return; }
  if (s === 'getup') { if (this.t >= 16) this.set('idle'); return; }
  if (s === 'jumpsq') { if (this.t >= 3) { this.set('jump'); this.vy = d.jumpV; this.vx = this.jumpDir * d.jumpX; this.airAtk = false; } return; }
  if (s === 'jump') {
    if (!this.airAtk && (this.pressed('lp') || this.pressed('lk') || this.pressed('hp') || this.pressed('hk'))) {
      this.eat('lp'); this.eat('lk'); this.eat('hp'); this.eat('hk');
      this.airAtk = true; this.start('jk', world); this.physics(world); return;
    }
    this.physics(world);
    if (this.y <= 0) { this.land(world); }
    return;
  }
  if (s === 'land') { if (this.t >= 4) this.set('idle'); this.face(opp); return; }
  if (s === 'dash' || s === 'back') {
    var dur = s === 'dash' ? 16 : 14;
    var sp = s === 'dash' ? d.dash : -d.dash * 0.85;
    this.x += sp * this.facing * (1 - this.t / dur);
    if (s === 'back') this.y = Math.max(0, Math.sin(this.t / dur * Math.PI) * 28);
    if (this.t % 3 === 0) this.ghost();
    if (s === 'back' && this.t < 8) this.inv = Math.max(this.inv, 1);
    if (this.t >= dur) { this.y = 0; this.set('idle'); }
    else if (s === 'dash' && this.t > 4 && this.tryAttack(inp, world)) return;
    return;
  }
  // ───────── 自由状态：idle / walk / crouch ─────────
  this.face(opp);
  if (this.tryAttack(inp, world)) return;
  if (dash) { this.set(dash); world.sfx('dash'); return; }
  if (inp.y < 0) { this.set('jumpsq'); this.jumpDir = fx; return; }
  this.guard = fx < 0;
  if (inp.y > 0) { if (s !== 'crouch') this.set('crouch'); this.crouching = true; return; }
  this.crouching = false;
  if (fx > 0) { this.x += d.walk * this.facing; if (s !== 'walk') this.set('walk'); this.walkDir = 1; this.stride += d.walk; this.step(world); }
  else if (fx < 0) { this.x -= d.back * this.facing; if (s !== 'walk') this.set('walk'); this.walkDir = -1; this.stride -= d.back; this.step(world); }
  else if (s !== 'idle') this.set('idle');
};

// 走路一个循环 4 帧，每帧走 STEP 个单位；落脚（第 0、2 帧开头）时扬一点尘、响一声脚步
var STEP = 34;
Fighter.prototype.walkFrame = function () { return ((Math.floor(this.stride / STEP) % 4) + 4) % 4; };
Fighter.prototype.step = function (world) {
  var f = this.walkFrame();
  if (f !== this.lastWalkF) { if (f === 0 || f === 2) world.footstep(this); this.lastWalkF = f; }
};

Fighter.prototype.face = function (opp) {
  if (this.airborne()) return;
  var f = opp.x >= this.x ? 1 : -1;
  if (Math.abs(opp.x - this.x) > 4) this.facing = f;
};

// 能不能出招、出哪一招（超必杀 > 升龙 > 波 > 普通技）
Fighter.prototype.tryAttack = function (inp, world) {
  var dp = this.motion([6, 2, 3], 16), qcf = this.motion([2, 3, 6], 16);
  var anyP = this.pressed('lp') || this.pressed('hp'), anyAtk = anyP || this.pressed('lk') || this.pressed('hk');
  if (this.pressed('su') && this.meter >= D.STOCK) { this.eat('su'); this.meter -= D.STOCK; this.start('su', world); return true; }
  if (this.pressed('s2') || (dp && anyP)) { this.eat('s2'); this.eat('lp'); this.eat('hp'); this.start('s2', world); return true; }
  if ((this.pressed('s1') || (qcf && anyP)) && !world.hasProj(this)) { this.eat('s1'); this.eat('lp'); this.eat('hp'); this.start('s1', world); return true; }
  if (!anyAtk) return false;
  var crouch = inp.y > 0, key;
  if (this.pressed('hp')) key = crouch ? 'ch' : 'hp';
  else if (this.pressed('hk')) key = crouch ? 'ch' : 'hk';
  else if (this.pressed('lk')) key = crouch ? 'cl' : 'lk';
  else key = crouch ? 'cl' : 'lp';
  this.eat('lp'); this.eat('lk'); this.eat('hp'); this.eat('hk');
  this.start(key, world);
  return true;
};

Fighter.prototype.start = function (key, world) {
  var m = this.moves[key];
  this.lastKey = key; this.moveCount = (this.moveCount || 0) + 1;     // 调试 / 无头断言用
  this.move = m; this.connected = false; this.hitCount = 0;
  if (!m.air) { this.state = 'attack'; this.t = 0; this.vx = 0; }
  else { this.state = 'attack'; this.t = 0; }
  this.crouching = !!m.low;
  if (m.inv) this.inv = m.inv;
  if (key === 'su') world.superFlash(this);
  if (!m.proj) world.sfx('swing' + (m.stop >= 10 ? 'H' : 'L'));      // 发波的「挥」由发波声代替
  world.shout(this, m);
};

Fighter.prototype.attackStep = function (inp, opp, world) {
  var m = this.move, t = this.t;
  if (m.lunge && t <= m.s + m.a) this.x += this.facing * m.lunge / (m.s + m.a) * (1 - t / (m.s + m.a + 1)) * 1.6;
  if (m.proj && t === m.s) world.spawn(this, m);
  if (m.rise && t === m.s) { this.vy = m.rise; this.vx = m.riseX * this.facing; world.burst(this, m); }
  if (m.stop >= 10 && t >= m.s && t < m.s + m.a && t % 2 === 0) this.ghost();
  // 取消：普通技打中 / 被挡后，收招期间可以直接出必杀（「轻拳 → 波」这种连段）
  if (m.cancel && this.connected && t >= m.s) {
    var save = this.state;
    if (this.cancelInto(inp, world)) return;
    this.state = save;
  }
  // 轻拳连打：打中 / 被挡后再按轻拳，从头再来一次（3 下以内）
  if (m.chain && this.connected && t >= m.s + m.a && this.pressed('lp') && (this.chainN || 0) < 2 && inp.y <= 0) {
    this.eat('lp'); this.chainN = (this.chainN || 0) + 1; this.start('lp', world); return;
  }
  if (m.air) {
    if (this.y <= 0 && t > 1) { this.land(world); }
    return;
  }
  if (m.rise) {
    if (this.y <= 0 && t > m.s + 2) { this.y = 0; this.vx = 0; this.move = null; this.set('land'); this.t = -m.r + 4; world.land(this); }
    return;
  }
  if (t >= m.s + m.a + m.r) { this.move = null; this.chainN = 0; this.set(inp.y > 0 ? 'crouch' : 'idle'); }
};
Fighter.prototype.cancelInto = function (inp, world) {
  var dp = this.motion([6, 2, 3], 16), qcf = this.motion([2, 3, 6], 16), anyP = this.pressed('lp') || this.pressed('hp');
  if (this.pressed('su') && this.meter >= D.STOCK) { this.eat('su'); this.meter -= D.STOCK; this.start('su', world); return true; }
  if (this.pressed('s2') || (dp && anyP)) { this.eat('s2'); this.start('s2', world); return true; }
  if ((this.pressed('s1') || (qcf && anyP)) && !world.hasProj(this)) { this.eat('s1'); this.start('s1', world); return true; }
  return false;
};

Fighter.prototype.land = function (world) {
  this.y = 0; this.vy = 0; this.vx = 0; this.move = null; this.set('land'); world.land(this);
};

Fighter.prototype.physics = function (world) {
  if (this.y > 0 || this.vy > 0) {
    this.x += this.vx; this.y += this.vy; this.vy -= GRAV;
    if (this.y < 0) this.y = 0;
  }
};

// 残影：冲刺 / 重击 / 升龙时记几帧前的样子，画的时候按颜色叠加淡出
Fighter.prototype.ghost = function () {
  this.ghosts.push({ x: this.x, y: this.y, pose: this.pose(), facing: this.facing, life: 1 });
  if (this.ghosts.length > 6) this.ghosts.shift();
};

// 被打：fight.js 判定打中后调
Fighter.prototype.takeHit = function (m, dmg, attacker, kd) {
  this.hp = Math.max(0, this.hp - dmg);
  this.flash = 4; this.move = null; this.crouching = false;
  var air = this.airborne();
  if (this.hp <= 0 || kd || air) {
    this.set('airhit');
    this.vy = Math.max(this.y > 0 ? 9 : 0, m.launch || (this.hp <= 0 ? 13 : 8));
    this.vx = -this.facingTo(attacker) * (this.hp <= 0 ? 6 : 3.5);
    if (this.y <= 0) this.y = 1;
    if (this.hp <= 0) this.ko = true;
  } else {
    this.set('hit'); this.stun = m.hs; this.push = m.push / 6;
  }
};
Fighter.prototype.takeBlock = function (m, chip) {
  this.hp = Math.max(1, this.hp - chip);
  this.set('block'); this.stun = m.bs; this.push = m.push * 0.7 / 6; this.move = null;
};
Fighter.prototype.facingTo = function (o) { return o.x >= this.x ? 1 : -1; };

// 当前该画哪张图
Fighter.prototype.pose = function () {
  var s = this.state;
  if (s === 'attack' && this.move) return this.move.pose;
  // 后退倒着放同一个循环（4 → 1），比前进少一点步幅感，和真人往后挪步一致
  if (s === 'walk' || s === 'dash') return 'walk' + (this.walkFrame() + 1);
  if (s === 'crouch' || s === 'jumpsq' || s === 'land' || s === 'getup') return 'crouch';
  if (s === 'jump' || s === 'back') return 'jump';
  if (s === 'block') return this.crouching ? 'crouch' : 'block';
  if (s === 'hit' || s === 'airhit' || s === 'lose') return 'hit';
  if (s === 'down') return 'down';
  if (s === 'win') return 'win';
  return 'idle';
};

// 受击框（世界坐标 [x0, y0, x1, y1]，y 向上）
Fighter.prototype.hurt = function () {
  if (this.state === 'down' || this.state === 'getup' || this.inv > 0 || this.ko) return null;
  var h = this.crouching || this.state === 'crouch' ? 165 : this.state === 'jump' ? 210 : 245;
  var y0 = this.y + (this.state === 'jump' ? 30 : 0);
  return [this.x - 42, y0, this.x + 42, this.y + h];
};
// 攻击框：只在持续帧里有
Fighter.prototype.hitbox = function () {
  var m = this.move;
  if (this.state !== 'attack' || !m || m.proj || this.t < m.s || this.t >= m.s + m.a) return null;
  var x1 = this.x + this.facing * m.reach, x0 = x1 - this.facing * m.w;
  return [Math.min(x0, x1), this.y + m.y0, Math.max(x0, x1), this.y + m.y1];
};

module.exports = Fighter;
