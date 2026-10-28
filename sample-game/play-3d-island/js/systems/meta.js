// 每日任务、成就、7 日签到：只听总线上的事件计数，领奖走 economy。跨天按本地日期重置（dayKey）。
var D = require('../data/defs.js');
var E = require('../core/economy.js');

function make(state, eco, bus) {
  function today() { return E.dayKey(Date.now()); }
  function rollDay() {
    var t = today();
    if (state.daily.day !== t) { state.daily = { day: t, counts: {}, claimed: {} }; bus.emit('meta'); }
  }
  function count(key, n) { rollDay(); state.daily.counts[key] = (state.daily.counts[key] || 0) + (n || 1); bus.emit('meta'); }
  bus.on('collected', function () { count('collect'); });
  bus.on('upgraded', function () { count('upgrade'); });
  bus.on('sold', function () { count('sell'); });
  bus.on('adWatched', function () { count('ad'); });
  bus.on('gacha', function (n) { count('gacha', n); });
  bus.on('made', function (x) { count('make:' + x.res, x.n); });
  bus.on('built', function () { bus.emit('meta'); });
  bus.on('levelup', function () { bus.emit('meta'); });

  // ───────── 每日任务 ─────────
  function dailyList() {
    rollDay();
    return D.DAILY.map(function (q) {
      var c = Math.min(q.n, Math.floor(state.daily.counts[q.key] || 0));
      return { def: q, progress: c, done: c >= q.n, claimed: !!state.daily.claimed[q.id] };
    });
  }
  function claimDaily(id) {
    var q = dailyList().filter(function (x) { return x.def.id === id; })[0];
    if (!q || !q.done || q.claimed) return null;
    state.daily.claimed[id] = true;
    eco.add(q.def.reward, 'daily');
    bus.emit('meta');
    return q.def.reward;
  }

  // ───────── 成就：按累计值分阶，一阶一阶领 ─────────
  function achValue(a) {
    if (a.key === 'pets') return D.PETS.filter(function (p) { return state.pets[p.id] && state.pets[p.id].n > 0; }).length;
    if (a.key === 'level') return state.lv;
    return state.stats[a.key] || 0;
  }
  function achList() {
    return D.ACHIEVEMENTS.map(function (a) {
      var got = state.ach[a.id] || 0, v = achValue(a), next = a.tiers[got];
      return { def: a, tier: got, value: v, target: next, done: next !== undefined && v >= next, maxed: next === undefined };
    });
  }
  function claimAch(id) {
    var a = achList().filter(function (x) { return x.def.id === id; })[0];
    if (!a || !a.done) return null;
    var r = a.def.reward[a.tier];
    state.ach[id] = a.tier + 1;
    eco.add(r, 'achievement');
    bus.emit('meta');
    return r;
  }

  // ───────── 签到：连续 7 天一轮，断一天从第 1 天重来 ─────────
  function signinInfo() {
    var t = today(), last = state.signin.last;
    var yesterday = E.dayKey(Date.now() - 86400000);
    var streak = last === t || last === yesterday ? state.signin.streak : 0;
    return { canClaim: last !== t, day: last === t ? (streak - 1) % 7 : streak % 7, streak: streak };
  }
  function claimSignin(double) {
    var info = signinInfo();
    if (!info.canClaim) return null;
    var r = D.SIGNIN[info.day], give = {};
    for (var k in r) if (k !== 'egg') give[k] = r[k] * (double ? 2 : 1);
    state.signin = { last: today(), streak: info.streak + 1, total: state.signin.total + 1 };
    eco.add(give, 'signin');
    bus.emit('meta');
    return { reward: give, egg: r.egg || 0 };
  }

  // 红点：有没有能领的
  function badges() {
    return {
      quests: dailyList().some(function (q) { return q.done && !q.claimed; }) || achList().some(function (a) { return a.done; }),
      signin: signinInfo().canClaim
    };
  }

  return { dailyList: dailyList, claimDaily: claimDaily, achList: achList, claimAch: claimAch, signinInfo: signinInfo, claimSignin: claimSignin, badges: badges, rollDay: rollDay };
}

module.exports = { make: make };
