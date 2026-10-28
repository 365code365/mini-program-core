// 宠物：两个页签 —— 孵化（单抽 / 十连、保底剩余次数、概率公示）与图鉴（全部宠物：没有的是剪影，有的显示加成、数量、入住开关）。
// 抽卡结果是单独的弹层：卡片一张张翻开（按稀有度上色，传说级金光），十连 2×5。
var D = require('../data/defs.js');
var E = require('../core/economy.js');
var U = require('./draw.js');

function make(G) {
  var W = G.W, H = G.H, ui = G.ui, icons = G.icons, T = U.T, tab = 'hatch';
  // 没有的宠物：半透明的蛋 + 问号（不用 source-atop 做剪影：本引擎的 2D 画布上它会把整个矩形涂满，不只涂不透明的像素）
  function petThumb(c, id, x, y, s, owned) {
    if (!owned) {
      c.globalAlpha = 0.45; icons.draw(c, 'egg', x + s * 0.1, y + s * 0.05, s * 0.8); c.globalAlpha = 1;
      U.textOutlined(c, '?', x + s / 2, y + s * 0.48, s * 0.32, '#fff', '#b8a990');
      return;
    }
    if (!(G.thumbs && G.thumbs.draw(c, G.thumbs.pet(id), x, y, s))) icons.draw(c, 'paw', x, y, s);
  }
  var v = ui.view('pets', { x: 10, y: G.safeTop + 40, w: W - 20, h: H - G.safeTop - 40 - 96 - G.safeBottom, z: 20, modal: true, anim: 'up',
    onBackdrop: function () { v.hide(); }, draw: function (c, v) {
      var s = G.state;
      U.panel(c, v.w, v.contentH, '萌宠', T.green);
      U.closeBtn(c, icons, v);
      var tw = (v.w - 24) / 2;
      [['hatch', '孵化'], ['book', '图鉴']].forEach(function (t, i) {
        var on = t[0] === tab; c.fillStyle = on ? '#34b24a' : '#f3e3cc'; U.rr(c, 14 + i * tw, 62, tw - 4, 38, 14); c.fill();
        U.text(c, t[1], 12 + i * tw + tw / 2, 81, 15, on ? '#fff' : T.ink2, 'center', true); v.hit('tab:' + t[0], 12 + i * tw, 62, tw, 40);
      });
      if (tab === 'hatch') {
        v.setContent(v.h);
        var cy = 220, bob = 0;
        var g = c.createRadialGradient(v.w / 2, cy, 10, v.w / 2, cy, 130); g.addColorStop(0, 'rgba(255,230,140,0.9)'); g.addColorStop(1, 'rgba(255,230,140,0)');
        c.fillStyle = g; c.beginPath(); c.arc(v.w / 2, cy, 130, 0, Math.PI * 2); c.fill();
        icons.draw(c, 'egg', v.w / 2 - 80, cy - 90 + bob, 160);
        U.text(c, '再孵 ' + G.gacha.pityLeft() + ' 次必出传说宠物', v.w / 2, cy + 96, 14, '#9b5de5', 'center', true);
        U.text(c, '已拥有 ' + D.PETS.filter(function (p) { return s.pets[p.id] && s.pets[p.id].n; }).length + ' / ' + D.PETS.length, v.w / 2, cy + 120, 12, T.ink2, 'center');
        var by = cy + 150, bw = (v.w - 40) / 2;
        v.hit('one', 14, by, bw, 64); U.button(c, 14, by, bw, 64, '', s.res.gem >= D.GACHA.single ? T.blue : T.gray);
        U.text(c, '孵 1 次', 14 + bw / 2, by + 18, 15, '#fff', 'center', true); U.costRow(c, icons, { gem: D.GACHA.single }, 14 + bw / 2, by + 40, 13, null, 'center');
        v.hit('ten', 26 + bw, by, bw, 64); U.button(c, 26 + bw, by, bw, 64, '', s.res.gem >= D.GACHA.ten ? T.orange : T.gray);
        U.text(c, '孵 10 次（9 折）', 26 + bw + bw / 2, by + 18, 15, '#fff', 'center', true); U.costRow(c, icons, { gem: D.GACHA.ten }, 26 + bw + bw / 2, by + 40, 13, null, 'center');
        U.text(c, '十连必出史诗以上', 26 + bw + bw / 2, by + 78, 11, '#e5383b', 'center', true);
        v.hit('rates', v.w / 2 - 60, by + 92, 120, 30); U.text(c, '概率公示 ›', v.w / 2, by + 107, 13, '#4ea8de', 'center', true);
        return;
      }
      // 图鉴：3 列
      var cols = 3, cw = (v.w - 24 - (cols - 1) * 8) / cols, ch = 176, home = G.gacha.homeCount(), maxHome = G.eco.maxPets();
      v.setContent(150 + Math.ceil(D.PETS.length / cols) * (ch + 8) + 10);
      U.text(c, '小屋 ' + home + ' / ' + maxHome + (maxHome ? '' : '（先造宠物小屋）'), 18, 122, 14, T.ink, 'left', true);
      U.text(c, '住进小屋的宠物才有加成', v.w - 18, 122, 11, T.ink2, 'right');
      D.PETS.forEach(function (p, i) {
        var x = 12 + (i % cols) * (cw + 8), y = 142 + Math.floor(i / cols) * (ch + 8), o = s.pets[p.id], own = o && o.n > 0, R = D.RARITY[p.rarity];
        c.fillStyle = own ? R.color : '#d9cbb5'; U.rr(c, x, y, cw, ch, 14); c.fill();
        c.fillStyle = own ? '#fffdf8' : '#efe4d2'; U.rr(c, x + 3, y + 3, cw - 6, ch - 6, 12); c.fill();
        petThumb(c, p.id, x + (cw - 86) / 2, y + 6, 86, own);
        U.tag(c, x + 6, y + 6, R.name, own ? R.color : '#b8a990');
        U.text(c, own ? p.name : '？？？', x + cw / 2, y + 102, 13, own ? T.ink : T.mute, 'center', true);
        var bonusName = p.bonus === 'all' ? '全部' : D.RES[p.bonus].name;
        U.text(c, bonusName + ' +' + p.pct + '%', x + cw / 2, y + 120, 11, own ? '#2f9e44' : T.mute, 'center', true);
        if (own) {
          if (o.n > 1) U.text(c, '×' + o.n, x + cw - 10, y + 16, 12, T.ink2, 'right', true);
          v.hit('home:' + p.id, x + 8, y + 132, cw - 16, 36);
          U.button(c, x + 8, y + 132, cw - 16, 36, o.home ? '已入住' : '入住', o.home ? T.green : (home < maxHome ? T.blue : T.gray), { size: 13 });
        }
      });
    }, onTap: function (id) {
      if (id === 'close') return v.hide();
      if (id.indexOf('tab:') === 0) { tab = id.slice(4); v.scrollTo(0); return v.markDirty(); }
      if (id === 'one' || id === 'ten') {
        var r = G.gacha.pull(id === 'ten' ? 10 : 1);
        if (!r) { G.ui.toast('钻石不够', 'gem'); G.open('shop', 'gems'); return; }
        showResult(r); G.save(); v.markDirty();
      } else if (id === 'rates') {
        var rt = G.gacha.rates(), lines = Object.keys(rt).map(function (k) { return D.RARITY[k].name + ' ' + rt[k].pct.toFixed(1) + '%：' + rt[k].pets.join('、'); });
        wx.showModal({ title: '概率公示', content: lines.join('\n') + '\n\n连续 ' + D.GACHA.pity + ' 次未出传说，下一次必出；十连至少 1 只史诗或以上。重复获得的宠物转换成金币。', showCancel: false });
      } else if (id.indexOf('home:') === 0) {
        var pid = id.slice(5), o = G.state.pets[pid];
        if (!G.gacha.setHome(pid, !o.home)) G.ui.toast(G.eco.maxPets() ? '小屋住满了（升级小屋多住一只）' : '先造一座宠物小屋', 'paw');
        else G.save();
        v.markDirty();
      }
    } });

  // ───────── 抽卡结果 ─────────
  var results = null, revealT = 0, revealed = 0;
  var rv = ui.view('gacha-result', { x: 10, y: G.safeTop + 80, w: W - 20, h: 470, z: 40, modal: true, anim: 'pop', draw: function (c, v) {
    if (!results) return;
    U.panel(c, v.w, v.h, results.length > 1 ? '十连结果' : '孵化结果', T.orange);
    var cols = results.length > 1 ? 5 : 1, cw = results.length > 1 ? (v.w - 24 - 4 * 6) / 5 : 180, ch = results.length > 1 ? 150 : 270;
    results.forEach(function (r, i) {
      var x = results.length > 1 ? 12 + (i % cols) * (cw + 6) : (v.w - cw) / 2, y = 70 + Math.floor(i / cols) * (ch + 10), R = D.RARITY[r.rarity];
      if (i >= revealed) { c.fillStyle = '#e8cfa6'; U.rr(c, x, y, cw, ch, 12); c.fill(); icons.draw(c, 'egg', x + cw / 2 - cw * 0.35, y + ch / 2 - cw * 0.35, cw * 0.7); return; }
      if (r.rarity === 'SSR' || r.rarity === 'SR') {
        var g = c.createRadialGradient(x + cw / 2, y + ch * 0.4, 4, x + cw / 2, y + ch * 0.4, cw * 0.9);
        g.addColorStop(0, r.rarity === 'SSR' ? 'rgba(255,215,0,0.95)' : 'rgba(190,120,255,0.9)'); g.addColorStop(1, 'rgba(255,255,255,0)');
        c.fillStyle = g; c.fillRect(x - 6, y - 6, cw + 12, ch + 12);
      }
      c.fillStyle = R.color; U.rr(c, x, y, cw, ch, 12); c.fill();
      c.fillStyle = '#fffdf8'; U.rr(c, x + 3, y + 3, cw - 6, ch - 6, 10); c.fill();
      var ts = Math.min(cw - 8, ch * 0.62);
      petThumb(c, r.pet.id, x + (cw - ts) / 2, y + 6, ts, true);
      U.text(c, R.name, x + cw / 2, y + ts + 16, results.length > 1 ? 11 : 15, R.color, 'center', true);
      U.text(c, r.pet.name, x + cw / 2, y + ts + (results.length > 1 ? 32 : 40), results.length > 1 ? 10 : 18, T.ink, 'center', true);
      if (r.dupe) U.text(c, '重复 → ' + E.fmt(r.refund) + ' 金币', x + cw / 2, y + ch - 14, results.length > 1 ? 9 : 13, '#f08c00', 'center', true);
      else U.tag(c, x + 4, y + 4, '新', '#ff3b30');
    });
    var done = revealed >= results.length;
    v.hit('ok', v.w / 2 - 90, v.h - 76, 180, 56);
    U.button(c, v.w / 2 - 90, v.h - 76, 180, 56, done ? '收下' : '全部翻开', done ? T.green : T.blue);
  }, onTap: function (id) {
    if (id !== 'ok') return;
    if (revealed < results.length) { revealed = results.length; rv.markDirty(); return; }
    rv.hide(); results = null;
  } });
  function showResult(r) {
    results = r; revealed = 0; revealT = 0; rv.markDirty(); rv.show();
    if (r.some(function (x) { return x.rarity === 'SSR'; })) { G.sfx('ssr'); G.vibrate(true); } else G.sfx('hatch');
  }

  return {
    open: function (t) { if (t) tab = t; v.markDirty(); v.scrollTo(0); v.show(); },
    showResult: showResult,
    step: function (dt) {
      if (results && revealed < results.length) {
        revealT += dt;
        var want = Math.floor(revealT / (results.length > 1 ? 0.18 : 0.5)) + (results.length > 1 ? 0 : 0);
        if (want > revealed) { revealed = Math.min(results.length, want); rv.markDirty(); var r = results[revealed - 1]; G.sfx(r.rarity === 'SSR' ? 'ssr' : r.rarity === 'SR' ? 'sr' : 'pop'); }
      }
    }
  };
}

module.exports = { make: make };
