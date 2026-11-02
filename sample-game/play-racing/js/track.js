// 赛道数据：一圈 N 段，每段长 SEG（世界单位）。弯道 = 每段的横向偏移增量 curve（投影时累加成弯）；
// 坡 = 段起点 / 终点的高度 y1 / y2。路面半宽 RW，三条车道中心在 -2/3、0、2/3（以半宽为 1 的归一化横坐标）。
// 赛道形状与路边景物用固定种子生成：每次打开都是同一条路，记得住弯；金币 / 油桶 / 车流每圈重新撒。
var SEG = 200, RW = 1200, RUMBLE = 3;
var LANE_X = [-2 / 3, 0, 2 / 3];
var LEN = { S: 25, M: 50, L: 100 }, CURVE = { E: 2, M: 4, H: 6 }, HILL = { L: 20, M: 40, H: 60 };

function easeIn(a, b, p) { return a + (b - a) * p * p; }
function easeInOut(a, b, p) { return a + (b - a) * (0.5 - Math.cos(p * Math.PI) / 2); }
function lcg(seed) {
  var s = seed >>> 0;
  return function () { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; };
}

function build() {
  var segs = [];
  function lastY() { return segs.length ? segs[segs.length - 1].y2 : 0; }
  function add(curve, y) {
    var n = segs.length;
    segs.push({
      i: n, curve: curve, y1: lastY(), y2: y, dark: Math.floor(n / RUMBLE) % 2,
      boost: -1, coin: -1, got: false, fuel: -1, fuelOn: false,
      deco: -1, decoX: 0, decoS: 1, cars: []
    });
  }
  // 进弯 / 保持 / 出弯三段：曲率缓入缓出，高度整段 easeInOut（坡顶坡底是圆的，不是折线）
  function road(enter, hold, leave, curve, height) {
    var y0 = lastY(), y1 = y0 + (height || 0) * SEG, total = enter + hold + leave, n;
    for (n = 0; n < enter; n++) add(easeIn(0, curve, n / enter), easeInOut(y0, y1, (n + 1) / total));
    for (n = 0; n < hold; n++) add(curve, easeInOut(y0, y1, (enter + n + 1) / total));
    for (n = 0; n < leave; n++) add(easeInOut(curve, 0, n / leave), easeInOut(y0, y1, (enter + hold + n + 1) / total));
  }
  var L = LEN, C = CURVE, H = HILL;
  road(L.L, L.L, L.S, 0, 0);                 // 起跑直道
  road(L.M, L.M, L.M, 0, H.L);
  road(L.M, L.M, L.M, C.M, 0);
  road(L.M, L.M, L.M, -C.E, H.M);
  road(L.S, L.S, L.S, 0, -H.L);
  road(L.M, L.S, L.M, C.H, 0);               // 急右弯
  road(L.S, L.M, L.S, 0, H.H);               // 大坡
  road(L.M, L.M, L.M, -C.M, -H.M);
  road(L.S, L.S, L.S, C.E, H.L);             // 连续 S 弯
  road(L.S, L.S, L.S, -C.M, -H.L);
  road(L.S, L.S, L.S, C.M, 0);
  road(L.M, L.L, L.M, -C.E, H.M);
  road(L.M, L.M, L.M, C.H, -H.L);
  road(L.L, L.M, L.L, 0, -lastY() / SEG);    // 回到起点高度：一圈首尾接得上

  var r = lcg(20261102);
  for (var i = 0; i < segs.length; i++) {
    var s = segs[i];
    // 路边每三段一处景物，左右交替：0 松树、1 圆树、2 灌木、3 路灯（再密，无头 CPU 后端上树的三角形就比路面还贵）
    if (i % 3 === 0 && i > 6) {
      var side = (i / 3) % 2 ? 1 : -1, k = r();
      s.deco = k < 0.42 ? 0 : k < 0.72 ? 1 : k < 0.9 ? 2 : 3;
      s.decoX = side * (s.deco === 3 ? 1.25 : 1.4 + r() * 1.4);
      s.decoS = 0.8 + r() * 0.5;
    }
  }
  return segs;
}

// 每圈重撒道具：金币一排 6 枚、加速带 4 段、油桶一个；越往后油桶出现得越少（lap 从 0 起）
function scatter(segs, lap) {
  var N = segs.length, i, lane;
  for (i = 0; i < N; i++) { var s = segs[i]; s.coin = -1; s.got = false; s.boost = -1; s.fuel = -1; s.fuelOn = false; }
  for (i = 60; i + 12 < N; i += 38 + Math.floor(Math.random() * 16)) {
    lane = Math.floor(Math.random() * 3);
    for (var c = 0; c < 6; c++) segs[i + c * 2].coin = lane;
  }
  for (i = 110; i + 4 < N; i += 120 + Math.floor(Math.random() * 40)) {
    lane = Math.floor(Math.random() * 3);
    for (var b = 0; b < 4; b++) { segs[i + b].boost = lane; segs[i + b].coin = -1; }
  }
  var pFuel = Math.max(0.35, 0.95 - lap * 0.12);
  for (i = 190; i < N; i += 230) {
    if (Math.random() > pFuel) continue;
    lane = Math.floor(Math.random() * 3);
    segs[i].fuel = lane; segs[i].fuelOn = true; segs[i].coin = -1;
  }
}

// 车流：都比玩家最高速慢，各走各的车道；偶尔慢慢并到相邻车道
function makeCars(n, trackLen, maxSpeed) {
  var cars = [];
  for (var i = 0; i < n; i++) cars.push(newCar(trackLen, maxSpeed, i / n));
  return cars;
}
function newCar(trackLen, maxSpeed, frac) {
  var lane = Math.floor(Math.random() * 3);
  return {
    z: ((frac === undefined ? Math.random() : frac) * 0.85 + 0.12) * trackLen,
    lane: lane, x: LANE_X[lane], speed: maxSpeed * (0.28 + Math.random() * 0.3),
    color: Math.floor(Math.random() * 5), kind: Math.random() < 0.25 ? 1 : 0, think: 2 + Math.random() * 6
  };
}

module.exports = { SEG: SEG, RW: RW, LANE_X: LANE_X, build: build, scatter: scatter, makeCars: makeCars, newCar: newCar };
