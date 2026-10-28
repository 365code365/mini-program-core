// 角色模型：胶囊拼的小人。层级按动画需要分（动作在 js/pose.js）：
//   root（位置、换道侧倾 / 转身、跑步左右晃）→ body（上下颠、铲地时整体压低）
//     ├ hips（骨盆，带两条腿：胯 → 膝两节）
//     └ chest（腰以上：躯干、背包、双臂（肩 → 肘两节）、头）
// 骨盆和胸口分开转：跑步时肩与胯反向扭、身体左右压重心 —— 相机在正后方，四肢前后摆从背后看是缩短的，
// 真正让人「看得出在跑」的是这几样（v1 只有四肢前后摆，从背后看就是一块板在平移）。
// 角度约定：面朝 -z；绕 x 轴正转 = 肢体末端往前（-z）甩、躯干往后仰。
function make(THREE, mat) {
  var skin = mat(0xffcf9e, null, { shiny: 8 }), hood = mat(0x2f6fe4, null, { shiny: 6 }), hoodD = mat(0x1f4fb0),
    pants = mat(0x4a4e5a), shoe = mat(0xffffff, null, { shiny: 30 }), sole = mat(0xff5d3a), capM = mat(0xf4f4f4, null, { shiny: 20 }),
    brim = mat(0x2a4db0), bagM = mat(0xff9f1c, null, { shiny: 10 }), bagD = mat(0xd9480f), hair = mat(0x3a2414), strap = mat(0x5a2a0a);
  var box = new THREE.BoxGeometry(1, 1, 1);
  function cap(r, l) { return new THREE.CapsuleGeometry(r, l, 4, 10); }
  function mesh(g, mt, x, y, z, par, sx, sy, sz) {
    var o = new THREE.Mesh(g, mt); o.position.set(x, y, z); if (sx) o.scale.set(sx, sy, sz); par.add(o); return o;
  }
  var root = new THREE.Group(), body = new THREE.Group(); root.add(body);
  var hips = new THREE.Group(); hips.position.y = 0.93; body.add(hips);
  mesh(cap(0.16, 0.1), pants, 0, 0.02, 0, hips, 1.3, 0.85, 0.85);
  var chest = new THREE.Group(); chest.position.y = 0.98; body.add(chest);
  mesh(cap(0.25, 0.3), hood, 0, 0.32, 0, chest, 1, 1, 0.72);
  mesh(new THREE.CylinderGeometry(0.27, 0.24, 0.08, 14), hoodD, 0, 0.05, 0, chest, 1, 1, 0.78);      // 衣摆
  mesh(new THREE.SphereGeometry(0.16, 12, 8), hoodD, 0, 0.66, 0.1, chest, 1.25, 0.65, 0.85);           // 背后的帽兜
  // 头
  var head = new THREE.Group(); head.position.set(0, 0.82, 0); chest.add(head);
  mesh(new THREE.SphereGeometry(0.235, 16, 12), skin, 0, 0, 0, head);
  mesh(new THREE.SphereGeometry(0.245, 16, 8, 0, Math.PI * 2, 0, Math.PI * 0.42), hair, 0, 0, 0.02, head);
  mesh(new THREE.SphereGeometry(0.255, 16, 8, 0, Math.PI * 2, 0, Math.PI * 0.36), capM, 0, 0.03, 0, head);
  mesh(box, brim, 0, 0.1, -0.27, head, 0.34, 0.035, 0.2);                                               // 帽檐朝前（-z）
  mesh(new THREE.SphereGeometry(0.06, 8, 6), skin, -0.24, -0.02, 0, head);                              // 耳朵
  mesh(new THREE.SphereGeometry(0.06, 8, 6), skin, 0.24, -0.02, 0, head);
  // 背包：圆角的包身 + 口袋 + 两条肩带；单独一组，pose.js 给它一点跟随的晃动（惯性）
  var bag = new THREE.Group(); bag.position.set(0, 0.36, 0.2); chest.add(bag);
  mesh(cap(0.17, 0.16), bagM, 0, 0, 0, bag, 1.12, 1, 0.55);
  mesh(cap(0.1, 0.05), bagD, 0, -0.1, 0.08, bag, 1.15, 1, 0.45);
  [-0.13, 0.13].forEach(function (x) { mesh(box, strap, x, 0.6, 0.0, chest, 0.06, 0.04, 0.46); });
  // 两节肢体：轴心在胯 / 肩，第二节轴心在膝 / 肘
  var shoeG = cap(0.085, 0.16); shoeG.rotateX(Math.PI / 2);
  function limb(par, x, y, r, l1, l2, mt, foot) {
    var a = new THREE.Group(); a.position.set(x, y, 0); par.add(a);
    mesh(cap(r, l1), mt, 0, -l1 / 2 - r * 0.3, 0, a);
    var b = new THREE.Group(); b.position.set(0, -l1 - r * 0.2, 0); a.add(b);
    mesh(cap(r * 0.88, l2), mt, 0, -l2 / 2 - r * 0.3, 0, b);
    if (foot) {
      mesh(shoeG, shoe, 0, -l2 - r * 0.95, -0.06, b, 1.15, 0.85, 1);
      mesh(box, sole, 0, -l2 - r * 0.95 - 0.07, -0.06, b, 0.19, 0.035, 0.33);
    } else mesh(new THREE.SphereGeometry(r * 1.15, 8, 6), skin, 0, -l2 - r * 0.6, 0, b);
    return { a: a, b: b };
  }
  var legL = limb(hips, -0.12, 0, 0.095, 0.4, 0.36, pants, true), legR = limb(hips, 0.12, 0, 0.095, 0.4, 0.36, pants, true);
  var armL = limb(chest, -0.33, 0.55, 0.072, 0.27, 0.25, hood), armR = limb(chest, 0.33, 0.55, 0.072, 0.27, 0.25, hood);
  return { root: root, body: body, hips: hips, chest: chest, head: head, bag: bag, legL: legL, legR: legR, armL: armL, armR: armR };
}
module.exports = { make: make };
