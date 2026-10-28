// 保龄球道：three.js 画面 + cannon-es 刚体（球、十个瓶、球道、两条边沟、瓶区后面的坑与挡板）。
// 尺寸按真实球道（米）：道宽 1.06、犯规线到头瓶 18.29、瓶间距 0.3048、瓶高 0.38、球半径 0.109。
// 坐标：犯规线 z = 0，往 -z 投；x 向右；y 向上，道面 y = 0。
var LANE_W = 1.06, GUTTER_W = 0.24, HEAD_Z = -18.29, PIN_H = 0.38, PIN_R = 0.06, BALL_R = 0.109, SPACING = 0.3048;

function pinSpots() {
  var out = [];
  for (var row = 0; row < 4; row++) for (var i = 0; i <= row; i++)
    out.push([(i - row / 2) * SPACING, HEAD_Z - row * SPACING * Math.cos(Math.PI / 6)]);
  return out;
}

function make(THREE, CANNON, aspect) {
  var scene = new THREE.Scene();
  scene.background = new THREE.Color(0x1a1530);
  scene.fog = new THREE.Fog(0x1a1530, 14, 34);
  var cam = new THREE.PerspectiveCamera(55, aspect, 0.05, 80);
  scene.add(new THREE.HemisphereLight(0xffffff, 0x302040, 0.85));
  var key = new THREE.DirectionalLight(0xffffff, 0.8); key.position.set(1, 6, 3); scene.add(key);
  var pinLight = new THREE.PointLight(0xfff2d0, 1.4, 8); pinLight.position.set(0, 2.2, HEAD_Z + 1); scene.add(pinLight);

  var world = new CANNON.World({ gravity: new CANNON.Vec3(0, -9.82, 0) });
  // 真机上物理每帧占 JS 时间的大头（一加 PLY110：每帧 JS 中位 13.9ms，p95 58ms）。所以：
  // 沿球道（z）的扫掠剪枝代替两两检查；瓶子摆好就睡（睡着的不进碰撞对、不进解算），球碰到才醒；步长 1/60
  world.broadphase = new CANNON.SAPBroadphase(world);
  world.broadphase.axisIndex = 2;
  world.allowSleep = true;
  world.solver.iterations = 8;
  var mLane = new CANNON.Material('lane'), mBall = new CANNON.Material('ball'), mPin = new CANNON.Material('pin');
  world.addContactMaterial(new CANNON.ContactMaterial(mBall, mLane, { friction: 0.04, restitution: 0.05 }));
  world.addContactMaterial(new CANNON.ContactMaterial(mPin, mLane, { friction: 0.3, restitution: 0.2 }));
  world.addContactMaterial(new CANNON.ContactMaterial(mBall, mPin, { friction: 0.1, restitution: 0.6 }));
  world.addContactMaterial(new CANNON.ContactMaterial(mPin, mPin, { friction: 0.15, restitution: 0.5 }));

  function staticBox(hx, hy, hz, x, y, z, material) {
    var b = new CANNON.Body({ mass: 0, material: material || mLane, shape: new CANNON.Box(new CANNON.Vec3(hx, hy, hz)) });
    b.position.set(x, y, z); world.addBody(b); return b;
  }
  function visBox(w, h, d, x, y, z, color, mat) {
    var m = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), mat || new THREE.MeshLambertMaterial({ color: color }));
    m.position.set(x, y, z); scene.add(m); return m;
  }

  // ───────── 球道：木纹 + 瞄准箭头画在离屏画布上当纹理 ─────────
  var LEN = -HEAD_Z + 1.2, APPROACH = 4;
  var tc = wx.createCanvas(); tc.width = 128; tc.height = 1024;
  var g = tc.getContext('2d');
  for (var k = 0; k < 39; k++) { g.fillStyle = k % 2 ? '#d9a86c' : '#cf9a5c'; g.fillRect(k * 128 / 39, 0, 128 / 39 + 1, 1024); }
  g.fillStyle = 'rgba(80,40,10,0.08)'; for (var n = 0; n < 300; n++) g.fillRect(Math.random() * 128, Math.random() * 1024, 1, 8 + Math.random() * 20);
  g.fillStyle = '#6b2d0f';
  for (var a = 0; a < 7; a++) {                       // 犯规线前方 4.6m 的七个箭头
    var ax = 64 + (a - 3) * 16, ay = 1024 - (4.6 + Math.abs(a - 3) * 0.3) / LEN * 1024;
    g.beginPath(); g.moveTo(ax, ay - 14); g.lineTo(ax - 4, ay); g.lineTo(ax + 4, ay); g.closePath(); g.fill();
  }
  var laneTex = new THREE.CanvasTexture(tc);
  if (THREE.SRGBColorSpace) laneTex.colorSpace = THREE.SRGBColorSpace;
  var laneMesh = new THREE.Mesh(new THREE.PlaneGeometry(LANE_W, LEN), new THREE.MeshPhongMaterial({ map: laneTex, shininess: 80, specular: 0x333333 }));
  laneMesh.rotation.x = -Math.PI / 2; laneMesh.position.set(0, 0, -LEN / 2); scene.add(laneMesh);
  visBox(LANE_W, 0.01, APPROACH, 0, -0.004, APPROACH / 2, 0xc28a4c);                 // 助走区
  visBox(LANE_W, 0.012, 0.03, 0, 0.001, 0, 0x222222);                                  // 犯规线
  staticBox(LANE_W / 2, 0.1, (LEN + APPROACH) / 2, 0, -0.1, (APPROACH - LEN) / 2);
  // 边沟：比道面低 0.06 的槽，外侧有挡板
  [-1, 1].forEach(function (s) {
    var gx = s * (LANE_W / 2 + GUTTER_W / 2);
    visBox(GUTTER_W, 0.02, LEN, gx, -0.07, -LEN / 2, 0x55596a);
    staticBox(GUTTER_W / 2, 0.1, LEN / 2, gx, -0.16, -LEN / 2);
    visBox(0.06, 0.5, LEN + APPROACH, s * (LANE_W / 2 + GUTTER_W + 0.03), 0.1, (APPROACH - LEN) / 2, 0x2b2540);
    staticBox(0.03, 0.4, (LEN + APPROACH) / 2, s * (LANE_W / 2 + GUTTER_W + 0.03), 0.1, (APPROACH - LEN) / 2);
  });
  // 瓶区后面：坑（低 0.4）与挡板
  var pitZ = -LEN - 0.8;
  staticBox(1.2, 0.1, 0.8, 0, -0.5, pitZ);
  visBox(2.4, 0.02, 1.6, 0, -0.4, pitZ, 0x111111);
  staticBox(1.2, 0.8, 0.05, 0, 0.2, pitZ - 0.8);
  visBox(2.4, 1.6, 0.1, 0, 0.2, pitZ - 0.8, 0x2a1f3a);
  visBox(2.4, 0.5, 0.1, 0, 1.3, HEAD_Z - 0.2, 0x3a2f55);                               // 上方的遮板

  // ───────── 瓶：车床体（白底 + 两道红环），物理用圆柱 ─────────
  // 瓶颈上两道红环用顶点色画在同一个网格上（一个瓶 = 一次 draw call；环做成单独网格时十个瓶是 30 次）。
  // 剖面在环的上下沿各放两个同高的点：环与白身各有自己的顶点，颜色不会在交界处糊成粉色渐变
  var prof = [[0.026, 0, 0], [0.045, 0.03, 0], [0.06, 0.1, 0], [0.055, 0.17, 0], [0.034, 0.236, 0],
    [0.0335, 0.244, 0], [0.0335, 0.244, 1], [0.0325, 0.258, 1], [0.0325, 0.258, 0],
    [0.031, 0.266, 0], [0.031, 0.266, 1], [0.031, 0.28, 1], [0.031, 0.28, 0],
    [0.038, 0.32, 0], [0.032, 0.36, 0], [0.012, 0.38, 0], [0, 0.381, 0]];
  var SEG = 18, pinGeo = new THREE.LatheGeometry(prof.map(function (p) { return new THREE.Vector2(p[0], p[1] - PIN_H / 2); }), SEG);
  var cols = new Float32Array(pinGeo.attributes.position.count * 3);
  for (var vi = 0; vi < pinGeo.attributes.position.count; vi++) {          // LatheGeometry 的顶点按「每一列（绕圈）× 剖面点」排
    var red = prof[vi % prof.length][2];
    cols[vi * 3] = red ? 0.84 : 1; cols[vi * 3 + 1] = red ? 0.16 : 1; cols[vi * 3 + 2] = red ? 0.16 : 1;
  }
  pinGeo.setAttribute('color', new THREE.BufferAttribute(cols, 3));
  var pinMat = new THREE.MeshPhongMaterial({ color: 0xffffff, vertexColors: true, shininess: 90 });
  var pins = pinSpots().map(function (s) {
    var mesh = new THREE.Mesh(pinGeo, pinMat);
    scene.add(mesh);
    var body = new CANNON.Body({ mass: 1.5, material: mPin, shape: new CANNON.Cylinder(PIN_R * 0.8, PIN_R, PIN_H, 10) });
    body.sleepSpeedLimit = 0.15; body.sleepTimeLimit = 0.4; body.linearDamping = 0.05; body.angularDamping = 0.2;
    return { mesh: mesh, body: body, spot: s, active: false };
  });

  // ───────── 球 ─────────
  var ballTex = (function () {
    var c = wx.createCanvas(); c.width = 128; c.height = 64; var x = c.getContext('2d');
    x.fillStyle = '#2b59c3'; x.fillRect(0, 0, 128, 64);
    for (var i = 0; i < 40; i++) { x.fillStyle = 'rgba(255,255,255,' + (0.05 + Math.random() * 0.15) + ')'; x.beginPath(); x.arc(Math.random() * 128, Math.random() * 64, 2 + Math.random() * 6, 0, Math.PI * 2); x.fill(); }
    x.fillStyle = '#111'; [[60, 20], [70, 20], [65, 30]].forEach(function (p) { x.beginPath(); x.arc(p[0], p[1], 3, 0, Math.PI * 2); x.fill(); });   // 指孔
    var t = new THREE.CanvasTexture(c); if (THREE.SRGBColorSpace) t.colorSpace = THREE.SRGBColorSpace; return t;
  })();
  var ballMesh = new THREE.Mesh(new THREE.SphereGeometry(BALL_R, 28, 18), new THREE.MeshPhongMaterial({ map: ballTex, shininess: 120, specular: 0x666666 }));
  scene.add(ballMesh);
  var ball = new CANNON.Body({ mass: 6.5, material: mBall, shape: new CANNON.Sphere(BALL_R) });
  ball.linearDamping = 0.02; ball.angularDamping = 0.02;
  world.addBody(ball);
  var aimLine = new THREE.Mesh(new THREE.PlaneGeometry(0.02, 4), new THREE.MeshBasicMaterial({ color: 0xffffff, transparent: true, opacity: 0.35, depthWrite: false }));
  aimLine.rotation.x = -Math.PI / 2; scene.add(aimLine);

  function placePin(p) {
    p.body.position.set(p.spot[0], PIN_H / 2 + 0.001, p.spot[1]);
    p.body.quaternion.set(0, 0, 0, 1);
    p.body.velocity.set(0, 0, 0); p.body.angularVelocity.set(0, 0, 0);
    if (!p.active) { world.addBody(p.body); p.active = true; }
    p.body.sleep(); p.mesh.visible = true;
    p.mesh.position.copy(p.body.position); p.mesh.quaternion.copy(p.body.quaternion);
  }
  function removePin(p) { if (p.active) { world.removeBody(p.body); p.active = false; } p.mesh.visible = false; }
  var up = new CANNON.Vec3();
  function isDown(p) {
    if (!p.active) return true;
    p.body.quaternion.vmult(new CANNON.Vec3(0, 1, 0), up);
    return up.y < 0.75 || p.body.position.y < PIN_H / 2 - 0.08 || Math.abs(p.body.position.x) > LANE_W / 2 + 0.02;
  }

  var camPos = new THREE.Vector3(0, 0.9, 2.6), camLook = new THREE.Vector3(0, 0.1, -8);
  return {
    scene: scene, cam: cam, LANE_W: LANE_W, HEAD_Z: HEAD_Z, BALL_R: BALL_R,
    rack: function () { pins.forEach(placePin); },
    // 拿掉倒了的瓶，返回倒了几个（这一投之前站着的里面）
    sweep: function () { var n = 0; pins.forEach(function (p) { if (p.active && isDown(p)) { removePin(p); n++; } }); return n; },
    standing: function () { return pins.filter(function (p) { return p.active && !isDown(p); }).length; },
    pinsSettled: function () { return pins.every(function (p) { return !p.active || p.body.sleepState === CANNON.Body.SLEEPING || p.body.velocity.length() < 0.05; }); },
    holdBall: function (x) {
      ball.type = CANNON.Body.KINEMATIC; ball.position.set(x, BALL_R + 0.002, 0.6); ball.velocity.set(0, 0, 0); ball.angularVelocity.set(0, 0, 0);
      ball.quaternion.set(0, 0, 0, 1);
    },
    throwBall: function (vx, vz, hook) {
      ball.type = CANNON.Body.DYNAMIC; ball.mass = 6.5; ball.updateMassProperties();
      // 纯滚动（接触点不滑）：v + ω × r = 0，r = (0, -R, 0) → ωx = vz / R、ωz = -vx / R。
      // 符号反了球就是带着倒旋出手，摩擦力把它从 8 m/s 刹到 1.6 m/s，滚不到瓶子
      ball.velocity.set(vx, 0, vz); ball.angularVelocity.set(vz / BALL_R, 0, -vx / BALL_R); ball.wakeUp();
      ball.userData = { hook: hook || 0 };
    },
    ball: function () { return ball; },
    step: function (dt, rolling) {
      // 弧线：球在油道段（离头瓶 > 3m）上受一点侧向力
      if (rolling && ball.userData && ball.userData.hook && ball.position.z > HEAD_Z + 3 && ball.position.y > 0)
        ball.applyForce(new CANNON.Vec3(ball.userData.hook * 9, 0, 0), ball.position);
      world.step(1 / 60, dt, 3);
      ballMesh.position.copy(ball.position); ballMesh.quaternion.copy(ball.quaternion);
      pins.forEach(function (p) { if (p.active) { p.mesh.position.copy(p.body.position); p.mesh.quaternion.copy(p.body.quaternion); } });
    },
    aim: function (x, angle, visible) {
      aimLine.visible = visible;
      aimLine.position.set(x + Math.sin(angle) * 2.2, 0.003, 0.6 - Math.cos(angle) * 2.2);
      aimLine.rotation.z = -angle;
    },
    camera: function (dt, phase) {
      var b = ball.position, z = Math.max(b.z, HEAD_Z + 5.5);
      var tp, tl;
      if (phase === 'aim') { tp = [b.x * 0.4, 0.75, 1.9]; tl = [b.x * 0.2, 0.05, -10]; }
      else if (phase === 'roll' && b.z > HEAD_Z + 5.5) { tp = [b.x * 0.5, 0.65, z + 1.6]; tl = [b.x * 0.3, 0.05, z - 6]; }
      else { tp = [0, 1.05, HEAD_Z + 3.4]; tl = [0, 0.1, HEAD_Z - 0.3]; }
      var k = 1 - Math.exp(-dt * 5);
      camPos.x += (tp[0] - camPos.x) * k; camPos.y += (tp[1] - camPos.y) * k; camPos.z += (tp[2] - camPos.z) * k;
      camLook.x += (tl[0] - camLook.x) * k; camLook.y += (tl[1] - camLook.y) * k; camLook.z += (tl[2] - camLook.z) * k;
      cam.position.copy(camPos); cam.lookAt(camLook);
    }
  };
}

module.exports = { make: make };
