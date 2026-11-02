// 画面：只读 game.js 的状态 S，不改游戏逻辑（命中框写回 S.cards / S.buttons，供点按判定）。
// 不变的东西（菜单背景、四条轨的分隔线）录成图层，每帧一次 drawArrays；黑块每帧现画（屏上最多十来块）。
// 无头 CPU 后端（swiftshader）上耗时几乎全在填充像素：整屏铺一块矩形 / 渐变 / 半透明遮罩一次 ~5ms
// （菜单去掉整屏渐变，pump 中位 15ms → 9ms）。所以底色一律交给 begin 的清屏色，弹层只画面板本身，不铺整屏遮罩。
// base.js 由 game.js 传进来（setup），不在这里再 require 一次：它在加载时就注册触摸 / 键盘监听，不该跑两遍
var B, W, H, LW;
function setup(b) { B = b; W = b.W; H = b.H; LW = W / 4; }

function draw(R, S, SONGS, GRADE) {
  if (S.scene === 'menu') { menu(R, S, SONGS); return R.end(); }
  R.begin('#fbfbfd');
  R.layer('lanes', 1, function () {
    for (var i = 1; i < 4; i++) R.rect(i * LW - 0.5, 0, 1, H, '#dfe2e8');
  });
  R.rect(0, S.JY - 1, W, 2, 'rgba(66,165,245,0.35)');
  tiles(R, S);
  ripples(R, S);
  hud(R, S, GRADE);
  if (S.scene === 'over') over(R, S);
  if (S.paused) {
    R.rrect(24, H / 2 - 70, W - 48, 130, 18, 'rgba(20,22,32,0.92)');
    R.text('已暂停', W / 2, H / 2 - 16, 30, '#ffffff', 'center', 'bold');
    R.text('点一下屏幕继续（继续前有 1 秒准备）', W / 2, H / 2 + 22, 14, '#d0d4e0');
  }
  R.end();
}

function tiles(R, S) {
  var list = S.chart.tiles, blink = Math.floor(S.clock * 8) % 2;
  for (var i = Math.max(0, S.next - 8); i < list.length; i++) {
    var t = list[i], bottom = S.JY - (t.start - S.pos) * S.U, top = S.JY - (t.start + t.dur - S.pos) * S.U;
    if (top > H) continue;
    if (bottom < 0) break;
    var x = t.lane * LW + 1, w = LW - 2, y = top + 1, h = bottom - top - 2;
    if (t.hit) {
      var k = Math.min(1, (S.clock - t.hitAt) / 0.25);
      R.rect(x, y, w, h, k < 1 ? [0.36 + k * 0.48, 0.38 + k * 0.47, 0.45 + k * 0.43, 1] : '#d6d9e0');
    } else if (S.scene !== 'play' && S.scene !== 'count' && S.failKind === 'miss' && i === S.next) {
      R.rect(x, y, w, h, blink ? '#e53935' : '#b71c1c');
    } else {
      R.gradRect(x, y, w, h, [[0, '#3c4152'], [1, '#0e1016']], true);
      if (h > 34) R.text(t.label, x + w / 2, bottom - 16, 13, '#8b91a8');
    }
  }
  if (S.failKind === 'wrong' && S.scene !== 'play' && S.scene !== 'count') {
    R.setAlpha(blink ? 1 : 0.6);
    R.rect(S.failLane * LW + 1, S.failY - S.U / 2, LW - 2, S.U, '#e53935');
    R.setAlpha(1);
  }
}

function ripples(R, S) {
  for (var i = 0; i < S.ripples.length; i++) {
    var r = S.ripples[i], k = (S.clock - r.t) / 0.35;
    if (k >= 1) continue;
    R.setAlpha(1 - k);
    R.strokeCircle(r.x, r.y, 12 + k * 40, 3, '#64b5f6');
    R.setAlpha(1);
  }
}

function hud(R, S, GRADE) {
  var y = B.hudTop + 14;
  // 顶部一条半透明白带托住分数（黑块从它下面滚过去，字不会压在黑块上看不清）；只占屏高 ~15%，填充代价小
  R.rect(0, 0, W, y + 48, 'rgba(251,251,253,0.86)');
  R.rect(0, y + 48, W, 1, 'rgba(0,0,0,0.06)');
  R.text(String(S.score), W / 2, y, 40, '#e53935', 'center', 'bold');
  R.text(S.song.name + '  ·  速度 ×' + S.mul.toFixed(2), W / 2, y + 32, 13, '#6f7590');
  if (S.lastGrade && S.clock - S.gradeAt < 0.55 && S.scene === 'play') {
    var k = (S.clock - S.gradeAt) / 0.55, s = S.lastGrade.name + (S.combo >= 3 ? ' ×' + S.combo : '');
    R.setAlpha(1 - k * k);
    var gy = H * 0.34 - k * 14, gw = s.length * 17 + 36;
    R.rrect(W / 2 - gw / 2, gy - 22, gw, 44, 22, 'rgba(20,22,32,0.78)');
    R.text(s, W / 2, gy, 24, S.lastGrade.color, 'center', 'bold');
    R.setAlpha(1);
  }
  if (S.scene === 'count') {
    var n = Math.ceil((S.startAt - S.nowA()) / 0.5);
    var pw = Math.min(W - 40, 280);
    R.rrect(W / 2 - pw / 2, H * 0.4 - 62, pw, 132, 20, 'rgba(14,16,22,0.8)');
    R.text(n > 0 ? String(Math.min(3, n)) : '开始', W / 2, H * 0.4 - 12, n > 0 ? 54 : 34, '#ffffff', 'center', 'bold');
    R.text(S.song.name + ' — ' + S.song.by, W / 2, H * 0.4 + 44, 13, '#c3c8da');
  }
}

function over(R, S) {
  var cx = W / 2, cy = H / 2, pw = Math.min(310, W - 40), ph = 360, top = cy - ph / 2;
  R.rrect(cx - pw / 2 - 3, top - 3, pw + 6, ph + 6, 21, 'rgba(14,16,22,0.35)');   // 一圈阴影代替整屏遮罩
  R.rrect(cx - pw / 2, top, pw, ph, 18, '#ffffff');
  R.text(S.newBest ? '新纪录！' : '游戏结束', cx, top + 38, 28, S.newBest ? '#ff8f00' : '#222533', 'center', 'bold');
  R.text(S.failKind === 'miss' ? '漏掉了一块黑块' : '按到了白键', cx, top + 70, 14, '#e53935');
  R.text(S.song.name, cx, top + 100, 15, '#6f7590');
  R.text(String(S.score), cx, top + 142, 48, '#222533', 'center', 'bold');
  R.text('完美 ' + S.grades[0] + '  ·  很好 ' + S.grades[1] + '  ·  不错 ' + S.grades[2], cx, top + 186, 14, '#555b70');
  R.text('最高连击 ' + S.maxCombo + '  ·  本曲最高 ' + (S.best[S.song.id] || 0) + '  ·  ×' + S.mul.toFixed(2), cx, top + 210, 13, '#8b91a8');
  S.buttons.again = R.button('再来一次', cx, top + 256, pw - 60, 44, '#222533', '#ffffff');
  S.buttons.next = R.button('下一首', cx - (pw - 60) / 4 - 4, top + 312, (pw - 60) / 2 - 8, 38, '#eef0f5', '#222533');
  S.buttons.menu = R.button('选曲', cx + (pw - 60) / 4 + 4, top + 312, (pw - 60) / 2 - 8, 38, '#eef0f5', '#222533');
}

function menu(R, S, SONGS) {
  R.begin('#232740');
  R.layer('menu-bg', 1, function () {
    R.gradRect(0, 0, W, B.hudTop + 70, [[0, '#14161f'], [1, '#232740']], true);
    // 底部一排琴键当装饰
    var kw = W / 14, ky = H - 70;
    for (var i = 0; i < 14; i++) R.rect(i * kw + 1, ky, kw - 2, 70, '#e9ebf2');
    [0, 1, 3, 4, 5, 7, 8, 10, 11, 12].forEach(function (k) { R.rect((k + 1) * kw - kw * 0.3, ky, kw * 0.6, 42, '#14161f'); });
  });
  var y0 = B.hudTop + 18;
  R.text('钢琴块', W / 2, y0, 34, '#ffffff', 'center', 'bold');
  R.text('黑块落到线上时按下去，漏一块就结束', W / 2, y0 + 34, 13, '#a7adc6');
  var ch = Math.min(64, Math.floor((H - 70 - 56 - (y0 + 60)) / SONGS.length) - 8), cy = y0 + 62;
  S.cards.length = 0;
  for (var i = 0; i < SONGS.length; i++) {
    var s = SONGS[i], x = 18, w = W - 36, y = cy + i * (ch + 8), on = i === S.songIdx;
    R.rrect(x, y, w, ch, 12, on ? '#3d4466' : 'rgba(255,255,255,0.08)');
    R.circle(x + 26, y + ch / 2, 15, on ? '#ffb300' : '#4a5175');
    R.text(String(i + 1), x + 26, y + ch / 2 + 1, 15, on ? '#1b1e2b' : '#ffffff', 'center', 'bold');
    R.text(s.name, x + 52, y + ch / 2 - 10, 17, '#ffffff', 'left', 'bold');
    R.text(s.by, x + 52, y + ch / 2 + 12, 12, '#9aa1bf', 'left');
    R.text('★★★★'.slice(0, s.stars) + '☆☆☆☆'.slice(0, 4 - s.stars), x + w - 14, y + ch / 2 - 10, 13, '#ffb300', 'right');
    var b = S.best[s.id] || 0;
    R.text(b ? '最高 ' + b : '未弹过', x + w - 14, y + ch / 2 + 12, 12, b ? '#e0e3ef' : '#7d84a3', 'right');
    S.cards.push({ x: x, y: y, w: w, h: ch });
  }
  S.buttons.mute = R.button(S.muted ? '声音：关' : '声音：开', W / 2, H - 70 - 26, 120, 32, 'rgba(255,255,255,0.14)', '#ffffff');
}

module.exports = { setup: setup, draw: draw };
