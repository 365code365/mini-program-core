// 商店：四个页签 —— 钻石（虚拟支付，首充翻倍角标）、金币（花钻石，按当前产量报价）、特惠（新手礼包限时、月卡）、免费（看广告、分享）。
// 支付结果走游戏内提示 + 图标飞到资源条；取消不算错误。
var D = require('../data/defs.js');
var E = require('../core/economy.js');
var U = require('./draw.js');

function make(G) {
  var W = G.W, H = G.H, ui = G.ui, icons = G.icons, T = U.T;
  var TABS = [['gems', '钻石'], ['coins', '金币'], ['deals', '特惠'], ['free', '免费']], tab = 'gems', busy = false;

  var v = ui.view('shop', { x: 10, y: G.safeTop + 40, w: W - 20, h: H - G.safeTop - 40 - 96 - G.safeBottom, z: 20, modal: true, anim: 'up',
    onBackdrop: function () { v.hide(); }, draw: function (c, v) {
      var y0 = 112, cw = (v.w - 36) / 2, s = G.state;
      var body = tab === 'gems' ? 3 * 168 : tab === 'coins' ? 3 * 104 : tab === 'deals' ? 2 * 190 : 2 * 110;
      v.setContent(y0 + body + 20);
      U.panel(c, v.w, v.contentH, '商店', T.purple);
      U.closeBtn(c, icons, v);
      tabs(c, v, 62);
      if (tab === 'gems') D.SHOP.gems.forEach(function (g, i) {
        var x = 12 + (i % 2) * (cw + 12), y = y0 + Math.floor(i / 2) * 168, first = G.shop.firstBuy(g.id);
        U.card(c, x, y, cw, 156, first);
        var sz = 54 + i * 6; icons.draw(c, 'gem', x + cw / 2 - sz / 2, y + 44 - sz / 2 + 4, sz);
        if (i >= 3) icons.draw(c, 'gem', x + cw / 2 + 4, y + 30, 34);
        U.textOutlined(c, E.fmt(g.gems), x + cw / 2, y + 90, 20, '#2f8fe8', '#fff');
        if (first) U.tag(c, x + 6, y + 6, '首充双倍', '#ff3b30');
        else if (g.tag) U.tag(c, x + 6, y + 6, g.tag, '#f08c00');
        v.hit('g:' + g.id, x + 10, y + 106, cw - 20, 42); U.button(c, x + 10, y + 106, cw - 20, 42, '¥' + g.price, T.green, { size: 17 });
      });
      else if (tab === 'coins') D.SHOP.coins.forEach(function (p, i) {
        var y = y0 + i * 104, coins = G.shop.coinsFor(p.id);
        U.card(c, 12, y, v.w - 24, 94);
        for (var k = 0; k <= i; k++) icons.draw(c, 'coin', 22 + k * 14, y + 14 - k * 4, 60);
        U.textOutlined(c, E.fmt(coins), 140, y + 34, 22, '#f08c00', '#fff', 'left');
        U.text(c, '约 ' + p.coinMinutes + ' 分钟的全岛产出', 140, y + 62, 12, T.ink2);
        v.hit('c:' + p.id, v.w - 120, y + 24, 98, 46); U.button(c, v.w - 120, y + 24, 98, 46, String(p.gems), s.res.gem >= p.gems ? T.blue : T.gray, { icon: 'gem', icons: icons, size: 16 });
      });
      else if (tab === 'deals') {
        var st = G.shop.starter(), it = D.SHOP.starter, y = y0;
        U.card(c, 12, y, v.w - 24, 178, st.show);
        icons.draw(c, 'gift', 20, y + 12, 86);
        U.text(c, '新手礼包', 116, y + 26, 19, T.ink, 'left', true);
        U.tag(c, 116 + 86, y + 17, '超值 10 倍', '#ff3b30');
        U.costRow(c, icons, { gem: it.gems, coin: it.coin }, 116, y + 56, 14);
        if (G.thumbs) G.thumbs.draw(c, G.thumbs.pet(it.pet), 116, y + 68, 44);
        U.text(c, '+ 史诗宠物「' + D.PETS.filter(function (p) { return p.id === it.pet; })[0].name + '」', 160, y + 90, 13, '#9b5de5', 'left', true);
        if (st.show) { v.hit('starter', v.w - 150, y + 118, 126, 48); U.button(c, v.w - 150, y + 118, 126, 48, '¥' + it.price, T.orange); }
        else U.text(c, s.shop.starterBought ? '已购买' : '已下架', v.w - 40, y + 140, 15, T.mute, 'right', true);
        y += 190;
        var mo = G.shop.monthly(), M = D.SHOP.monthly;
        U.card(c, 12, y, v.w - 24, 178, mo.canClaim);
        icons.draw(c, 'crown', 20, y + 12, 86);
        U.text(c, '月卡', 116, y + 26, 19, T.ink, 'left', true);
        U.wrap(c, '立得 ' + M.gems + ' 钻石，之后 ' + M.days + ' 天每天再领 ' + M.daily + ' 钻石（共 ' + (M.gems + M.daily * M.days) + '）', 116, y + 54, v.w - 150, 12, T.ink2, 16, 3);
        if (mo.active) {
          U.text(c, '剩余 ' + mo.daysLeft + ' 天', 20, y + 140, 14, '#2f9e44', 'left', true);
          v.hit('mclaim', v.w - 150, y + 118, 126, 48); U.button(c, v.w - 150, y + 118, 126, 48, mo.canClaim ? '领 ' + M.daily : '明天再来', mo.canClaim ? T.green : T.gray, { icon: mo.canClaim ? 'gem' : null, icons: icons, size: 15 });
        } else { v.hit('monthly', v.w - 150, y + 118, 126, 48); U.button(c, v.w - 150, y + 118, 126, 48, '¥' + M.price, T.orange); }
      } else {
        var y2 = y0;
        U.card(c, 12, y2, v.w - 24, 98); icons.draw(c, 'video', 22, y2 + 16, 66);
        U.text(c, '看视频领钻石', 100, y2 + 32, 17, T.ink, 'left', true); U.costRow(c, icons, { gem: 20 }, 100, y2 + 64, 14);
        v.hit('adgem', v.w - 124, y2 + 26, 100, 46); U.button(c, v.w - 124, y2 + 26, 100, 46, '观看', T.green, { icon: 'video', icons: icons, size: 16 });
        y2 += 110;
        var shared = s.stats.shareDay === E.dayKey(Date.now());
        U.card(c, 12, y2, v.w - 24, 98); icons.draw(c, 'share', 22, y2 + 16, 66);
        U.text(c, '分享给好友', 100, y2 + 32, 17, T.ink, 'left', true); U.text(c, shared ? '今天的奖励已领' : '每天第一次分享得 10 钻石', 100, y2 + 64, 12, T.ink2);
        v.hit('share', v.w - 124, y2 + 26, 100, 46); U.button(c, v.w - 124, y2 + 26, 100, 46, '分享', shared ? T.gray : T.blue, { icon: 'share', icons: icons, size: 16 });
      }
    }, onTap: function (id) {
      if (id === 'close') return v.hide();
      if (id.indexOf('tab:') === 0) { tab = id.slice(4); v.scrollTo(0); return v.markDirty(); }
      if (busy) return;
      if (id.indexOf('g:') === 0) purchase('gems', id.slice(2));
      else if (id === 'starter') purchase('starter', 'starter');
      else if (id === 'monthly') purchase('monthly', 'monthly');
      else if (id === 'mclaim') { var n = G.shop.claimMonthly(); if (n) { G.reward({ gem: n }, '月卡每日 +'); } else G.ui.toast('今天已经领过了'); }
      else if (id.indexOf('c:') === 0) { var got = G.shop.buyCoins(id.slice(2)); if (got === null) G.ui.toast('钻石不够', 'gem'); else G.reward({ coin: got }, '获得'); }
      else if (id === 'adgem') G.shop.watchAd('shop').then(function (ok) { if (ok) G.reward(G.eco.add({ gem: 20 }, 'ad'), '看完视频 +'); v.markDirty(); });
      else if (id === 'share') { var r = G.shop.share(); if (r) G.reward(r, '分享奖励 +'); else G.ui.toast('今天的分享奖励已经领过了'); }
      v.markDirty();
    } });
  // 新手礼包的「剩余 h:mm:ss」走单独的小条（kit.ticker）：整块商店面板每秒重画一次，真机上每秒顿 70ms
  var starterLeft = false;
  ui.ticker(v, 'shop-clock', 14, 112 + 129, 170, 22, function (c) {
    var st = G.shop.starter(); starterLeft = st.show;
    U.text(c, '剩余 ' + E.fmtClock(st.left), 6, 11, 14, '#e5383b', 'left', true);
  }, function () { return tab === 'deals' && G.shop.starter().show; });
  function tabs(c, v, y) {
    var tw = (v.w - 24) / TABS.length;
    TABS.forEach(function (t, i) {
      var on = t[0] === tab, x = 12 + i * tw;
      c.fillStyle = on ? '#9b5de5' : '#f3e3cc'; U.rr(c, x + 2, y, tw - 4, 38, 14); c.fill();
      U.text(c, t[1], x + tw / 2, y + 19, 15, on ? '#fff' : T.ink2, 'center', true);
      if (t[0] === 'deals' && (G.shop.starter().show || G.shop.monthly().canClaim)) U.badge(c, x + tw - 10, y + 4);
      v.hit('tab:' + t[0], x, y, tw, 40);
    });
  }
  function purchase(kind, id) {
    busy = true;
    G.shop.buy(kind, id).then(function (gain) {
      busy = false; v.markDirty();
      G.reward(gain, '购买成功 +');
      G.sfx('buy');
    }, function (err) {
      busy = false; v.markDirty();
      G.ui.toast(err.errMsg || '支付失败');
    });
  }
  return {
    open: function (t) { if (t) tab = t; v.markDirty(); v.scrollTo(0); v.show(); },
    // 礼包过期的那一刻面板要换成「已下架」：只在 show 翻转时重画
    step: function () { if (v.shown && tab === 'deals' && G.shop.starter().show !== starterLeft) { starterLeft = !starterLeft; v.markDirty(); } }
  };
}

module.exports = { make: make };
