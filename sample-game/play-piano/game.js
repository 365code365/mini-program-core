// 钢琴块（竖屏）：四条轨上黑块跟着曲子落下，按到黑块就弹出那个音；漏掉一块或按到白键就结束。
// 曲子弹完自动从头再来一遍，一遍比一遍快；每弹 10 块也会再快一点。
// 时间轴对着音频时钟：谱面位置 pos（格）每帧按 WebAudio 的 currentTime 增量推进，倒计时四拍与伴奏都用 start(when)
// 排在同一个时钟上 —— 画面、伴奏、按键判定三者共用一个时间源，掉帧时黑块跳一下但拍子不会漂。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var SONGS = require('js/songs.js');
var Chart = require('js/chart.js');
var Piano = require('js/piano.js');
var View = require('js/view.js');
View.setup(B);
var W = B.W, H = B.H;
var R = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2);
var piano = Piano.make();

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';
var LW = W / 4;
var GRADE = [{ name: '完美', win: 0.07, pts: 3, color: '#ffb300' }, { name: '很好', win: 0.16, pts: 2, color: '#42a5f5' }, { name: '不错', win: 1e9, pts: 1, color: '#9e9e9e' }];

// 全部状态放在 S 里：view.js 只读它画
var S = {
  scene: 'menu', songIdx: Number(B.load('piano.last', 0)) || 0, best: B.load('piano.best', {}) || {},
  chart: null, song: null, U: 74, JY: H - 104, pos: 0, mul: 1, next: 0, bassNext: 0,
  score: 0, combo: 0, maxCombo: 0, hits: 0, grades: [0, 0, 0], lastGrade: null, gradeAt: -9,
  startAt: 0, holdUntil: 0, failAt: 0, failLane: -1, failY: 0, failKind: '', newBest: false,
  paused: false, clock: 0, ripples: [], cards: [], buttons: {}, clockSrc: 'audio', runs: 0, muted: !!B.load('piano.muted', false)
};
if (typeof S.best !== 'object') S.best = {};
piano.setMuted(S.muted);
// 先把每首曲子用到的音排进预合成队列：菜单里空闲的那几帧一帧做一个
SONGS.forEach(function (s) { piano.preload(Chart.notesOf(s)); });

// ───────── 时钟：优先音频时钟；上下文没在跑（无声设备、被挂起）就退回帧时间 ─────────
var lastAudio = -1, frozen = 0;
function audioDelta(dt) {
  if (S.clockSrc === 'audio' && piano.running()) {
    var now = piano.now();
    if (lastAudio < 0) lastAudio = now;
    var d = now - lastAudio;
    lastAudio = now;
    if (d > 0) { frozen = 0; return Math.min(d, 0.1); }
    frozen += dt;
    if (frozen < 0.3) return 0;                         // 音频线程偶尔一帧不出数：等一等，别马上切
    S.clockSrc = 'frame'; console.log('[piano] 音频时钟不走，改用帧时间');
  }
  return dt;
}
function nowA() { return S.clockSrc === 'audio' && piano.running() ? piano.now() : S.clock; }
function speed() { return S.song.ups * S.mul; }        // 每秒几格
S.nowA = nowA;

// ───────── 开局 / 结束 ─────────
function start(idx) {
  S.songIdx = (idx + SONGS.length) % SONGS.length;
  B.save('piano.last', S.songIdx);
  S.song = SONGS[S.songIdx];
  S.chart = new Chart(S.song); S.chart.append();
  S.U = S.song.def === 1 ? Math.round(H / 12) : Math.round(H / 9);
  S.JY = H - Math.round(S.U * 1.4);
  S.mul = 1; S.next = 0; S.bassNext = 0; S.score = 0; S.combo = 0; S.maxCombo = 0; S.hits = 0; S.grades = [0, 0, 0];
  S.lastGrade = null; S.failLane = -1; S.newBest = false; S.ripples.length = 0;
  piano.preload(Chart.notesOf(S.song).reverse());
  // 倒计时：三拍提示音 + 第四拍开弹，四个音一次排进音频时钟；pos 从负数开始，第一块的下沿正好在第四拍压到判定线
  var beat = 0.5, t0 = nowA() + 0.12;
  for (var i = 0; i < 3; i++) piano.blip(i < 2 ? 880 : 988, t0 + i * beat, 0.12, 'sine', 0.25);
  piano.blip(1319, t0 + 3 * beat, 0.25, 'triangle', 0.3);
  S.startAt = t0 + 3 * beat;
  S.pos = -(S.startAt - nowA()) * speed();
  S.holdUntil = 0;
  S.scene = 'count';
  lastAudio = -1;
  autoPlan();
}
function fail(kind, lane, y) {
  if (S.scene !== 'play' && S.scene !== 'count') return;
  S.scene = 'fail'; S.failAt = S.clock; S.failKind = kind; S.failLane = lane; S.failY = y;
  piano.blip(185, 0, 0.5, 'sawtooth', 0.22); piano.blip(196, 0, 0.5, 'square', 0.12);   // 小二度叠在一起：刺耳的「错了」
  B.buzz();
  var key = S.song.id;
  if (S.score > (S.best[key] || 0)) { S.best[key] = S.score; S.newBest = true; B.save('piano.best', S.best); }
  S.runs++;
  console.log('[piano] ' + S.song.name + ' 结束（' + (kind === 'miss' ? '漏按' : '按错') + '）：' + S.hits + ' 块 ' + S.score + ' 分，速度 ×' + S.mul.toFixed(2) + '，音频时钟 ' + nowA().toFixed(2) + 's');
}

// ───────── 按键判定 ─────────
// 点在第 lane 条轨、纵坐标 y：落在「下一块」上（上下各放宽 0.4 格）就算弹中；落在已弹过 / 更后面的黑块上不算错；
// 落在白键（这条轨这个高度没有黑块）上就结束
function yOf(u) { return S.JY - (u - S.pos) * S.U; }
function press(lane, y) {
  if (S.scene !== 'play') return;
  var tiles = S.chart.tiles, nx = tiles[S.next], slack = S.U * 0.4;
  if (nx && nx.lane === lane && y <= yOf(nx.start) + slack && y >= yOf(nx.start + nx.dur) - slack) return hitTile(nx, lane, y);
  for (var i = Math.max(0, S.next - 4); i < tiles.length && i < S.next + 12; i++) {
    var t = tiles[i];
    if (t.lane === lane && y <= yOf(t.start) && y >= yOf(t.start + t.dur)) return;     // 按在别的黑块上：不算
  }
  fail('wrong', lane, y);
}
function hitTile(t, lane, y) {
  t.hit = true; t.hitAt = S.clock;
  piano.play(t.midi, 0);
  var dt = Math.abs(S.pos - t.start) / speed(), g = 0;
  while (dt > GRADE[g].win) g++;
  S.grades[g]++; S.lastGrade = GRADE[g]; S.gradeAt = S.clock;
  S.combo = g < 2 ? S.combo + 1 : 0; S.maxCombo = Math.max(S.maxCombo, S.combo);
  S.score += GRADE[g].pts + (S.combo >= 10 ? 1 : 0);
  S.hits++; S.next++;
  if (S.hits % 10 === 0) S.mul = Math.min(2.6, S.mul + 0.03);
  ripple(lane * LW + LW / 2, y);
}
function ripple(x, y) {
  var r = S.ripples.length < 8 ? {} : S.ripples.shift();
  r.x = x; r.y = y; r.t = S.clock; S.ripples.push(r);
}

// ───────── 每帧 ─────────
function update(dt) {
  S.clock += dt;
  piano.pump();
  if (S.paused) return;
  if (S.scene === 'count' || S.scene === 'play') {
    var d = audioDelta(dt), now = nowA();
    if (now >= S.holdUntil) S.pos += d * speed();
    if (S.scene === 'count' && S.pos >= -0.5) S.scene = 'play';
    var tiles = S.chart.tiles, nx = tiles[S.next];
    // 漏按：下一块的下沿出了屏幕底边
    if (nx && S.scene === 'play' && yOf(nx.start) > H + 2) { fail('miss', nx.lane, H - S.U); return; }
    // 快弹到这一遍末尾：接下一遍，并且整体提速
    if (S.chart.end - S.pos < 24) { S.chart.append(); S.mul = Math.min(2.6, S.mul + 0.12); }
    scheduleBass(now);
    if (AUTO) autoStep();
  } else if (S.scene === 'fail') {
    // 结束动画：谱面往回倒，把漏掉 / 按错的那一块拉回判定线附近给人看清楚
    if (S.failKind === 'miss') {
      var t = S.chart.tiles[S.next], target = t.start - 1.2;
      S.pos += (target - S.pos) * Math.min(1, dt * 8);
    }
    if (S.clock - S.failAt > 0.9) S.scene = 'over';
  } else if (S.scene === 'over') {
    if (AUTO && S.clock - S.failAt > 2.4) start(S.songIdx + 1);
  } else if (S.scene === 'menu') {
    if (AUTO && S.clock > 0.3) start(S.songIdx);
  }
  // 低于 2 格的谱面不再需要（早就滚出屏幕）：截掉，免得数组越滚越长
  if (S.chart && S.next > 64) { S.chart.tiles.splice(0, 32); S.next -= 32; }
}
// 伴奏：把未来 0.3s 内要响的伴奏音按「格 → 音频时刻」换算好，start(when) 排进去
function scheduleBass(now) {
  var ev = S.chart.bass, from = Math.max(now, S.holdUntil);
  while (S.bassNext < ev.length) {
    var e = ev[S.bassNext], when = from + (e.at - S.pos) / speed();
    if (when > now + 0.3) break;
    if (when >= now - 0.02 && S.clockSrc === 'audio') piano.play(e.midi, Math.max(when, now), true);
    else if (when >= now - 0.02) piano.play(e.midi, 0, true);
    S.bassNext++;
  }
  if (S.bassNext > 64) { ev.splice(0, 32); S.bassNext -= 32; }
}

// ───────── 自动试玩：到点就按，偶尔早晚一点；弹够一定数量后故意漏一块 / 按错一次，好推进到结算 ─────────
var plan = { limit: 0, jitter: 0 };
function autoPlan() { plan.limit = 28 + Math.floor(Math.random() * 30); plan.jitter = (Math.random() - 0.5) * 0.12; }
function autoStep() {
  if (S.scene !== 'play') return;
  var t = S.chart.tiles[S.next];
  if (!t) return;
  if (S.hits >= plan.limit) {
    if (S.runs % 2 && S.pos >= t.start - 1) press((t.lane + 2) % 4, S.JY - S.U * 0.5);     // 按到白键
    return;                                                                              // 否则就让它漏掉
  }
  if ((S.pos - t.start) / speed() >= plan.jitter) {
    press(t.lane, Math.max(yOf(t.start + t.dur) + 6, Math.min(yOf(t.start) - 6, S.JY - 4)));
    plan.jitter = (Math.random() - 0.5) * 0.12;
  }
}

// ───────── 输入 ─────────
B.on('down', function (x, y) {
  if (S.paused) { resume(); return; }
  if (S.scene === 'play' || S.scene === 'count') { press(Math.max(0, Math.min(3, Math.floor(x / LW))), y); return; }
});
B.on('tap', function (x, y) {
  if (S.scene === 'menu') {
    for (var i = 0; i < S.cards.length; i++) if (B.hit(S.cards[i], x, y)) return start(i);
    if (B.hit(S.buttons.mute, x, y)) toggleMute();
  } else if (S.scene === 'over') {
    if (B.hit(S.buttons.again, x, y)) start(S.songIdx);
    else if (B.hit(S.buttons.next, x, y)) start(S.songIdx + 1);
    else if (B.hit(S.buttons.menu, x, y)) S.scene = 'menu';
  }
});
function toggleMute() { S.muted = !S.muted; piano.setMuted(S.muted); B.save('piano.muted', S.muted); }
var LANE_KEYS = { KeyD: 0, d: 0, KeyF: 1, f: 1, KeyJ: 2, j: 2, KeyK: 3, k: 3, Digit1: 0, Digit2: 1, Digit3: 2, Digit4: 3 };
if (wx.onKeyDown) wx.onKeyDown(function (e) {
  var lane = LANE_KEYS[e.code]; if (lane === undefined) lane = LANE_KEYS[e.key];
  if (lane === undefined || S.scene !== 'play') return;
  var t = S.chart.tiles[S.next];
  // 键盘没有纵坐标：下一块在这条轨上就算按在它身上，否则按在判定线上
  press(lane, t && t.lane === lane ? Math.max(yOf(t.start + t.dur) + 2, Math.min(yOf(t.start) - 2, S.JY)) : S.JY - 2);
});
B.on('key', function (k) {
  if (k === 'pause') { if (S.paused) resume(); else if (S.scene === 'play' || S.scene === 'count') pause(); }
  else if (k === 'ok') { if (S.paused) resume(); else if (S.scene === 'menu' || S.scene === 'over') start(S.songIdx); }
  else if (S.scene === 'menu' && (k === 'up' || k === 'down')) S.songIdx = (S.songIdx + (k === 'up' ? -1 : 1) + SONGS.length) % SONGS.length;
});

// ───────── 前后台：挂起音频上下文（currentTime 跟着停，谱面自然停住）；回来点一下再继续 ─────────
function pause() { S.paused = true; piano.suspend(); }
function resume() {
  S.paused = false; piano.resume(); lastAudio = -1;
  // 给 1 秒缓冲：这 1 秒里谱面不动，伴奏的排程也顺延（scheduleBass 从 holdUntil 起算）
  if (S.scene === 'play' || S.scene === 'count') { S.holdUntil = nowA() + 1; piano.blip(880, S.holdUntil - 0.5, 0.1, 'sine', 0.2); piano.blip(1319, S.holdUntil, 0.15, 'triangle', 0.25); }
}
B.onHide(function () { if (S.scene === 'play' || S.scene === 'count') pause(); else piano.suspend(); });
B.onShow(function () { if (!S.paused) piano.resume(); });

GameGlobal.__play = function () {
  return {
    scene: S.scene, song: S.song ? S.song.name : null, score: S.score, hits: S.hits, combo: S.combo, maxCombo: S.maxCombo,
    grades: S.grades.slice(), speed: Math.round(S.mul * 100) / 100, pos: Math.round(S.pos * 10) / 10, runs: S.runs,
    best: S.best, clock: S.clockSrc, audioTime: Math.round(piano.now() * 1000) / 1000, audio: piano.stats, pendingSynth: piano.pending(),
    loops: S.chart ? S.chart.loops : 0, fail: S.failKind
  };
};

B.loop(update, function () { View.draw(R, S, SONGS, GRADE); });
