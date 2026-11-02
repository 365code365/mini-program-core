// 战场绘制（js/gl2d.js）：地图录成图层（换关才重录），塔 / 敌人 / 子弹 / 特效每帧现拼。
// 坐标：模拟里是「格」，这里乘 CELL 加 (OX, OY) 变成逻辑像素。
var Maps = require('maps.js');
var D = require('data.js');
var TAU = Math.PI * 2;

function make(R, L) {
  var OX = L.OX, OY = L.OY, CELL = L.CELL;
  function X(x) { return OX + x * CELL; }
  function Y(y) { return OY + y * CELL; }

  function mapLayer(m, idx) {
    R.layer('map', 'm' + idx, function () {
      var COLS = Maps.COLS, ROWS = Maps.ROWS;
      R.rect(OX - 4, OY - 4, COLS * CELL + 8, ROWS * CELL + 8, '#3e5a2f');
      for (var r = 0; r < ROWS; r++) for (var c = 0; c < COLS; c++) {
        var ch = m.g[r * COLS + c], x = X(c), y = Y(r);
        if (ch === '#' || ch === 'S' || ch === 'E') {
          R.rect(x, y, CELL, CELL, '#d9c08c');
          R.rect(x + CELL * 0.2, y + CELL * 0.3, 3, 3, 'rgba(120,90,50,0.35)');
          R.rect(x + CELL * 0.7, y + CELL * 0.65, 2, 2, 'rgba(120,90,50,0.35)');
        } else {
          R.rect(x, y, CELL, CELL, (r + c) % 2 ? '#7fb35a' : '#76aa52');
          if (ch === 'x') { R.ellipse(x + CELL / 2, y + CELL * 0.62, CELL * 0.34, CELL * 0.24, 0, '#8d8d8d'); R.ellipse(x + CELL * 0.44, y + CELL * 0.54, CELL * 0.16, CELL * 0.1, 0, '#b0b0b0'); }
          if (ch === 't') {
            R.rect(x + CELL * 0.45, y + CELL * 0.55, CELL * 0.1, CELL * 0.3, '#6d4c41');
            R.circle(x + CELL / 2, y + CELL * 0.42, CELL * 0.3, '#2e7d32'); R.circle(x + CELL * 0.42, y + CELL * 0.36, CELL * 0.12, '#43a047');
          }
        }
      }
      // 路的边：相邻是草地的那一边画一道深色
      for (var r2 = 0; r2 < ROWS; r2++) for (var c2 = 0; c2 < COLS; c2++) {
        var ch2 = m.g[r2 * COLS + c2]; if (ch2 === '.' || ch2 === 'x' || ch2 === 't') continue;
        var x2 = X(c2), y2 = Y(r2), edge = 'rgba(90,70,40,0.45)';
        if (r2 > 0 && grass(m.g[(r2 - 1) * COLS + c2])) R.rect(x2, y2, CELL, 2, edge);
        if (r2 < ROWS - 1 && grass(m.g[(r2 + 1) * COLS + c2])) R.rect(x2, y2 + CELL - 2, CELL, 2, edge);
        if (c2 > 0 && grass(m.g[r2 * COLS + c2 - 1])) R.rect(x2, y2, 2, CELL, edge);
        if (c2 < COLS - 1 && grass(m.g[r2 * COLS + c2 + 1])) R.rect(x2 + CELL - 2, y2, 2, CELL, edge);
      }
      // 出兵口与城堡
      var sx = X(m.S.c + 0.5), sy = Y(m.S.r + 0.5);
      R.circle(sx, sy, CELL * 0.4, '#4a2c5e'); R.circle(sx, sy, CELL * 0.28, '#7b3fa0'); R.circle(sx, sy, CELL * 0.14, '#ce93d8');
      var ex = X(m.E.c), ey = Y(m.E.r);
      R.rect(ex + CELL * 0.12, ey + CELL * 0.3, CELL * 0.76, CELL * 0.62, '#90a4ae');
      for (var k = 0; k < 3; k++) R.rect(ex + CELL * (0.12 + k * 0.29), ey + CELL * 0.16, CELL * 0.18, CELL * 0.16, '#90a4ae');
      R.rrect(ex + CELL * 0.38, ey + CELL * 0.56, CELL * 0.24, CELL * 0.36, 4, '#5d4037');
      R.rect(ex + CELL * 0.48, ey - CELL * 0.18, 2, CELL * 0.36, '#455a64');
      R.poly([[ex + CELL * 0.5, ey - CELL * 0.18], [ex + CELL * 0.8, ey - CELL * 0.08], [ex + CELL * 0.5, ey + CELL * 0.02]], '#e53935');
    }, 0, 0);
  }
  var hex = [[0, 0], [0, 0], [0, 0], [0, 0], [0, 0], [0, 0]];
  function hexagon(r) { for (var k = 0; k < 6; k++) { hex[k][0] = Math.cos(k * TAU / 6) * r; hex[k][1] = Math.sin(k * TAU / 6) * r; } return hex; }
  function grass(ch) { return ch === '.' || ch === 'x' || ch === 't'; }

  function tower(t, sel) {
    var x = X(t.x), y = Y(t.y), s = CELL, col = t.def.color;
    R.rrect(x - s * 0.42, y - s * 0.42, s * 0.84, s * 0.84, 6, sel ? '#fff59d' : '#4e5b4a');
    R.rrect(x - s * 0.36, y - s * 0.36, s * 0.72, s * 0.72, 5, '#a1887f');
    R.save(); R.translate(x, y);
    if (t.type === 'arrow') {
      R.circle(0, 0, s * 0.26, col);
      R.rotate(t.angle); R.rect(0, -s * 0.05, s * 0.36, s * 0.1, '#5d4037'); R.poly([[s * 0.36, -s * 0.1], [s * 0.48, 0], [s * 0.36, s * 0.1]], '#eeeeee');
    } else if (t.type === 'cannon') {
      R.circle(0, 0, s * 0.27, col);
      R.rotate(t.angle); R.rect(0, -s * 0.09, s * 0.4 - t.flash * 30, s * 0.18, '#263238'); R.circle(0, 0, s * 0.14, '#90a4ae');
    } else if (t.type === 'ice') {
      R.poly(hexagon(s * 0.28), col); R.circle(0, 0, s * 0.13, '#e1f5fe');
    } else {
      R.circle(0, 0, s * 0.27, '#6a1b9a'); R.circle(0, -s * 0.06, s * 0.17, col); R.circle(0, -s * 0.1, s * 0.08, t.flash > 0 ? '#ffffff' : '#e1bee7');
    }
    R.restore();
    for (var p = 0; p <= t.lvl; p++) R.circle(x - s * 0.2 + p * s * 0.2, y + s * 0.33, 3, '#ffd54f');
  }

  function enemy(e, clock) {
    var x = X(e.x), y = Y(e.y), r = e.def.r * CELL, col = e.hitT > 0 ? '#ffffff' : e.def.color;
    if (e.air) {
      R.ellipse(x, y + CELL * 0.35, r, r * 0.4, 0, 'rgba(0,0,0,0.22)');
      var fl = Math.sin(clock * 14 + e.wob) * 0.5;
      R.ellipse(x - r * 0.9, y - 6, r * 0.8, r * (0.3 + fl * 0.2), -0.4, '#9575cd');
      R.ellipse(x + r * 0.9, y - 6, r * 0.8, r * (0.3 + fl * 0.2), 0.4, '#9575cd');
      y -= 6;
    } else R.ellipse(x, y + r * 0.7, r * 0.9, r * 0.35, 0, 'rgba(0,0,0,0.2)');
    if (e.def.armor) R.circle(x, y, r + 2.5, e.def.boss ? '#ffd54f' : '#cfd8dc');
    R.circle(x, y, r, col);
    if (e.slowT > 0) R.strokeCircle(x, y, r + 1, 2, '#81d4fa');
    R.circle(x - r * 0.35, y - r * 0.2, Math.max(1.5, r * 0.18), '#ffffff'); R.circle(x + r * 0.35, y - r * 0.2, Math.max(1.5, r * 0.18), '#ffffff');
    if (e.hp < e.max || e.def.boss) {
      var w = Math.max(18, r * 2.2), k = Math.max(0, e.hp / e.max);
      R.rect(x - w / 2, y - r - 8, w, 4, 'rgba(0,0,0,0.5)');
      R.rect(x - w / 2, y - r - 8, w * k, 4, k > 0.5 ? '#66bb6a' : k > 0.25 ? '#ffca28' : '#ef5350');
    }
  }

  function shot(s) {
    var x = X(s.x), y = Y(s.y);
    if (s.kind === 'arrow') R.circle(x, y, 2.5, '#fff3e0');
    else if (s.kind === 'ice') { R.circle(x, y, 4, '#b3e5fc'); R.circle(x, y, 2, '#ffffff'); }
    else { R.ellipse(x, y + 3, 4, 2, 0, 'rgba(0,0,0,0.25)'); R.circle(x, y - (s.h || 0) * CELL, 4.5, '#263238'); }
  }

  function effect(f) {
    var k = f.t / f.T, x = X(f.x), y = Y(f.y), a = 1 - k;
    if (f.kind === 'boom') { R.setAlpha(a); R.circle(x, y, f.r * CELL * (0.4 + k * 0.6), '#ff9800'); R.circle(x, y, f.r * CELL * 0.35 * (1 - k), '#fff59d'); R.setAlpha(1); }
    else if (f.kind === 'frost') { R.setAlpha(a * 0.8); R.strokeCircle(x, y, f.r * CELL * (0.3 + k * 0.7), 3, '#e1f5fe'); R.setAlpha(1); }
    else if (f.kind === 'ring') { R.setAlpha(a); R.strokeCircle(x, y, CELL * (0.3 + k * f.r), 3, '#ffd54f'); R.setAlpha(1); }
    else if (f.kind === 'puff') { R.setAlpha(a * 0.7); R.circle(x, y, f.r * CELL * (0.6 + k), '#eeeeee'); R.setAlpha(1); }
    else if (f.kind === 'leak') { R.setAlpha(a * 0.6); R.circle(x, y, CELL * f.r * (0.5 + k), '#ef5350'); R.setAlpha(1); }
    else if (f.kind === 'gold') { R.setAlpha(a); R.text(f.text, x, y - k * CELL * 0.6, 13, '#ffe082', 'center', 'bold', ['rgba(0,0,0,0.5)', 1.5]); R.setAlpha(1); }
    else if (f.kind === 'zap') {
      // 闪电：每段两点之间抖三下
      var pts = [];
      for (var i = 0; i + 3 < f.pts.length; i += 2) {
        var x0 = X(f.pts[i]), y0 = Y(f.pts[i + 1]), x1 = X(f.pts[i + 2]), y1 = Y(f.pts[i + 3]);
        if (!pts.length) pts.push([x0, y0]);
        for (var j = 1; j <= 3; j++) { var q = j / 3, jit = j < 3 ? (Math.random() - 0.5) * 10 : 0; pts.push([x0 + (x1 - x0) * q + jit, y0 + (y1 - y0) * q + jit]); }
      }
      R.setAlpha(a); R.line(pts, 4, 'rgba(206,147,216,0.7)', true); R.line(pts, 1.6, '#ffffff', false); R.setAlpha(1);
    }
  }

  function field(S, sel, clock) {
    mapLayer(S.map, S.mapIdx);
    var i;
    if (sel) {
      var cx = X(sel.c + 0.5), cy = Y(sel.r + 0.5), rng = sel.tower ? sel.tower.def.levels[sel.tower.lvl].range : 0;
      if (rng) { R.circle(cx, cy, rng * CELL, 'rgba(255,255,255,0.12)'); R.strokeCircle(cx, cy, rng * CELL, 1.5, 'rgba(255,255,255,0.6)'); }
      else R.strokeRect(X(sel.c) + 2, Y(sel.r) + 2, CELL - 4, CELL - 4, 2, '#fff59d');
    }
    for (i = 0; i < S.towers.length; i++) tower(S.towers[i], sel && sel.tower === S.towers[i]);
    for (i = 0; i < S.enemies.length; i++) if (S.enemies[i].alive && !S.enemies[i].air) enemy(S.enemies[i], clock);
    for (i = 0; i < S.shots.length; i++) if (S.shots[i].alive) shot(S.shots[i]);
    for (i = 0; i < S.enemies.length; i++) if (S.enemies[i].alive && S.enemies[i].air) enemy(S.enemies[i], clock);   // 飞的画在最上面
    for (i = 0; i < S.fx.length; i++) if (S.fx[i].alive) effect(S.fx[i]);
  }

  // 塔的小图标（底部面板用）：把 (0.5, 0.5) 格上的一座 1 级塔缩放后摆到 (x, y)
  var icons = {};
  function icon(type, x, y, s) {
    var t = icons[type] || (icons[type] = { x: 0.5, y: 0.5, type: type, def: D.TOWERS[type], lvl: 0, angle: -Math.PI / 2, flash: 0 });
    R.save(); R.translate(x, y); R.scale(s, s); R.translate(-X(0.5), -Y(0.5));
    tower(t, false);
    R.restore();
  }

  return { field: field, icon: icon, X: X, Y: Y };
}

module.exports = { make: make };
