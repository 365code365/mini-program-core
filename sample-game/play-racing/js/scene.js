// 伪 3D 公路的投影与绘制（分段投影，Lou's Pseudo 3d / Jake Gordon 那一套）：
// 每段起点投到屏幕上得到 (x, y, 半宽)，相邻两点之间是一块梯形路面；弯道是逐段累加的横向偏移，坡是段端点的高度。
// 绘制顺序：先从近到远算出哪些段露在坡顶之上（maxy 裁剪），再**从远到近**画路面并紧跟着画这一段上的东西
// （树、车、金币），近处的坡顶自然盖住远处的车 —— 画家算法，不用逐像素深度。
// 每帧不分配：投影结果放在预分配的 Float32Array，颜色（含雾化 8 档）开局算好查表。
// 性能（无头 CPU 后端 swiftshader + 4x MSAA，同一进程里逐帧交替量，gl.finish 计时）：一帧大头是整屏宽的东西，
// 不是顶点数 —— 十几条深色草条（总共 69 个顶点）就占 ~5ms。所以整屏宽的纯色（草条、天空上半、草地底色）一律
// 用 gl.scissor + gl.clear 铺（草条那一步 10.2 → 7.4ms），渐变只留贴着地平线的窄带；远处三段并一块画路面。
// 真机上（GPU）填充不要钱，瓶颈是 JS 拼顶点：所以不变的远山仍录成图层，远处的树 / 车 / 金币按屏上大小降成一两块。
var T = require('./track.js');
var SEG = T.SEG, RW = T.RW, LANE_X = T.LANE_X;
var DRAW = 110, FOG_LV = 8, NEAR = 24, RUMBLE = 3;  // NEAR 段以内逐段画，再远三段并一块（与 track.js 的 RUMBLE 一致）
var CAMH = 1300, FOV = 100, DEPTH = 1 / Math.tan(FOV / 2 * Math.PI / 180);
var PLAYER_Z = CAMH * DEPTH;
var CAR_W = 360;                                      // 车宽（世界单位），车道宽 = RW × 2/3 = 800

var PALETTES = [
  { name: '晴天', sky0: '#3d8fd6', sky1: '#bfe4f7', fog: '#cfe8f2', mount: '#7aa6c2', mount2: '#5b8aa8',
    grassL: '#5fb53a', grassD: '#52a632', roadL: '#6b6b6b', roadD: '#666666', rumL: '#f2f2f2', rumD: '#d93a2b', lane: '#f5f5f5' },
  { name: '黄昏', sky0: '#3a2a6b', sky1: '#ff9a5a', fog: '#f0a878', mount: '#6b4a7a', mount2: '#4e3560',
    grassL: '#4f8f3a', grassD: '#44803a', roadL: '#5a5560', roadD: '#555059', rumL: '#e8e0e0', rumD: '#c73a2b', lane: '#f0e6d8' }
];
var CAR_COLORS = ['#1e88e5', '#fdd835', '#8e24aa', '#43a047', '#fb8c00'];

function make(R, W, H) {
  var UX = W / 2, UY = H * 0.55, HY = H * 0.42;       // 横向 / 纵向投影尺度与地平线（横屏屏幕扁，纵向单独定尺度）
  var PX = new Float32Array(DRAW + 1), PY = new Float32Array(DRAW + 1), PW = new Float32Array(DRAW + 1),
    PS = new Float32Array(DRAW + 1), PZ = new Float32Array(DRAW + 1), VIS = new Uint8Array(DRAW), CLIP = new Float32Array(DRAW);
  var FOGK = new Uint8Array(DRAW + 1);
  for (var k = 0; k <= DRAW; k++) FOGK[k] = Math.min(FOG_LV - 1, Math.floor(Math.pow(k / DRAW, 1.5) * FOG_LV));
  var C = null, palIdx = -1, carCols = [], carDark = [];

  function mix(a, b, t) { return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, 1]; }
  function fogged(hex, fog) {
    var c = R.color(hex), out = [];
    for (var i = 0; i < FOG_LV; i++) out.push(mix(c, fog, Math.pow(i / FOG_LV, 1.3) * 0.75));
    return out;
  }
  // 地面底色渐变按平路上各段的屏幕位置排雾化档：段 k 落在 y = HY + CAMH·DEPTH·UY / (k·SEG)，
  // 雾化档从 k_L = DRAW·(L/8)^(1/1.5) 开始。这样浅色草地（底色）与逐段铺的深色草条在同一行是同一档雾，远处不出白条
  // 渐变只画到最后一档（再往下是纯色，由裁剪清屏铺好了）
  var groundStops = [], groundH = 1;
  function buildGround() {
    groundH = Math.min(H - HY, CAMH * DEPTH * UY / (DRAW * Math.pow(1 / FOG_LV, 1 / 1.5) * SEG));
    groundStops = [[0, C.grassL[FOG_LV - 1]]];
    for (var L = FOG_LV - 1; L >= 1; L--) {
      var kL = DRAW * Math.pow(L / FOG_LV, 1 / 1.5), y = CAMH * DEPTH * UY / (kL * SEG);
      groundStops.push([Math.min(1, y / groundH), C.grassL[L - 1]]);
    }
  }
  function setPalette(i) {
    if (i === palIdx) return;
    palIdx = i;
    var p = PALETTES[i], fog = R.color(p.fog);
    C = { p: p };
    ['grassL', 'grassD', 'roadL', 'roadD', 'rumL', 'rumD', 'lane'].forEach(function (k) { C[k] = fogged(p[k], fog); });
    C.boostA = fogged('#ffb300', fog); C.boostB = fogged('#ff6f00', fog);
    C.trunk = fogged('#6d4c41', fog); C.pine = fogged('#2e7d32', fog); C.pine2 = fogged('#388e3c', fog);
    C.tree = fogged('#43a047', fog); C.tree2 = fogged('#66bb6a', fog); C.bush = fogged('#558b2f', fog);
    C.pole = fogged('#9e9e9e', fog); C.lamp = fogged('#fff59d', fog);
    C.coin = fogged('#ffca28', fog); C.coin2 = fogged('#fff3b0', fog);
    C.can = fogged('#e53935', fog); C.can2 = fogged('#ffeb3b', fog);
    C.glass = fogged('#263238', fog); C.tyre = fogged('#212121', fog); C.tail = fogged('#ff1744', fog);
    buildGround();
    carCols = CAR_COLORS.map(function (h) { return fogged(h, fog); });
    carDark = CAR_COLORS.map(function (h) { var c = R.color(h); return fogged('rgb(' + [c[0] * 160, c[1] * 160, c[2] * 160].map(Math.round).join(',') + ')', fog); });
  }

  // 每段起点投影：段 base+k 的起点到相机的距离 zr，按透视缩放
  function project(segs, pos, playerX, playerY) {
    var N = segs.length, base = Math.floor(pos / SEG) % N, pct = (pos % SEG) / SEG;
    var camX = playerX * RW, camY = CAMH + playerY;
    var x = 0, dx = -segs[base].curve * pct;
    for (var k = 0; k <= DRAW; k++) {
      var s = segs[(base + k) % N], zr = (Math.floor(pos / SEG) + k) * SEG - pos;
      var sc = zr > 1 ? DEPTH / zr : 0;
      PZ[k] = zr; PS[k] = sc;
      PX[k] = UX + sc * (x - camX) * UX;
      PY[k] = HY - sc * (s.y1 - camY) * UY;
      PW[k] = sc * RW * UX;
      x += dx; dx += s.curve;
    }
    // 从近到远：只有比已画部分更高（屏幕 y 更小）的段才露得出来
    var maxy = H;
    for (k = 0; k < DRAW; k++) {
      VIS[k] = 0; CLIP[k] = maxy;
      if (PZ[k] <= DEPTH) continue;
      if (PY[k + 1] >= PY[k] || PY[k + 1] >= maxy) continue;
      VIS[k] = 1; maxy = PY[k + 1];
    }
    return base;
  }

  // 路面一块：近端点 k、远端点 kf（近处 kf = k + 1；远处三段并成一块，kf = k + 3）。
  // 远处一段只有一两个像素高，三段并一块看不出差别（同一组三段颜色本来就一样：RUMBLE = 3），三角形少 2/3。
  // 草条不在这里画（grass() 用清屏铺）
  function drawRoadSeg(s, k, kf) {
    var x1 = PX[k], y1 = PY[k], w1 = PW[k], x2 = PX[kf], y2 = PY[kf], w2 = PW[kf], f = FOGK[k], d = s.dark;
    var r1 = w1 / 7, r2 = w2 / 7, rc = d ? C.rumD[f] : C.rumL[f];
    R.quad(x1 - w1 - r1, y1, x1 - w1, y1, x2 - w2, y2, x2 - w2 - r2, y2, rc);
    R.quad(x1 + w1, y1, x1 + w1 + r1, y1, x2 + w2 + r2, y2, x2 + w2, y2, rc);
    R.quad(x1 - w1, y1, x1 + w1, y1, x2 + w2, y2, x2 - w2, y2, d ? C.roadD[f] : C.roadL[f]);
    if (s.boost >= 0) {                              // 加速带：车道里一块橙黄相间的路面
      var c = LANE_X[s.boost];
      R.quad(x1 + (c - 0.28) * w1, y1, x1 + (c + 0.28) * w1, y1, x2 + (c + 0.28) * w2, y2, x2 + (c - 0.28) * w2, y2,
        s.i % 2 ? C.boostA[f] : C.boostB[f]);
    }
    if (!d && w1 > 6) {                              // 车道线只画在浅色段上 = 虚线
      var l1 = w1 / 36, l2 = w2 / 36, lc = C.lane[f];
      for (var j = -1; j <= 1; j += 2) {
        var a1 = x1 + w1 * j / 3, a2 = x2 + w2 * j / 3;
        R.quad(a1 - l1, y1, a1 + l1, y1, a2 + l2, y2, a2 - l2, y2, lc);
      }
    }
  }

  // 路边景物（底边中点 bx, by，px = 每世界单位多少像素）
  function drawDeco(s, k, bx, by, px) {
    var f = FOGK[k], sc = s.decoS, h, w;
    if (s.deco === 0) {                              // 松树：两层三角 + 树干
      h = 1300 * sc * px; w = 620 * sc * px; if (h < 2) return;
      if (h < 16) { R.quad(bx, by - h, bx + w / 2, by, bx - w / 2, by, bx, by - h, C.pine[f]); return; }   // 远处只画一个三角
      R.rect(bx - w * 0.07, by - h * 0.22, w * 0.14, h * 0.22, C.trunk[f]);
      R.quad(bx, by - h, bx + w * 0.38, by - h * 0.5, bx - w * 0.38, by - h * 0.5, bx, by - h, C.pine2[f]);
      R.quad(bx, by - h * 0.72, bx + w / 2, by - h * 0.2, bx - w / 2, by - h * 0.2, bx, by - h * 0.72, C.pine[f]);
    } else if (s.deco === 1) {                       // 圆树
      h = 1100 * sc * px; w = 700 * sc * px; if (h < 2) return;
      if (h < 16) { R.rect(bx - w * 0.4, by - h, w * 0.8, h * 0.8, C.tree[f]); return; }
      R.rect(bx - w * 0.06, by - h * 0.4, w * 0.12, h * 0.4, C.trunk[f]);
      R.circle(bx, by - h * 0.62, w * 0.42, C.tree[f]);
      if (w > 40) R.circle(bx - w * 0.12, by - h * 0.72, w * 0.18, C.tree2[f]);
    } else if (s.deco === 2) {                       // 灌木
      w = 520 * sc * px; if (w < 2) return;
      if (w < 16) { R.rect(bx - w / 2, by - w * 0.36, w, w * 0.36, C.bush[f]); return; }
      R.ellipse(bx, by - w * 0.18, w / 2, w * 0.22, 0, C.bush[f]);
    } else {                                         // 路灯：灯杆朝路面一侧伸出
      h = 1500 * px; if (h < 3) return;
      var dir = s.decoX < 0 ? 1 : -1, lw = Math.max(1, 40 * px);
      R.rect(bx - lw / 2, by - h, lw, h, C.pole[f]);
      R.rect(dir > 0 ? bx : bx - h * 0.22, by - h, h * 0.22, lw, C.pole[f]);
      R.rect(bx + dir * h * 0.22 - (dir > 0 ? lw * 2 : 0), by - h + lw, lw * 2, lw, C.lamp[f]);
    }
  }

  // 后视角的车：车宽 w 像素，底边中点 (cx, by)。远处的车只画一块，省顶点
  function drawCar(cx, by, w, col, f, kind) {
    var body = carCols[col][f], dark = carDark[col][f];
    if (w < 14) { R.rect(cx - w / 2, by - w * 0.5, w, w * 0.5, body); return; }   // 远处的车一块色
    if (w < 30) {                                     // 再近一点：车身 + 后窗 + 尾灯三块
      R.rect(cx - w / 2, by - w * 0.5, w, w * 0.5, body);
      R.rect(cx - w * 0.32, by - w * 0.48, w * 0.64, w * 0.14, C.glass[f]);
      R.rect(cx - w * 0.46, by - w * 0.3, w * 0.92, w * 0.06, C.tail[f]);
      return;
    }
    var h = w * (kind ? 0.72 : 0.55);
    R.ellipse(cx, by - w * 0.02, w * 0.56, w * 0.07, 0, 'rgba(0,0,0,0.25)');
    R.rect(cx - w * 0.47, by - h * 0.3, w * 0.17, h * 0.3, C.tyre[f]);
    R.rect(cx + w * 0.3, by - h * 0.3, w * 0.17, h * 0.3, C.tyre[f]);
    if (kind) {                                       // 面包车：方头高车身
      R.rect(cx - w / 2, by - h * 0.92, w, h * 0.72, body);
      R.rect(cx - w * 0.36, by - h * 0.86, w * 0.72, h * 0.26, C.glass[f]);
    } else {
      R.rect(cx - w / 2, by - h * 0.72, w, h * 0.5, body);
      R.quad(cx - w * 0.38, by - h * 0.72, cx + w * 0.38, by - h * 0.72, cx + w * 0.3, by - h, cx - w * 0.3, by - h, dark);
      R.quad(cx - w * 0.31, by - h * 0.74, cx + w * 0.31, by - h * 0.74, cx + w * 0.25, by - h * 0.95, cx - w * 0.25, by - h * 0.95, C.glass[f]);
    }
    R.rect(cx - w * 0.45, by - h * 0.6, w * 0.17, h * 0.1, C.tail[f]);
    R.rect(cx + w * 0.28, by - h * 0.6, w * 0.17, h * 0.1, C.tail[f]);
    R.rect(cx - w * 0.12, by - h * 0.42, w * 0.24, h * 0.1, C.lane[f]);
  }

  function drawCoin(cx, by, px, f, t) {
    var r = 80 * px; if (r < 1) return;
    var cy = by - 190 * px, sx = Math.abs(Math.cos(t * 4)) * 0.85 + 0.15;
    if (r < 4) { R.rect(cx - r * sx, cy - r, r * sx * 2, r * 2, C.coin[f]); return; }
    R.ellipse(cx, cy, r * sx, r, 0, C.coin[f]);
    if (r > 10) R.ellipse(cx, cy, r * sx * 0.55, r * 0.6, 0, C.coin2[f]);
  }
  function drawCan(cx, by, px, f) {
    var w = 220 * px; if (w < 2) return;
    var h = w * 1.25;
    R.rect(cx - w / 2, by - h, w, h, C.can[f]);
    R.rect(cx - w * 0.3, by - h * 1.15, w * 0.35, h * 0.15, C.can[f]);
    if (w > 6) R.circle(cx, by - h * 0.45, w * 0.22, C.can2[f]);
  }

  // 远 → 近：路面一段 + 这一段上的景物 / 道具 / 车
  // 草条：整屏宽的水平条，用裁剪清屏铺（见文件头）。每条只铺到比它近的那段的顶上（CLIP），条与条不重叠，
  // 所以先后无所谓；整批在路面之前、远山之后（先 flush 掉已攒的天空与远山）。ox / oy 是撞车抖动的平移
  function grass(segs, base, ox, oy) {
    var N = segs.length, gl = R.gl, bw = gl.drawingBufferWidth, bh = gl.drawingBufferHeight, kk = bh / H, cur = null;
    R.flush();
    gl.enable(gl.SCISSOR_TEST);
    for (var k = 0; k < DRAW; k++) {
      if (!VIS[k]) continue;
      var s = segs[(base + k) % N], y2 = PY[k + 1], y1 = Math.min(PY[k], CLIP[k]), c;
      if (s.dark) c = C.grassD[FOGK[k]];
      // 浅色条平时不铺（底色就是）；但地平线附近那条雾化渐变是按平路排的，上坡时近处的段会落到这里，
      // 照渐变就成了一条条白边 —— 落在地平线以上或渐变带里的浅色条也铺上
      else if (y2 < HY + groundH) { c = C.grassL[FOGK[k]]; y1 = Math.min(y1, HY + groundH); }
      else continue;
      var t = Math.round((y2 + oy) * kk), b = Math.round((y1 + oy) * kk);
      if (b <= t) continue;
      if (c !== cur) { gl.clearColor(c[0], c[1], c[2], 1); cur = c; }
      gl.scissor(0, bh - b, bw, b - t);
      gl.clear(gl.COLOR_BUFFER_BIT);
    }
    gl.disable(gl.SCISSOR_TEST);
  }
  function render(segs, base, clock, ox, oy) {
    var N = segs.length;
    grass(segs, base, ox || 0, oy || 0);
    for (var k = DRAW - 1; k >= 0; k--) {
      var s = segs[(base + k) % N], ks = k - s.i % RUMBLE;
      if (ks < NEAR) { if (VIS[k]) drawRoadSeg(s, k, k + 1); }
      else if (k === Math.min(ks + RUMBLE - 1, DRAW - 1)) {
        // 远处一组的最远一段（从远往近第一个碰到的）：整组一块画掉，再画这组里各段的东西
        var top = Math.min(ks + RUMBLE, DRAW), any = 0;
        for (var j = ks; j < top; j++) any |= VIS[j];
        if (any) drawRoadSeg(segs[(base + ks) % N], ks, top);
      }
      if (PZ[k] <= DEPTH * 2 || PS[k] <= 0) continue;
      var px = PS[k] * UX, f = FOGK[k];
      if (s.deco >= 0) drawDeco(s, k, PX[k] + s.decoX * PW[k], PY[k], px);
      if (s.coin >= 0 && !s.got) drawCoin(PX[k] + LANE_X[s.coin] * PW[k], PY[k], px, f, clock + s.i * 0.2);
      if (s.fuel >= 0 && s.fuelOn) drawCan(PX[k] + LANE_X[s.fuel] * PW[k], PY[k], px, f);
      for (var c = 0; c < s.cars.length; c++) {
        var car = s.cars[c], t = (car.z % SEG) / SEG;
        var cx = PX[k] + (PX[k + 1] - PX[k]) * t, cy = PY[k] + (PY[k + 1] - PY[k]) * t, cw = PW[k] + (PW[k + 1] - PW[k]) * t;
        drawCar(cx + car.x * cw, cy, CAR_W * cw / RW, car.color, f, car.kind);
      }
    }
  }

  // 天空 + 两层远山（图层录一次，横向视差平移；铺两份接缝相连）
  function background(skyX) {
    var p = C.p;
    // 大块纯色用裁剪清屏铺（清屏走快速路径，比画两个半屏三角形便宜得多：无头 CPU 后端 4x MSAA 下填充是大头）；
    // 渐变只画贴着地平线的两条窄带。必须在这一帧还没攒任何顶点时调（清屏立即生效，攒着的三角形要到 flush 才画）
    var gl = R.gl, bw = gl.drawingBufferWidth, bh = gl.drawingBufferHeight, k = bh / H;
    gl.enable(gl.SCISSOR_TEST);
    var sk = R.color(p.sky0), gr = C.grassL[0];
    gl.scissor(0, Math.round(bh - HY * 0.5 * k), bw, Math.round(HY * 0.5 * k) + 1);
    gl.clearColor(sk[0], sk[1], sk[2], 1); gl.clear(gl.COLOR_BUFFER_BIT);
    gl.scissor(0, 0, bw, Math.round((H - HY) * k) + 1);
    gl.clearColor(gr[0], gr[1], gr[2], 1); gl.clear(gl.COLOR_BUFFER_BIT);
    gl.disable(gl.SCISSOR_TEST);
    R.gradRect(0, HY * 0.5 - 1, W, HY * 0.5 + 3, [[0, p.sky0], [1, p.sky1]], true);
    R.gradRect(0, HY, W, groundH, groundStops, true);
    var o1 = ((skyX * 0.5) % W + W) % W, o2 = ((skyX) % W + W) % W;
    // 同一个图层摆两次（sig 不变不重录）：一份从 -o 开始，一份接在它右边
    for (var j = 0; j < 2; j++) R.layer('mount_far', palIdx, farFn, -o1 + j * W, 0);
    for (j = 0; j < 2; j++) R.layer('mount_near', palIdx, nearFn, -o2 + j * W, 0);
  }
  function farFn() { mountains(W, HY, 0.32, C.p.mount, 7); }
  function nearFn() { mountains(W, HY, 0.2, C.p.mount2, 11); }
  // 起伏的山脊：n 个峰，首尾高度一致（平铺不露缝）
  function mountains(w, base, hk, col, n) {
    var step = w / (n * 2), hmax = base * hk * 2;
    for (var i = 0; i < n * 2; i++) {
      var xa = i * step, xb = xa + step;
      var ha = hmax * (0.45 + 0.55 * Math.abs(Math.sin(i * 1.7 + n))) * (i % 2 ? 0.6 : 1);
      var hb = hmax * (0.45 + 0.55 * Math.abs(Math.sin((i + 1) % (n * 2) * 1.7 + n))) * ((i + 1) % 2 ? 0.6 : 1);
      R.quad(xa, base, xa, base - ha, xb, base - hb, xb, base, col);
    }
  }

  // 玩家的车：画在固定位置（相机跟着车），三个姿态（左转 / 直行 / 右转）各录一个精灵
  function player(cx, by, steer, brake, palette) {
    var pose = steer < -0.2 ? -1 : steer > 0.2 ? 1 : 0, w = CAR_W / CAMH * UX;
    R.sprite('player' + pose + (brake ? 'b' : ''), palette, function () { playerShape(w, pose, brake); }, cx, by, 1, 1);
  }
  function playerShape(w, pose, brake) {
    var h = w * 0.52, sk = pose * w * 0.05;
    R.ellipse(0, -2, w * 0.58, w * 0.08, 0, 'rgba(0,0,0,0.3)');
    R.rrect(-w * 0.49, -h * 0.34, w * 0.18, h * 0.34, 3, '#1a1a1a');
    R.rrect(w * 0.31, -h * 0.34, w * 0.18, h * 0.34, 3, '#1a1a1a');
    R.rrect(-w / 2, -h * 0.76, w, h * 0.54, 6, '#e53935');
    R.quad(-w * 0.36 + sk, -h * 1.02, w * 0.36 + sk, -h * 1.02, w * 0.42, -h * 0.74, -w * 0.42, -h * 0.74, '#b71c1c');
    R.quad(-w * 0.3 + sk, -h * 0.98, w * 0.3 + sk, -h * 0.98, w * 0.35, -h * 0.78, -w * 0.35, -h * 0.78, '#37474f');
    R.rect(-w * 0.5, -h * 1.08 , w, h * 0.08, '#212121');             // 尾翼
    R.rect(-w * 0.44, -h * 0.66, w * 0.2, h * 0.12, brake ? '#ff5252' : '#b71c1c');
    R.rect(w * 0.24, -h * 0.66, w * 0.2, h * 0.12, brake ? '#ff5252' : '#b71c1c');
    R.rect(-w * 0.5, -h * 0.5, w, h * 0.05, '#ffffff');                // 腰线
    R.rect(-w * 0.13, -h * 0.4, w * 0.26, h * 0.12, '#fafafa');        // 车牌
    R.rect(-w * 0.3, -h * 0.26, w * 0.1, h * 0.06, '#424242');         // 双排气
    R.rect(w * 0.2, -h * 0.26, w * 0.1, h * 0.06, '#424242');
  }

  return {
    clearColor: function () { return C.grassL[FOG_LV - 1]; },
    setPalette: setPalette, project: project, render: render, background: background, player: player,
    palettes: PALETTES, HY: HY, UY: UY,
    playerY: function () { return HY + UY; }
  };
}

module.exports = { make: make, DRAW: DRAW, CAMH: CAMH, DEPTH: DEPTH, PLAYER_Z: PLAYER_Z, CAR_W: CAR_W };
