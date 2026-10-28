// 任务：每日（进度条 + 领取，0 点重置倒计时）与成就（分阶，显示下一阶目标）。
var D = require('../data/defs.js');
var E = require('../core/economy.js');
var U = require('./draw.js');

function make(G) {
  var W = G.W, H = G.H, ui = G.ui, icons = G.icons, T = U.T, tab = 'daily', ROW = 92;
  var v = ui.view('quests', { x: 10, y: G.safeTop + 40, w: W - 20, h: H - G.safeTop - 40 - 96 - G.safeBottom, z: 20, modal: true, anim: 'up',
    onBackdrop: function () { v.hide(); }, draw: function (c, v) {
      var list = tab === 'daily' ? G.meta.dailyList() : G.meta.achList();
      v.setContent(130 + list.length * ROW + 10);
      U.panel(c, v.w, v.contentH, '任务', T.blue);
      U.closeBtn(c, icons, v);
      var tw = (v.w - 24) / 2, b = { daily: G.meta.dailyList().some(function (q) { return q.done && !q.claimed; }), ach: G.meta.achList().some(function (a) { return a.done; }) };
      [['daily', '每日任务'], ['ach', '成就']].forEach(function (t, i) {
        var on = t[0] === tab; c.fillStyle = on ? '#2f8fe8' : '#f3e3cc'; U.rr(c, 14 + i * tw, 62, tw - 4, 38, 14); c.fill();
        U.text(c, t[1], 12 + i * tw + tw / 2, 81, 15, on ? '#fff' : T.ink2, 'center', true);
        if (b[t[0]]) U.badge(c, 12 + i * tw + tw - 14, 66);
        v.hit('tab:' + t[0], 12 + i * tw, 62, tw, 40);
      });
      list.forEach(function (q, i) {
        var y = 130 + i * ROW, d = q.def, reward, prog, target, title, state;
        if (tab === 'daily') { reward = d.reward; prog = q.progress; target = d.n; title = d.name; state = q.claimed ? 'claimed' : q.done ? 'claim' : 'go'; }
        else { reward = q.maxed ? d.reward[d.reward.length - 1] : d.reward[q.tier]; prog = Math.min(q.value, q.target || q.value); target = q.target || q.value;
          title = d.name + (q.maxed ? '（全部完成）' : ' · 第 ' + (q.tier + 1) + ' 阶'); state = q.maxed ? 'claimed' : q.done ? 'claim' : 'go'; }
        U.card(c, 12, y, v.w - 24, ROW - 8, state === 'claim');
        icons.draw(c, tab === 'daily' ? 'scroll' : 'crown', 18, y + 12, 58);
        U.text(c, title, 84, y + 22, 15, state === 'claimed' ? T.mute : T.ink, 'left', true);
        U.bar(c, 84, y + 40, v.w - 220, 14, target ? prog / target : 1, state === 'claim' ? T.orange : T.blue);
        U.text(c, E.fmt(prog) + ' / ' + E.fmt(target), 84 + (v.w - 220) / 2, y + 47.5, 10, '#fff', 'center', true);
        U.costRow(c, icons, reward, 84, y + 70, 12);
        var bx = v.w - 118;
        if (state === 'claim') { v.hit('c:' + d.id, bx, y + 18, 94, 48); U.button(c, bx, y + 18, 94, 48, '领取', T.orange); }
        else if (state === 'claimed') { icons.draw(c, 'check', bx + 28, y + 18, 44); }
        else { v.hit('go:' + d.key, bx, y + 18, 94, 48); U.button(c, bx, y + 18, 94, 48, '去完成', T.blue, { size: 15 }); }
      });
    }, onTap: function (id) {
      if (id === 'close') return v.hide();
      if (id.indexOf('tab:') === 0) { tab = id.slice(4); v.scrollTo(0); return v.markDirty(); }
      if (id.indexOf('c:') === 0) {
        var r = tab === 'daily' ? G.meta.claimDaily(id.slice(2)) : G.meta.claimAch(id.slice(2));
        if (r) { G.reward(r, '奖励 +'); G.save(); }
        return v.markDirty();
      }
      if (id.indexOf('go:') === 0) {
        var k = id.slice(3); v.hide();
        if (k === 'ad') G.open('boost'); else if (k === 'gacha' || k === 'pets') G.open('pets', 'hatch'); else if (k === 'sell') G.open('sell');
        else if (k === 'build' || k === 'upgrade') G.open('build'); else G.ui.toast('点建筑头顶的图标收获', 'wheat');
      }
    } });
  // 刷新倒计时走单独的小条（kit.ticker），面板只在任务进度变了时重画
  ui.ticker(v, 'quests-clock', v.w / 2 - 110, 106, 220, 20, function (c) {
    var now = new Date(), mid = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
    U.text(c, '刷新倒计时 ' + E.fmtClock((mid - now) / 1000), 110, 10, 12, T.ink2, 'center', true);
  }, function () { return tab === 'daily'; });
  var lastSig = '';
  return {
    open: function (t) { if (t) tab = t; v.markDirty(); v.scrollTo(0); v.show(); },
    step: function () {
      if (!v.shown) return;
      // 只比当前页签上「显示出来的」东西（格式化后的进度、状态、两个红点）：成就计数器（累计产出之类）随被动产出一直在涨，
      // 拿原始 G.state.ach 当签名时面板开着就每 0.5 秒整块重画一次（真机 60ms 一次）。
      var list = tab === 'daily' ? G.meta.dailyList() : G.meta.achList();
      var s = tab + list.map(function (q) {
        return tab === 'daily' ? E.fmt(q.progress) + (q.claimed ? 'c' : q.done ? 'd' : '')
          : E.fmt(Math.min(q.value, q.target || q.value)) + ':' + q.tier + (q.maxed ? 'm' : q.done ? 'd' : '');
      }).join(',') + G.meta.dailyList().some(function (q) { return q.done && !q.claimed; }) + G.meta.achList().some(function (a) { return a.done; });
      if (s !== lastSig) { lastSig = s; v.markDirty(); }
    }
  };
}

module.exports = { make: make };
