// 程序生成的图标集：启动时在一张 1024×1024 离屏画布上画好（每格 128），界面用 drawImage 从里面取，
// 3D 气泡用同一张图做纹理（每个 Sprite 一份 Texture.clone，共用同一个图像源，只传一次）。
// 画法：渐变底 + 高光 + 深色描边 + 底部投影，统一 3/4 俯视的糖果风。
var S = 128, COLS = 8;

// art：AI 图标集（art/icons.webp + manifest）：有的图标用 AI 画的（等比放进格子里），没有的用下面代码画的
function make(THREE, art) {
  var AI = art && art.icons && art.manifest && art.manifest.icons ? art.manifest.icons : {}, AS = (art && art.manifest && art.manifest.iconSize) || 128;
  function aiDraw(c, name) { var p = AI[name]; if (!p) return false; c.drawImage(art.icons, p[0], p[1], AS, AS, 0, 0, S, S); return true; }
  var cv = wx.createCanvas(); cv.width = S * COLS; cv.height = S * COLS;
  var g = cv.getContext('2d'), slots = {}, n = 0;
  function cell(name, draw) {
    var x = (n % COLS) * S, y = Math.floor(n / COLS) * S; n++;
    slots[name] = { x: x, y: y };
    g.save(); g.translate(x, y); g.beginPath(); g.rect(0, 0, S, S); g.clip();
    draw(g);
    g.restore();
  }
  function lin(c, y0, y1, a, b) { var gr = c.createLinearGradient(0, y0, 0, y1); gr.addColorStop(0, a); gr.addColorStop(1, b); return gr; }
  function rad(c, x, y, r, a, b) { var gr = c.createRadialGradient(x - r * 0.35, y - r * 0.4, r * 0.1, x, y, r); gr.addColorStop(0, a); gr.addColorStop(1, b); return gr; }
  function shadow(c, x, y, w) { c.fillStyle = 'rgba(0,0,0,0.18)'; c.beginPath(); c.ellipse(x, y, w, w * 0.22, 0, 0, Math.PI * 2); c.fill(); }
  function outline(c, w, col) { c.lineWidth = w || 5; c.strokeStyle = col || 'rgba(60,35,10,0.85)'; c.lineJoin = 'round'; c.stroke(); }
  function shine(c, x, y, rx, ry, a) { c.fillStyle = 'rgba(255,255,255,' + (a || 0.55) + ')'; c.beginPath(); c.ellipse(x, y, rx, ry, -0.5, 0, Math.PI * 2); c.fill(); }
  function rr(c, x, y, w, h, r) { c.beginPath(); c.moveTo(x + r, y); c.arcTo(x + w, y, x + w, y + h, r); c.arcTo(x + w, y + h, x, y + h, r); c.arcTo(x, y + h, x, y, r); c.arcTo(x, y, x + w, y, r); c.closePath(); }
  // 第一个点用 moveTo：规范里空路径上的 lineTo 等于 moveTo，但本引擎的 2D 画布会从原点连一条线过来（图标上多一道斜线）
  function star(c, cx, cy, R, r, k) { c.beginPath(); for (var i = 0; i < k * 2; i++) { var a = -Math.PI / 2 + i * Math.PI / k, d = i % 2 ? r : R; c[i ? 'lineTo' : 'moveTo'](cx + Math.cos(a) * d, cy + Math.sin(a) * d); } c.closePath(); }

  // ───────── 资源 ─────────
  var DRAW = {
    coin: function (c) {
      shadow(c, 64, 112, 38);
      c.fillStyle = '#c98a12'; c.beginPath(); c.ellipse(64, 66, 44, 44, 0, 0, Math.PI * 2); c.fill();
      c.fillStyle = rad(c, 64, 60, 44, '#fff3a1', '#f5b819'); c.beginPath(); c.ellipse(64, 60, 44, 44, 0, 0, Math.PI * 2); c.fill(); outline(c, 4, '#a86c05');
      c.fillStyle = '#f1a90c'; c.beginPath(); c.ellipse(64, 60, 32, 32, 0, 0, Math.PI * 2); c.fill();
      c.fillStyle = '#fff7c2'; star(c, 64, 60, 20, 9, 5); c.fill(); shine(c, 46, 40, 14, 7);
    },
    gem: function (c) {
      shadow(c, 64, 114, 36);
      var top = 30, mid = 52, bot = 108;
      c.beginPath(); c.moveTo(34, mid); c.lineTo(48, top); c.lineTo(80, top); c.lineTo(94, mid); c.lineTo(64, bot); c.closePath();
      c.fillStyle = lin(c, top, bot, '#8ef6ff', '#1a8fe3'); c.fill(); outline(c, 4, '#0d5a9e');
      c.fillStyle = 'rgba(255,255,255,0.5)'; c.beginPath(); c.moveTo(48, top); c.lineTo(64, mid); c.lineTo(80, top); c.closePath(); c.fill();
      c.fillStyle = 'rgba(0,60,140,0.25)'; c.beginPath(); c.moveTo(64, mid); c.lineTo(94, mid); c.lineTo(64, bot); c.closePath(); c.fill();
      c.strokeStyle = 'rgba(255,255,255,0.6)'; c.lineWidth = 2; c.beginPath(); c.moveTo(34, mid); c.lineTo(94, mid); c.stroke();
      shine(c, 52, 40, 8, 4, 0.8);
    },
    wheat: function (c) {
      shadow(c, 64, 114, 30);
      c.strokeStyle = '#8b6b1f'; c.lineWidth = 5; c.beginPath(); c.moveTo(64, 110); c.quadraticCurveTo(62, 70, 66, 26); c.stroke();
      for (var i = 0; i < 6; i++) for (var s = -1; s <= 1; s += 2) {
        var y = 34 + i * 12;
        c.fillStyle = lin(c, y - 9, y + 9, '#ffe27a', '#e0a21b');
        c.beginPath(); c.ellipse(64 + s * 12, y, 11, 7, s * 0.7, 0, Math.PI * 2); c.fill(); outline(c, 2.5, '#9c6d0b');
      }
      c.fillStyle = lin(c, 16, 34, '#ffe27a', '#e0a21b'); c.beginPath(); c.ellipse(66, 24, 7, 11, 0, 0, Math.PI * 2); c.fill(); outline(c, 2.5, '#9c6d0b');
    },
    wood: function (c) {
      shadow(c, 64, 112, 44);
      [[40, 84], [88, 84], [64, 52]].forEach(function (p) {
        c.fillStyle = lin(c, p[1] - 20, p[1] + 20, '#b07a47', '#7a4b23'); rr(c, p[0] - 30, p[1] - 18, 60, 36, 18); c.fill(); outline(c, 4, '#4f2d10');
        c.fillStyle = '#e8c08a'; c.beginPath(); c.ellipse(p[0] + 24, p[1], 12, 16, 0, 0, Math.PI * 2); c.fill(); outline(c, 3, '#4f2d10');
        c.strokeStyle = '#c99b62'; c.lineWidth = 2; c.beginPath(); c.ellipse(p[0] + 24, p[1], 6, 9, 0, 0, Math.PI * 2); c.stroke();
      });
    },
    stone: function (c) {
      shadow(c, 64, 112, 44);
      c.beginPath(); c.moveTo(22, 96); c.lineTo(30, 56); c.lineTo(56, 34); c.lineTo(90, 40); c.lineTo(108, 70); c.lineTo(100, 100); c.closePath();
      c.fillStyle = lin(c, 34, 100, '#d5dbe1', '#7b8794'); c.fill(); outline(c, 4, '#3e4a56');
      c.fillStyle = 'rgba(255,255,255,0.45)'; c.beginPath(); c.moveTo(30, 56); c.lineTo(56, 34); c.lineTo(66, 56); c.lineTo(40, 70); c.closePath(); c.fill();
      c.fillStyle = 'rgba(0,0,0,0.12)'; c.beginPath(); c.moveTo(66, 56); c.lineTo(108, 70); c.lineTo(100, 100); c.lineTo(60, 98); c.closePath(); c.fill();
    },
    bread: function (c) {
      shadow(c, 64, 112, 46);
      c.fillStyle = lin(c, 34, 100, '#f7c873', '#c4741e'); rr(c, 16, 44, 96, 58, 28); c.fill(); outline(c, 4, '#7a3f0c');
      c.strokeStyle = '#fff1c9'; c.lineWidth = 5; c.lineCap = 'round';
      for (var i = 0; i < 3; i++) { c.beginPath(); c.moveTo(38 + i * 22, 56); c.lineTo(50 + i * 22, 78); c.stroke(); }
      shine(c, 40, 54, 16, 6, 0.45);
    },
    fish: function (c) {
      shadow(c, 64, 112, 40);
      c.beginPath(); c.moveTo(98, 64); c.lineTo(120, 44); c.lineTo(118, 86); c.closePath(); c.fillStyle = '#3a86ff'; c.fill(); outline(c, 4, '#123f88');
      c.fillStyle = lin(c, 38, 92, '#9bd1ff', '#2f6fe0'); c.beginPath(); c.ellipse(60, 66, 44, 26, 0, 0, Math.PI * 2); c.fill(); outline(c, 4, '#123f88');
      c.fillStyle = '#fff'; c.beginPath(); c.arc(36, 60, 8, 0, Math.PI * 2); c.fill(); c.fillStyle = '#123'; c.beginPath(); c.arc(34, 60, 4, 0, Math.PI * 2); c.fill();
      shine(c, 58, 52, 18, 5, 0.5);
    },
    honey: function (c) {
      shadow(c, 64, 114, 36);
      c.fillStyle = lin(c, 36, 108, '#ffd75e', '#e08a00'); rr(c, 30, 40, 68, 70, 22); c.fill(); outline(c, 4, '#8a4b00');
      c.fillStyle = '#a0522d'; rr(c, 36, 26, 56, 18, 6); c.fill(); outline(c, 3, '#5a2a0a');
      c.fillStyle = '#fff6d0'; rr(c, 44, 62, 40, 26, 8); c.fill();
      c.fillStyle = '#e08a00'; c.font = 'bold 22px sans-serif'; c.textAlign = 'center'; c.textBaseline = 'middle'; c.fillText('蜜', 64, 76);
      shine(c, 44, 52, 8, 12, 0.5);
    },
    xp: function (c) { shadow(c, 64, 114, 34); c.fillStyle = rad(c, 64, 62, 46, '#ffe9a8', '#ff9f1c'); star(c, 64, 62, 46, 20, 5); c.fill(); outline(c, 4, '#a85d00'); shine(c, 50, 46, 10, 5); },
    hammer: function (c) {
      c.fillStyle = '#ffffff'; c.beginPath(); c.arc(64, 64, 54, 0, Math.PI * 2); c.fill(); outline(c, 5, '#f4a300');
      c.save(); c.translate(64, 66); c.rotate(-0.6);
      c.fillStyle = lin(c, -6, 40, '#c98f5a', '#8a5a2b'); rr(c, -6, -6, 12, 46, 5); c.fill(); outline(c, 3, '#4f2d10');
      c.fillStyle = lin(c, -26, -6, '#dfe6ec', '#8996a3'); rr(c, -24, -26, 48, 22, 6); c.fill(); outline(c, 3, '#3e4a56');
      c.restore();
    },
    egg: function (c) {
      shadow(c, 64, 114, 34);
      c.fillStyle = rad(c, 64, 66, 48, '#fffaf0', '#f2d9b6'); c.beginPath(); c.ellipse(64, 66, 36, 46, 0, 0, Math.PI * 2); c.fill(); outline(c, 4, '#b07a3e');
      ['#ff8fab', '#7bdff2', '#b2f7ef', '#ffd166'].forEach(function (col, i) { c.fillStyle = col; c.beginPath(); c.arc(46 + (i % 2) * 34, 50 + Math.floor(i / 2) * 30, 8 - (i % 3), 0, Math.PI * 2); c.fill(); });
      c.strokeStyle = '#ff8fab'; c.lineWidth = 5; c.beginPath(); for (var x = 30; x <= 98; x += 8) c[x === 30 ? 'moveTo' : 'lineTo'](x, 70 + ((x / 8) % 2 ? -6 : 6)); c.stroke();
      shine(c, 48, 38, 10, 14, 0.6);
    },
    gift: function (c) {
      shadow(c, 64, 114, 44);
      c.fillStyle = lin(c, 50, 108, '#ff6b8b', '#d62857'); rr(c, 22, 50, 84, 58, 8); c.fill(); outline(c, 4, '#7a0c2e');
      c.fillStyle = lin(c, 34, 54, '#ff8fab', '#e63967'); rr(c, 16, 36, 96, 20, 6); c.fill(); outline(c, 4, '#7a0c2e');
      c.fillStyle = '#ffd166'; c.fillRect(56, 36, 16, 72); c.strokeStyle = '#a86c05'; c.lineWidth = 2; c.strokeRect(56, 36, 16, 72);
      c.fillStyle = '#ffd166'; c.beginPath(); c.ellipse(50, 28, 16, 10, -0.5, 0, Math.PI * 2); c.ellipse(78, 28, 16, 10, 0.5, 0, Math.PI * 2); c.fill(); outline(c, 3, '#a86c05');
    },
    bag: function (c) {
      shadow(c, 64, 114, 40);
      c.strokeStyle = '#6a3d9a'; c.lineWidth = 7; c.beginPath(); c.arc(64, 44, 20, Math.PI, 0); c.stroke();
      c.fillStyle = lin(c, 40, 108, '#c19bff', '#7b42d6'); rr(c, 24, 42, 80, 66, 14); c.fill(); outline(c, 4, '#3d1a78');
      c.fillStyle = '#ffd166'; star(c, 64, 74, 16, 7, 5); c.fill(); shine(c, 40, 54, 8, 12, 0.4);
    },
    scroll: function (c) {
      shadow(c, 64, 114, 40);
      c.fillStyle = lin(c, 26, 104, '#fff8e1', '#efd9a7'); rr(c, 28, 24, 72, 82, 8); c.fill(); outline(c, 4, '#8a5a2b');
      c.fillStyle = '#c98f5a'; rr(c, 20, 18, 88, 14, 7); c.fill(); outline(c, 3, '#5a3510'); rr(c, 20, 98, 88, 14, 7); c.fill(); outline(c, 3, '#5a3510');
      c.strokeStyle = '#b08b5a'; c.lineWidth = 4; c.lineCap = 'round'; for (var i = 0; i < 4; i++) { c.beginPath(); c.moveTo(42, 46 + i * 13); c.lineTo(i === 3 ? 70 : 86, 46 + i * 13); c.stroke(); }
      c.fillStyle = '#e63946'; c.beginPath(); c.arc(84, 86, 9, 0, Math.PI * 2); c.fill();
    },
    paw: function (c) {
      shadow(c, 64, 114, 36);
      c.fillStyle = lin(c, 30, 104, '#ffb3c6', '#ff6b9a');
      c.beginPath(); c.ellipse(64, 78, 26, 22, 0, 0, Math.PI * 2); c.fill(); outline(c, 4, '#a3123f');
      [[36, 50], [54, 36], [74, 36], [92, 50]].forEach(function (p) { c.beginPath(); c.ellipse(p[0], p[1], 10, 12, 0, 0, Math.PI * 2); c.fill(); outline(c, 3.5, '#a3123f'); });
    },
    calendar: function (c) {
      shadow(c, 64, 114, 40);
      c.fillStyle = '#ffffff'; rr(c, 22, 30, 84, 78, 12); c.fill(); outline(c, 4, '#2b4a7a');
      c.fillStyle = lin(c, 30, 54, '#5ab0ff', '#2f74d0'); rr(c, 22, 30, 84, 24, 12); c.fill(); c.fillRect(22, 44, 84, 10);
      c.fillStyle = '#2b4a7a'; c.font = 'bold 40px sans-serif'; c.textAlign = 'center'; c.textBaseline = 'middle'; c.fillText('7', 64, 82);
      c.fillStyle = '#2b4a7a'; [44, 84].forEach(function (x) { rr(c, x - 4, 20, 8, 20, 4); c.fill(); });
    },
    gear: function (c) {
      shadow(c, 64, 114, 36);
      c.fillStyle = lin(c, 20, 108, '#e3e8ee', '#8b98a6'); c.beginPath();
      for (var i = 0; i < 16; i++) { var a = i * Math.PI / 8, r = i % 2 ? 34 : 46; c[i ? 'lineTo' : 'moveTo'](64 + Math.cos(a) * r, 62 + Math.sin(a) * r); }
      c.closePath(); c.fill(); outline(c, 4, '#3e4a56');
      c.fillStyle = '#5c6b7a'; c.beginPath(); c.arc(64, 62, 14, 0, Math.PI * 2); c.fill();
    },
    video: function (c) {
      c.fillStyle = lin(c, 24, 104, '#ff7a7a', '#e5383b'); rr(c, 16, 28, 96, 72, 18); c.fill(); outline(c, 4, '#7a0c0c');
      c.fillStyle = '#fff'; c.beginPath(); c.moveTo(52, 46); c.lineTo(84, 64); c.lineTo(52, 82); c.closePath(); c.fill();
    },
    share: function (c) {
      c.fillStyle = lin(c, 16, 112, '#7be495', '#2a9d50'); c.beginPath(); c.arc(64, 64, 48, 0, Math.PI * 2); c.fill(); outline(c, 4, '#145a2a');
      c.strokeStyle = '#fff'; c.lineWidth = 6; c.beginPath(); c.moveTo(48, 66); c.lineTo(80, 48); c.moveTo(48, 66); c.lineTo(80, 84); c.stroke();
      c.fillStyle = '#fff'; [[44, 66], [82, 46], [82, 86]].forEach(function (p) { c.beginPath(); c.arc(p[0], p[1], 9, 0, Math.PI * 2); c.fill(); });
    },
    boost: function (c) {
      c.fillStyle = lin(c, 16, 112, '#ffe066', '#ff9f1c'); c.beginPath(); c.arc(64, 64, 50, 0, Math.PI * 2); c.fill(); outline(c, 4, '#a85d00');
      c.fillStyle = '#fff'; c.beginPath(); c.moveTo(70, 22); c.lineTo(40, 70); c.lineTo(62, 70); c.lineTo(56, 106); c.lineTo(88, 56); c.lineTo(66, 56); c.closePath(); c.fill(); outline(c, 3, '#a85d00');
    },
    crown: function (c) {
      shadow(c, 64, 112, 40);
      c.fillStyle = lin(c, 30, 100, '#fff09a', '#f2a900'); c.beginPath(); c.moveTo(20, 96); c.lineTo(24, 40); c.lineTo(46, 64); c.lineTo(64, 30); c.lineTo(82, 64); c.lineTo(104, 40); c.lineTo(108, 96); c.closePath(); c.fill(); outline(c, 4, '#8a5a00');
      ['#ff4d6d', '#4cc9f0', '#80ed99'].forEach(function (col, i) { c.fillStyle = col; c.beginPath(); c.arc(40 + i * 24, 82, 7, 0, Math.PI * 2); c.fill(); });
    },
    lock: function (c) {
      c.strokeStyle = '#6c7a89'; c.lineWidth = 10; c.beginPath(); c.arc(64, 54, 20, Math.PI, 0); c.stroke();
      c.fillStyle = lin(c, 50, 104, '#ffd166', '#e09f00'); rr(c, 34, 54, 60, 48, 10); c.fill(); outline(c, 4, '#7a5200');
      c.fillStyle = '#7a5200'; c.beginPath(); c.arc(64, 74, 7, 0, Math.PI * 2); c.fill(); c.fillRect(61, 76, 6, 14);
    },
    clock: function (c) {
      c.fillStyle = '#fff'; c.beginPath(); c.arc(64, 64, 48, 0, Math.PI * 2); c.fill(); outline(c, 6, '#3a86ff');
      c.strokeStyle = '#1d3557'; c.lineWidth = 6; c.lineCap = 'round'; c.beginPath(); c.moveTo(64, 64); c.lineTo(64, 36); c.moveTo(64, 64); c.lineTo(84, 74); c.stroke();
    },
    check: function (c) { c.fillStyle = '#2dc653'; c.beginPath(); c.arc(64, 64, 48, 0, Math.PI * 2); c.fill(); c.strokeStyle = '#fff'; c.lineWidth = 12; c.lineCap = 'round'; c.lineJoin = 'round'; c.beginPath(); c.moveTo(40, 66); c.lineTo(58, 84); c.lineTo(90, 46); c.stroke(); },
    close: function (c) { c.fillStyle = 'rgba(0,0,0,0.25)'; c.beginPath(); c.arc(64, 64, 46, 0, Math.PI * 2); c.fill(); c.strokeStyle = '#fff'; c.lineWidth = 11; c.lineCap = 'round'; c.beginPath(); c.moveTo(44, 44); c.lineTo(84, 84); c.moveTo(84, 44); c.lineTo(44, 84); c.stroke(); },
    plus: function (c) { c.fillStyle = lin(c, 16, 112, '#7be495', '#2a9d50'); c.beginPath(); c.arc(64, 64, 46, 0, Math.PI * 2); c.fill(); outline(c, 4, '#145a2a'); c.strokeStyle = '#fff'; c.lineWidth = 12; c.lineCap = 'round'; c.beginPath(); c.moveTo(64, 40); c.lineTo(64, 88); c.moveTo(40, 64); c.lineTo(88, 64); c.stroke(); },
    up: function (c) { c.fillStyle = lin(c, 16, 112, '#7be495', '#2a9d50'); c.beginPath(); c.arc(64, 64, 46, 0, Math.PI * 2); c.fill(); outline(c, 4, '#145a2a'); c.fillStyle = '#fff'; c.beginPath(); c.moveTo(64, 30); c.lineTo(94, 64); c.lineTo(74, 64); c.lineTo(74, 94); c.lineTo(54, 94); c.lineTo(54, 64); c.lineTo(34, 64); c.closePath(); c.fill(); },
    island: function (c) {
      c.fillStyle = lin(c, 60, 120, '#7fdbff', '#2f8fd0'); c.beginPath(); c.ellipse(64, 92, 58, 22, 0, 0, Math.PI * 2); c.fill();
      c.fillStyle = lin(c, 50, 96, '#a8e06a', '#5fa83a'); c.beginPath(); c.ellipse(64, 84, 44, 16, 0, 0, Math.PI * 2); c.fill(); outline(c, 3, '#2e6b1a');
      c.fillStyle = '#8b5a2b'; c.fillRect(60, 46, 7, 36);
      c.fillStyle = '#3fae49'; c.beginPath(); c.ellipse(64, 44, 26, 14, 0, 0, Math.PI * 2); c.fill(); outline(c, 3, '#1f6b2a');
      c.fillStyle = '#e05a47'; c.beginPath(); c.moveTo(80, 82); c.lineTo(80, 64); c.lineTo(92, 56); c.lineTo(104, 64); c.lineTo(104, 82); c.closePath(); c.fill();
    }
  };
  Object.keys(DRAW).forEach(function (k) { cell(k, function (c) { if (!aiDraw(c, k)) DRAW[k](c); }); });
  // 3D 气泡：白底圆 + 资源图标；满仓版加红圈和感叹号
  ['wheat', 'wood', 'stone', 'bread', 'fish', 'honey'].forEach(function (k) {
    [false, true].forEach(function (full) {
      cell(k + (full ? '!' : '') + '@b', function (c) {
        c.fillStyle = 'rgba(0,0,0,0.2)'; c.beginPath(); c.arc(64, 68, 54, 0, Math.PI * 2); c.fill();
        c.fillStyle = '#fff'; c.beginPath(); c.arc(64, 62, 54, 0, Math.PI * 2); c.fill();
        c.lineWidth = 7; c.strokeStyle = full ? '#ff4d4d' : '#ffd166'; c.stroke();
        c.save(); c.translate(64, 62); c.scale(0.72, 0.72); c.translate(-64, -64); if (!aiDraw(c, k)) DRAW[k](c); c.restore();
        if (full) { c.fillStyle = '#ff4d4d'; c.beginPath(); c.arc(102, 26, 18, 0, Math.PI * 2); c.fill(); c.fillStyle = '#fff'; c.font = 'bold 26px sans-serif'; c.textAlign = 'center'; c.textBaseline = 'middle'; c.fillText('!', 102, 27); }
      });
    });
  });
  var base = new THREE.CanvasTexture(cv);
  base.minFilter = THREE.LinearFilter; base.generateMipmaps = false;
  if (THREE.SRGBColorSpace) base.colorSpace = THREE.SRGBColorSpace;
  var texCache = {};
  return {
    canvas: cv,
    has: function (name) { return !!slots[name]; },
    // 界面里画：drawImage 从图集里取一格
    draw: function (ctx, name, x, y, size) { var s = slots[name]; if (!s) return; ctx.drawImage(cv, s.x, s.y, S, S, x, y, size, size); },
    // 3D 用：名字后面加 @b 是气泡版（hammer 没有气泡版，直接用）
    texture: function (name) {
      var key = slots[name + '@b'] ? name + '@b' : name;
      if (texCache[key]) return texCache[key];
      var s = slots[key], t = base.clone();
      t.repeat.set(1 / COLS, 1 / COLS); t.offset.set(s.x / cv.width, 1 - (s.y + S) / cv.height);
      t.needsUpdate = true;
      return (texCache[key] = t);
    }
  };
}

module.exports = { make: make };
