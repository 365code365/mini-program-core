// 对战以外的画面：加载、标题、选人、VS 过场、暂停、结算。每个画面画出来时返回按钮命中框，game.js 收到点按再判。
var B = require('./base.js');
var D = require('./data.js');

var W = B.W, H = B.H;

// 图按「铺满」画（等比放大到盖住整屏，多出来的裁掉；gl2d 不裁剪，所以算好源矩形只取看得见的那块）
function cover(g, tex, dim) {
  if (!tex) { g.gradRect(0, 0, W, H, [[0, '#1a0a20'], [1, '#05030a']], true); return; }
  var s = Math.max(W / tex.w, H / tex.h), sw = W / s, sh = H / s;
  g.image(tex, (tex.w - sw) / 2, (tex.h - sh) / 2, sw, sh, 0, 0, W, H);
  if (dim) g.rect(0, 0, W, H, [0, 0, 0, dim]);
}

function btn(g, label, cx, cy, w, h, hot, t) {
  var c = hot ? ['#ff9a2a', '#c8320a'] : ['#3a4a7a', '#1a2040'];
  g.poly([[cx - w / 2 + 10, cy - h / 2], [cx + w / 2, cy - h / 2], [cx + w / 2 - 10, cy + h / 2], [cx - w / 2, cy + h / 2]], 'rgba(255,255,255,0.85)');
  g.poly([[cx - w / 2 + 11.5, cy - h / 2 + 2], [cx + w / 2 - 2, cy - h / 2 + 2], [cx + w / 2 - 11.5, cy + h / 2 - 2], [cx - w / 2 + 2, cy + h / 2 - 2]], c[1]);
  g.poly([[cx - w / 2 + 11.5, cy - h / 2 + 2], [cx + w / 2 - 2, cy - h / 2 + 2], [cx + w / 2 - 6.5, cy], [cx - w / 2 + 7, cy]], c[0]);
  if (hot) { g.setBlend('add'); g.rect(cx - w / 2, cy - h / 2, w, h, [1, 0.6, 0.2, 0.12 + 0.08 * Math.sin(t * 4)]); g.setBlend(); }
  g.text(label, cx, cy + 1, Math.round(h * 0.42), '#ffffff', 'center', 'bold', ['rgba(0,0,0,0.6)', 1.5]);
  return { x: cx - w / 2, y: cy - h / 2, w: w, h: h };
}

function loading(g, art, k, t) {
  g.gradRect(0, 0, W, H, [[0, '#14081e'], [1, '#040208']], true);
  title(g, W / 2, H * 0.4, t);
  g.rrect(W / 2 - 120, H * 0.66, 240, 8, 4, 'rgba(255,255,255,0.15)');
  g.rrect(W / 2 - 120, H * 0.66, 240 * k, 8, 4, '#ff8a2a');
  g.text('载入中 ' + Math.round(k * 100) + '%', W / 2, H * 0.66 + 24, 12, '#c8b8d8');
}

function title(g, cx, cy, t) {
  g.setBlend('add');
  g.glow(cx, cy, 150, [1, 0.35, 0.1, 0.22 + 0.06 * Math.sin(t * 3)]);
  g.setBlend();
  g.text('拳 魂', cx, cy, 64, '#ffe2a0', 'center', 'bold', ['#9a1a00', 5]);
  g.text('F I S T   S O U L', cx, cy + 46, 14, '#ffb070', 'center', 'bold', ['rgba(0,0,0,0.8)', 2]);
}

// 标题：封面 + 标题 + 三个按钮
function titleScreen(g, art, st, t) {
  cover(g, art.cover, 0.15);
  g.gradRect(0, H * 0.55, W, H * 0.45, [[0, 'rgba(0,0,0,0)'], [1, 'rgba(0,0,0,0.85)']], true);
  g.gradRect(0, 0, W, H * 0.3, [[0, 'rgba(0,0,0,0.6)'], [1, 'rgba(0,0,0,0)']], true);
  title(g, W / 2, H * 0.2, t);
  var y = H * 0.72;
  var b = {
    start: btn(g, '开 始 对 战', W / 2, y, 190, 42, true, t),
    diff: btn(g, '难度：' + D.DIFFS[st.diff].name, W / 2 - 160, y + 4, 130, 32, false, t),
    sound: btn(g, '音效：' + (st.muted ? '关' : '开'), W / 2 + 160, y + 4, 130, 32, false, t)
  };
  g.text('左侧摇杆移动 / 跳 / 蹲，按住「后」防御；右侧 A B C D 出拳脚，「波」「升」放必杀，能量满一格放「超」', W / 2, H - 30, 11, 'rgba(255,255,255,0.75)');
  g.text('搓招：↓↘→ + 拳 = 波，→↓↘ + 拳 = 升；双击前 / 后冲刺、后撤　·　美术由 AI 生成（豆包 Seedream）', W / 2, H - 13, 10, 'rgba(255,255,255,0.5)');
  return b;
}

// 选人：左右两张角色卡
function selectScreen(g, art, st, t) {
  cover(g, art.stages[D.STAGES[st.stage].id], 0.55);
  g.text('选择你的格斗家', W / 2, 34, 22, '#ffe2a0', 'center', 'bold', ['#7a1a00', 3]);
  var hits = {}, cw = Math.min(W * 0.3, 250), chH = H * 0.62;
  D.ORDER.forEach(function (id, i) {
    var cx = i ? W / 2 + cw * 0.62 : W / 2 - cw * 0.62, cy = H * 0.5, def = D.CHARS[id], on = st.pick === id;
    var x = cx - cw / 2, y = cy - chH / 2;
    g.poly([[x + 16, y], [x + cw, y], [x + cw - 16, y + chH], [x, y + chH]], on ? def.color : 'rgba(255,255,255,0.25)');
    g.poly([[x + 18, y + 2.5], [x + cw - 3, y + 2.5], [x + cw - 18, y + chH - 2.5], [x + 3, y + chH - 2.5]], 'rgba(8,6,16,0.82)');
    if (on) { g.setBlend('add'); g.gradRect(x, y + chH * 0.4, cw, chH * 0.6, [[0, 'rgba(0,0,0,0)'], [1, [def.glow[0], def.glow[1], def.glow[2], 0.4]]], true); g.setBlend(); }
    var a = art.chars[id];
    if (a) {
      var p = a.poses[on ? 'win' : 'idle'], s = chH * 0.86 / p[3];
      var bob = on ? Math.sin(t * 3) * 2 : 0;
      g.image(a.tex, p[0], p[1], p[2], p[3], cx - p[4] * s * (i ? -1 : 1) - (i ? p[2] * s : 0), y + chH * 0.1 + bob, p[2] * s, p[3] * s, i === 1, on ? null : [0.75, 0.75, 0.8, 1]);
    }
    g.text(def.full, cx, y + chH - 44, 20, '#ffffff', 'center', 'bold', [def.color, 2.5]);
    g.text(def.title + '　' + def.s1.name + ' · ' + def.s2.name, cx, y + chH - 20, 11, '#e0d8f0', 'center', 'bold', ['rgba(0,0,0,0.9)', 1.5]);
    hits[id] = { x: x, y: y, w: cw, h: chH };
  });
  hits.stage = btn(g, '场景：' + D.STAGES[st.stage].name, W / 2, H - 28, 150, 30, false, t);
  hits.go = btn(g, '开 战', W - 90, H - 34, 120, 40, true, t);
  hits.back = btn(g, '返回', 70, H - 30, 90, 30, false, t);
  g.text('VS', W / 2, H * 0.5, 40, '#ffcc33', 'center', 'bold', ['#9a1a00', 4]);
  return hits;
}

// VS 过场：两人从两边滑进来
function vsScreen(g, art, st, t) {
  var k = Math.min(1, t / 0.35), e = 1 - Math.pow(1 - k, 3);
  cover(g, art.stages[D.STAGES[st.stage].id], 0.6);
  g.poly([[0, H * 0.2], [W, H * 0.12], [W, H * 0.88], [0, H * 0.8]], 'rgba(10,4,20,0.75)');
  [st.pick, st.cpu].forEach(function (id, i) {
    var a = art.chars[id], def = D.CHARS[id];
    if (!a) return;
    var p = a.poses.idle, s = H * 0.74 / p[3], w = p[2] * s;
    var x = i ? W - w - 20 + (1 - e) * W * 0.5 : 20 - (1 - e) * W * 0.5;
    g.setBlend('add');
    g.image(a.tex, p[0], p[1], p[2], p[3], x + (i ? 8 : -8), H * 0.14, w, p[3] * s, i === 1, [def.glow[0], def.glow[1], def.glow[2], 0.5]);
    g.setBlend();
    g.image(a.tex, p[0], p[1], p[2], p[3], x, H * 0.14, w, p[3] * s, i === 1);
    g.text(def.full, i ? W - 110 : 110, H * 0.86, 22, '#ffffff', 'center', 'bold', [def.color, 3]);
  });
  var vs = t < 0.3 ? 3 - t / 0.3 * 2 : 1;
  g.save(); g.translate(W / 2, H * 0.48); g.scale(vs, vs);
  g.setBlend('add'); g.glow(0, 0, 110, [1, 0.5, 0.1, 0.6]); g.setBlend();
  g.text('VS', 0, 0, 72, '#ffd23a', 'center', 'bold', ['#a01000', 5]);
  g.restore();
  g.text(D.STAGES[st.stage].name, W / 2, H * 0.68, 14, '#ffe2a0', 'center', 'bold', ['rgba(0,0,0,0.8)', 2]);
}

function pauseScreen(g, t) {
  g.rect(0, 0, W, H, 'rgba(4,2,10,0.72)');
  g.text('暂 停', W / 2, H * 0.22, 34, '#ffffff', 'center', 'bold', ['#2a3a8a', 3]);
  var b = {
    resume: btn(g, '继续', W / 2, H * 0.42, 170, 38, true, t),
    restart: btn(g, '重新开始', W / 2, H * 0.56, 170, 34, false, t),
    quit: btn(g, '返回标题', W / 2, H * 0.69, 170, 34, false, t)
  };
  g.text('A 轻拳  B 轻脚  C 重拳  D 重脚　蹲 + 脚 = 扫腿　跳 + 任意键 = 飞踢', W / 2, H * 0.84, 11, 'rgba(255,255,255,0.8)');
  g.text('普通技打中后马上按「波」「升」可以取消成必杀（连段）；轻拳可连打三下', W / 2, H * 0.84 + 18, 11, 'rgba(255,255,255,0.6)');
  return b;
}

function resultScreen(g, res, t, art, pick) {
  var k = Math.min(1, t / 0.4);
  g.rect(0, 0, W, H, [0.02, 0.01, 0.05, 0.6 * k]);
  var win = res.result === 'win';
  g.save(); g.setAlpha(k);
  g.text(win ? '胜 利' : res.result === 'draw' ? '平 局' : '败 北', W / 2, H * 0.22, 52, win ? '#ffd23a' : '#c8d0ff', 'center', 'bold', [win ? '#a01000' : '#1a2060', 4]);
  g.text('回合 ' + res.wins[0] + ' : ' + res.wins[1] + '　·　最大连击 ' + res.stats.maxCombo + '　·　用时 ' + Math.round(res.stats.frames / 60) + ' 秒' +
    (res.stats.perfect ? '　·　完美 ×' + res.stats.perfect : ''), W / 2, H * 0.36, 13, '#ffffff', 'center', 'bold', ['rgba(0,0,0,0.8)', 1.5]);
  var b = {
    again: btn(g, '再 战', W / 2, H * 0.52, 170, 40, true, t),
    select: btn(g, '换角色', W / 2 - 100, H * 0.68, 140, 32, false, t),
    quit: btn(g, '返回标题', W / 2 + 100, H * 0.68, 140, 32, false, t)
  };
  g.restore();
  return b;
}

module.exports = { loading: loading, titleScreen: titleScreen, selectScreen: selectScreen, vsScreen: vsScreen, pauseScreen: pauseScreen, resultScreen: resultScreen, btn: btn };
