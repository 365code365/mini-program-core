// 画面：2D 画布（wx.createCanvas().getContext('2d')）。不变的东西先画到离屏画布上：
// 木纹棋盘 + 网格 + 星位 + 坐标（一张）、黑白棋子（各一张，带径向渐变高光与投影），每帧只 drawImage 贴上去，
// 所以一盘下满 225 子也只是 225 次贴图，不用每帧重算渐变。
var N = 15;
var LEVEL_NAME = ['', '初级', '中级', '高级'];

function make(B) {
  var W = B.W, H = B.H, dpr = B.dpr;
  var top = Math.max(B.hudTop, 44) + 2;
  var S = Math.floor(Math.min(W - 20, H - top - 66 - 150)), BX = Math.floor((W - S) / 2), BY = top + 64;
  var PAD = S * 0.05, GAP = (S - PAD * 2) / (N - 1), R = GAP * 0.46;
  var BOTTOM = BY + S;
  function px(i) { return BX + PAD + (i % N) * GAP; }
  function py(i) { return BY + PAD + Math.floor(i / N) * GAP; }
  // 屏幕坐标 → 最近的交叉点（离得太远返回 -1）
  function pick(x, y) {
    var cx = Math.round((x - BX - PAD) / GAP), cy = Math.round((y - BY - PAD) / GAP);
    if (cx < 0 || cy < 0 || cx >= N || cy >= N) return -1;
    var dx = x - (BX + PAD + cx * GAP), dy = y - (BY + PAD + cy * GAP);
    return dx * dx + dy * dy <= GAP * GAP * 0.6 ? cy * N + cx : -1;
  }

  function offscreen(w, h) {
    var c = wx.createCanvas();
    c.width = Math.ceil(w * dpr); c.height = Math.ceil(h * dpr);
    var g = c.getContext('2d');
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    return { c: c, g: g };
  }
  // 木纹棋盘：线性渐变底 + 几十条半透明的弯曲细线当木纹（固定种子，每次打开一样）
  var board = offscreen(S, S);
  (function () {
    var g = board.g, seed = 7;
    function rnd() { seed = (seed * 16807) % 2147483647; return seed / 2147483647; }
    var grad = g.createLinearGradient(0, 0, S, S);
    grad.addColorStop(0, '#f0c987'); grad.addColorStop(0.5, '#e2b06a'); grad.addColorStop(1, '#d49d55');
    g.fillStyle = grad;
    g.beginPath(); g.moveTo(10, 0); g.arcTo(S, 0, S, S, 10); g.arcTo(S, S, 0, S, 10); g.arcTo(0, S, 0, 0, 10); g.arcTo(0, 0, S, 0, 10); g.closePath(); g.fill();
    for (var k = 0; k < 36; k++) {
      var y0 = rnd() * S, amp = 2 + rnd() * 6;
      g.strokeStyle = 'rgba(140,90,40,' + (0.05 + rnd() * 0.08).toFixed(3) + ')';
      g.lineWidth = 0.6 + rnd() * 1.4;
      g.beginPath(); g.moveTo(4, y0);
      g.bezierCurveTo(S * 0.33, y0 + amp, S * 0.66, y0 - amp, S - 4, y0 + amp * 0.5);
      g.stroke();
    }
    g.strokeStyle = '#5d3a1a'; g.lineWidth = 1;
    for (var i = 0; i < N; i++) {
      var p = PAD + i * GAP;
      g.beginPath(); g.moveTo(PAD, p); g.lineTo(S - PAD, p); g.stroke();
      g.beginPath(); g.moveTo(p, PAD); g.lineTo(p, S - PAD); g.stroke();
    }
    g.lineWidth = 2; g.strokeRect(PAD, PAD, S - PAD * 2, S - PAD * 2);
    g.fillStyle = '#5d3a1a';
    [[3, 3], [11, 3], [3, 11], [11, 11], [7, 7]].forEach(function (s) {
      g.beginPath(); g.arc(PAD + s[0] * GAP, PAD + s[1] * GAP, 3.2, 0, Math.PI * 2); g.fill();
    });
    g.font = '7px sans-serif'; g.fillStyle = 'rgba(93,58,26,0.7)'; g.textAlign = 'center'; g.textBaseline = 'middle';
    for (var j = 0; j < N; j++) {
      g.fillText('ABCDEFGHIJKLMNO'[j], PAD + j * GAP, PAD * 0.42);
      g.fillText(String(N - j), PAD * 0.42, PAD + j * GAP);
    }
  })();
  // 棋子：投影 + 径向渐变（左上高光）
  function stone(black) {
    var size = R * 2 + 6, o = offscreen(size, size), g = o.g, c = size / 2;
    g.fillStyle = 'rgba(40,20,0,0.28)';
    g.beginPath(); g.arc(c + 1.2, c + 2, R, 0, Math.PI * 2); g.fill();
    var gr = g.createRadialGradient(c - R * 0.35, c - R * 0.4, R * 0.1, c, c, R);
    if (black) { gr.addColorStop(0, '#6b6b6b'); gr.addColorStop(0.35, '#2a2a2a'); gr.addColorStop(1, '#050505'); }
    else { gr.addColorStop(0, '#ffffff'); gr.addColorStop(0.6, '#ecebe6'); gr.addColorStop(1, '#b9b6ad'); }
    g.fillStyle = gr;
    g.beginPath(); g.arc(c, c, R, 0, Math.PI * 2); g.fill();
    o.size = size;
    return o;
  }
  var STONE = [null, stone(true), stone(false)];
  function drawStone(ctx, i, p, alpha) {
    var s = STONE[p];
    if (alpha !== undefined) ctx.globalAlpha = alpha;
    ctx.drawImage(s.c, px(i) - s.size / 2, py(i) - s.size / 2, s.size, s.size);
    if (alpha !== undefined) ctx.globalAlpha = 1;
  }

  function card(ctx, x, w, who, p, active, sub, thinking, clock) {
    var y = top, h = 54;
    B.rrect(x, y, w, h, 12);
    ctx.fillStyle = active ? '#fffaf0' : 'rgba(255,250,240,0.55)'; ctx.fill();
    if (active) { ctx.strokeStyle = '#c0843c'; ctx.lineWidth = 2; ctx.stroke(); }
    var s = STONE[p], d = 26;
    ctx.drawImage(s.c, x + 12, y + h / 2 - d / 2, d, d);
    B.text(who, x + 46, y + 19, 15, '#4e342e', 'left', 'bold');
    var tail = thinking ? '思考中' + '...'.slice(0, 1 + Math.floor(clock * 3) % 3) : sub;
    B.text(tail, x + 46, y + 38, 12, thinking ? '#c0843c' : '#8d6e63', 'left');
  }

  // st：{ board, moves, human, level, turn, state, thinking, win, result, record, clock, ghost, nextLevel, nextHuman, aiMs }
  function draw(ctx, st) {
    var buttons = [];
    ctx.fillStyle = '#efe3cf'; ctx.fillRect(0, 0, W, H);
    B.text('五子棋', BX + 4, Math.max(B.hudTop - 24, 22), 19, '#4e342e', 'left', 'bold');
    var ai = 3 - st.human, cw = (S - 10) / 2, rec = st.record;
    card(ctx, BX, cw, '你', st.human, st.state === 'play' && st.turn === st.human, (st.human === 1 ? '执黑先行' : '执白后手') + ' · 胜 ' + rec.w + ' 负 ' + rec.l, false, st.clock);
    card(ctx, BX + cw + 10, cw, 'AI · ' + LEVEL_NAME[st.level], ai, st.state === 'play' && st.turn === ai,
      st.aiMs >= 0 ? '上一手用时 ' + st.aiMs + 'ms' : (ai === 1 ? '执黑先行' : '执白后手'), st.thinking, st.clock);

    ctx.drawImage(board.c, BX, BY, S, S);
    var last = st.moves.length ? st.moves[st.moves.length - 1] : -1;
    for (var i = 0; i < st.moves.length; i++) drawStone(ctx, st.moves[i], i % 2 === 0 ? 1 : 2);
    if (last >= 0) {
      ctx.fillStyle = '#e53935';
      ctx.fillRect(px(last) - 3, py(last) - 3, 6, 6);
    }
    if (st.win) {
      var a = st.win[0], b = st.win[st.win.length - 1], pulse = 0.75 + 0.25 * Math.sin(st.clock * 6);
      ctx.strokeStyle = 'rgba(229,57,53,' + pulse.toFixed(3) + ')'; ctx.lineWidth = 4; ctx.lineCap = 'round';
      ctx.beginPath(); ctx.moveTo(px(a), py(a)); ctx.lineTo(px(b), py(b)); ctx.stroke();
      ctx.lineWidth = 2;
      for (var k = 0; k < st.win.length; k++) { ctx.beginPath(); ctx.arc(px(st.win[k]), py(st.win[k]), R + 2, 0, Math.PI * 2); ctx.stroke(); }
    }
    if (st.ghost >= 0 && !st.board[st.ghost]) {
      ctx.strokeStyle = 'rgba(229,57,53,0.6)'; ctx.lineWidth = 1;
      ctx.beginPath(); ctx.moveTo(BX + PAD, py(st.ghost)); ctx.lineTo(BX + S - PAD, py(st.ghost)); ctx.stroke();
      ctx.beginPath(); ctx.moveTo(px(st.ghost), BY + PAD); ctx.lineTo(px(st.ghost), BY + S - PAD); ctx.stroke();
      drawStone(ctx, st.ghost, st.human, 0.55);
    }

    // 状态行
    var msg, color = '#5d4037';
    if (st.state === 'over') {
      var R0 = { human: ['你赢了！', '#2e7d32'], ai: ['AI 获胜', '#c62828'], resign: ['你认输了', '#c62828'], draw: ['和棋', '#5d4037'] }[st.result];
      msg = R0[0] + ' · 共 ' + st.moves.length + ' 手'; color = R0[1];
    } else if (st.thinking) msg = 'AI 正在思考…';
    else msg = st.moves.length ? '轮到你 · 第 ' + (st.moves.length + 1) + ' 手' : (st.turn === st.human ? '你先手，点棋盘落子' : '');
    B.text(msg, W / 2, BOTTOM + 20, st.state === 'over' ? 18 : 15, color, 'center', 'bold');

    // 按钮行：悔棋 / 认输 / 新局
    var y1 = BOTTOM + 58, bw = (S - 20) / 3, can = st.state === 'play' && st.moves.length > 0;
    buttons.push(['undo', B.button('悔棋', BX + bw / 2, y1, bw, 40, can ? '#8d5524' : '#cdbba3', '#ffffff')]);
    buttons.push(['resign', B.button('认输', BX + bw * 1.5 + 10, y1, bw, 40, st.state === 'play' ? '#8d5524' : '#cdbba3', '#ffffff')]);
    buttons.push(['new', B.button(st.state === 'over' ? '再来一局' : '新局', BX + bw * 2.5 + 20, y1, bw, 40, '#c0843c', '#ffffff')]);
    // 难度与先后手（对局中改了下一局生效，标在右上角小字）
    var y2 = y1 + 50, sw = (S - 10) * 0.6 / 3, hw = (S - 10) * 0.4 / 2;
    for (var lv = 1; lv <= 3; lv++) {
      var on = st.nextLevel === lv;
      buttons.push(['lv' + lv, B.button(LEVEL_NAME[lv], BX + sw * (lv - 0.5), y2, sw - 6, 32, on ? '#4e342e' : 'rgba(78,52,46,0.12)', on ? '#ffffff' : '#6d4c41')]);
    }
    var hx = BX + sw * 3 + 10;
    [[1, '我先手'], [2, 'AI 先手']].forEach(function (o, j) {
      var on = st.nextHuman === o[0];
      buttons.push(['first' + o[0], B.button(o[1], hx + hw * (j + 0.5), y2, hw - 6, 32, on ? '#4e342e' : 'rgba(78,52,46,0.12)', on ? '#ffffff' : '#6d4c41')]);
    });
    if (st.state === 'play' && st.moves.length && (st.nextLevel !== st.level || st.nextHuman !== st.human))
      B.text('设置下一局生效', W / 2, y2 + 28, 11, '#a1887f', 'center');
    var best = rec.fastest ? '最快 ' + rec.fastest + ' 手取胜' : '还没有胜局';
    B.text(LEVEL_NAME[st.level] + '战绩：胜 ' + rec.w + ' · 负 ' + rec.l + ' · 和 ' + rec.d + ' · 最长连胜 ' + rec.bestStreak + ' · ' + best,
      W / 2, Math.min(H - 14, y2 + 46), 11, '#8d6e63', 'center');
    return buttons;
  }
  return { draw: draw, pick: pick, px: px, py: py, BX: BX, BY: BY, S: S };
}

module.exports = { make: make, LEVEL_NAME: LEVEL_NAME };
