// 跑酷的模型工厂：角色、障碍（矮栏 / 高栏 / 地铁）、金币、楼房、路灯。都在开局前建好，游戏中途只从对象池里拿。
// 贴图来自 js/art.js（AI 生成）；哪张没有就用纯色（tex() 返回 null，材质只吃 color）。
//
// draw call 是这里的预算：几何按「一个物体一种材质」来拼。
// - 楼房：盒子五个面都贴同一张立面（顶面、底面从相机高度 4.3m 看不到 —— 楼最矮 7m），一栋一次；
// - 金币：圆柱侧面的 UV 全挤到贴图边缘的一个金色点上，侧面与正面同一张图，一枚一次（三组材质就是三次）；
// - 地铁：车身三组（侧面 / 车头 / 顶），其余零件纯色。
function make(THREE, art) {
  var img = (art && art.img) || {};
  var texs = {};
  function tex(name, rx, ry) {
    if (!img[name]) return null;
    var key = name + '|' + (rx || 1) + '|' + (ry || 1);
    if (texs[key]) return texs[key];
    var t = new THREE.Texture(img[name]);
    t.needsUpdate = true;
    if (THREE.SRGBColorSpace) t.colorSpace = THREE.SRGBColorSpace;
    if (rx || ry) { t.wrapS = t.wrapT = THREE.RepeatWrapping; t.repeat.set(rx || 1, ry || 1); }
    t.anisotropy = 4;
    return (texs[key] = t);
  }
  var mats = {};
  // mat(颜色, 贴图名, 选项)：有贴图时颜色当染色（贴图本身带色就给白）
  function mat(color, texName, o) {
    o = o || {};
    var key = color + '|' + (texName || '') + '|' + JSON.stringify(o);
    if (mats[key]) return mats[key];
    var t = texName ? tex(texName, o.rx, o.ry) : null;
    var p = { color: t ? (o.tint || 0xffffff) : color };
    if (t) p.map = t;
    if (o.emissive) p.emissive = o.emissive;
    var m = o.shiny ? new THREE.MeshPhongMaterial(Object.assign(p, { shininess: o.shiny, specular: o.spec || 0x444444 })) : new THREE.MeshLambertMaterial(p);
    return (mats[key] = m);
  }
  var boxG = new THREE.BoxGeometry(1, 1, 1);

  // 角色在 js/hero.js（层级按动画需要分）
  function hero() { return require('./hero.js').make(THREE, mat); }

  // ───────── 障碍 ─────────
  var metal = mat(0xb8bec8, 'metal', { shiny: 40, spec: 0x666666 }), dark = mat(0x30343c);
  function part(g, mt, sx, sy, sz, x, y, z, par) { var o = new THREE.Mesh(g, mt); o.scale.set(sx, sy, sz); o.position.set(x, y, z); par.add(o); return o; }
  function at(g, mt, x, y, z, par) { return part(g, mt, 1, 1, 1, x, y, z, par); }
  function low() {
    var grp = new THREE.Group();
    part(boxG, mat(0xe63946, 'hazard', { rx: 2.4, ry: 0.5 }), 1.9, 0.38, 0.12, 0, 0.78, 0, grp);
    part(boxG, mat(0xe63946, 'hazard', { rx: 2.4, ry: 0.5 }), 1.9, 0.18, 0.1, 0, 0.36, 0, grp);
    [-0.82, 0.82].forEach(function (x) {
      part(boxG, metal, 0.09, 1.0, 0.09, x, 0.5, 0, grp);
      part(boxG, dark, 0.14, 0.06, 0.7, x, 0.03, 0, grp);                                    // 脚
      at(new THREE.SphereGeometry(0.07, 8, 6), mat(0xffb703, null, { emissive: 0xcc6a00 }), x, 1.05, 0, grp);   // 警示灯
    });
    return grp;
  }
  function high() {
    var grp = new THREE.Group();
    part(boxG, mat(0xf4a300, 'hazard', { rx: 2.6, ry: 0.7 }), 2.05, 0.55, 0.16, 0, 1.92, 0, grp);
    part(boxG, mat(0x14213d), 1.2, 0.3, 0.17, 0, 1.92, 0, grp);                                // 中间的牌子
    part(boxG, mat(0xffffff, null, { emissive: 0x666666 }), 0.9, 0.08, 0.18, 0, 1.92, 0, grp);
    [-0.95, 0.95].forEach(function (x) {
      part(boxG, metal, 0.12, 2.25, 0.12, x, 1.12, 0, grp);
      part(boxG, dark, 0.3, 0.08, 0.5, x, 0.04, 0, grp);
    });
    return grp;
  }
  // 地铁：车身盒子的六个面按 [侧, 侧, 顶, 底, 车头, 车尾] 分组；BoxGeometry 自带的六组正好是这个顺序，把材质数组排成三种就行
  function train() {
    var grp = new THREE.Group();
    var side = mat(0x3d5a80, 'train_side'), front = mat(0x3d5a80, 'train_front'), roof = mat(0xd8dde4, 'metal', { shiny: 30 });
    var body = new THREE.Mesh(boxG, [side, side, roof, dark, front, front]);
    body.scale.set(2.0, 2.5, 12); body.position.y = 1.62; grp.add(body);
    part(boxG, dark, 1.7, 0.42, 11.4, 0, 0.21, 0, grp);                                       // 底盘
    [-4.2, 4.2].forEach(function (z) { part(boxG, mat(0x1b1d22), 1.8, 0.36, 1.6, 0, 0.3, z, grp); });   // 转向架
    part(boxG, roof, 1.5, 0.16, 10, 0, 2.94, 0, grp);                                         // 车顶设备舱
    [-0.6, 0.6].forEach(function (x) {
      at(new THREE.CircleGeometry(0.13, 12), mat(0xffffff, null, { emissive: 0xfff2c0 }), x, 0.85, 6.012, grp);   // 车灯
    });
    var shadow = new THREE.Mesh(new THREE.PlaneGeometry(2.6, 13), new THREE.MeshBasicMaterial({ color: 0, transparent: true, opacity: 0.32, depthWrite: false }));
    shadow.rotation.x = -Math.PI / 2; shadow.position.y = 0.03; grp.add(shadow);
    return grp;
  }

  // ───────── 金币：正面贴 AI 画的金币，侧面 UV 压到贴图上沿一个金色点 ─────────
  var coinG = new THREE.CylinderGeometry(0.42, 0.42, 0.09, 24);
  var nSide = (24 + 1) * 2, uv = coinG.attributes.uv;
  for (var i = 0; i < nSide; i++) uv.setXY(i, 0.5, 0.96);
  coinG.clearGroups();
  var coinM = img.coin ? new THREE.MeshPhongMaterial({ map: tex('coin'), emissive: 0x3a2400, shininess: 60, specular: 0x886622 })
    : new THREE.MeshLambertMaterial({ color: 0xffd23f, emissive: 0x553300 });
  function coin() { var c = new THREE.Mesh(coinG, coinM); c.rotation.x = Math.PI / 2; return c; }

  // ───────── 楼房：四种立面，比例约束在立面图附近（宽:高 ≈ 1:2），不然窗户被拉扁 ─────────
  var FACADES = ['facade_a', 'facade_b', 'facade_c', 'facade_d'], FALL = [0xf4a261, 0xc8553d, 0x5b8fb8, 0x8fd1b8];
  var facadeMats = FACADES.map(function (n, k) { return mat(FALL[k], n); });
  function building() { return new THREE.Mesh(boxG, facadeMats[0]); }
  function shapeBuilding(m) {
    var d = 4.5 + Math.random() * 3, h = d * (1.6 + Math.random() * 0.9), w = d * (0.85 + Math.random() * 0.3);
    m.scale.set(w, h, d);
    m.material = facadeMats[Math.floor(Math.random() * facadeMats.length)];
    return m;
  }
  // 路灯：站台外沿一排，给速度感一个近处的参照
  var poleM = mat(0x5c6370, null, { shiny: 20 }), lampM = mat(0xfff4d6, null, { emissive: 0xffe9a8 });
  var poleG = new THREE.CylinderGeometry(0.06, 0.08, 4.2, 8), armG = new THREE.BoxGeometry(0.9, 0.08, 0.1), headG = new THREE.BoxGeometry(0.36, 0.1, 0.22);
  function lamp(side) {
    var g = new THREE.Group();
    var p = new THREE.Mesh(poleG, poleM); p.position.y = 2.1; g.add(p);
    var a = new THREE.Mesh(armG, poleM); a.position.set(-side * 0.4, 4.15, 0); g.add(a);
    var h = new THREE.Mesh(headG, lampM); h.position.set(-side * 0.8, 4.08, 0); g.add(h);
    return g;
  }
  return { tex: tex, mat: mat, hero: hero, low: low, high: high, train: train, coin: coin, building: building, shapeBuilding: shapeBuilding, lamp: lamp, has: function (n) { return !!img[n]; } };
}
module.exports = { make: make };
