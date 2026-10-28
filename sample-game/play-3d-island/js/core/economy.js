// 经济系统：资源加减、上限、建筑产量与花费、宠物 / 灯塔 / 广告加成、经验与等级、离线收益、数字格式化。
// 只读写 state（存档），派生值每次现算。所有「有变化」的地方发事件（bus），任务 / 成就 / 界面都只听事件。
var D = require('../data/defs.js');

function make(state, bus) {
  // ───────── 资源 ─────────
  function cap(res) {
    if (res === 'coin' || res === 'gem') return Infinity;
    var mult = 1;
    state.buildings.forEach(function (b) { var d = D.BUILDINGS[b.type]; if (d.storage && !b.busyKind || d.storage && b.busyKind === 'upgrade') mult += d.storage * b.lv; });
    return Math.round(200 * mult);
  }
  function has(cost) { for (var k in cost) if ((state.res[k] || 0) < cost[k]) return false; return true; }
  function missing(cost) { var out = {}; for (var k in cost) if ((state.res[k] || 0) < cost[k]) out[k] = cost[k] - (state.res[k] || 0); return out; }
  // 加资源：超过上限的部分丢掉，返回实际加了多少
  function add(gain, why) {
    var got = {};
    for (var k in gain) {
      var before = state.res[k] || 0, after = Math.min(cap(k), before + gain[k]);
      if (after < before) after = before;                 // 已经超上限（仓库拆了）不往下砍
      state.res[k] = after; got[k] = after - before;
      if (got[k] > 0) { stat('earn:' + k, got[k]); bus.emit('earn', { res: k, n: got[k], why: why }); }
    }
    bus.emit('res');
    return got;
  }
  function spend(cost, why) {
    if (!has(cost)) return false;
    for (var k in cost) { state.res[k] -= cost[k]; stat('spend:' + k, cost[k]); }
    bus.emit('spend', { cost: cost, why: why });
    bus.emit('res');
    return true;
  }
  function stat(key, n) { state.stats[key] = (state.stats[key] || 0) + (n === undefined ? 1 : n); }

  // ───────── 建筑数值 ─────────
  function scale(base, growth, lv) { var out = {}; for (var k in base) out[k] = Math.round(base[k] * Math.pow(growth, lv - 1)); return out; }
  function upgradeCost(b) { var d = D.BUILDINGS[b.type]; return scale(d.cost, d.costGrowth, b.lv); }
  function upgradeTime(b) { var d = D.BUILDINGS[b.type]; return Math.round(d.time * Math.pow(d.timeGrowth, b.lv - 1)); }
  function buildCost(type) {
    var d = D.BUILDINGS[type], n = state.buildings.filter(function (b) { return b.type === type; }).length;
    return scale(d.buildCost, 1.5, n + 1);                 // 同种建第 n 座贵 1.5^n
  }
  // 宠物加成：住进小屋的宠物（蜂场在时翻倍），按资源累加百分比
  function petBonus(res) {
    var pct = 0, apiary = state.buildings.some(function (b) { return b.type === 'apiary' && b.busyKind !== 'build'; });
    D.PETS.forEach(function (p) {
      var o = state.pets[p.id];
      if (!o || !o.home) return;
      if (p.bonus === res || p.bonus === 'all') pct += p.pct * (apiary ? 2 : 1);
    });
    return pct / 100;
  }
  function globalMult(now, res) {
    var m = 1 + petBonus(res);
    state.buildings.forEach(function (b) { var d = D.BUILDINGS[b.type]; if (d.boost && b.busyKind !== 'build') m += d.boost * b.lv; });
    if (state.boostUntil > now) m *= 2;
    return m;
  }
  // 每分钟产量（含加成）
  function rate(b, now) {
    var d = D.BUILDINGS[b.type];
    if (!d.produce || b.busyKind === 'build') return 0;
    return d.rate * Math.pow(d.rateGrowth, b.lv - 1) * globalMult(now || Date.now(), d.produce);
  }
  function storeCap(b) { var d = D.BUILDINGS[b.type]; return d.cap ? Math.round(d.cap * Math.pow(1.35, b.lv - 1)) : 0; }
  function maxPets() { var h = state.buildings.filter(function (b) { return b.type === 'pethouse' && b.busyKind !== 'build'; })[0]; return h ? h.lv : 0; }

  // ───────── 推进时间：建筑往自己的库存里攒（有消耗的先扣原料），施工到点就完工 ─────────
  // 返回这段时间里完工的建筑（界面要放庆祝特效）
  function tick(now) {
    var dt = Math.max(0, (now - state.lastTick) / 1000);
    state.lastTick = now;
    var done = [];
    state.buildings.forEach(function (b) {
      if (b.busyKind && now >= b.busyUntil) {
        var kind = b.busyKind;
        b.busyKind = null;
        if (kind === 'upgrade') b.lv++;
        done.push({ b: b, kind: kind });
      }
      var d = D.BUILDINGS[b.type];
      if (!d.produce || b.busyKind === 'build') return;
      var room = storeCap(b) - b.store;
      if (room <= 0) return;
      var make = Math.min(room, rate(b, now) / 60 * dt);
      if (d.consume) {                                      // 原料不够就按能做的做
        for (var k in d.consume) make = Math.min(make, (state.res[k] || 0) / d.consume[k]);
        for (var k2 in d.consume) state.res[k2] -= make * d.consume[k2];
      }
      b.store += make;
      if (make > 0) b.made = (b.made || 0) + make;
    });
    done.forEach(function (x) {
      if (x.kind === 'upgrade') { gainXp(5 * x.b.lv); stat('upgrade'); bus.emit('upgraded', x.b); }
      else { gainXp(10); stat('build'); bus.emit('built', x.b); }
    });
    return done;
  }
  // 收获：建筑库存 → 全岛资源（受上限）；返回收了多少
  function collect(b) {
    var d = D.BUILDINGS[b.type], n = Math.floor(b.store);
    if (!d.produce || n <= 0) return 0;
    var got = add(Object.defineProperty({}, d.produce, { value: n, enumerable: true }), 'collect')[d.produce] || 0;
    b.store -= got;
    if (got > 0) { stat('collect'); gainXp(1); bus.emit('collected', { b: b, res: d.produce, n: got }); if (d.produce === 'bread') bus.emit('made', { res: 'bread', n: got }); }
    return got;
  }

  // ───────── 经验 / 等级 ─────────
  function gainXp(n) {
    state.xp += n;
    var ups = 0;
    while (state.xp >= D.xpFor(state.lv)) { state.xp -= D.xpFor(state.lv); state.lv++; ups++; bus.emit('levelup', state.lv); }
    bus.emit('xp');
    return ups;
  }

  // ───────── 离线收益：上次 lastTick 到现在（最多 8 小时）按产量直接算进库存 ─────────
  // 返回 { seconds, gain: {res: n} } —— 先只算不加，欢迎回来面板上让玩家选「领取」或「看广告领双倍」
  function offline(now) {
    var dt = Math.min(D.OFFLINE_CAP_H * 3600, Math.max(0, (now - state.lastTick) / 1000));
    if (dt < 60) return null;
    var gain = {};
    state.buildings.forEach(function (b) {
      var d = D.BUILDINGS[b.type];
      if (!d.produce || b.busyKind === 'build' || d.consume) return;       // 有消耗的（面包坊）离线不算，免得把小麦吃光
      gain[d.produce] = (gain[d.produce] || 0) + rate(b, state.lastTick) / 60 * dt * 0.5;   // 离线按五折
    });
    for (var k in gain) gain[k] = Math.floor(gain[k]);
    state.lastTick = now;
    return { seconds: dt, gain: gain };
  }
  // 全岛「每分钟金币当量」（金币包、离线面板按它报价，随进度涨）
  function coinPerMinute(now) {
    var v = 0;
    state.buildings.forEach(function (b) { var d = D.BUILDINGS[b.type]; if (d.produce && D.RES[d.produce].sell) v += rate(b, now) * D.RES[d.produce].sell; });
    return Math.max(20, v);
  }

  return { cap: cap, has: has, missing: missing, add: add, spend: spend, stat: stat, upgradeCost: upgradeCost, upgradeTime: upgradeTime,
    buildCost: buildCost, rate: rate, storeCap: storeCap, maxPets: maxPets, petBonus: petBonus, globalMult: globalMult,
    tick: tick, collect: collect, gainXp: gainXp, offline: offline, coinPerMinute: coinPerMinute };
}

// ───────── 格式化 ─────────
var UNITS = ['', 'K', 'M', 'B', 'T', 'aa', 'ab'];
function fmt(n) {
  n = Math.floor(n);
  if (n < 10000) return String(n);
  var u = 0, v = n;
  while (v >= 1000 && u < UNITS.length - 1) { v /= 1000; u++; }
  return (v >= 100 ? v.toFixed(0) : v >= 10 ? v.toFixed(1) : v.toFixed(2)).replace(/\.?0+$/, '') + UNITS[u];
}
function fmtTime(sec) {
  sec = Math.max(0, Math.ceil(sec));
  if (sec < 60) return sec + '秒';
  if (sec < 3600) return Math.floor(sec / 60) + '分' + (sec % 60 ? (sec % 60) + '秒' : '');
  if (sec < 86400) return Math.floor(sec / 3600) + '小时' + (Math.floor(sec % 3600 / 60) ? Math.floor(sec % 3600 / 60) + '分' : '');
  return Math.floor(sec / 86400) + '天' + Math.floor(sec % 86400 / 3600) + '小时';
}
function fmtClock(sec) {
  sec = Math.max(0, Math.floor(sec));
  var h = Math.floor(sec / 3600), m = Math.floor(sec % 3600 / 60), s = sec % 60;
  return (h ? h + ':' : '') + (h && m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s;
}
// 按本地时区算「哪一天」：每日任务、签到、月卡都按它切
function dayKey(t) { var d = new Date(t); return d.getFullYear() + '-' + (d.getMonth() + 1) + '-' + d.getDate(); }
// 加速：每分钟剩余 1 钻，最少 1
function skipCost(sec) { return Math.max(1, Math.ceil(sec / 60)); }

module.exports = { make: make, fmt: fmt, fmtTime: fmtTime, fmtClock: fmtClock, dayKey: dayKey, skipCost: skipCost };
