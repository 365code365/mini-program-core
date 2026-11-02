// 界面：顶栏（金币 / 生命 / 波次 / 倍速 / 暂停）、底部面板（建塔 / 升级出售 / 出兵）、标题页选关、暂停与结算。
// 每帧画到的按钮记进 btns（{ k, arg, x, y, w, h }），game.js 点按时按画的倒序判（后画的在上面）。
var B = require('base.js');
var D = require('data.js');
var Maps = require('maps.js');
var W = B.W, H = B.H;

function stars(n) { var s = ''; for (var i = 0; i < 3; i++) s += i < n ? '★' : '☆'; return s; }

function make(R, L, draw) {
  var btns = [];
  function btn(k, arg, label, cx, cy, w, h, bg, fg) { var b = R.button(label, cx, cy, w, h, bg, fg); b.k = k; b.arg = arg; btns.push(b); return b; }
  function area(k, arg, x, y, w, h) { btns.push({ k: k, arg: arg, x: x, y: y, w: w, h: h }); }
  function panel(x, y, w, h, c) { R.rrect(x, y, w, h, 12, c || 'rgba(20,28,20,0.82)'); }

  function top(S, G) {
    var y = B.hudTop + 16;
    R.circle(22, y, 8, '#ffca28'); R.circle(22, y, 4.5, '#ffe082');
    R.text(S.gold, 36, y, 17, '#ffffff', 'left', 'bold');
    R.text('♥', 98, y, 17, '#ef5350', 'center', 'bold');
    R.text(S.lives, 110, y, 17, '#ffffff', 'left', 'bold');
    R.text('第 ' + Math.max(1, S.wave) + ' / ' + D.WAVES.length + ' 波', 150, y, 15, '#e8f5e9', 'left', 'bold');
    btn('speed', 0, '×' + G.speed, W - 92, y, 52, 30, G.speed > 1 ? '#ffd54f' : 'rgba(255,255,255,0.9)', '#2e3b2e');
    btn('pause', 0, 'Ⅱ', W - 34, y, 40, 30, 'rgba(255,255,255,0.9)', '#2e3b2e');
  }

  function bottom(S, G, sim) {
    var y0 = L.OY + Maps.ROWS * L.CELL + 8, h = Math.max(70, H - y0 - Math.max(8, H - B.safe.bottom)), x0 = 8, w = W - 16;
    panel(x0, y0, w, h);
    var sel = G.sel;
    if (sel && !sel.tower) {
      // 建塔：四张卡片
      var cw = (w - 10 - 3 * 6) / 4;
      D.ORDER.forEach(function (type, i) {
        var def = D.TOWERS[type], x = x0 + 5 + i * (cw + 6), ok = S.gold >= def.cost;
        R.rrect(x, y0 + 5, cw, h - 10, 10, ok ? 'rgba(255,255,255,0.14)' : 'rgba(0,0,0,0.25)');
        R.setAlpha(ok ? 1 : 0.45);
        draw.icon(type, x + cw / 2, y0 + 5 + (h - 10) * 0.36, 0.8);
        R.text(def.name, x + cw / 2, y0 + h - 30, 13, '#ffffff', 'center', 'bold');
        R.text(def.cost + ' 金', x + cw / 2, y0 + h - 14, 12, ok ? '#ffe082' : '#ef9a9a', 'center', 'bold');
        R.setAlpha(1);
        area('build', type, x, y0 + 5, cw, h - 10);
      });
      return;
    }
    if (sel && sel.tower) {
      var t = sel.tower, Lv = t.def.levels[t.lvl], cost = sim.upgradeCost(t);
      draw.icon(t.type, x0 + 32, y0 + h / 2, 0.9);
      R.text(t.def.name + '  Lv' + (t.lvl + 1), x0 + 62, y0 + h / 2 - 18, 15, '#ffffff', 'left', 'bold');
      var stat = '伤害 ' + Lv.dmg + ' · 射程 ' + Lv.range + (Lv.chain ? ' · 连锁 ' + Lv.chain : Lv.slow ? ' · 减速 ' + Math.round(Lv.slow * 100) + '%' : '');
      R.text(stat, x0 + 62, y0 + h / 2 + 2, 12, '#c8e6c9', 'left');
      R.text(t.def.desc, x0 + 62, y0 + h / 2 + 20, 11, 'rgba(255,255,255,0.6)', 'left');
      var bx = x0 + w - 52;
      if (cost) btn('upgrade', 0, '升级 ' + cost, bx - 2, y0 + h / 2 - 16, 92, 30, S.gold >= cost ? '#ffd54f' : 'rgba(255,255,255,0.3)', '#2e3b2e');
      else R.text('已满级', bx - 2, y0 + h / 2 - 16, 14, '#ffd54f', 'center', 'bold');
      btn('sell', 0, '出售 +' + sim.refund(t), bx - 2, y0 + h / 2 + 18, 92, 28, 'rgba(255,255,255,0.85)', '#c62828');
      return;
    }
    // 出兵
    var next = S.wave < D.WAVES.length ? S.wave : -1;
    var title = next < 0 ? '最后一波！守住城堡' : S.countdown > 0 ? '下一波 ' + Math.ceil(S.countdown) + ' 秒后' : S.wave ? '第 ' + S.wave + ' 波进行中' : '点空地建塔，准备好就出兵';
    R.text(title, x0 + 14, y0 + h / 2 - 14, 14, '#ffffff', 'left', 'bold');
    if (next >= 0) R.text('第 ' + (next + 1) + ' 波：' + D.waveText(next) + (D.hasAir(next) ? '（有飞行）' : ''), x0 + 14, y0 + h / 2 + 12, 11, '#c8e6c9', 'left');
    if (sim.canCall()) {
      var early = S.countdown > 0 ? Math.floor(S.countdown) * 3 : 0;
      btn('wave', 0, S.wave ? (early ? '提前 +' + early : '下一波') : '出兵', x0 + w - 52, y0 + h / 2, 88, 40, '#ffd54f', '#2e3b2e');
    }
  }

  function title(G, P) {
    R.gradRect(0, 0, W, H, [[0, '#24452b'], [1, '#0f1f14']], true);
    R.text('塔防守卫', W / 2, H * 0.14, 40, '#ffffff', 'center', 'bold', ['rgba(0,0,0,0.35)', 2]);
    R.text('四种塔、十波敌人，守住城堡', W / 2, H * 0.14 + 40, 15, '#c8e6c9', 'center');
    Maps.list.forEach(function (m, i) {
      var y = H * 0.3 + i * 108, lock = i + 1 > P.unlocked, b = P.best[i + 1];
      R.rrect(28, y, W - 56, 92, 16, lock ? 'rgba(255,255,255,0.08)' : 'rgba(255,255,255,0.16)');
      R.text('第 ' + (i + 1) + ' 关 · ' + m.name, 48, y + 26, 19, lock ? 'rgba(255,255,255,0.4)' : '#ffffff', 'left', 'bold');
      R.text(lock ? '通关上一关解锁' : m.tip + (m.hp > 1 ? '  敌人血量 ×' + m.hp : ''), 48, y + 52, 12, lock ? 'rgba(255,255,255,0.35)' : '#c8e6c9', 'left');
      R.text(b ? stars(b.stars) : lock ? '未解锁' : '未通关', W - 48, y + 26, b ? 18 : 13, b ? '#ffd54f' : 'rgba(255,255,255,0.55)', 'right', 'bold');
      if (b) R.text('最高 ' + b.score, W - 48, y + 54, 12, '#ffe082', 'right');
      if (!lock) R.text('路线 ' + Math.round(m.len) + ' 格 · 空地 ' + m.cover.filter(function (v) { return v >= 0; }).length + ' 块', 48, y + 74, 11, 'rgba(255,255,255,0.5)', 'left');
      if (!lock) area('start', i, 28, y, W - 56, 92);
    });
    R.text('点空地建塔 · 点塔升级或出售 · 右上角切换倍速', W / 2, H - 44, 12, 'rgba(255,255,255,0.6)', 'center');
    if (G.wasm) R.text('战斗计算：' + (G.wasm === 'wasm' ? 'WebAssembly' : 'JS'), W / 2, H - 24, 11, 'rgba(255,255,255,0.4)', 'center');
  }

  function pause() {
    R.rect(0, 0, W, H, 'rgba(0,0,0,0.55)');
    R.text('已暂停', W / 2, H * 0.34, 32, '#ffffff', 'center', 'bold');
    btn('resume', 0, '继续', W / 2, H * 0.46, 180, 48, '#ffd54f', '#2e3b2e');
    btn('restart', 0, '重新开始', W / 2, H * 0.46 + 60, 180, 44, 'rgba(255,255,255,0.9)', '#2e3b2e');
    btn('menu', 0, '返回选关', W / 2, H * 0.46 + 114, 180, 44, 'rgba(255,255,255,0.25)', '#ffffff');
  }

  function result(S, G, P) {
    var a = Math.min(1, (G.clock - G.endAt) / 0.4), win = S.state === 'win', cx = W / 2, cy = H * 0.42;
    R.setAlpha(a);
    R.rect(0, 0, W, H, 'rgba(0,0,0,0.55)');
    panel(30, cy - 150, W - 60, 300, 'rgba(30,40,30,0.95)');
    R.text(win ? '守住了！' : '城堡失守', cx, cy - 112, 32, win ? '#ffd54f' : '#ef9a9a', 'center', 'bold');
    if (win) R.text(stars(S.stars), cx, cy - 68, 30, '#ffd54f', 'center', 'bold');
    else R.text('撑到第 ' + S.wave + ' 波', cx, cy - 68, 18, '#ffffff', 'center', 'bold');
    R.text('击杀 ' + S.kills + '   剩余生命 ' + S.lives + '   建塔 ' + S.built, cx, cy - 30, 14, '#e8f5e9', 'center');
    R.text('得分 ' + S.score + (G.newBest ? '  新纪录！' : ''), cx, cy, 20, G.newBest ? '#ffd54f' : '#ffffff', 'center', 'bold');
    R.setAlpha(1);
    if (G.clock - G.endAt < 0.6) return;
    var hasNext = win && S.mapIdx + 1 < Maps.list.length;
    if (hasNext) btn('next', 0, '下一关', cx, cy + 50, 180, 44, '#ffd54f', '#2e3b2e');
    btn('restart', 0, win ? '再玩一次' : '再来一次', cx, cy + (hasNext ? 102 : 56), 180, 42, 'rgba(255,255,255,0.9)', '#2e3b2e');
    btn('menu', 0, '返回选关', cx, cy + (hasNext ? 150 : 106), 180, 38, 'rgba(255,255,255,0.25)', '#ffffff');
  }

  return {
    btns: btns,
    draw: function (S, G, P, sim) {
      btns.length = 0;
      if (G.screen === 'title') { title(G, P); return; }
      top(S, G);
      bottom(S, G, sim);
      if (S.state === 'win' || S.state === 'lose') result(S, G, P);
      else if (G.paused) pause();
    },
    hit: function (x, y) {
      for (var i = btns.length - 1; i >= 0; i--) if (B.hit(btns[i], x, y)) return btns[i];
      return null;
    }
  };
}

module.exports = { make: make };
