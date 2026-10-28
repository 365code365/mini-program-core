// 3D 缩略图：启动时把每种建筑（1 级 / 5 级外观）和每只宠物各渲染一次到 160×160 的离屏渲染目标，readPixels 读回来，
// putImageData 进一张缩略图集画布；界面里 drawImage 从这张画布取（建造菜单、宠物图鉴、抽卡结果用的都是真模型的样子）。
// 失败（readPixels 不支持、渲染目标建不出来）就返回 null，界面退回用图标 —— 不影响游戏。
var D = require('../data/defs.js');
var S = 160, COLS = 6;

// art 里有 AI 立绘（portraits.webp）就直接用它（宠物 / 建筑都有），不再离屏渲染 3D 缩略图
function make(THREE, renderer, M, art) {
  if (art && art.portraits && art.manifest && art.manifest.portraits) {
    var P = art.manifest.portraits, PS = art.manifest.portraitSize || 256;
    var key = function (name) { var p = name.split(':'); return p[0] === 'b' ? 'bld_' + p[1] : 'pet_' + p[1]; };
    return {
      ai: true, image: art.portraits, slot: function (id) { return P[id]; }, size: PS,
      has: function (name) { return !!P[key(name)]; },
      draw: function (c, name, x, y, size) { var s = P[key(name)]; if (!s) return false; c.drawImage(art.portraits, s[0], s[1], PS, PS, x, y, size, size); return true; },
      building: function (type, lv) { return 'b:' + type + ':' + (lv >= 5 ? 5 : 1); },
      pet: function (id) { return 'p:' + id; }
    };
  }
  try {
    var names = [];
    D.BUILD_ORDER.forEach(function (t) { names.push('b:' + t + ':1', 'b:' + t + ':5'); });
    D.PETS.forEach(function (p) { names.push('p:' + p.id); });
    var rows = Math.ceil(names.length / COLS);
    var atlas = wx.createCanvas(); atlas.width = S * COLS; atlas.height = S * rows;
    var ac = atlas.getContext('2d'), slots = {};
    var rt = new THREE.WebGLRenderTarget(S, S, { depthBuffer: true });
    if (THREE.SRGBColorSpace) rt.texture.colorSpace = THREE.SRGBColorSpace;
    var scene = new THREE.Scene();
    scene.add(new THREE.HemisphereLight(0xffffff, 0x8899aa, 1.1));
    var sun = new THREE.DirectionalLight(0xffffff, 1.4); sun.position.set(3, 5, 4); scene.add(sun);
    var cam = new THREE.PerspectiveCamera(30, 1, 0.1, 50);
    var px = new Uint8Array(S * S * 4), img = ac.createImageData(S, S);
    var prevTarget = renderer.getRenderTarget(), clear = new THREE.Color(), prevAlpha = renderer.getClearAlpha();
    renderer.getClearColor(clear);
    renderer.setClearColor(0x000000, 0);
    names.forEach(function (name, i) {
      var parts = name.split(':'), obj, d, h;
      if (parts[0] === 'b') { obj = M.building(parts[1], +parts[2]).group; d = 2.6; h = 0.45; }
      else { obj = M.pet(D.PETS.filter(function (p) { return p.id === parts[1]; })[0]); obj.rotation.y = -0.5; d = 1.25; h = 0.22; }
      scene.add(obj);
      cam.position.set(d * 0.75, h + d * 0.62, d * 0.9); cam.lookAt(0, h, 0);
      renderer.setRenderTarget(rt);
      renderer.clear();
      renderer.render(scene, cam);
      renderer.readRenderTargetPixels(rt, 0, 0, S, S, px);
      for (var y = 0; y < S; y++) img.data.set(px.subarray((S - 1 - y) * S * 4, (S - y) * S * 4), y * S * 4);   // GL 的行是倒的
      var sx = (i % COLS) * S, sy = Math.floor(i / COLS) * S;
      ac.putImageData(img, sx, sy);
      slots[name] = { x: sx, y: sy };
      scene.remove(obj);
    });
    renderer.setRenderTarget(prevTarget);
    renderer.setClearColor(clear, prevAlpha);
    rt.dispose();
    return {
      has: function (name) { return !!slots[name]; },
      draw: function (c, name, x, y, size) { var s = slots[name]; if (!s) return false; c.drawImage(atlas, s.x, s.y, S, S, x, y, size, size); return true; },
      building: function (type, lv) { return 'b:' + type + ':' + (lv >= 5 ? 5 : 1); },
      pet: function (id) { return 'p:' + id; }
    };
  } catch (e) {
    console.warn('[thumbs] 3D 缩略图生成失败，界面用图标代替：' + (e && e.message));
    try { renderer.setRenderTarget(null); } catch (e2) {}
    return null;
  }
}

module.exports = { make: make };
