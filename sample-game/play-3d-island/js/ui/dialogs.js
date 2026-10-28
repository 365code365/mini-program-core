// 其余弹窗：签到（7 天）、双倍产量（看广告）、欢迎回来（离线收益，可看广告双倍）、升级、扩地、集市卖货（数量步进器）、设置。
var D = require('../data/defs.js');
var E = require('../core/economy.js');
var U = require('./draw.js');

function make(G) {
  var W = G.W, H = G.H, ui = G.ui, icons = G.icons, T = U.T;
  function dialog(id, h, draw, onTap, opts) {
    opts = opts || {};
    var v = ui.view(id, { x: 16, y: Math.max(G.safeTop + 40, (H - h) / 2 - 20), w: W - 32, h: h, z: opts.z || 30, modal: true, anim: 'pop',
      onBackdrop: opts.sticky ? null : function () { v.hide(); }, draw: draw, onTap: function (hid) { if (hid === 'close') return v.hide(); onTap(hid, v); } });
    return v;
  }

  // ───────── 签到 ─────────
  var signin = dialog('signin', 420, function (c, v) {
    var info = G.meta.signinInfo();
    U.panel(c, v.w, v.h, '七日签到', T.blue); U.closeBtn(c, icons, v);
    var cw = (v.w - 24 - 3 * 8) / 4;
    D.SIGNIN.forEach(function (r, i) {
      var big = i === 6, x = big ? 12 + 3 * (cw + 8) : 12 + (i % 4) * (cw + 8), y = big ? 70 + 1 * 0 : 70 + Math.floor(i / 4) * 128;
      var w = cw, h = big ? 120 : 120;
      if (big) { x = 12 + 3 * (cw + 8); y = 198; }
      var got = i < info.day || (!info.canClaim && i === info.day), today = info.canClaim && i === info.day;
      U.card(c, x, y, w, h, today);
      U.text(c, '第 ' + (i + 1) + ' 天', x + w / 2, y + 16, 12, T.ink2, 'center', true);
      var k = Object.keys(r).filter(function (q) { return q !== 'egg'; })[0];
      icons.draw(c, r.egg ? 'egg' : k, x + w / 2 - 26, y + 28, 52);
      U.text(c, (r.egg ? '+蛋 ' : '') + E.fmt(r[k]), x + w / 2, y + 96, 14, T.ink, 'center', true);
      if (got) { c.fillStyle = 'rgba(255,255,255,0.6)'; U.rr(c, x, y, w, h, 14); c.fill(); icons.draw(c, 'check', x + w / 2 - 20, y + 40, 40); }
    });
    U.text(c, '已连续签到 ' + info.streak + ' 天，断签从第 1 天重来', v.w / 2, 336, 12, T.ink2, 'center');
    if (info.canClaim) {
      var bw = (v.w - 36) / 2;
      v.hit('claim', 12, 352, bw, 54); U.button(c, 12, 352, bw, 54, '签到', T.green);
      v.hit('double', 24 + bw, 352, bw, 54); U.button(c, 24 + bw, 352, bw, 54, '双倍领取', T.orange, { icon: 'video', icons: icons, size: 15 });
    } else { v.hit('close', v.w / 2 - 80, 352, 160, 54); U.button(c, v.w / 2 - 80, 352, 160, 54, '明天再来', T.gray); }
  }, function (id, v) {
    var go = function (double) {
      var r = G.meta.claimSignin(double);
      if (!r) return;
      G.reward(r.reward, '签到 +');
      if (r.egg) { var res = G.gacha.pull(1, true); if (res) G.pets.showResult(res); }
      G.save(); v.markDirty();
    };
    if (id === 'claim') go(false);
    else if (id === 'double') G.shop.watchAd('signin').then(function (ok) { if (ok) go(true); });
  });

  // ───────── 双倍产量 ─────────
  var boost = dialog('boost', 300, function (c, v) {
    U.panel(c, v.w, v.h, '产量翻倍', T.orange); U.closeBtn(c, icons, v);
    icons.draw(c, 'boost', v.w / 2 - 48, 64, 96);
    var left = (G.state.boostUntil - Date.now()) / 1000; boostOn = left > 0; boostFull = left > 3.5 * 3600;
    if (!boostOn) U.text(c, '看一段视频，全岛产量翻倍 30 分钟', v.w / 2, 182, 14, T.ink, 'center', true);
    U.text(c, '可叠加，最多 4 小时', v.w / 2, 204, 12, T.ink2, 'center');
    v.hit('watch', v.w / 2 - 100, 222, 200, 56); U.button(c, v.w / 2 - 100, 222, 200, 56, left > 3.5 * 3600 ? '已叠满' : '看视频', left > 3.5 * 3600 ? T.gray : T.green, { icon: 'video', icons: icons });
  }, function (id, v) {
    if (id === 'watch') G.shop.adBoost().then(function (ok) { if (ok) { G.ui.toast('全岛产量翻倍 30 分钟！', 'boost'); G.save(); } v.markDirty(); });
  });

  // 「还剩 m:ss」走小条（kit.ticker），对话框只在开始 / 结束翻倍、叠满时重画
  var boostOn = false, boostFull = false;
  ui.ticker(boost, 'boost-clock', boost.w / 2 - 120, 171, 240, 22, function (c) {
    U.text(c, '双倍中，还剩 ' + E.fmtClock((G.state.boostUntil - Date.now()) / 1000), 120, 11, 14, T.ink, 'center', true);
  }, function () { return G.state.boostUntil > Date.now(); });

  // ───────── 欢迎回来 ─────────
  var away = null;
  var offline = dialog('offline', 340, function (c, v) {
    if (!away) return;
    U.panel(c, v.w, v.h, '欢迎回来', T.green);
    U.text(c, '你离开了 ' + E.fmtTime(away.seconds) + '，小岛帮你攒了：', v.w / 2, 80, 14, T.ink, 'center', true);
    var keys = Object.keys(away.gain).filter(function (k) { return away.gain[k] > 0; }), cw = Math.min(90, (v.w - 24) / Math.max(1, keys.length));
    keys.forEach(function (k, i) {
      var x = v.w / 2 - keys.length * cw / 2 + i * cw;
      icons.draw(c, k, x + cw / 2 - 28, 100, 56);
      U.text(c, E.fmt(away.gain[k]), x + cw / 2, 170, 15, T.ink, 'center', true);
    });
    if (!keys.length) U.text(c, '（还没有会生产的建筑）', v.w / 2, 140, 13, T.ink2, 'center');
    U.text(c, '离线收益按五折、最多 ' + D.OFFLINE_CAP_H + ' 小时', v.w / 2, 204, 11, T.ink2, 'center');
    var bw = (v.w - 36) / 2;
    v.hit('take', 12, 256, bw, 56); U.button(c, 12, 256, bw, 56, '领取', T.blue);
    v.hit('double', 24 + bw, 256, bw, 56); U.button(c, 24 + bw, 256, bw, 56, '双倍领取', T.orange, { icon: 'video', icons: icons, size: 15 });
  }, function (id, v) {
    var give = function (k) { var g = {}; for (var r in away.gain) g[r] = away.gain[r] * k; G.reward(G.eco.add(g, 'offline'), '离线收益 +'); away = null; v.hide(); G.save(); };
    if (id === 'take') give(1);
    else if (id === 'double') G.shop.watchAd('offline').then(function (ok) { give(ok ? 2 : 1); });
  }, { sticky: true });

  // ───────── 升级 ─────────
  var newLv = 0;
  var levelup = dialog('levelup', 330, function (c, v) {
    U.panel(c, v.w, v.h, null);
    var g = c.createRadialGradient(v.w / 2, 90, 10, v.w / 2, 90, 120); g.addColorStop(0, 'rgba(255,214,90,0.9)'); g.addColorStop(1, 'rgba(255,214,90,0)');
    c.fillStyle = g; c.fillRect(0, 0, v.w, 200);
    icons.draw(c, 'xp', v.w / 2 - 56, 26, 112);
    U.textOutlined(c, newLv, v.w / 2, 84, 34, '#fff', '#b35c00');
    U.textOutlined(c, '岛屿升到 ' + newLv + ' 级！', v.w / 2, 160, 22, '#ff9f1c', '#fff');
    var unlocked = D.BUILD_ORDER.filter(function (t) { return D.BUILDINGS[t].unlock === newLv; });
    U.text(c, unlocked.length ? '解锁：' + unlocked.map(function (t) { return D.BUILDINGS[t].name; }).join('、') : '更多建筑在后面的等级等着你', v.w / 2, 194, 13, T.ink, 'center', true);
    U.costRow(c, icons, { gem: 10 + newLv * 5 }, v.w / 2, 226, 15, null, 'center');
    v.hit('ok', v.w / 2 - 90, 254, 180, 56); U.button(c, v.w / 2 - 90, 254, 180, 56, '太棒了', T.green);
  }, function (id, v) { if (id === 'ok') { G.reward(G.eco.add({ gem: 10 + newLv * 5 }, 'levelup'), '升级奖励 +'); v.hide(); G.save(); } }, { sticky: true, z: 35 });

  // ───────── 扩地 ─────────
  var exp = null;
  var expand = dialog('expand', 280, function (c, v) {
    if (!exp) return;
    U.panel(c, v.w, v.h, '开拓新地块', T.green); U.closeBtn(c, icons, v);
    icons.draw(c, 'island', v.w / 2 - 50, 60, 100);
    U.text(c, '清理这块地，就能在上面盖房子', v.w / 2, 172, 14, T.ink, 'center', true);
    U.costRow(c, icons, exp.cost, v.w / 2, 198, 16, G.state.res, 'center');
    v.hit('ok', v.w / 2 - 90, 214, 180, 54); U.button(c, v.w / 2 - 90, 214, 180, 54, '开拓', G.eco.has(exp.cost) ? T.green : T.gray);
  }, function (id, v) { if (id === 'ok') { if (G.act.expand(exp.x, exp.z)) v.hide(); } });

  // ───────── 卖货 ─────────
  var qty = {};
  var sell = dialog('sell', 520, function (c, v) {
    U.panel(c, v.w, v.h, '集市', T.red); U.closeBtn(c, icons, v);
    var total = 0;
    D.GOODS.forEach(function (k, i) {
      var y = 66 + i * 62, have = Math.floor(G.state.res[k] || 0), n = Math.min(qty[k] || 0, have);
      qty[k] = n; total += n * D.RES[k].sell;
      U.card(c, 12, y, v.w - 24, 56, n > 0);
      icons.draw(c, k, 18, y + 4, 48);
      U.text(c, D.RES[k].name, 70, y + 18, 14, T.ink, 'left', true);
      icons.draw(c, 'coin', 70 + 32, y + 9, 18); U.text(c, D.RES[k].sell, 70 + 50, y + 18, 11, '#c46a00', 'left', true);
      U.text(c, '库存 ' + E.fmt(have), 70, y + 38, 11, T.ink2);
      var bx = v.w - 196;
      v.hit('-:' + k, bx, y + 10, 36, 36); U.button(c, bx, y + 10, 36, 36, '−', T.gray, { size: 20, disabled: n <= 0 });
      U.text(c, E.fmt(n), bx + 62, y + 27, 15, T.ink, 'center', true);
      v.hit('+:' + k, bx + 88, y + 10, 36, 36); U.button(c, bx + 88, y + 10, 36, 36, '+', T.blue, { size: 20, disabled: n >= have });
      v.hit('all:' + k, bx + 130, y + 10, 48, 36); U.button(c, bx + 130, y + 10, 48, 36, '全部', T.orange, { size: 12, disabled: have <= 0 });
    });
    U.text(c, '合计', 20, 452, 15, T.ink, 'left', true);
    U.costRow(c, icons, { coin: total }, 64, 452, 17);
    v.hit('sell', v.w - 150, 428, 134, 52); U.button(c, v.w - 150, 428, 134, 52, '出售', total > 0 ? T.green : T.gray);
  }, function (id, v) {
    var parts = id.split(':'), k = parts[1], have = Math.floor(G.state.res[k] || 0), step = Math.max(1, Math.round(have / 10));
    if (parts[0] === '-') qty[k] = Math.max(0, (qty[k] || 0) - step);
    else if (parts[0] === '+') qty[k] = Math.min(have, (qty[k] || 0) + step);
    else if (parts[0] === 'all') qty[k] = have;
    else if (id === 'sell') {
      var cost = {}, coins = 0;
      D.GOODS.forEach(function (g) { if (qty[g] > 0) { cost[g] = qty[g]; coins += qty[g] * D.RES[g].sell; } });
      if (coins > 0 && G.eco.spend(cost, 'sell')) {
        G.eco.add({ coin: coins }, 'sell'); G.bus.emit('sold', coins); G.eco.stat('sell');
        G.reward({ coin: coins }, '卖出 +'); qty = {}; G.sfx('buy'); G.save();
      }
    }
    v.markDirty();
  });

  // ───────── 设置 ─────────
  var settings = dialog('settings', 500, function (c, v) {
    var st = G.state.settings;
    U.panel(c, v.w, v.h, '设置', T.purple); U.closeBtn(c, icons, v);
    [['music', '背景音乐'], ['sfx', '音效'], ['vibrate', '振动']].forEach(function (r, i) {
      var y = 70 + i * 56;
      U.text(c, r[1], 24, y + 22, 16, T.ink, 'left', true);
      var on = st[r[0]], x = v.w - 90;
      c.fillStyle = on ? '#34b24a' : '#c9b9a0'; U.rr(c, x, y + 8, 66, 30, 15); c.fill();
      c.fillStyle = '#fff'; c.beginPath(); c.arc(on ? x + 51 : x + 15, y + 23, 12, 0, Math.PI * 2); c.fill();
      v.hit('t:' + r[0], x - 10, y, 86, 46);
    });
    U.text(c, '画质', 24, 262, 16, T.ink, 'left', true);
    ['low', 'mid', 'high', 'auto'].forEach(function (q, i) {
      var x = 90 + i * ((v.w - 106) / 4), w = (v.w - 106) / 4 - 6, on = st.quality === q;
      c.fillStyle = on ? '#9b5de5' : '#f3e3cc'; U.rr(c, x, 246, w, 32, 12); c.fill();
      U.text(c, { low: '流畅', mid: '均衡', high: '精美', auto: '自动' }[q], x + w / 2, 262, 13, on ? '#fff' : T.ink2, 'center', true);
      v.hit('q:' + q, x, 242, w, 40);
    });
    var bw = (v.w - 36) / 2;
    v.hit('redeem', 12, 300, bw, 50); U.button(c, 12, 300, bw, 50, '兑换码', T.blue, { icon: 'gift', icons: icons, size: 15 });
    v.hit('login', 24 + bw, 300, bw, 50); U.button(c, 24 + bw, 300, bw, 50, '登录', T.green, { size: 15 });
    v.hit('reset', 12, 360, v.w - 24, 46); U.button(c, 12, 360, v.w - 24, 46, '清空存档重新开始', T.red, { size: 15 });
    U.text(c, '存档 v' + G.saveVersion + ' · 已玩 ' + E.fmtTime((Date.now() - G.state.created) / 1000), v.w / 2, 428, 11, T.ink2, 'center');
    U.text(c, G.loginCode ? '登录 code：' + G.loginCode : '素材全部由代码生成 · three.js r160', v.w / 2, 448, 11, T.mute, 'center');
  }, function (id, v) {
    var st = G.state.settings;
    if (id.indexOf('t:') === 0) { st[id.slice(2)] = !st[id.slice(2)]; G.applySettings(); G.save(); }
    else if (id.indexOf('q:') === 0) { st.quality = id.slice(2); G.applySettings(); G.save(); G.ui.toast('画质：' + { low: '流畅', mid: '均衡', high: '精美', auto: '自动' }[st.quality]); }
    else if (id === 'redeem') G.askCode(function (code) { var r = G.shop.redeem(code); if (r.ok) G.reward(r.reward, '兑换成功 +'); else G.ui.toast(r.msg); v.markDirty(); });
    else if (id === 'login') wx.login({ success: function (r) { G.loginCode = r.code; G.ui.toast('登录成功'); v.markDirty(); }, fail: function (e) { G.ui.toast('登录失败：' + (e.errMsg || '')); } });
    else if (id === 'reset') wx.showModal({ title: '清空存档', content: '岛上的一切都会消失，确定吗？', confirmText: '清空', confirmColor: '#e5383b', success: function (r) { if (r.confirm) G.reset(); } });
    v.markDirty();
  });

  return {
    signin: function () { signin.markDirty(); signin.show(); },
    boost: function () { boost.markDirty(); boost.show(); },
    offline: function (a) { away = a; offline.markDirty(); offline.show(); },
    levelup: function (lv) { newLv = lv; levelup.markDirty(); levelup.show(); G.sfx('levelup'); },
    expand: function (x, z, cost) { exp = { x: x, z: z, cost: cost }; expand.markDirty(); expand.show(); },
    sell: function () { qty = {}; sell.markDirty(); sell.show(); },
    settings: function () { settings.markDirty(); settings.show(); },
    step: function () {
      // 签到页上没有按秒走的东西（v4 之前每秒整页重画一次，白白卡 30ms）；跨天由 onShow / 领取时重画
      if (boost.shown) { var left = (G.state.boostUntil - Date.now()) / 1000; if ((left > 0) !== boostOn || (left > 3.5 * 3600) !== boostFull) boost.markDirty(); }
    }
  };
}

module.exports = { make: make };
