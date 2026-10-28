// 镜头：透视相机绕「看着的点」转（俯角固定 52°）。单指拖 = 平移（松手有惯性），双指捏 = 缩放，双指拧 = 转向；
// 点地面 → 地格（射线与 y=0.3 平面求交）。focus(x, z) 平滑飞过去（点了建筑、新建完成时用）。
function make(THREE, W, H, N) {
  var cam = new THREE.PerspectiveCamera(40, W / H, 0.1, 200);
  var target = new THREE.Vector3(0, 0.3, 0.5), yaw = Math.PI / 4, dist = 13.5, PITCH = 52 * Math.PI / 180;
  var MIN_D = 6, MAX_D = 22, LIM = N / 2;
  var vel = { x: 0, z: 0 }, fly = null;
  var ray = new THREE.Raycaster(), ndc = new THREE.Vector2(), plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -0.3), hit = new THREE.Vector3();

  function apply() {
    var h = dist * Math.sin(PITCH), r = dist * Math.cos(PITCH);
    cam.position.set(target.x - Math.sin(yaw) * r, target.y + h, target.z - Math.cos(yaw) * r);
    cam.lookAt(target);
  }
  apply();
  // 屏幕上的位移 → 地面上的位移（按当前距离与视角换算，手指下的地面跟着手指走）
  function screenToGround(dx, dy) {
    var k = 2 * dist * Math.tan(cam.fov * Math.PI / 360) / H;
    var fx = Math.sin(yaw), fz = Math.cos(yaw), rx = -Math.cos(yaw), rz = Math.sin(yaw);
    var gy = dy * k / Math.sin(PITCH);
    return { x: -(dx * k) * rx + gy * fx, z: -(dx * k) * rz + gy * fz };
  }
  function clamp() { target.x = Math.max(-LIM, Math.min(LIM, target.x)); target.z = Math.max(-LIM, Math.min(LIM, target.z)); }

  var gesture = null;
  return {
    cam: cam,
    get yaw() { return yaw; },
    // 手势：touches = [{id, x, y}]（当前按着的全部）；返回 'pan' | 'pinch' | null
    begin: function (touches) {
      fly = null; vel.x = vel.z = 0;
      gesture = touches.length >= 2 ? pinchState(touches) : { kind: 'pan', x: touches[0].x, y: touches[0].y, t: Date.now(), hist: [] };
    },
    move: function (touches) {
      if (!gesture) return;
      if (touches.length >= 2) {
        if (gesture.kind !== 'pinch') gesture = pinchState(touches);
        var p = pinchState(touches);
        dist = Math.max(MIN_D, Math.min(MAX_D, dist * gesture.span / p.span));
        yaw += p.angle - gesture.angle;
        gesture = p;
      } else if (gesture.kind === 'pan') {
        var dx = touches[0].x - gesture.x, dy = touches[0].y - gesture.y;
        var g = screenToGround(dx, dy);
        target.x += g.x; target.z += g.z; clamp();
        var now = Date.now();
        gesture.hist.push({ x: g.x, z: g.z, t: now });
        gesture.hist = gesture.hist.filter(function (h) { return now - h.t < 100; });
        gesture.x = touches[0].x; gesture.y = touches[0].y;
      } else gesture = { kind: 'pan', x: touches[0].x, y: touches[0].y, hist: [] };   // 双指松了一根：接着平移
      apply();
    },
    end: function (touchesLeft) {
      if (gesture && gesture.kind === 'pan' && !touchesLeft.length && gesture.hist.length > 1) {
        // 惯性：最后 0.1 秒的平均速度
        var sx = 0, sz = 0, span = Math.max(16, Date.now() - gesture.hist[0].t);
        gesture.hist.forEach(function (h) { sx += h.x; sz += h.z; });
        vel.x = sx / span * 1000; vel.z = sz / span * 1000;
      }
      gesture = touchesLeft.length ? (touchesLeft.length >= 2 ? pinchState(touchesLeft) : { kind: 'pan', x: touchesLeft[0].x, y: touchesLeft[0].y, hist: [] }) : null;
    },
    zoomBy: function (k) { dist = Math.max(MIN_D, Math.min(MAX_D, dist * k)); apply(); },
    rotateBy: function (a) { yaw += a; apply(); },
    // 调试 / 截图：直接摆到某处（不受 MIN_D 限制，近看模型用）
    place: function (x, z, d, y) { target.x = x; target.z = z; if (d) dist = d; if (y !== undefined && !isNaN(y)) yaw = y; fly = null; apply(); },
    focus: function (x, z, d) { fly = { x: x, z: z, d: d || dist }; vel.x = vel.z = 0; },
    step: function (dt) {
      if (fly) {
        var k = 1 - Math.exp(-dt * 6);
        target.x += (fly.x - target.x) * k; target.z += (fly.z - target.z) * k; dist += (fly.d - dist) * k;
        if (Math.abs(fly.x - target.x) + Math.abs(fly.z - target.z) < 0.01) fly = null;
        apply();
      } else if (!gesture && (Math.abs(vel.x) + Math.abs(vel.z) > 0.01)) {
        target.x += vel.x * dt; target.z += vel.z * dt; clamp();
        var damp = Math.exp(-dt * 4.5); vel.x *= damp; vel.z *= damp;
        apply();
      }
    },
    // 屏幕点 → 地面世界坐标（y = 0.3 平面）
    pick: function (sx, sy) {
      ndc.set(sx / W * 2 - 1, -(sy / H) * 2 + 1);
      ray.setFromCamera(ndc, cam);
      return ray.ray.intersectPlane(plane, hit) ? { x: hit.x, z: hit.z } : null;
    },
    // 世界坐标 → 屏幕（逻辑像素）：3D 上方的提示要和界面对齐时用
    project: function (x, y, z) { var v = new THREE.Vector3(x, y, z).project(cam); return { x: (v.x + 1) / 2 * W, y: (1 - v.y) / 2 * H, front: v.z < 1 }; },
    state: function () { return { x: +target.x.toFixed(2), z: +target.z.toFixed(2), dist: +dist.toFixed(2), yaw: +yaw.toFixed(2) }; }
  };
  function pinchState(t) {
    var dx = t[1].x - t[0].x, dy = t[1].y - t[0].y;
    return { kind: 'pinch', span: Math.max(10, Math.sqrt(dx * dx + dy * dy)), angle: Math.atan2(dy, dx) };
  }
}

module.exports = { make: make };
