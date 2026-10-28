// 内购与广告：钻石包（wx.requestMidasPayment）、首充翻倍、新手礼包（限时）、月卡（每日领）、钻石换金币、
// 激励视频（wx.createRewardedVideoAd）、分享（wx.shareAppMessage）、兑换码。
//
// 支付的单据流程（照正式游戏的做法）：先记单 → 拉支付 → 支付成功先在单上记 paid 并**立刻存档** → 再发货、记 granted、存档。
// 中途被杀：下次启动 recover() 把 paid 但没 granted 的补发；没 paid 的按取消处理。真正上线要服务器验单发货，这里在本地做同一套状态机。
//
// 宿主没接支付 / 广告（errMsg 带「未实现」、广告 errCode 1004）时弹「模拟」确认框 —— 明说是模拟，不悄悄发奖。
// opts.auto（无头自动驾驶）时模拟直接确认。
var D = require('../data/defs.js');
var E = require('../core/economy.js');

function make(state, eco, bus, saveNow, opts) {
  opts = opts || {};
  var DAY = 86400000;
  if (!state.shop.starterUntil) state.shop.starterUntil = state.created + D.SHOP.starter.hours * 3600000;

  function unhosted(err) { return !!err && (/未实现|not support|no advertisement/.test(err.errMsg || '') || err.errCode === 1004); }
  function confirm(title, content, okText) {
    if (opts.auto) return Promise.resolve(true);
    // 「支付中」的 loading 还开着：先收掉，不然它压在确认框正中间，把正文挡住一半
    wx.hideLoading();
    return new Promise(function (resolve) {
      wx.showModal({ title: title, content: content, confirmText: okText || '确定', cancelText: '取消',
        success: function (r) { resolve(!!r.confirm); }, fail: function () { resolve(false); } });
    });
  }

  // ───────── 支付 ─────────
  function pay(price) {
    return new Promise(function (resolve, reject) {
      wx.requestMidasPayment({
        mode: 'game', env: D.MIDAS.env, offerId: D.MIDAS.offerId, currencyType: D.MIDAS.currencyType,
        platform: 'android', buyQuantity: price * D.MIDAS.rate, zoneId: D.MIDAS.zoneId,
        success: resolve, fail: reject
      });
    }).catch(function (err) {
      if (!unhosted(err)) throw err;
      return confirm('模拟支付', '当前宿主没有接入虚拟支付（wx.requestMidasPayment）。\n用模拟支付 ¥' + price + ' 走完流程？', '模拟支付')
        .then(function (ok) { if (!ok) throw { errMsg: 'requestMidasPayment:fail cancel', errCode: 1 }; return { mock: true }; });
    });
  }
  function grant(order) {
    var it = find(order.item), gain = {};
    if (order.kind === 'gems') {
      var first = !state.shop.firstBuy[it.id];
      gain.gem = it.gems * (first ? 2 : 1);
      state.shop.firstBuy[it.id] = true;
    } else if (order.kind === 'starter') {
      gain = { gem: it.gems, coin: it.coin };
      state.shop.starterBought = true;
      var o = state.pets[it.pet] || (state.pets[it.pet] = { n: 0, home: false }); o.n++;
      bus.emit('pets');
    } else if (order.kind === 'monthly') {
      gain.gem = it.gems;
      state.shop.monthlyUntil = Math.max(Date.now(), state.shop.monthlyUntil) + it.days * DAY;
    }
    eco.add(gain, 'purchase');
    order.granted = true;
    eco.stat('pay:yuan', it.price);
    return gain;
  }
  function find(id) {
    if (id === D.SHOP.starter.id) return D.SHOP.starter;
    if (id === D.SHOP.monthly.id) return D.SHOP.monthly;
    return D.SHOP.gems.filter(function (g) { return g.id === id; })[0];
  }
  // 买：返回 Promise<gain>；失败 reject（取消的 errCode 1 / cancel 为真）
  function buy(kind, id) {
    var it = find(id);
    if (!it) return Promise.reject({ errMsg: '没有这个商品' });
    if (kind === 'starter' && (state.shop.starterBought || Date.now() > state.shop.starterUntil)) return Promise.reject({ errMsg: '礼包已下架' });
    var order = { id: 'o' + Date.now().toString(36) + Math.floor(Math.random() * 1e6).toString(36), kind: kind, item: id, at: Date.now() };
    state.orders.push(order);
    saveNow();
    if (!opts.auto) wx.showLoading({ title: '支付中', mask: true });
    return pay(it.price).then(function (res) {
      order.paid = true; order.mock = !!(res && res.mock);
      saveNow();                                            // 先落「已付」，再发货：发货前被杀，下次能补
      var gain = grant(order);
      state.orders = state.orders.filter(function (o) { return o !== order; });
      saveNow();
      bus.emit('purchased', { kind: kind, item: it, gain: gain, mock: order.mock });
      console.log('[pay] 发货 ' + kind + ':' + id + ' ¥' + it.price + ' → ' + JSON.stringify(gain) + (order.mock ? '（游戏内模拟）' : '（宿主支付）') + ' 钻石余额 ' + Math.floor(state.res.gem));
      return gain;
    }, function (err) {
      state.orders = state.orders.filter(function (o) { return o !== order; });
      saveNow();
      var cancel = err && (err.errCode === 1 || /cancel/.test(err.errMsg || ''));
      console.log('[pay] 未发货 ' + kind + ':' + id + ' ' + ((err && err.errMsg) || ''));
      throw { errMsg: cancel ? '已取消支付' : '支付失败：' + ((err && err.errMsg) || '未知错误'), cancel: cancel };
    }).then(function (g) { if (!opts.auto) wx.hideLoading(); return g; }, function (e) { if (!opts.auto) wx.hideLoading(); throw e; });
  }
  // 启动时补单：已付没发的补发；没付的丢掉
  function recover() {
    var fixed = [];
    state.orders.forEach(function (o) { if (o.paid && !o.granted) fixed.push({ order: o, gain: grant(o) }); });
    state.orders = [];
    if (fixed.length) saveNow();
    return fixed;
  }

  // ───────── 钻石换金币 ─────────
  function buyCoins(id) {
    var p = D.SHOP.coins.filter(function (c) { return c.id === id; })[0];
    var coins = Math.round(eco.coinPerMinute(Date.now()) * p.coinMinutes / 10) * 10;
    if (!eco.spend({ gem: p.gems }, 'coins')) return null;
    eco.add({ coin: coins }, 'shop');
    return coins;
  }
  function coinsFor(id) { var p = D.SHOP.coins.filter(function (c) { return c.id === id; })[0]; return Math.round(eco.coinPerMinute(Date.now()) * p.coinMinutes / 10) * 10; }

  // ───────── 月卡 ─────────
  function monthly() {
    var now = Date.now(), active = state.shop.monthlyUntil > now;
    return { active: active, daysLeft: active ? Math.ceil((state.shop.monthlyUntil - now) / DAY) : 0, canClaim: active && state.shop.monthlyClaimed !== E.dayKey(now) };
  }
  function claimMonthly() {
    if (!monthly().canClaim) return null;
    state.shop.monthlyClaimed = E.dayKey(Date.now());
    eco.add({ gem: D.SHOP.monthly.daily }, 'monthly');
    return D.SHOP.monthly.daily;
  }
  function starter() { var now = Date.now(); return { show: !state.shop.starterBought && now < state.shop.starterUntil, left: (state.shop.starterUntil - now) / 1000 }; }

  // ───────── 激励视频 ─────────
  var ad = null, adErr = null, adWait = null;
  try {
    ad = wx.createRewardedVideoAd({ adUnitId: 'adunit-island-reward' });
    ad.onError(function (e) { adErr = e; });
    ad.onLoad(function () { adErr = null; });
    ad.onClose(function (res) { var w = adWait; adWait = null; if (w) w(!res || res.isEnded !== false); });
  } catch (e) { ad = null; }
  // 看广告：resolve(true) = 看完了（发奖），false = 中途关了
  function watchAd(why) {
    var done = function (ok) { if (ok) { eco.stat('ad'); bus.emit('adWatched', why); } return ok; };
    var mock = function () { return confirm('模拟广告', '当前宿主没有接入激励视频（RewardedVideoAd）。\n模拟「看完了」来领取奖励？', '看完了').then(done); };
    if (!ad || unhosted(adErr)) return mock();
    return new Promise(function (resolve) {
      adWait = resolve;
      ad.show().catch(function () { return ad.load().then(function () { return ad.show(); }); }).catch(function (err) {
        adWait = null;
        if (unhosted(err)) mock().then(resolve); else { wx.showToast({ title: '广告暂时不可用', icon: 'none' }); resolve(false); }
      });
    }).then(done);
  }
  // 看广告 30 分钟双倍产量（可叠加，最多 4 小时）
  function adBoost() {
    return watchAd('boost').then(function (ok) {
      if (!ok) return false;
      var now = Date.now();
      state.boostUntil = Math.min(now + 4 * 3600000, Math.max(now, state.boostUntil) + 30 * 60000);
      bus.emit('res');
      return true;
    });
  }

  // ───────── 分享（每天第一次给奖励） ─────────
  function share() {
    try { wx.shareAppMessage({ title: '我的萌宠小岛已经 ' + state.lv + ' 级啦，快来看看！', query: 'from=share&lv=' + state.lv }); } catch (e) {}
    var t = E.dayKey(Date.now());
    if (state.stats.shareDay === t) return null;
    state.stats.shareDay = t;
    eco.add({ gem: 10 }, 'share');
    return { gem: 10 };
  }

  // ───────── 兑换码 ─────────
  function redeem(code) {
    code = String(code || '').trim().toUpperCase();
    var r = D.REDEEM[code];
    if (!r) return { ok: false, msg: '兑换码无效' };
    if (state.redeemed[code]) return { ok: false, msg: '这个兑换码已经用过了' };
    state.redeemed[code] = Date.now();
    eco.add(r, 'redeem');
    saveNow();
    return { ok: true, reward: r };
  }

  return { buy: buy, recover: recover, buyCoins: buyCoins, coinsFor: coinsFor, monthly: monthly, claimMonthly: claimMonthly, starter: starter,
    watchAd: watchAd, adBoost: adBoost, share: share, redeem: redeem, confirm: confirm, firstBuy: function (id) { return !state.shop.firstBuy[id]; } };
}

module.exports = { make: make };
