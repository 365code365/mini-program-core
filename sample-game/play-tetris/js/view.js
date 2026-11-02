// 画面：只读游戏状态，不改规则。走 js/gl2d.js（WebGL 批量画形状 + 文字图集）。
// 不变的东西录成图层：背景 + 棋盘框 + 侧栏框（'bg'，只录一次）、已锁定的格子（'cells'，棋盘变了才重录）、
// 暂存 / 预览（'side'，换块时才重录）。每帧现拼的只有当前块、幽灵块、数字、粒子与弹层。
var T = require('./tetris.js');
var COLS = T.COLS, ROWS = T.ROWS, HIDDEN = T.HIDDEN, VIS = ROWS - HIDDEN, CELLS = T.CELLS;
var PAL = {
  I: ['#26c6da', '#80deea', '#00838f'], J: ['#4f7cff', '#9db5ff', '#2747b3'], L: ['#ff9f1c', '#ffd08a', '#c66a00'],
  O: ['#fdd835', '#fff59d', '#c9a400'], S: ['#5ccf4f', '#a8eb9f', '#2e8b24'], T: ['#b05ce0', '#dcb0f5', '#7428a3'],
  Z: ['#ef5350', '#ffa4a2', '#b0201d']
};
var BG = '#0e1435', WELL = '#151c42';

function make(R, B) {
  var W = B.W, H = B.H;
  var top = Math.max(B.hudTop, 44) + 4, bottom = Math.min(H, B.safe.bottom || H) - 34;
  var SIDE = 88, GAP = 10, MX = 12;
  var CELL = Math.floor(Math.min((bottom - top) / VIS, (W - MX * 2 - GAP - SIDE) / COLS));
  var BW = COLS * CELL, BH = VIS * CELL;
  var OX = Math.floor((W - (BW + GAP + SIDE)) / 2), OY = Math.floor(top + (bottom - top - BH) / 2);
  var SX = OX + BW + GAP, SW = W - SX - OX;
  var HOLD = { x: SX, y: OY + 20, w: SW, h: 64 };
  var NEXT = { x: SX, y: HOLD.y + HOLD.h + 28, w: SW, h: 170 };
  var STATS = NEXT.y + NEXT.h + 18;
  var PAUSE = { x: SX, y: OY + BH - 36, w: SW, h: 36 };

  // 一格方块：亮边（左上）+ 暗边（右下）+ 主色 + 一点高光，四个矩形
  function block(x, y, s, t, a) {
    var p = PAL[t];
    if (a !== undefined) R.setAlpha(a);
    R.rect(x, y, s, s, p[1]);
    R.rect(x + 2, y + 2, s - 2, s - 2, p[2]);
    R.rect(x + 2, y + 2, s - 4, s - 4, p[0]);
    R.rect(x + 3, y + 3, s * 0.3, s * 0.18, 'rgba(255,255,255,0.35)');
    if (a !== undefined) R.setAlpha(1);
  }
  // 按钮：圆角 12（gl2d 的 button 用 h/2 的圆角，小按钮上四分之一圆只有两三段，看着是八边形）
  function btn(label, cx, cy, w, h, bg, fg) {
    R.rrect(cx - w / 2, cy - h / 2, w, h, 12, bg);
    R.text(label, cx, cy + 1, Math.round(h * 0.4), fg, 'center', 'bold');
    return { x: cx - w / 2, y: cy - h / 2, w: w, h: h };
  }
  function cellXY(x, y) { return [OX + x * CELL, OY + (y - HIDDEN) * CELL]; }
  // 预览里的小方块：按格子包围盒居中摆进框里
  function mini(t, cx, cy, s) {
    var c = CELLS[t][0], x0 = 9, x1 = -9, y0 = 9, y1 = -9;
    for (var i = 0; i < 4; i++) { x0 = Math.min(x0, c[i][0]); x1 = Math.max(x1, c[i][0]); y0 = Math.min(y0, c[i][1]); y1 = Math.max(y1, c[i][1]); }
    var ox = cx - (x1 - x0 + 1) * s / 2 - x0 * s, oy = cy - (y1 - y0 + 1) * s / 2 - y0 * s;
    for (var k = 0; k < 4; k++) block(ox + c[k][0] * s, oy + c[k][1] * s, s, t);
  }
  function box(b, label) {
    R.text(label, b.x + 2, b.y - 11, 13, '#9fa8da', 'left', 'bold');
    R.rrect(b.x, b.y, b.w, b.h, 10, '#121838');
  }
  // 静态底图（录成图层只录一次）。无头 CPU 后端（swiftshader）每帧的时间主要花在填像素上：
  // 起初整屏铺一层渐变、棋盘外框是整块圆角矩形再叠井，叠了两三层整屏，中位帧 ~15ms（play-snake ~9ms）。
  // 现在背景就是清屏色，渐变只铺顶部标题那一条，外框画成四条边，每个像素基本只填一次
  function drawStatic() {
    R.gradRect(0, 0, W, OY - 6, [[0, '#1d2763'], [1, BG]], true);
    R.text('俄罗斯方块', OX + 2, Math.max(B.hudTop - 24, OY / 2), 18, '#ffffff', 'left', 'bold');
    R.strokeRect(OX - 2.5, OY - 2.5, BW + 5, BH + 5, 5, '#2a3570');
    R.rect(OX, OY, BW, BH, WELL);
    for (var x = 1; x < COLS; x++) R.rect(OX + x * CELL, OY, 1, BH, 'rgba(255,255,255,0.05)');
    for (var y = 1; y < VIS; y++) R.rect(OX, OY + y * CELL, BW, 1, 'rgba(255,255,255,0.05)');
    box(HOLD, '暂存');
    box(NEXT, '下一个');
    var labels = ['分数', '最高', '等级', '行数'];
    for (var i = 0; i < 4; i++) R.text(labels[i], SX + 2, STATS + i * 40, 12, '#7986cb', 'left');
    R.text('点击旋转 · 左右滑移动 · 下滑硬降 · 长按软降 · 上滑暂存', W / 2, bottom + 18, 11, '#6c76b8', 'center');
  }
  function drawCells(g) {
    for (var y = HIDDEN; y < ROWS; y++) for (var x = 0; x < COLS; x++) {
      var t = g.board[y * COLS + x];
      if (t) { var p = cellXY(x, y); block(p[0], p[1], CELL, t); }
    }
  }
  function drawSide(g) {
    if (g.hold) { R.setAlpha(g.canHold ? 1 : 0.35); mini(g.hold, HOLD.x + HOLD.w / 2, HOLD.y + HOLD.h / 2, 15); R.setAlpha(1); }
    for (var i = 0; i < 3; i++) {
      var s = i === 0 ? 16 : 13;
      mini(g.queue[i], NEXT.x + NEXT.w / 2, NEXT.y + 34 + i * 52, s);
    }
  }
  function piece(c, y, alphaGhost) {
    var cells = CELLS[c.t][c.r];
    for (var i = 0; i < 4; i++) {
      var cy = y + cells[i][1];
      if (cy < HIDDEN) continue;
      var p = cellXY(c.x + cells[i][0], cy);
      if (alphaGhost) {
        R.rect(p[0] + 1, p[1] + 1, CELL - 2, CELL - 2, 'rgba(255,255,255,0.07)');
        R.strokeRect(p[0] + 2, p[1] + 2, CELL - 4, CELL - 4, 2, PAL[c.t][1]);
      } else block(p[0], p[1], CELL, c.t);
    }
  }
  function drawClear(g) {
    var cl = g.clearing; if (!cl) return;
    var p = cl.t / T.CLEAR_TIME;
    for (var i = 0; i < cl.rows.length; i++) {
      var y = OY + (cl.rows[i] - HIDDEN) * CELL;
      if (p < 0.35) { R.setAlpha(1 - p / 0.35 * 0.4); R.rect(OX, y, BW, CELL, '#ffffff'); R.setAlpha(1); }
      else { var w = BW * Math.min(1, (p - 0.35) / 0.55); R.rect(OX + (BW - w) / 2, y, w, CELL, WELL); }
    }
  }
  function panel(h) {
    var cx = OX + BW / 2, cy = OY + BH / 2, w = BW - 16;
    R.rrect(cx - w / 2, cy - h / 2, w, h, 16, 'rgba(8,11,32,0.92)');
    R.strokeRect(cx - w / 2 + 6, cy - h / 2 + 6, w - 12, h - 12, 1, 'rgba(159,168,218,0.25)');
    return { cx: cx, cy: cy, w: w, top: cy - h / 2 };
  }
  function overlays(st, buttons) {
    var g = st.game, p;
    if (st.state === 'ready') {
      p = panel(st.saved ? 270 : 214);
      R.text('俄罗斯方块', p.cx, p.top + 44, 30, '#ffffff', 'center', 'bold', ['#3f51b5', 2]);
      R.text('最高分 ' + st.best, p.cx, p.top + 84, 15, '#ffd54f', 'center');
      R.text('↑/X 旋转 Z 逆转 空格硬降 C 暂存', p.cx, p.top + 112, 11, '#9fa8da', 'center');
      var y = p.top + 160;
      if (st.saved) {
        buttons.push(['resume', btn('继续上局 ' + st.saved.score + ' 分', p.cx, y, 190, 44, '#ffd54f', '#3e2723')]);
        y += 58;
      }
      buttons.push(['start', btn(st.saved ? '新游戏' : '开始游戏', p.cx, y, 160, 44, '#5c7cfa', '#ffffff')]);
    } else if (st.state === 'over') {
      p = panel(260);
      R.text(st.newBest ? '新纪录！' : '游戏结束', p.cx, p.top + 44, 28, st.newBest ? '#ffd54f' : '#ffffff', 'center', 'bold');
      R.text(String(g.score), p.cx, p.top + 92, 34, '#ffffff', 'center', 'bold');
      R.text('等级 ' + g.level + ' · 消除 ' + g.lines + ' 行 · 四消 ' + g.stats.tetris + ' 次', p.cx, p.top + 130, 12, '#c5cae9', 'center');
      R.text('最高分 ' + st.best, p.cx, p.top + 152, 13, '#ffd54f', 'center');
      if (st.clock - st.overAt > 0.6) buttons.push(['start', btn('再来一局', p.cx, p.top + 210, 160, 44, '#5c7cfa', '#ffffff')]);
    } else if (st.paused) {
      p = panel(220);
      R.text('已暂停', p.cx, p.top + 46, 28, '#ffffff', 'center', 'bold');
      buttons.push(['resume', btn('继续', p.cx, p.top + 110, 150, 44, '#5c7cfa', '#ffffff')]);
      buttons.push(['restart', btn('重新开始', p.cx, p.top + 168, 150, 40, '#2a3570', '#c5cae9')]);
    }
  }

  // 每帧：st = { game, state, paused, best, newBest, saved, clock, overAt, shake, fx }；返回这帧可点的按钮
  function draw(st) {
    var g = st.game, buttons = [];
    R.begin(BG);
    R.layer('bg', 1, drawStatic);
    R.save();
    if (st.shake > 0) R.translate(0, Math.sin(st.shake * 60) * st.shake * 18);
    R.layer('cells', g.ver, function () { drawCells(g); });   // 图层跟着当前平移走（硬降时整块棋盘一起震）
    drawClear(g);
    if (g.cur && st.state === 'play' && !st.paused) {   // 暂停时不露当前块（不给「暂停想一想」的机会）
      var gy = g.ghostY();
      if (gy !== g.cur.y) piece(g.cur, gy, true);
      piece(g.cur, g.cur.y, false);
    }
    st.fx.draw(R);
    R.restore();
    R.layer('side', (g.hold || '-') + g.canHold + g.queue.slice(0, 3).join(''), function () { drawSide(g); });
    var vals = [g.score, Math.max(st.best, g.score), g.level, g.lines];
    for (var i = 0; i < 4; i++) R.text(String(vals[i]), SX + 2, STATS + 18 + i * 40, i === 0 ? 20 : 17, i === 1 ? '#ffd54f' : '#ffffff', 'left', 'bold');
    if (st.state === 'play' && !st.paused) buttons.push(['pause', btn('暂停', PAUSE.x + PAUSE.w / 2, PAUSE.y + PAUSE.h / 2, PAUSE.w, PAUSE.h, '#2a3570', '#c5cae9')]);
    buttons.push(['hold', HOLD]);
    overlays(st, buttons);
    R.end();
    return buttons;
  }
  return { draw: draw, CELL: CELL, OX: OX, OY: OY, BW: BW, BH: BH, cellXY: cellXY, PAL: PAL };
}

module.exports = { make: make };
