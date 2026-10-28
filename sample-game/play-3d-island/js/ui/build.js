// 建筑相关的界面：
// - 选中建筑的底部面板：等级、产量 / 库存 / 上限、收获、升级（费用对比当前 → 下一级）、施工加速（钻石）、旋转、集市进卖货、小屋进宠物；
// - 建造菜单（可滚动列表，3D 缩略图，等级没到的锁着、钱不够的标红）；
// - 摆放条（拖地图挪虚影 / 点地格，确认 / 旋转 / 取消）。
var D = require('../data/defs.js');
var E = require('../core/economy.js');
var U = require('./draw.js');

function make(G) {
  var W = G.W, H = G.H, ui = G.ui, icons = G.icons, T = U.T;
  function thumb(c, type, lv, x, y, s) { if (!(G.thumbs && G.thumbs.draw(c, G.thumbs.building(type, lv), x, y, s))) icons.draw(c, 'island', x, y, s); }

  // ───────── 选中建筑面板 ─────────
  var PH = 236, sel = null;
  var info = ui.view('building', { x: 8, y: H - PH - 92 - G.safeBottom, w: W - 16, h: PH, z: 5, anim: 'up', draw: function (c, v) {
    var b = sel; if (!b) return;
    var d = D.BUILDINGS[b.type], now = Date.now();
    U.panel(c, v.w, v.h, null);
    thumb(c, b.type, b.lv, 10, 6, 92);
    U.text(c, d.name, 106, 26, 20, T.ink, 'left', true);
    var lvw = U.tag(c, 106 + measure(c, d.name, 20) + 8, 17, 'Lv.' + b.lv + (b.lv >= d.maxLv ? ' 满级' : ''), b.lv >= d.maxLv ? '#f08c00' : '#4ea8de');
    U.wrap(c, d.desc, 106, 50, v.w - 160, 12, T.ink2, 16, 2);
    U.closeBtn(c, icons, v);
    var y = 102;
    if (b.busyKind) {
      // 施工中：进度条 + 剩余时间 + 加速
      var total = b.busyKind === 'build' ? Math.max(1, b.busyTotal || 1) : G.eco.upgradeTime({ type: b.type, lv: b.lv }), left = (b.busyUntil - now) / 1000;
      U.text(c, b.busyKind === 'build' ? '建造中…' : '升级到 Lv.' + (b.lv + 1) + ' 中…', 16, y + 6, 14, T.ink, 'left', true);
      U.bar(c, 16, y + 20, v.w - 32, 18, 1 - left / total, T.orange);
      U.text(c, E.fmtTime(left), v.w / 2, y + 29, 12, '#fff', 'center', true);
      var sk = E.skipCost(left);
      v.hit('skip', 16, y + 52, v.w - 32, 50);
      U.button(c, 16, y + 52, v.w - 32, 50, '立即完成  ' + sk, T.purple, { icon: 'gem', icons: icons, size: 17 });
      return;
    }
    if (d.produce) {
      var rate = G.eco.rate(b, now), cap = G.eco.storeCap(b), st = Math.floor(b.store);
      U.text(c, '产量 ' + (rate >= 10 ? Math.round(rate) : rate.toFixed(1)) + ' / 分钟', 16, y + 4, 13, T.ink, 'left', true);
      var mult = G.eco.globalMult(now, d.produce);
      if (mult > 1.001) U.text(c, '加成 ×' + mult.toFixed(2), v.w - 16, y + 4, 12, '#2f9e44', 'right', true);
      icons.draw(c, d.produce, 12, y + 14, 30);
      U.bar(c, 46, y + 20, v.w - 62, 18, st / cap, st >= cap ? T.red : T.green);
      U.text(c, st + ' / ' + cap + (st >= cap ? '  已满，快收！' : ''), 46 + (v.w - 62) / 2, y + 29, 11, '#fff', 'center', true);
      if (d.consume) U.text(c, '消耗：每个 ' + Object.keys(d.consume).map(function (k) { return d.consume[k] + ' ' + D.RES[k].name; }).join('、'), 16, y + 50, 11, T.ink2);
    } else {
      var line = d.storage ? '全岛货物上限 +' + Math.round(d.storage * b.lv * 100) + '%' : d.market ? '点「卖货」把货物换成金币' : d.pets ? '可住宠物 ' + b.lv + ' 只（已住 ' + G.gacha.homeCount() + '）' : d.boost ? '全岛产量 +' + Math.round(d.boost * b.lv * 100) + '%' : '';
      U.text(c, line, 16, y + 8, 14, T.ink, 'left', true);
    }
    // 底部按钮：收获 / 特殊 + 升级 + 旋转
    var by = PH - 72, bw = (v.w - 32 - 54) / 2;
    if (d.produce) { v.hit('collect', 16, by, bw, 54); U.button(c, 16, by, bw, 54, '收获', T.green, { disabled: b.store < 1, icon: d.produce, icons: icons, size: 17 }); }
    else if (d.market) { v.hit('sell', 16, by, bw, 54); U.button(c, 16, by, bw, 54, '卖货', T.orange, { icon: 'coin', icons: icons, size: 17 }); }
    else if (d.pets) { v.hit('pets', 16, by, bw, 54); U.button(c, 16, by, bw, 54, '宠物', T.orange, { icon: 'paw', icons: icons, size: 17 }); }
    if (b.lv < d.maxLv) {
      var cost = G.eco.upgradeCost(b), ok = G.eco.has(cost);
      v.hit('upgrade', 24 + bw, by, bw, 54);
      U.button(c, 24 + bw, by, bw, 54, '', ok ? T.blue : T.gray);
      // 与其他按钮的字一样带描边：白字直接压在按钮上半截的高光带上，对比度不够，看着发灰
      U.textOutlined(c, '升级', 24 + bw + bw / 2, by + 14, 13, '#fff', (ok ? T.blue : T.gray)[2]);
      U.costRow(c, icons, cost, 24 + bw + bw / 2, by + 34, 12, G.state.res, 'center');
    }
    v.hit('rotate', v.w - 62, by, 48, 54);
    U.button(c, v.w - 62, by, 48, 54, '↻', T.purple, { size: 22 });
  }, onTap: function (id) {
    var b = sel; if (!b) return;
    if (id === 'close') G.select(null);
    else if (id === 'collect') G.act.collect(b);
    else if (id === 'upgrade') G.act.upgrade(b);
    else if (id === 'skip') G.act.skip(b);
    else if (id === 'rotate') G.act.rotate(b);
    else if (id === 'sell') G.open('sell');
    else if (id === 'pets') G.open('pets', 'book');
  } });
  function measure(c, s, size) { U.font(c, size, true); return c.measureText(s).width; }

  // ───────── 建造菜单 ─────────
  // 外框（不滚：标题带、关闭钮、四边的框一直都在）+ 里面一块滚动列表（跟着外框一起滑入）。
  // v3 是整块面板跟着内容一起滚：标题滚走、下沿被视图边缘一刀切开没有框，底下和两边露出一截乱糟糟的 3D 场景
  var ROW = 108, MY = G.safeTop + 56, MH = H - MY - 90 - G.safeBottom, HEAD = 62;
  var menu = ui.view('buildmenu', { x: 8, y: MY, w: W - 16, h: MH, z: 20, modal: true, anim: 'up',
    onBackdrop: function () { menu.hide(); }, draw: function (c, v) {
      U.panel(c, v.w, v.h, '建造');
      U.closeBtn(c, icons, v);
      c.fillStyle = 'rgba(167,116,63,0.18)'; U.rr(c, 10, HEAD - 2, v.w - 20, v.h - HEAD - 12, 14); c.fill();   // 列表槽：略深的内凹底
    }, onTap: function (id) { if (id === 'close') menu.hide(); } });
  var list = ui.view('buildlist', { x: 14, y: MY + HEAD, w: W - 28, h: MH - HEAD - 18, z: 21, follow: menu, draw: function (c, v) {
      var types = D.BUILD_ORDER, s = G.state;
      v.setContent(6 + types.length * ROW);
      c.fillStyle = '#f7ead2'; c.fillRect(0, 0, v.w, v.contentH);
      types.forEach(function (t, i) {
        var d = D.BUILDINGS[t], y = 6 + i * ROW, locked = s.lv < d.unlock;
        var count = s.buildings.filter(function (b) { return b.type === t; }).length, maxed = d.unique && count >= 1;
        var cost = G.eco.buildCost(t), ok = !locked && !maxed && G.eco.has(cost);
        U.card(c, 4, y, v.w - 8, ROW - 8, ok);
        thumb(c, t, 1, 10, y + 4, 92);
        if (locked) { c.fillStyle = "rgba(255,248,236,0.7)"; c.fillRect(10, y + 4, 92, 92); icons.draw(c, "lock", 36, y + 30, 40); }
        U.text(c, d.name, 110, y + 22, 17, locked ? T.mute : T.ink, 'left', true);
        if (count) U.tag(c, 110 + measure(c, d.name, 17) + 6, y + 13, '已有 ' + count, '#4ea8de');
        U.wrap(c, d.desc, 110, y + 44, v.w - 236, 11, T.ink2, 14, 2);
        if (locked) U.text(c, '岛屿 ' + d.unlock + ' 级解锁', 110, y + 80, 13, '#e5383b', 'left', true);
        else if (maxed) U.text(c, '只能建一座', 110, y + 80, 13, T.ink2, 'left', true);
        else U.costRow(c, icons, cost, 110, y + 80, 13, s.res);
        if (!locked && !maxed) { v.hit('b:' + t, v.w - 98, y + 26, 86, 48); U.button(c, v.w - 98, y + 26, 86, 48, '建造', ok ? T.green : T.gray); }
      });
    }, onTap: function (id) {
      if (id.indexOf('b:') === 0) {
        var t = id.slice(2), cost = G.eco.buildCost(t);
        if (!G.eco.has(cost)) { var m = G.eco.missing(cost), k = Object.keys(m)[0]; return G.ui.toast(D.RES[k].name + '不够（还差 ' + E.fmt(m[k]) + '）', k); }
        menu.hide(); G.act.startPlace(t);
      }
    } });

  // ───────── 摆放条 ─────────
  var place = ui.view('place', { x: 10, y: H - 150 - G.safeBottom, w: W - 20, h: 140, z: 6, anim: 'up', draw: function (c, v) {
    var p = G.placing; if (!p) return;
    var d = D.BUILDINGS[p.type];
    U.panel(c, v.w, v.h, null);
    U.text(c, '摆放 ' + d.name, 16, 26, 17, T.ink, 'left', true);
    U.text(c, p.ok ? '拖动地图或点地块挪位置' : p.why, 16, 50, 13, p.ok ? T.ink2 : '#e5383b', 'left', true);
    U.costRow(c, icons, G.eco.buildCost(p.type), v.w - 16, 30, 13, G.state.res, 'right');
    var bw = (v.w - 48) / 3;
    v.hit('cancel', 12, 72, bw, 54); U.button(c, 12, 72, bw, 54, '取消', T.red);
    v.hit('rot', 24 + bw, 72, bw, 54); U.button(c, 24 + bw, 72, bw, 54, '旋转', T.purple);
    v.hit('ok', 36 + bw * 2, 72, bw, 54); U.button(c, 36 + bw * 2, 72, bw, 54, '确定', p.ok ? T.green : T.gray);
  }, onTap: function (id) {
    if (id === 'cancel') G.act.cancelPlace();
    else if (id === 'rot') G.act.rotatePlace();
    else if (id === 'ok') G.act.confirmPlace();
  } });

  var lastSig = '';
  return {
    open: function () { menu.markDirty(); list.markDirty(); list.scrollTo(0); menu.show(); },
    select: function (b) { sel = b; if (b) { info.markDirty(); info.show(); } else info.hide(); },
    showPlace: function (on) { if (on) { place.markDirty(); place.show(); } else place.hide(); },
    step: function () {
      var b = sel, now = Date.now();
      if (b && info.shown) {
        var s = [b.lv, b.busyKind, Math.floor(b.store), b.busyKind ? Math.floor((b.busyUntil - now) / 1000) : 0, G.eco.has(G.eco.upgradeCost(b)), Math.floor(G.eco.rate(b, now) * 10), G.gacha.homeCount()].join(',');
        if (s !== lastSig) { lastSig = s; info.markDirty(); }
      }
      if (G.placing && place.shown && G.placing.dirty) { G.placing.dirty = false; place.markDirty(); }
    },
    refreshMenu: function () { if (menu.shown) list.markDirty(); }
  };
}

module.exports = { make: make };
