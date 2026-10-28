// 常驻界面：顶部资源条（等级 + 经验、金币、钻石、六种货物与上限）、右侧快捷（签到、双倍、新手礼包倒计时、月卡）、底部五个页签（带红点）。
// 只在数字 / 红点 / 倒计时的秒数变了时重画（每个视图自己算一个签名比较）。
var D = require('../data/defs.js');
var E = require('../core/economy.js');
var U = require('./draw.js');

function make(G) {
  var W = G.W, H = G.H, ui = G.ui, icons = G.icons, top = G.safeTop;
  var sig = {}, topAt = 0;
  function changed(v, s) { if (sig[v.id] !== s) { sig[v.id] = s; v.markDirty(); } }

  // ───────── 顶部：两行 ─────────
  // 顶部条按格子画：等级、金币、钻石、六种货物各一格，每格有自己的签名（显示出来的字）。
  // 资源随被动产出一直在涨，整条重画在真机上 7ms 左右，几乎每隔几帧一次，p95 被它抬到 20ms 以上；
  // 现在只擦掉、重画变了的那一两格（kit 的 partial），整条只在第一次和尺寸变了时画。
  var gw = (W - 16) / 6, cw = (W - 144) / 2 - 8, cellSig = {};
  function cells() {
    var s = G.state, out = [{ id: 'lv', x: 0, y: 0, w: 129, h: 46, sig: s.lv + ':' + Math.floor(s.xp / D.xpFor(s.lv) * 100) }];
    [['coin', 136], ['gem', 136 + (W - 144) / 2]].forEach(function (p) { out.push({ id: p[0], x: p[1] - 6, y: 0, w: cw + 8, h: 46, sig: E.fmt(s.res[p[0]]) }); });
    D.GOODS.forEach(function (k, i) { var cap = G.eco.cap(k), n = s.res[k] || 0; out.push({ id: k, i: i, x: 8 + i * gw + 1, y: 48, w: gw - 2, h: 38, sig: E.fmt(n) + '/' + E.fmt(cap) + (n >= cap) }); });
    return out;
  }
  function drawCell(c, v, e) {
    var s = G.state;
    if (e.id === 'lv') {
      c.fillStyle = 'rgba(30,40,70,0.55)'; U.rr(c, 8, 4, 120, 40, 20); c.fill();
      var gr = c.createRadialGradient(28, 18, 4, 28, 24, 20); gr.addColorStop(0, '#ffe066'); gr.addColorStop(1, '#f08c00');
      c.fillStyle = gr; c.beginPath(); c.arc(28, 24, 19, 0, Math.PI * 2); c.fill(); c.lineWidth = 3; c.strokeStyle = '#fff'; c.stroke();
      U.textOutlined(c, s.lv, 28, 25, 17, '#fff', '#b35c00');
      U.bar(c, 52, 17, 68, 12, s.xp / D.xpFor(s.lv), [ '#9be7ff', '#2fa8f0' ]);
      U.text(c, '经验 ' + Math.floor(s.xp / D.xpFor(s.lv) * 100) + '%', 86, 36, 9, '#fff', 'center', true);
    } else if (e.id === 'coin' || e.id === 'gem') {
      var x = e.x + 6;
      c.fillStyle = 'rgba(30,40,70,0.55)'; U.rr(c, x, 8, cw, 32, 16); c.fill();
      icons.draw(c, e.id, x - 6, 2, 42);
      U.text(c, E.fmt(s.res[e.id]), x + 38, 25, 16, '#fff', 'left', true);
      icons.draw(c, 'plus', x + cw - 30, 10, 28);
    } else {
      var gx = 8 + e.i * gw, cap = G.eco.cap(e.id), n = s.res[e.id] || 0;
      c.fillStyle = n >= cap ? 'rgba(200,40,40,0.7)' : 'rgba(30,40,70,0.45)'; U.rr(c, gx + 2, 50, gw - 4, 34, 12); c.fill();
      icons.draw(c, e.id, gx + 2, 50, 32);
      // 四位数（1037）比胶囊里剩的 22px 宽，原来直接画出胶囊、压到隔壁格上：按宽度缩字号
      fit(c, E.fmt(n), gx + 33, 61, 12, gx + gw - 4, '#fff', true);
      fit(c, '/' + E.fmt(cap), gx + 33, 75, 9, gx + gw - 4, 'rgba(255,255,255,0.75)', false);
    }
    cellSig[e.id] = e.sig;
  }
  function fit(c, str, x, y, size, x1, color, bold) {
    U.font(c, size, bold); var w = c.measureText(str).width, k = Math.min(1, (x1 - x) / Math.max(1, w));
    U.text(c, str, x, y, Math.max(7, Math.floor(size * k * 2) / 2), color, 'left', bold);
  }
  // 每格只在自己的矩形里画（字按宽度缩，fit）：局部重画只擦本格，画出格的部分会被隔壁格擦掉、与整画不一致。
  // 不用 clip 兜底：本引擎 2D 画布上 save+clip+restore 一次的开销是同样 fillRect 的二十多倍（桌面实测 300 次 26ms 对 1ms）
  function clipped(c, v, e) { drawCell(c, v, e); }
  var topBar = ui.view('hud-top', { x: 0, y: top, w: W, h: 92, z: 1, draw: function (c, v) {
    cells().forEach(function (e) { clipped(c, v, e); });
    v.hit('level', 8, 4, 120, 40);
    v.hit('shop:coins', 136, 4, cw, 40); v.hit('shop:gems', 136 + (W - 144) / 2, 4, cw, 40);
    D.GOODS.forEach(function (k, i) { v.hit('good:' + k, 8 + i * gw, 48, gw, 38); });
  }, partial: function (c, v) {
    cells().forEach(function (e) { if (cellSig[e.id] !== e.sig) { c.clearRect(e.x, e.y, e.w, e.h); clipped(c, v, e); } });
    return true;
  }, onTap: function (id) {
    if (id === 'level') G.ui.toast('岛屿等级 ' + G.state.lv + '：升级解锁新建筑', 'xp');
    else if (id.indexOf('shop:') === 0) G.open('shop', id.slice(5));
    else if (id.indexOf('good:') === 0) { var k = id.slice(5); G.ui.toast(D.RES[k].name + '：' + E.fmt(G.state.res[k]) + ' / ' + E.fmt(G.eco.cap(k)) + '（仓库能加上限）', k); }
  } });

  // ───────── 右侧快捷按钮（竖排） ─────────
  // 倒计时（双倍、新手礼包）每秒变一次：布局（有哪几项、文字、红点）没变时只擦 / 画那条红色小胶囊（partial），整列不重画
  function sideItems(now) {
    var out = [], si = G.meta.signinInfo(), boost = G.state.boostUntil > now, st = G.shop.starter(), mo = G.shop.monthly();
    out.push({ id: 'signin', icon: 'calendar', label: '签到', sub: null, dot: si.canClaim });
    out.push({ id: 'boost', icon: 'boost', label: boost ? '双倍中' : '双倍', sub: boost ? E.fmtClock((G.state.boostUntil - now) / 1000) : null, dot: !boost });
    if (st.show) out.push({ id: 'starter', icon: 'gift', label: '新手礼包', sub: E.fmtClock(st.left), dot: true });
    if (mo.active) out.push({ id: 'monthly', icon: 'crown', label: '月卡', sub: mo.daysLeft + '天', dot: mo.canClaim });
    return out;
  }
  function sideLayout(list) { return list.map(function (e) { return e.id + e.label + !!e.sub + e.dot; }).join(','); }
  function pill(c, sub, y) { c.fillStyle = 'rgba(229,56,59,0.92)'; U.rr(c, 2, y + 62, 58, 15, 7); c.fill(); U.text(c, sub, 31, y + 70, 9, '#fff', 'center', true); }
  var sideDrawn = { layout: '', subs: [] };
  var side = ui.view('hud-side', { x: W - 66, y: top + 100, w: 62, h: 300, z: 1, draw: function (c, v) {
    var y = 0, list = sideItems(Date.now());
    sideDrawn.layout = sideLayout(list); sideDrawn.subs = [];
    list.forEach(function (e) {
      c.fillStyle = 'rgba(30,40,70,0.45)'; c.beginPath(); c.arc(31, y + 26, 26, 0, Math.PI * 2); c.fill();
      icons.draw(c, e.icon, 7, y + 2, 48);
      if (e.dot) U.badge(c, 52, y + 6);
      U.textOutlined(c, e.label, 31, y + 54, 11, '#fff', 'rgba(20,30,60,0.9)', 'center', 1.5);
      if (e.sub) pill(c, e.sub, y);
      sideDrawn.subs.push({ y: y, sub: e.sub });
      v.hit(e.id, 0, y, 62, e.sub ? 78 : 64);
      y += e.sub ? 84 : 70;
    });
  }, partial: function (c, v) {
    var list = sideItems(Date.now());
    if (sideLayout(list) !== sideDrawn.layout) return false;
    list.forEach(function (e, i) {
      var d = sideDrawn.subs[i];
      if (e.sub && e.sub !== d.sub) { c.clearRect(0, d.y + 61, 62, 18); pill(c, e.sub, d.y); d.sub = e.sub; }
    });
    return true;
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
      // 资源随被动产出一直在涨：签名用显示出来的字（E.fmt），最多 0.25 秒看一次；变了也只重画变了的格子（见 cells）
      if (now - topAt >= 250 || topBar.dirty) {
        topAt = now;
        changed(topBar, [s.lv, Math.floor(s.xp / D.xpFor(s.lv) * 100), E.fmt(s.res.coin), E.fmt(s.res.gem)].concat(D.GOODS.map(function (k) { return E.fmt(s.res[k] || 0) + '/' + G.eco.cap(k); })).join(','));
      }
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
