// 自动驾驶（启动参数 auto=1）：像真人一样玩 —— 收获、按顺序造建筑、升级最便宜的、货快满了去卖、钱多了扩地、
// 签到、领任务、钻石够了孵蛋、宠物住进小屋、买一次钻石包（模拟支付）、看一次广告双倍；顺手把每个界面打开一遍（截图覆盖）。
// 只调 G.act / 系统接口，不绕过规则（花费、等级解锁、摆放检查都照走）。
var D = require('./data/defs.js');

function make(G) {
  var t = 0, step = 0, tour = ['shop', 'quests', 'pets', 'signin', 'build', 'settings'], tourAt = 0, bought = false, boosted = false;
  function act() {
    var s = G.state;
    // 弹着的结算弹窗先关
    ['offline', 'levelup'].forEach(function (id) { var v = G.ui.get(id); if (v && v.shown) v.onTap(id === 'offline' ? 'take' : 'ok', v); });
    var gr = G.ui.get('gacha-result'); if (gr && gr.shown) { gr.onTap('ok', gr); gr.onTap('ok', gr); }
    s.buildings.forEach(function (b) { if (!b.busyKind && D.BUILDINGS[b.type].produce && b.store >= 5) G.act.collect(b); });
    // 签到、任务、成就、月卡
    if (G.meta.signinInfo().canClaim) { var r = G.meta.claimSignin(false); if (r && r.egg) G.gacha.pull(1, true); }
    G.meta.dailyList().forEach(function (q) { if (q.done && !q.claimed) G.meta.claimDaily(q.def.id); });
    G.meta.achList().forEach(function (a) { if (a.done) G.meta.claimAch(a.def.id); });
    G.shop.claimMonthly();
    // 造：按 BUILD_ORDER 第一种「解锁了、买得起、没造够」的
    var counts = {}; s.buildings.forEach(function (b) { counts[b.type] = (counts[b.type] || 0) + 1; });
    var want = { farm: 3, lumber: 2, quarry: 2, warehouse: 1, market: 1, bakery: 1, dock: 1, pethouse: 1, lighthouse: 1, apiary: 1 };
    for (var i = 0; i < D.BUILD_ORDER.length; i++) {
      var ty = D.BUILD_ORDER[i], d = D.BUILDINGS[ty];
      if (s.lv < d.unlock || (counts[ty] || 0) >= want[ty] || !G.eco.has(G.eco.buildCost(ty))) continue;
      var spot = G.act.bestSpot(ty, G.terrain.N >> 1, G.terrain.N >> 1);
      if (G.act.check(ty, spot[0], spot[1])) continue;
      G.act.startPlace(ty); G.act.movePlace(spot[0], spot[1]); G.act.confirmPlace();
      break;
    }
    // 升级：最便宜的一座（不在施工中）
    var cands = s.buildings.filter(function (b) { return !b.busyKind && b.lv < D.BUILDINGS[b.type].maxLv && G.eco.has(G.eco.upgradeCost(b)); });
    cands.sort(function (a, b) { return (G.eco.upgradeCost(a).coin || 0) - (G.eco.upgradeCost(b).coin || 0); });
    if (cands[0] && Math.random() < 0.6) G.act.upgrade(cands[0]);
    // 卖：快满了的卖一半
    if (s.buildings.some(function (b) { return b.type === 'market' && !b.busyKind; })) {
      var cost = {}, coins = 0;
      D.GOODS.forEach(function (k) { if (s.res[k] > G.eco.cap(k) * 0.7) { cost[k] = Math.floor(s.res[k] / 2); coins += cost[k] * D.RES[k].sell; } });
      if (coins && G.eco.spend(cost, 'sell')) { G.eco.add({ coin: coins }, 'sell'); G.bus.emit('sold', coins); }
    }
    // 扩地：金币是价格 3 倍以上
    var ec = D.expandCost(s.expanded.length);
    if (s.res.coin > ec.coin * 3) {
      for (var z = 0; z < G.terrain.N; z++) for (var x = 0; x < G.terrain.N; x++) if (G.terrain.expandable(x, z)) { G.act.expand(x, z); z = 1e9; break; }
    }
    // 施工太久的用钻石加速（留 100 钻抽卡）
    s.buildings.forEach(function (b) { if (b.busyKind && s.res.gem > 160 && b.busyUntil - Date.now() > 20000) G.act.skip(b); });
    if (s.res.gem >= D.GACHA.single) { G.gacha.pull(s.res.gem >= D.GACHA.ten + 100 ? 10 : 1); }
    D.PETS.forEach(function (p) { if (s.pets[p.id] && s.pets[p.id].n && !s.pets[p.id].home) G.gacha.setHome(p.id, true); });
    if (!bought && t > 8) { bought = true; G.shop.buy('gems', 'g30').then(function (g) { G.reward(g, '购买成功 +'); }, function () {}); }
    if (!boosted && t > 12) { boosted = true; G.shop.adBoost(); }
    G.save();
  }
  return {
    step: function (dt) {
      t += dt;
      if (t - step >= 0.5) { step = t; act(); }
      // 界面巡游：每 2.5 秒换一个打开（6 个转一圈），给截图 / 覆盖率用
      if (t - tourAt > 2.5) {
        tourAt = t;
        G.ui.hideAll();
        var k = Math.floor(t / 2.5) % (tour.length + 2);
        if (k < tour.length) G.open(tour[k]);
      }
    }
  };
}

module.exports = { make: make };
