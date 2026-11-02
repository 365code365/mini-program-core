// 3D 场景：一关一套几何体，材质全程复用（不 dispose 材质：three.js 删掉最后一个用某着色器的材质时会删 program，
// 下一关第一帧就要重编着色器，换关时一顿；见 play-3d-stack 的材质池注释）。
//
// draw call 预算（与 __play().calls 对得上）：墙 1（全部墙砖合并成一个 BufferGeometry，只生成露在外面的面）、
// 地面 1（路砖合并、顶点色画棋盘格）、宝石 1（InstancedMesh）、出口 2（底座 + 光柱）、玩家 1、影子 1 = 7 次。
// 为什么不用 BufferGeometryUtils.mergeGeometries：它在 examples/jsm 里，要另带一份 addons；
// 墙砖都是轴对齐的单位方块，自己拼面更省（相邻两块墙之间的面根本不生成，31×31 的图少一半多顶点）。
var M = require('maze.js');
var WALL_H = 0.8;

function make(THREE, scene) {
  var mats = {
    wall: new THREE.MeshLambertMaterial({ vertexColors: true }),
    floor: new THREE.MeshLambertMaterial({ vertexColors: true }),
    gem: new THREE.MeshLambertMaterial({ color: 0x40e0ff, emissive: 0x1070a0 }),
    pad: new THREE.MeshLambertMaterial({ color: 0x9e9e9e, emissive: 0x000000 }),
    beam: new THREE.MeshBasicMaterial({ color: 0x7cff9c, transparent: true, opacity: 0.35, depthWrite: false }),
    ball: new THREE.MeshLambertMaterial({ color: 0xff8a3d, emissive: 0x401800 }),
    shadow: new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.28, depthWrite: false })
  };
  var gemGeo = new THREE.OctahedronGeometry(0.2, 0);
  var player = new THREE.Mesh(new THREE.SphereGeometry(0.28, 20, 14), mats.ball);
  var shadow = new THREE.Mesh(new THREE.CircleGeometry(0.3, 20), mats.shadow);
  shadow.rotation.x = -Math.PI / 2;
  var pad = new THREE.Mesh(new THREE.CylinderGeometry(0.42, 0.42, 0.06, 24), mats.pad);
  var beam = new THREE.Mesh(new THREE.CylinderGeometry(0.34, 0.34, 3, 20, 1, true), mats.beam);
  scene.add(player); scene.add(shadow); scene.add(pad); scene.add(beam);
  var walls = null, floor = null, gems = null, dummy = new THREE.Object3D(), col = new THREE.Color();

  // 一块墙砖露出来的面：顶面一定有，四个侧面只在邻格不是墙时生成
  function wallGeometry(m, hue) {
    var pos = [], nor = [], cols = [];
    var top = new THREE.Color().setHSL(hue, 0.35, 0.62), side = new THREE.Color().setHSL(hue, 0.4, 0.42);
    function face(a, b, c, d, n, cc) {
      [a, b, c, a, c, d].forEach(function (p) { pos.push(p[0], p[1], p[2]); nor.push(n[0], n[1], n[2]); cols.push(cc.r, cc.g, cc.b); });
    }
    for (var j = 0; j < m.T; j++) for (var i = 0; i < m.T; i++) {
      if (!M.wall(m, i, j)) continue;
      // 每块砖颜色有一点随机明暗：大片同色墙看不出远近
      var k = 0.9 + ((i * 7 + j * 13) % 5) * 0.04, t = col.copy(top).multiplyScalar(k).clone(), s = col.copy(side).multiplyScalar(k).clone();
      var x0 = i, x1 = i + 1, z0 = j, z1 = j + 1, h = WALL_H;
      face([x0, h, z1], [x1, h, z1], [x1, h, z0], [x0, h, z0], [0, 1, 0], t);
      if (!M.wall(m, i, j + 1) || j === m.T - 1) face([x0, 0, z1], [x1, 0, z1], [x1, h, z1], [x0, h, z1], [0, 0, 1], s);
      if (!M.wall(m, i, j - 1) || j === 0) face([x1, 0, z0], [x0, 0, z0], [x0, h, z0], [x1, h, z0], [0, 0, -1], s);
      if (!M.wall(m, i + 1, j) || i === m.T - 1) face([x1, 0, z1], [x1, 0, z0], [x1, h, z0], [x1, h, z1], [1, 0, 0], s);
      if (!M.wall(m, i - 1, j) || i === 0) face([x0, 0, z0], [x0, 0, z1], [x0, h, z1], [x0, h, z0], [-1, 0, 0], s);
    }
    return geometry(pos, nor, cols);
  }
  function floorGeometry(m, hue) {
    var pos = [], nor = [], cols = [];
    var a = new THREE.Color().setHSL((hue + 0.5) % 1, 0.18, 0.78), b = new THREE.Color().setHSL((hue + 0.5) % 1, 0.18, 0.7);
    // 外面一圈大底板：从迷宫边上往外看不是一片空
    var pad = 6, lo = -pad, hi = m.T + pad, base = new THREE.Color().setHSL(hue, 0.25, 0.3);
    [[lo, -0.02, hi], [hi, -0.02, hi], [hi, -0.02, lo], [lo, -0.02, hi], [hi, -0.02, lo], [lo, -0.02, lo]].forEach(function (p) {
      pos.push(p[0], p[1], p[2]); nor.push(0, 1, 0); cols.push(base.r, base.g, base.b);
    });
    for (var j = 0; j < m.T; j++) for (var i = 0; i < m.T; i++) {
      if (M.wall(m, i, j)) continue;
      var c = (i + j) % 2 ? a : b;
      [[i, j + 1], [i + 1, j + 1], [i + 1, j], [i, j + 1], [i + 1, j], [i, j]].forEach(function (p) {
        pos.push(p[0], 0, p[1]); nor.push(0, 1, 0); cols.push(c.r, c.g, c.b);
      });
    }
    return geometry(pos, nor, cols);
  }
  function geometry(pos, nor, cols) {
    var g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
    g.setAttribute('normal', new THREE.Float32BufferAttribute(nor, 3));
    g.setAttribute('color', new THREE.Float32BufferAttribute(cols, 3));
    g.computeBoundingSphere();
    return g;
  }

  function build(m, gemList, hue) {
    if (walls) { scene.remove(walls); walls.geometry.dispose(); }
    if (floor) { scene.remove(floor); floor.geometry.dispose(); }
    if (gems) { scene.remove(gems); gems.dispose(); }
    walls = new THREE.Mesh(wallGeometry(m, hue), mats.wall);
    floor = new THREE.Mesh(floorGeometry(m, hue), mats.floor);
    gems = new THREE.InstancedMesh(gemGeo, mats.gem, Math.max(1, gemList.length));
    gems.count = gemList.length;
    // 实例在动（转、上下浮），包围球按整张图算一次，不每帧重算
    gems.frustumCulled = false;
    scene.add(floor); scene.add(walls); scene.add(gems);
    pad.position.set(m.exit.i + 0.5, 0.03, m.exit.j + 0.5);
    beam.position.set(m.exit.i + 0.5, 1.5, m.exit.j + 0.5);
    var tris = (walls.geometry.attributes.position.count + floor.geometry.attributes.position.count) / 3;
    return { tris: tris };
  }

  // 每帧：宝石转 + 浮，已拿的缩成 0；出口没开是灰的、开了是绿的并亮起光柱
  function update(t, gemList, open, px, pz, roll) {
    for (var k = 0; k < gemList.length; k++) {
      var g = gemList[k], s = g.taken ? Math.max(0, 1 - g.fade * 4) : 1;
      dummy.position.set(g.i + 0.5, 0.45 + Math.sin(t * 2.4 + k) * 0.08 + (g.taken ? g.fade * 1.5 : 0), g.j + 0.5);
      dummy.rotation.set(0, t * 1.8 + k, 0);
      dummy.scale.setScalar(s);
      dummy.updateMatrix();
      gems.setMatrixAt(k, dummy.matrix);
    }
    gems.instanceMatrix.needsUpdate = true;
    mats.pad.color.setHex(open ? 0x3ddc84 : 0x9e9e9e);
    mats.pad.emissive.setHex(open ? 0x0d5a2a : 0x000000);
    beam.visible = open;
    mats.beam.opacity = 0.25 + Math.sin(t * 4) * 0.1;
    player.position.set(px, 0.28, pz);
    player.rotation.set(roll.x, 0, roll.z);
    shadow.position.set(px + 0.06, 0.01, pz + 0.06);
  }

  return { build: build, update: update, player: player };
}

module.exports = { make: make, WALL_H: WALL_H };
