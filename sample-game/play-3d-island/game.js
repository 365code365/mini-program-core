// 萌宠小岛（3D 经营）：在小岛上造麦田、伐木场、面包坊、渔港……收获、升级、卖货、扩地、孵宠物，带内购、广告、签到、任务、成就、离线收益。
// three.js r160（libs/three.js，`bash tools/fetch-play-libs.sh` 拉）经 js/adapter.js 跑；所有模型、贴图、图标、声音都是代码生成的。
//
// 启动参数（wx.getLaunchOptionsSync().query）：
//   auto=1    自动驾驶（无头长稳 / 演示：自己玩，界面挨个打开）
//   speed=N   时间加速 N 倍（只给测试用：产量、施工、广告双倍都按加速后的时间算）
//   fresh=1   不读存档，从头开始
var B = require('js/base.js');
var canvas = require('js/adapter.js');
var THREE = require('libs/three.js');
var D = require('js/data/defs.js');
var Save = require('js/core/save.js');
var Bus = require('js/core/bus.js');
var Eco = require('js/core/economy.js');
var Meta = require('js/systems/meta.js');
var Gacha = require('js/systems/gacha.js');
var Money = require('js/systems/monetize.js');
var Models = require('js/world/models.js');
var Terrain = require('js/world/terrain.js');
var City = require('js/world/city.js');
var Camera = require('js/world/camera.js');
var Kit = require('js/ui/kit.js');
var Icons = require('js/ui/icons.js');
var Thumbs = require('js/ui/thumbs.js');
var Hud = require('js/ui/hud.js');
var Build = require('js/ui/build.js');
var Shop = require('js/ui/shop.js');
var Pets = require('js/ui/pets.js');
var Quests = require('js/ui/quests.js');
var Dialogs = require('js/ui/dialogs.js');
var Audio = require('js/audio.js');
var Actions = require('js/actions.js');
var Input = require('js/input.js');
var Auto = require('js/autopilot.js');
var U = require('js/ui/draw.js');
var Art = require('js/art.js');

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1', SPEED = Math.max(1, Number(q.speed) || 1);
// 测试用时间加速：Date.now 按 SPEED 倍走（只在给了 speed 时替换；存档里记的都是加速后的时刻）
if (SPEED > 1) { var realNow = Date.now.bind(Date), t0 = realNow(); Date.now = function () { return t0 + (realNow() - t0) * SPEED; }; }

var W = B.W, H = B.H, sys = B.sys, PR = sys.pixelRatio || 2;
var G = { W: W, H: H, auto: AUTO, safeTop: Math.max(8, (sys.safeArea && sys.safeArea.top) || sys.statusBarHeight || 20), safeBottom: Math.max(4, H - ((sys.safeArea && sys.safeArea.bottom) || H)) };
G.saveVersion = Save.VERSION;

// ───────── 存档、系统 ─────────
if (q.fresh === '1') Save.wipe();
var loaded = Save.load(Date.now());
G.state = loaded.state;
G.bus = Bus.make();
G.eco = Eco.make(G.state, G.bus);
G.meta = Meta.make(G.state, G.eco, G.bus);
G.gacha = Gacha.make(G.state, G.eco, G.bus);
var lastSave = 0;
G.save = function () { Save.save(G.state, Date.now()); lastSave = Date.now(); };
G.shop = Money.make(G.state, G.eco, G.bus, G.save, { auto: AUTO });

// ───────── 渲染 ─────────
var renderer = new THREE.WebGLRenderer({ canvas: canvas, antialias: true });
renderer.setSize(W, H, false);
var scene = new THREE.Scene();
scene.fog = new THREE.Fog(0x8fd3ff, 26, 60);
scene.background = new THREE.Color(0x8fd3ff);
var hemi = new THREE.HemisphereLight(0xffffff, 0x6a8f5a, 0.85); scene.add(hemi);
var sun = new THREE.DirectionalLight(0xfff2d6, 1.1); scene.add(sun); scene.add(sun.target);
sun.shadow.mapSize.set(1024, 1024);
var sc = sun.shadow.camera; sc.left = -9; sc.right = 9; sc.top = 9; sc.bottom = -9; sc.near = 1; sc.far = 40;
sun.shadow.bias = -0.0015;
function quality() {
  var qs = G.state.settings.quality;
  if (qs === 'auto') { var lv = sys.benchmarkLevel || 20; qs = lv >= 30 ? 'high' : lv >= 15 ? 'mid' : 'low'; }
  return qs;
}
G.applySettings = function () {
  var qs = quality();
  renderer.setPixelRatio(qs === 'low' ? 1 : qs === 'mid' ? Math.min(2, PR) : Math.min(3, PR));
  renderer.setSize(W, H, false);
  renderer.shadowMap.enabled = qs === 'high';
  sun.castShadow = qs === 'high';
  scene.traverse(function (o) { if (o.material && o.material.needsUpdate !== undefined) o.material.needsUpdate = true; });
  G.audio.set(G.state.settings.sfx, G.state.settings.music);
};

// ───────── 加载页：先只拉封面（AI 画的标题图），整屏贴上 + 进度条；材质 / 图标 / 立绘都到了再建世界 ─────────
var loadScene = new THREE.Scene(), loadCam = new THREE.OrthographicCamera(0, 1, 1, 0, -1, 1), loadK = 0, started = false;
var barBg = new THREE.Mesh(new THREE.PlaneGeometry(1, 1), new THREE.MeshBasicMaterial({ color: 0x0b2a4a, transparent: true, opacity: 0.55, depthTest: false }));
var bar = new THREE.Mesh(new THREE.PlaneGeometry(1, 1), new THREE.MeshBasicMaterial({ color: 0xffd166, depthTest: false }));
barBg.position.set(0.5, 0.11, 0); barBg.scale.set(0.72, 0.016, 1); bar.position.z = 0.1; loadScene.add(barBg); loadScene.add(bar);
Art.loadImage('art/cover.jpg').then(function (img) {
  if (!img || started) return;
  var t = new THREE.Texture(img); t.needsUpdate = true; if (THREE.SRGBColorSpace) t.colorSpace = THREE.SRGBColorSpace;
  // 封面 9:16，按屏幕比例裁（cover 适配）
  var ia = img.width / img.height, sa = W / H, q = new THREE.Mesh(new THREE.PlaneGeometry(1, 1), new THREE.MeshBasicMaterial({ map: t, depthTest: false }));
  if (ia > sa) { t.repeat.set(sa / ia, 1); t.offset.set((1 - sa / ia) / 2, 0); } else { t.repeat.set(1, ia / sa); t.offset.set(0, (1 - ia / sa) / 2); }
  q.position.set(0.5, 0.5, -0.5); q.renderOrder = -1; loadScene.add(q);
});
function loadingFrame() {
  if (started) return;
  bar.scale.set(0.72 * Math.max(0.02, loadK), 0.016, 1); bar.position.set(0.14 + 0.36 * Math.max(0.02, loadK), 0.11, 0.1);
  renderer.render(loadScene, loadCam);
  requestAnimationFrame(loadingFrame);
}
requestAnimationFrame(loadingFrame);
var loadT0 = Date.now();
Art.load(function (k) { loadK = k * 0.85; }).then(function (art) {
  loadK = 1;
  // 封面至少停 0.6 秒（一闪而过像出了错）
  setTimeout(function () { start(art); }, Math.max(0, 600 - (Date.now() - loadT0)));
});

function start(art) {
started = true;
G.art = art;
var M = Models.make(THREE, art); G.M = M;
G.terrain = Terrain.make(THREE, M, G.state); scene.add(G.terrain.group);
G.icons = Icons.make(THREE, art);
G.thumbs = Thumbs.make(THREE, renderer, M, art);
G.city = City.make(THREE, M, G.terrain, G.state, G.icons, function (x, y, z) { return G.camera.project(x, y, z); }); scene.add(G.city.group);
G.camera = Camera.make(THREE, W, H, D.MAP);
if (q.demo === '1') require('js/demo.js').fill(G.state, G.terrain);
if (q.cam) { var cp = q.cam.split(',').map(Number); G.camera.place(cp[0], cp[1], cp[2], cp[3]); }
// look=<建筑类型>[,距离]：镜头对准第一座这种建筑（截图近看模型）
if (q.look) { var lk = q.look.split(','), lb = G.state.buildings.filter(function (b) { return b.type === lk[0]; })[0]; if (lb) { var lt = G.terrain.tile(lb.x, lb.z); G.camera.place(lt.cx, lt.cz, Number(lk[1]) || 3.2); } }
var clouds = [], crnd = Terrain.seeded(3);
for (var ci = 0; ci < 7; ci++) { var cl = M.cloud(crnd); cl.position.set((crnd() - 0.5) * 30, 4 + crnd() * 2.5, (crnd() - 0.5) * 30); cl.scale.setScalar(0.8 + crnd() * 0.8); scene.add(cl); clouds.push(cl); }
G.terrain.rebuild(G.city.occupied());
G.city.sync();

// ───────── 界面 ─────────
G.ui = Kit.make(THREE, renderer, W, H, PR);
G.ui.icons = G.icons;
G.audio = Audio.make();
G.sfx = function (n) { G.audio.sfx(n); };
G.vibrate = function (long) { if (!G.state.settings.vibrate) return; try { long ? wx.vibrateLong() : wx.vibrateShort({ type: 'light' }); } catch (e) {} };
G.hud = Hud.make(G);
G.build = Build.make(G);
G.shopUI = Shop.make(G);
G.pets = Pets.make(G);
G.quests = Quests.make(G);
G.dialogs = Dialogs.make(G);
G.act = Actions.make(G);
G.placing = null;
G.open = function (what, sub) {
  if (G.placing) G.act.cancelPlace();
  G.ui.hideAll(); G.select(null); G.sfx('pop');
  if (what === 'build') G.build.open();
  else if (what === 'shop') G.shopUI.open(sub);
  else if (what === 'pets') G.pets.open(sub);
  else if (what === 'quests') G.quests.open(sub);
  else if (what === 'settings') G.dialogs.settings();
  else if (what === 'signin') G.dialogs.signin();
  else if (what === 'boost') G.dialogs.boost();
  else if (what === 'sell') G.dialogs.sell();
};
G.select = function (b) { G.city.select(b); G.build.select(b); if (b) { var p = G.city.pos(b); G.camera.focus(p.x, p.z + 0.8); } };
// 发奖统一走这里：提示 + 图标从屏幕中间飞到资源条
G.reward = function (gain, prefix) {
  var keys = Object.keys(gain || {}).filter(function (k) { return gain[k] > 0; });
  if (!keys.length) return;
  G.ui.toast((prefix || '+') + keys.map(function (k) { return Eco.fmt(gain[k]) + ' ' + D.RES[k].name; }).join('  '), keys[0]);
  keys.forEach(function (k) { var to = G.hud.slot(k); G.ui.fly(k, W / 2, H * 0.45, to.x, to.y, 6); });
  G.sfx('coin');
};
// 兑换码：系统键盘（wx.showKeyboard），确认时回调
G.askCode = function (cb) {
  if (AUTO) return cb('MINI2026');
  var done = false;
  function onConfirm(r) { if (done) return; done = true; off(); cb(r.value); }
  function off() { try { wx.offKeyboardConfirm(onConfirm); wx.hideKeyboard(); } catch (e) {} }
  wx.onKeyboardConfirm(onConfirm);
  wx.showKeyboard({ defaultValue: '', maxLength: 16, multiple: false, confirmHold: false, confirmType: 'done',
    fail: function () { done = true; off(); wx.showModal({ title: '兑换码', content: '当前环境不能弹键盘。试试兑换码 MINI2026', showCancel: false }); } });
};
G.reset = function () { Save.wipe(); wx.restartMiniProgram ? wx.restartMiniProgram({}) : G.ui.toast('重启小游戏后生效'); };
G.applySettings();

// ───────── 事件 → 表现 ─────────
G.bus.on('levelup', function (lv) { G.dialogs.levelup(lv); G.build.refreshMenu(); G.vibrate(true); });
G.bus.on('built', function (b) { G.city.sync(); G.sfx('upgrade'); G.ui.toast(D.BUILDINGS[b.type].name + ' 建好了！', 'hammer'); G.save(); });
G.bus.on('upgraded', function (b) { G.city.sync(); G.sfx('upgrade'); G.ui.toast(D.BUILDINGS[b.type].name + ' 升到 ' + b.lv + ' 级', 'up'); G.save(); });
G.bus.on('pets', function () { G.city.sync(); });

// 补单：上次付了钱没发货的（被杀在半路）
G.shop.recover().forEach(function (f) { G.reward(f.gain, '补发上次的购买 +'); });
// 离线收益（第一次进游戏没有）
function checkOffline() { var a = G.eco.offline(Date.now()); if (a && Object.keys(a.gain).some(function (k) { return a.gain[k] > 0; })) G.dialogs.offline(a); }
if (loaded.status === 'ok') checkOffline();
if (loaded.status === 'corrupt') G.ui.toast('存档损坏，已重新开始');
B.onHide(function () { G.save(); });
B.onShow(function () { checkOffline(); G.meta.rollDay(); });
// 新手：一开始送一块已建好的麦田，引导第一次建造与收获
if (!G.state.buildings.length && loaded.status !== 'ok') {
  G.state.buildings.push({ uid: G.state.nextUid++, type: 'farm', x: (D.MAP >> 1) - 1, z: (D.MAP >> 1), rot: 0, lv: 1, store: 20, busyKind: null, busyUntil: 0 });
  G.terrain.rebuildDecor(G.city.occupied()); G.city.sync(); G.save();
}

// 新手提示条：0 = 去建造，1 = 去收获，2 = 完成
var tipV = G.ui.view('tip', { x: 20, y: H - 150 - G.safeBottom, w: W - 40, h: 52, z: 2, anim: 'up', draw: function (c, v) {
  var s = G.state.tutorial;
  c.fillStyle = 'rgba(20,30,60,0.78)'; U.rr(c, 0, 0, v.w, v.h, 20); c.fill();
  G.icons.draw(c, s === 0 ? 'hammer' : 'wheat', 8, 6, 40);
  U.text(c, s === 0 ? '点下面的「建造」，再造一座伐木场' : '点建筑头顶的图标收获，图标会飞进资源条', 54, 27, 14, '#fff', 'left', true);
} });

var auto = AUTO ? Auto.make(G) : null;
// 截图 / 调试：open=shop&sub=deals 一进来就打开某个界面；focus=1 镜头对准岛中心
if (q.open) setTimeout(function () { G.open(q.open, q.sub); }, 50);
Input.make(G);

// ───────── 主循环 ─────────
// prof=1：一帧超过 30ms 时把各段耗时打出来（[prof] 总 / 段名=毫秒），真机上找长帧是谁
var PROF = q.prof === '1', pt = {}, pt0 = 0, pf = 0;
function lap(name) { if (!PROF) return; var t = Date.now(); pt[name] = (pt[name] || 0) + t - pt0; pt0 = t; }
var clock = 0, saveEvery = 10000;
function update(dt) {
  if (PROF) { pt = {}; pf = pt0 = Date.now(); }
  clock += dt;
  var now = Date.now();
  G.eco.tick(now); lap('eco');
  G.camera.step(dt);
  G.act.followCenter();
  var ds = G.terrain.step(clock);
  scene.background.copy(ds.sky); scene.fog.color.copy(ds.sky);
  hemi.intensity = 0.45 + ds.amb * 0.8; sun.intensity = ds.sun * 1.55;
  // 太阳跟着镜头方位走：总在镜头的左后上方（主光打在看得见的那两面墙上）。v3 太阳固定在 +z 一侧，
  // 镜头默认从 −x −z 往里看，看到的全是背光面：门窗、木纹都是一片灰，楼像没打光的泥模型
  var a = ds.t * Math.PI * 2 - Math.PI / 2, sa = G.camera.yaw + Math.PI + 0.7 + Math.cos(a) * 0.35;
  sun.position.set(Math.sin(sa) * 10, Math.max(3, Math.sin(a) * 10 + 7), Math.cos(sa) * 10); sun.target.position.set(0, 0, 0);
  clouds.forEach(function (cl, i) { cl.position.x += dt * (0.15 + i * 0.03); if (cl.position.x > 18) cl.position.x = -18; });
  lap('scene');
  G.city.step(dt, now, ds.night, G.eco); lap('city');
  G.ui.step(dt); G.hud.step(); G.build.step(); G.shopUI.step(); G.quests.step(); G.dialogs.step(); G.pets.step(dt);
  G.audio.step(); lap('ui');
  if (G.state.tutorial < 2 && !G.placing && !G.ui.anyModal() && !G.city.selected()) { if (!tipV.shown) { tipV.markDirty(); tipV.show(); } if (tipV.data.s !== G.state.tutorial) { tipV.data.s = G.state.tutorial; tipV.markDirty(); } }
  else if (tipV.shown) tipV.hide();
  if (auto) auto.step(dt); lap('auto');
  if (now - lastSave > saveEvery) G.save(); lap('save');
}
var sceneInfo = { calls: 0, tris: 0 };
function render() {
  renderer.render(scene, G.camera.cam); lap('gl');
  sceneInfo.calls = renderer.info.render.calls; sceneInfo.tris = renderer.info.render.triangles;
  G.ui.render(); lap('uiDraw');
  if (PROF && Date.now() - pf > 30) console.log('[prof] ' + (Date.now() - pf) + 'ms ' + Object.keys(pt).filter(function (k) { return pt[k] >= 2; }).map(function (k) { return k + '=' + pt[k]; }).join(' ') + ' prog=' + renderer.info.programs.length + ' views ' + JSON.stringify(G.ui.prof));
  if (PROF) G.ui.prof = {};
}

GameGlobal.__play = function () {
  var s = G.state;
  return { lv: s.lv, xp: s.xp, coin: Math.floor(s.res.coin), gem: Math.floor(s.res.gem), goods: D.GOODS.map(function (k) { return k + ':' + Math.floor(s.res[k]); }).join(' '),
    buildings: s.buildings.map(function (b) { return b.type + (b.busyKind ? '*' : '') + b.lv; }).join(' '), expanded: s.expanded.length,
    pets: Object.keys(s.pets).filter(function (k) { return s.pets[k].n; }).length, pulls: s.pulls, pity: s.pity, orders: s.orders.length,
    daily: G.meta.dailyList().filter(function (x) { return x.claimed; }).length, signin: s.signin.streak, walkers: G.city.walkers(), petsOut: G.city.petsOut(), bubbles: G.city.bubblesShown(),
    calls: sceneInfo.calls, tris: sceneInfo.tris, uiCalls: renderer.info.render.calls, cam: G.camera.state(), quality: quality() };
};
GameGlobal.__G = G;

B.loop(update, render);
}
