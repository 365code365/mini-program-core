// 界面层（js/hud.js 画在 3D 上面）：游戏中的计时 / 宝石 / 小地图 / 摇杆 / 按钮，标题页选关，过关与超时的结算页。
// 每个画出来的按钮把命中框记进 btns（game.js 按下时逐个判），这一帧没画的按钮不在表里、点不到。
var B = require('base.js');
var W = B.W, H = B.H;
var MAP = 104;                       // 小地图边长（逻辑像素）

function mmss(s) { s = Math.max(0, Math.ceil(s)); var m = (s / 60) | 0, r = s % 60; return (m < 10 ? '0' : '') + m + ':' + (r < 10 ? '0' : '') + r; }
function stars(n) { var s = ''; for (var i = 0; i < 3; i++) s += i < n ? '★' : '☆'; return s; }

function make(hud) {
  var btns = {};
  function btn(key, label, cx, cy, w, h, bg, fg) { btns[key] = hud.button(key, label, cx, cy, w, h, bg, fg); }

  // 小地图：墙、宝石、出口录在一块面板里，拿到宝石 / 出口开了才重画（sig 变）；玩家点是每帧挪的小方块，不重画面板
  function minimap(G) {
    var m = G.maze, x = W - MAP - 12, y = B.hudTop, taken = G.gems.filter(function (g) { return g.taken; }).length;
    var sig = G.seed + '|' + taken + '|' + (G.open ? 1 : 0);
    hud.panel('map', sig, x, y, MAP, MAP, function (c, w, h) {
      var s = (w - 8) / m.T;
      c.fillStyle = 'rgba(20,24,40,0.72)'; c.fillRect(0, 0, w, h);
      c.fillStyle = 'rgba(255,255,255,0.85)';
      // 一行里连着的墙砖并成一个 fillRect：31×31 的图从 ~500 次降到 ~150 次（本引擎 2D 画布每次调用都过一次原生）
      for (var j = 0; j < m.T; j++) for (var i = 0; i < m.T; i++) {
        if (!m.g[j * m.T + i]) continue;
        var i0 = i; while (i + 1 < m.T && m.g[j * m.T + i + 1]) i++;
        c.fillRect(4 + i0 * s, 4 + j * s, (i - i0 + 1) * s + 0.3, s + 0.3);
      }
      c.fillStyle = G.open ? '#3ddc84' : '#9e9e9e';
      c.fillRect(4 + m.exit.i * s, 4 + m.exit.j * s, s, s);
      c.fillStyle = '#40e0ff';
      G.gems.forEach(function (g) { if (!g.taken) { c.beginPath(); c.arc(4 + (g.i + 0.5) * s, 4 + (g.j + 0.5) * s, Math.max(1.5, s * 0.45), 0, Math.PI * 2); c.fill(); } });
    });
    var s = (MAP - 8) / m.T;
    hud.rect('me', x + 4 + G.px * s - 2.5, y + 4 + G.pz * s - 2.5, 5, 5, '#ff5722');
  }

  function joystick(ctl, G) {
    var j = ctl.joy, R = ctl.R;
    if (j.id === null && ctl.tilt.on) return;
    var ox = j.id === null ? 24 + R : j.ox, oy = j.id === null ? H - 40 - R : j.oy, a = j.id === null ? 0.45 : 0.9;
    hud.panel('joyBase', 1, ox - R - 6, oy - R - 6, 2 * R + 12, 2 * R + 12, function (c, w, h) {
      c.beginPath(); c.arc(w / 2, h / 2, R, 0, Math.PI * 2);
      c.fillStyle = 'rgba(0,0,0,0.22)'; c.fill();
      c.lineWidth = 3; c.strokeStyle = 'rgba(255,255,255,0.75)'; c.stroke();
    }, a);
    hud.panel('joyKnob', 1, ox + j.x * R - 22, oy + j.y * R - 22, 44, 44, function (c, w, h) {
      c.beginPath(); c.arc(w / 2, h / 2, 20, 0, Math.PI * 2);
      c.fillStyle = 'rgba(255,255,255,0.92)'; c.fill();
    }, a);
    if (j.id === null && G.time < 6) hud.text('joyHint', '按住屏幕下方拖动', ox + R + 18, oy, 14, '#ffffff', 'left', 'bold', { outline: ['rgba(0,0,0,0.35)', 1.5] });
  }

  function play(G, ctl) {
    var left = 14, y = B.hudTop + 10, remain = G.limit - G.time;
    minimap(G);
    hud.text('lv', '第 ' + G.level + ' 关', left, y, 15, '#ffffff', 'left', 'bold', { outline: ['rgba(0,0,0,0.35)', 1.5] });
    hud.text('time', mmss(remain), left, y + 30, 30, remain < 10 ? '#ff5252' : '#ffffff', 'left', 'bold', { outline: ['rgba(0,0,0,0.35)', 2] });
    var got = G.gems.filter(function (g) { return g.taken; }).length;
    hud.text('gem', '宝石 ' + got + ' / ' + G.gems.length, left, y + 62, 16, '#bff6ff', 'left', 'bold', { outline: ['rgba(0,0,0,0.35)', 1.5] });
    if (G.open && G.clock - G.openAt < 2.5) hud.text('open', '宝石集齐，出口已开启！', W / 2, H * 0.3, 20, '#7cff9c', 'center', 'bold', { outline: ['rgba(0,0,0,0.4)', 2] });
    joystick(ctl, G);
    var bx = W - 46, by = H - 44;
    btn('pause', '暂停', bx, by, 72, 36, 'rgba(255,255,255,0.9)', '#333344');
    btn('view', G.view ? '近景' : '俯视', bx, by - 46, 72, 36, 'rgba(255,255,255,0.9)', '#333344');
    btn('tilt', ctl.tilt.on ? '摇杆' : '重力', bx, by - 92, 72, 36, ctl.tilt.on ? '#ffd166' : 'rgba(255,255,255,0.9)', '#333344');
    if (G.paused) {
      hud.rect('mask', 0, 0, W, H, '#000000', 0.45);
      hud.text('p1', '已暂停', W / 2, H * 0.36, 32, '#ffffff', 'center', 'bold');
      btn('resume', '继续', W / 2, H * 0.5, 180, 48, '#ffffff', '#333344');
      btn('quit', '返回选关', W / 2, H * 0.5 + 62, 180, 44, 'rgba(255,255,255,0.25)', '#ffffff');
    }
  }

  function title(G, P) {
    hud.rect('mask', 0, 0, W, H, '#0b1020', 0.35);
    hud.text('t1', '3D 迷宫', W / 2, H * 0.2, 42, '#ffffff', 'center', 'bold', { outline: ['rgba(0,0,0,0.3)', 2] });
    hud.text('t2', '收集全部宝石，赶在时间耗尽前找到出口', W / 2, H * 0.2 + 44, 15, '#e0e6ff', 'center');
    var L = G.pick, b = P.best[L];
    btn('prev', '◀', W / 2 - 120, H * 0.46, 48, 48, L > 1 ? '#ffffff' : 'rgba(255,255,255,0.3)', '#333344');
    btn('next', '▶', W / 2 + 120, H * 0.46, 48, 48, L < P.unlocked ? '#ffffff' : 'rgba(255,255,255,0.3)', '#333344');
    hud.text('lvl', '第 ' + L + ' 关', W / 2, H * 0.46 - 8, 30, '#ffffff', 'center', 'bold');
    hud.text('lvs', (5 + L) + '×' + (5 + L) + ' 迷宫 · ' + (3 + Math.ceil(L / 2)) + ' 颗宝石', W / 2, H * 0.46 + 24, 13, '#cfd8ff', 'center');
    hud.text('lvb', b ? stars(b.stars) + '   最佳 ' + b.score + ' 分 · ' + mmss(b.time) : '尚未通关', W / 2, H * 0.46 + 52, 15, b ? '#ffd166' : '#aab4d8', 'center', 'bold');
    btn('start', '开始', W / 2, H * 0.66, 200, 52, '#ffd166', '#333344');
    hud.text('tot', '已解锁 ' + P.unlocked + ' / 10 关    总分 ' + total(P), W / 2, H * 0.66 + 50, 14, '#e0e6ff', 'center');
    hud.text('help', '摇杆 / 方向键移动，也可以开「重力」倾斜手机', W / 2, H - 46, 13, 'rgba(255,255,255,0.7)', 'center');
  }

  function result(G, P) {
    var a = Math.min(1, (G.clock - G.endAt) / 0.4), win = G.state === 'clear';
    hud.rect('mask', 0, 0, W, H, '#000000', 0.5 * a);
    hud.text('r1', win ? (G.level === 10 ? '全部通关！' : '过关！') : '时间到', W / 2, H * 0.2, 36, win ? '#ffd166' : '#ff8a80', 'center', 'bold', { alpha: a });
    if (win) {
      hud.text('r2', stars(G.stars), W / 2, H * 0.2 + 52, 36, '#ffd166', 'center', 'bold', { alpha: a });
      hud.text('r3', '用时 ' + mmss(G.time) + '   宝石 ' + G.gems.length + ' / ' + G.gems.length, W / 2, H * 0.2 + 100, 16, '#ffffff', 'center', 'normal', { alpha: a });
      hud.text('r4', '得分 ' + G.score + (G.newBest ? '   新纪录！' : ''), W / 2, H * 0.2 + 130, 20, G.newBest ? '#ffd166' : '#ffffff', 'center', 'bold', { alpha: a });
    } else {
      var got = G.gems.filter(function (g) { return g.taken; }).length;
      hud.text('r3', '拿到宝石 ' + got + ' / ' + G.gems.length + (G.open ? '，出口就在前面' : ''), W / 2, H * 0.2 + 60, 16, '#ffffff', 'center', 'normal', { alpha: a });
    }
    if (G.clock - G.endAt < 0.6) return;
    if (win && G.level < 10) btn('nextLv', '下一关', W / 2, H * 0.62, 200, 52, '#ffd166', '#333344');
    btn('retry', win ? '重玩本关' : '再试一次', W / 2, H * 0.62 + (win && G.level < 10 ? 64 : 0), 200, 48, '#ffffff', '#333344');
    btn('menu', '选关', W / 2, H * 0.62 + (win && G.level < 10 ? 124 : 60), 200, 44, 'rgba(255,255,255,0.25)', '#ffffff');
  }

  function total(P) { var s = 0; for (var k in P.best) s += P.best[k].score; return s; }

  return {
    btns: btns,
    draw: function (G, ctl, P) {
      for (var k in btns) delete btns[k];
      if (G.state === 'title') title(G, P);
      else {
        play(G, ctl);
        if (G.state === 'clear' || G.state === 'over') result(G, P);
      }
    },
    total: total
  };
}

module.exports = { make: make, mmss: mmss };
