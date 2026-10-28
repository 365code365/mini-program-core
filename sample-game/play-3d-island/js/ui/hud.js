// 常驻界面：顶部资源条（等级 + 经验、金币、钻石、六种货物与上限）、右侧快捷（签到、双倍、新手礼包倒计时、月卡）、底部五个页签（带红点）。
// 只在数字 / 红点 / 倒计时的秒数变了时重画（每个视图自己算一个签名比较）。
var D = require('../data/defs.js');
var E = require('../core/economy.js');
var U = require('./draw.js');

function make(G) {
  var W = G.W, H = G.H, ui = G.ui, icons = G.icons, top = G.safeTop;
  var sig = {};
  function changed(v, s) { if (sig[v.id] !== s) { sig[v.id] = s; v.markDirty(); } }

  // ───────── 顶部：两行 ─────────
  var topBar = ui.view('hud-top', { x: 0, y: top, w: W, h: 92, z: 1, draw: function (c, v) {
    var s = G.state;
    // 等级徽章
    c.fillStyle = 'rgba(30,40,70,0.55)'; U.rr(c, 8, 4, 120, 40, 20); c.fill();
    var gr = c.createRadialGradient(28, 18, 4, 28, 24, 20); gr.addColorStop(0, '#ffe066'); gr.addColorStop(1, '#f08c00');
    c.fillStyle = gr; c.beginPath(); c.arc(28, 24, 19, 0, Math.PI * 2); c.fill(); c.lineWidth = 3; c.strokeStyle = '#fff'; c.stroke();
    U.textOutlined(c, s.lv, 28, 25, 17, '#fff', '#b35c00');
    U.bar(c, 52, 17, 68, 12, s.xp / D.xpFor(s.lv), [ '#9be7ff', '#2fa8f0' ]);
    U.text(c, '经验 ' + Math.floor(s.xp / D.xpFor(s.lv) * 100) + '%', 86, 36, 9, '#fff', 'center', true);
    v.hit('level', 8, 4, 120, 40);
    // 金币、钻石胶囊
    [['coin', 136, 'shop:coins'], ['gem', 136 + (W - 144) / 2, 'shop:gems']].forEach(function (p) {
      var x = p[1], w = (W - 144) / 2 - 8;
      c.fillStyle = 'rgba(30,40,70,0.55)'; U.rr(c, x, 8, w, 32, 16); c.fill();
      icons.draw(c, p[0], x - 6, 2, 42);
      U.text(c, E.fmt(s.res[p[0]]), x + 38, 25, 16, '#fff', 'left', true);
      icons.draw(c, 'plus', x + w - 30, 10, 28);
      v.hit(p[2], x, 4, w, 40);
    });
    // 货物：一行 6 个（满了标红）
    var gw = (W - 16) / 6;
    D.GOODS.forEach(function (k, i) {
      var x = 8 + i * gw, cap = G.eco.cap(k), n = s.res[k] || 0, full = n >= cap;
      c.fillStyle = full ? 'rgba(200,40,40,0.7)' : 'rgba(30,40,70,0.45)'; U.rr(c, x + 2, 50, gw - 4, 34, 12); c.fill();
      icons.draw(c, k, x + 2, 50, 32);
      U.text(c, E.fmt(n), x + 34, 61, 12, '#fff', 'left', true);
      U.text(c, '/' + E.fmt(cap), x + 34, 75, 9, 'rgba(255,255,255,0.75)', 'left');
      v.hit('good:' + k, x, 48, gw, 38);
    });
  }, onTap: function (id) {
    if (id === 'level') G.ui.toast('岛屿等级 ' + G.state.lv + '：升级解锁新建筑', 'xp');
    else if (id.indexOf('shop:') === 0) G.open('shop', id.slice(5));
    else if (id.indexOf('good:') === 0) { var k = id.slice(5); G.ui.toast(D.RES[k].name + '：' + E.fmt(G.state.res[k]) + ' / ' + E.fmt(G.eco.cap(k)) + '（仓库能加上限）', k); }
  } });

  // ───────── 右侧快捷按钮（竖排） ─────────
  var side = ui.view('hud-side', { x: W - 66, y: top + 100, w: 62, h: 300, z: 1, draw: function (c, v) {
    var y = 0, now = Date.now();
    function item(id, icon, label, sub, dot) {
      c.fillStyle = 'rgba(30,40,70,0.45)'; c.beginPath(); c.arc(31, y + 26, 26, 0, Math.PI * 2); c.fill();
      icons.draw(c, icon, 7, y + 2, 48);
      if (dot) U.badge(c, 52, y + 6);
      U.textOutlined(c, label, 31, y + 54, 11, '#fff', 'rgba(20,30,60,0.9)', 'center', 1.5);
      if (sub) { c.fillStyle = 'rgba(229,56,59,0.92)'; U.rr(c, 2, y + 62, 58, 15, 7); c.fill(); U.text(c, sub, 31, y + 70, 9, '#fff', 'center', true); }
      v.hit(id, 0, y, 62, sub ? 78 : 64);
      y += sub ? 84 : 70;
    }
    var si = G.meta.signinInfo();
    item('signin', 'calendar', '签到', null, si.canClaim);
    var boost = G.state.boostUntil > now;
    item('boost', 'boost', boost ? '双倍中' : '双倍', boost ? E.fmtClock((G.state.boostUntil - now) / 1000) : null, !boost);
    var st = G.shop.starter();
    if (st.show) item('starter', 'gift', '新手礼包', E.fmtClock(st.left), true);
    var mo = G.shop.monthly();
    if (mo.active) item('monthly', 'crown', '月卡', mo.daysLeft + '天', mo.canClaim);
  }, onTap: function (id) {
    if (id === 'signin') G.open('signin');
    else if (id === 'boost') G.open('boost');
    else if (id === 'starter') G.open('shop', 'deals');
    else if (id === 'monthly') G.open('shop', 'deals');
  } });

  // ───────── 底部页签 ─────────
  var TABS = [['build', 'hammer', '建造'], ['quests', 'scroll', '任务'], ['shop', 'bag', '商店'], ['pets', 'paw', '宠物'], ['settings', 'gear', '设置']];
  var bottom = ui.view('hud-bottom', { x: 0, y: H - 84 - G.safeBottom, w: W, h: 84, z: 1, draw: function (c, v) {
    var g = c.createLinearGradient(0, 10, 0, 84); g.addColorStop(0, 'rgba(255,248,236,0.96)'); g.addColorStop(1, 'rgba(255,236,206,0.98)');
    c.fillStyle = 'rgba(80,45,10,0.25)'; U.rr(c, 6, 14, W - 12, 70, 26); c.fill();
    c.fillStyle = g; U.rr(c, 6, 10, W - 12, 70, 26); c.fill();
    c.lineWidth = 3; c.strokeStyle = '#e8cfa6'; c.stroke();
    var b = G.meta.badges(), tw = (W - 12) / TABS.length;
    TABS.forEach(function (t, i) {
      var x = 6 + i * tw, cx = x + tw / 2;
      icons.draw(c, t[1], cx - 24, 4, 48);
      U.text(c, t[2], cx, 64, 12, U.T.ink, 'center', true);
      if ((t[0] === 'quests' && b.quests) || (t[0] === 'pets' && G.state.res.gem >= D.GACHA.single)) U.badge(c, cx + 20, 10);
      v.hit(t[0], x, 0, tw, 84);
    });
  }, onTap: function (id) { G.open(id); } });

  topBar.show(); side.show(); bottom.show();
  return {
    // 每帧：按签名决定要不要重画（倒计时每秒变一次）
    step: function () {
      var s = G.state, now = Date.now();
      changed(topBar, [s.lv, s.xp, Math.floor(s.res.coin), Math.floor(s.res.gem)].concat(D.GOODS.map(function (k) { return Math.floor(s.res[k]) + '/' + G.eco.cap(k); })).join(','));
      var st = G.shop.starter(), mo = G.shop.monthly(), si = G.meta.signinInfo();
      changed(side, [si.canClaim, Math.floor(Math.max(0, s.boostUntil - now) / 1000), st.show ? Math.floor(st.left) : -1, mo.active, mo.canClaim, mo.daysLeft].join(','));
      var b = G.meta.badges();
      changed(bottom, [b.quests, s.res.gem >= D.GACHA.single].join(','));
    },
    // 货物在顶部条上的位置（收获时图标飞过去）
    slot: function (res) {
      if (res === 'coin') return { x: 150, y: top + 24 };
      if (res === 'gem') return { x: 150 + (W - 144) / 2, y: top + 24 };
      var i = D.GOODS.indexOf(res), gw = (W - 16) / 6;
      return { x: 8 + i * gw + 18, y: top + 66 };
    },
    hide: function () { topBar.hide(); side.hide(); bottom.hide(); },
    show: function () { topBar.show(); side.show(); bottom.show(); }
  };
}

module.exports = { make: make };
