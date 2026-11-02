// 造型：战机 / 三种敌机 / 首领录成精灵（R.sprite：录一次，之后每架一次 drawArrays），
// 子弹与光点烘焙成图集里的小图（R.stamp：每颗 6 个顶点，可染色、可叠加），星空录成三层图层，滚动只改偏移。
var TAU = Math.PI * 2;

function make(R, W, H) {
  // ───────── 烘焙小图（离屏 2D 画布画一次） ─────────
  function disc(c, x, y, r, color) { c.beginPath(); c.arc(x, y, r, 0, TAU); c.fillStyle = color; c.fill(); }
  // 软光点：一圈圈半透明白叠出来的衰减（不依赖 createRadialGradient）
  var glow = R.bake(32, 32, function (c, w, h) {
    for (var i = 8; i >= 1; i--) disc(c, w / 2, h / 2, w / 2 * i / 8, 'rgba(255,255,255,' + (0.06 + (8 - i) * 0.018) + ')');
    disc(c, w / 2, h / 2, w * 0.12, '#ffffff');
  });
  function orb(rim, core) {
    return function (c, w, h) {
      disc(c, w / 2, h / 2, w / 2, rim.replace('A', '0.35'));
      disc(c, w / 2, h / 2, w * 0.36, rim.replace('A', '1'));
      disc(c, w / 2, h / 2, w * 0.2, core);
    };
  }
  var ebTex = [
    R.bake(14, 14, orb('rgba(255,64,129,A)', '#ffffff')),   // 0 粉色小弹：侦察机、战斗机
    R.bake(20, 20, orb('rgba(255,145,0,A)', '#fff3e0')),    // 1 橙色大弹：轰炸机、首领
    R.bake(14, 14, orb('rgba(0,229,255,A)', '#ffffff'))     // 2 青色弹：首领的螺旋
  ];
  var pshot = R.bake(10, 26, function (c, w, h) {
    c.fillStyle = 'rgba(128,216,255,0.35)';
    c.beginPath(); c.arc(w / 2, w / 2, w / 2, Math.PI, 0); c.lineTo(w, h - w / 2); c.arc(w / 2, h - w / 2, w / 2, 0, Math.PI); c.closePath(); c.fill();
    c.fillStyle = '#e1f5fe'; c.fillRect(w / 2 - 1.6, 3, 3.2, h - 6);
  });

  // ───────── 精灵造型（录在原点，机头朝上为玩家、朝下为敌机） ─────────
  function player(p) {
    R.poly([[0, -4], [21, 12], [21, 18], [0, 12]], p.wing);
    R.poly([[0, -4], [0, 12], [-21, 18], [-21, 12]], p.wing);
    R.rect(18, 6, 3, 12, p.tip); R.rect(-21, 6, 3, 12, p.tip);
    R.poly([[0, -24], [7, -9], [8, 14], [0, 20], [-8, 14], [-7, -9]], p.body);
    R.ellipse(0, -6, 3.6, 7.5, 0, p.glass);
    R.rect(-6, 16, 4, 5, p.dark); R.rect(2, 16, 4, 5, p.dark);
  }
  function scout(p) {
    R.poly([[0, 15], [14, -9], [0, -3], [-14, -9]], p.wing);
    R.poly([[0, 12], [5, -6], [0, -11], [-5, -6]], p.body);
    R.circle(0, 1, 2.8, p.glass);
  }
  function fighter(p) {
    R.poly([[0, 8], [22, -6], [22, 2], [0, 16]], p.wing);
    R.poly([[0, 8], [0, 16], [-22, 2], [-22, -6]], p.wing);
    R.ellipse(0, 0, 8, 17, 0, p.body);
    R.rect(-17, -2, 3, 13, p.dark); R.rect(14, -2, 3, 13, p.dark);
    R.circle(0, 5, 4, p.glass);
  }
  function tank(p) {
    R.rrect(-30, -22, 60, 40, 10, p.wing);
    R.rrect(-22, -26, 44, 52, 12, p.body);
    R.circle(-26, 0, 9, p.dark); R.circle(26, 0, 9, p.dark);
    R.circle(0, 2, 11, p.dark); R.rect(-3, 4, 6, 18, p.dark);
    R.circle(0, 2, 5, p.glass);
  }
  function boss(p) {
    R.poly([[-78, -12], [-30, -30], [-30, 18], [-70, 10]], p.wing);
    R.poly([[78, -12], [70, 10], [30, 18], [30, -30]], p.wing);
    R.poly([[-36, -34], [36, -34], [44, 10], [20, 36], [-20, 36], [-44, 10]], p.body);
    R.rect(-66, 6, 6, 18, p.dark); R.rect(60, 6, 6, 18, p.dark);
    R.rect(-28, 24, 8, 20, p.dark); R.rect(20, 24, 8, 20, p.dark);
    R.circle(0, 2, 18, p.dark);
    R.circle(0, 2, 11, p.glass);
  }
  var FLASH = { wing: '#ffffff', body: '#ffffff', glass: '#ffe0e0', dark: '#ffd0d0', tip: '#ffffff' };
  var PAL = {
    player: { wing: '#3d6fe0', body: '#e3ecf7', glass: '#29b6f6', dark: '#455a64', tip: '#ff5252' },
    scout: { wing: '#ff7043', body: '#ffccbc', glass: '#263238' },
    fighter: { wing: '#7e57c2', body: '#d1c4e9', glass: '#ff4081', dark: '#4527a0' },
    tank: { wing: '#546e7a', body: '#90a4ae', glass: '#ffab00', dark: '#37474f' },
    boss: { wing: '#8e1d2c', body: '#c62828', glass: '#ffeb3b', dark: '#4a0d14' }
  };
  var SHAPES = [scout, fighter, tank];
  var NAMES = ['scout', 'fighter', 'tank'];
  // 闭包在启动时建好：画的时候不再现造函数
  var recipes = {};
  function recipe(name, shape, p) { return (recipes[name] = recipes[name] || function () { shape(p); }); }
  NAMES.forEach(function (n, i) { recipe(n, SHAPES[i], PAL[n]); recipe(n + '_f', SHAPES[i], FLASH); });
  recipe('player', player, PAL.player); recipe('boss', boss, PAL.boss); recipe('boss_f', boss, FLASH);

  function enemy(type, x, y, flash) { var k = NAMES[type] + (flash ? '_f' : ''); R.sprite(k, 1, recipes[k], x, y, 1, 1); }
  function ship(x, y, sc, a) { R.sprite('player', 1, recipes.player, x, y, sc, a); }
  function bossShip(x, y, flash) { var k = flash ? 'boss_f' : 'boss'; R.sprite(k, 1, recipes[k], x, y, 1, 1); }

  // ───────── 背景：清屏色打底 + 两团星云（一层），三层星星各自滚动 ─────────
  // 不铺整屏渐变：无头 CPU 后端（swiftshader）上一块全屏混合四边形 ~3.7ms、三团大星云 ~2.3ms（本包实测 pump 中位 11.5 → 5.6ms），
  // 真机 GPU 上不算钱，但低端机走 CPU 时一样吃填充率。星云缩小成两团、透明度压低
  var seed = 20261102;
  function rnd() { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed / 0x7fffffff; }
  function sky() {
    R.setAlpha(0.12);
    R.ellipse(W * 0.22, H * 0.32, 110, 60, 0.5, '#5c6bc0');
    R.ellipse(W * 0.82, H * 0.68, 90, 70, -0.4, '#8e24aa');
    R.setAlpha(1);
  }
  var starSets = [[70, 0.8, 'rgba(255,255,255,0.45)'], [36, 1.3, 'rgba(200,220,255,0.75)'], [14, 2.0, '#ffffff']];
  var starFns = starSets.map(function (s) {
    var pts = []; for (var i = 0; i < s[0]; i++) pts.push([rnd() * W, rnd() * H]);
    return function () { for (var i = 0; i < pts.length; i++) R.rect(pts[i][0], pts[i][1], s[1], s[1], s[2]); };
  });
  var SPEEDS = [14, 34, 70];
  function background(scroll) {
    R.layer('sky', 1, sky);
    for (var i = 0; i < 3; i++) {
      var off = (scroll * SPEEDS[i]) % H;
      R.layer('stars' + i, 1, starFns[i], 0, off);
      R.layer('stars' + i, 1, starFns[i], 0, off - H);
    }
  }

  return { glow: glow, ebTex: ebTex, pshot: pshot, enemy: enemy, ship: ship, boss: bossShip, background: background };
}

module.exports = { make: make };
