// 电脑对手：输出与屏幕按键同格式的输入（{ x, y, press }），所以它出招、挡招走的是和玩家完全同一条路径。
// 「人味」来自反应延迟：它看到的是 react 帧以前的局面（ring buffer 存对手的状态），难度只调延迟与几个概率。
// 决策每隔几帧做一次（plan），中间按住同一个方向（真人也不会每帧换方向）。
function AI(diff) {
  this.d = diff;
  this.seen = [];          // 对手过去的快照
  this.plan = null; this.planT = 0; this.queue = [];
}

function snap(o) {
  return { x: o.x, y: o.y, state: o.state, t: o.t, move: o.move, atk: o.state === 'attack' && o.move && o.t < o.move.s + o.move.a, low: o.move && o.move.lv === 'low', air: o.airborne(), vx: o.vx };
}

AI.prototype.think = function (me, opp, world) {
  this.seen.push(snap(opp));
  if (this.seen.length > 40) this.seen.shift();
  var o = this.seen[Math.max(0, this.seen.length - 1 - this.d.react)];
  var dx = o.x - me.x, dist = Math.abs(dx), toward = dx >= 0 ? 1 : -1;
  var out = { x: 0, y: 0, press: {} };
  // 排好的连段：一帧一个动作
  if (this.queue.length) {
    var q = this.queue[0];
    if (q.wait > 0) { q.wait--; out.x = (q.x || 0) * toward; out.y = q.y || 0; return out; }
    this.queue.shift();
    if (q.key) out.press[q.key] = true;
    out.x = (q.x || 0) * toward; out.y = q.y || 0;
    return out;
  }
  var free = me.state === 'idle' || me.state === 'walk' || me.state === 'crouch' || me.state === 'block' || me.state === 'land';
  var R = Math.random;
  // 防守：看到对手在出招、离得够近 → 按后（下段就蹲着按后）
  var incoming = null;
  for (var i = 0; i < world.proj.length; i++) {
    var pj = world.proj[i];
    if (pj.owner !== me && Math.abs(pj.x - me.x) < 330 && (pj.x - me.x) * pj.vx < 0) incoming = pj;
  }
  if (free && (o.atk && dist < 300 || incoming)) {
    if (this.guarding === undefined || this.guardT-- <= 0) { this.guarding = R() < this.d.block; this.guardT = 10; }
    if (this.guarding) {
      // 波快到了：有时跳过去顺势飞踢
      if (incoming && !incoming.big && Math.abs(incoming.x - me.x) < 260 && R() < this.d.aa * 0.08) {
        this.queue = [{ x: 1, y: -1 }, { wait: 14, x: 1 }, { key: 'hk', x: 1 }];
        return out;
      }
      out.x = -toward; out.y = o.low || (o.move && o.move.key === 'ch') ? 1 : 0;
      return out;
    }
  }
  // 对空：对手跳过来 → 升龙
  if (free && o.air && o.state !== 'airhit' && dist < 230 && o.y > 40 && R() < this.d.aa * 0.25) { out.press.s2 = true; return out; }
  // 对手倒地 / 收招破绽：上前惩罚
  var punish = (opp.state === 'attack' && opp.move && opp.t >= opp.move.s + opp.move.a && opp.move.r > 14) || opp.state === 'land' && opp.move === null && opp.t < 2;
  if (!free) return out;
  if (punish && dist < 190 && R() < this.d.combo) { this.combo(out, toward); return out; }
  if (--this.planT > 0 && this.plan) { out.x = this.plan.x * toward; out.y = this.plan.y; return this.planned(out, dist, toward, me); }
  // 重新决策
  this.planT = 8 + Math.floor(R() * 14);
  var aggro = this.d.aggro;
  if (dist > 480) this.plan = R() < 0.35 ? { x: 0, y: 0, fire: true } : { x: 1, y: 0 };
  else if (dist > 210) {
    var r = R();
    this.plan = r < 0.22 ? { x: 0, y: 0, fire: true } : r < 0.22 + aggro * 0.2 ? { x: 1, y: 0, jumpIn: true } : r < 0.5 + aggro * 0.3 ? { x: 1, y: 0 } : r < 0.85 ? { x: -1, y: 0 } : { x: 0, y: 0 };
  } else {
    var s = R();
    this.plan = s < aggro ? { x: 0, y: 0, attack: true } : s < aggro + 0.15 ? { x: -1, y: 0 } : { x: -1, y: 1 };
  }
  out.x = this.plan.x * toward; out.y = this.plan.y;
  return this.planned(out, dist, toward, me);
};

AI.prototype.planned = function (out, dist, toward, me) {
  var p = this.plan, R = Math.random;
  if (p.fire && !p.done) { p.done = true; out.press.s1 = true; out.x = 0; }
  if (p.jumpIn && !p.done && dist < 380) { p.done = true; this.queue = [{ x: 1, y: -1 }, { wait: 16, x: 1 }, { key: R() < 0.5 ? 'hk' : 'lk', x: 1 }, { wait: 14 }]; if (R() < this.d.combo) this.queue.push({ key: 'lp' }, { wait: 6 }, { key: 's1' }); }
  if (p.attack && !p.done) { p.done = true; this.combo(out, toward); }
  if (me.meter >= 100 && dist < 520 && R() < 0.01 * this.d.aggro) out.press.su = true;
  return out;
};

// 一组连段（照真人常用的套路写）
AI.prototype.combo = function (out, toward) {
  var R = Math.random, c = this.d.combo, r = R();
  if (r < 0.25) this.queue = [{ key: 'lp' }, { wait: 6 }, { key: 'lp' }, { wait: 6 }, c > R() ? { key: 's1' } : { key: 'lk' }];
  else if (r < 0.45) this.queue = [{ key: 'lk', y: 1 }, { wait: 8, y: 1 }, { key: 'hk', y: 1 }];
  else if (r < 0.65) this.queue = [{ key: 'hp' }, { wait: 9 }, c > R() ? { key: 's2' } : { key: 's1' }];
  else if (r < 0.8) this.queue = [{ key: 'lk' }, { wait: 8 }, { key: 's1' }];
  else this.queue = [{ key: 'hk' }];
  var first = this.queue.shift();
  if (first.key) out.press[first.key] = true;
  out.y = first.y || 0;
};

module.exports = AI;
