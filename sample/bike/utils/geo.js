// 地理工具：默认城市、附近车辆生成、距离、轨迹
// 车辆按「地图中心 + 网格种子」确定性生成：同一片区域刷新几次车还在原处，拖到别处再换一批，像真的车辆服务返回的那样
const DEFAULT_CENTER = { latitude: 30.274085, longitude: 120.155070, city: '杭州市', place: '武林广场' };

// 武林商圈附近真实存在的地点，用作行程的起终点名
const POIS = [
  { name: '武林广场', latitude: 30.274085, longitude: 120.155070 },
  { name: '杭州百货大楼', latitude: 30.272630, longitude: 120.158360 },
  { name: '西湖文化广场', latitude: 30.279460, longitude: 120.161610 },
  { name: '凤起路地铁站 B 口', latitude: 30.265420, longitude: 120.160580 },
  { name: '浙江展览馆', latitude: 30.275360, longitude: 120.154030 },
  { name: '杭州大厦', latitude: 30.276030, longitude: 120.158990 },
  { name: '中山北路公交站', latitude: 30.270760, longitude: 120.165390 },
  { name: '天水桥', latitude: 30.270120, longitude: 120.170300 },
  { name: '杭州市第一人民医院', latitude: 30.258730, longitude: 120.168820 },
  { name: '湖滨银泰 in77', latitude: 30.259760, longitude: 120.163980 },
  { name: '黄龙体育中心', latitude: 30.268270, longitude: 120.136280 },
  { name: '浙江大学玉泉校区', latitude: 30.264940, longitude: 120.124680 }
];

function rad(d) { return d * Math.PI / 180; }

// 两点距离（米）
function distance(a, b) {
  const R = 6378137;
  const dLat = rad(b.latitude - a.latitude);
  const dLng = rad(b.longitude - a.longitude);
  const s = Math.sin(dLat / 2) ** 2 + Math.cos(rad(a.latitude)) * Math.cos(rad(b.latitude)) * Math.sin(dLng / 2) ** 2;
  return Math.round(2 * R * Math.asin(Math.sqrt(s)));
}

// 往某个方向走 m 米
function offset(p, meters, headingDeg) {
  const h = rad(headingDeg);
  const dLat = (meters * Math.cos(h)) / 111320;
  const dLng = (meters * Math.sin(h)) / (111320 * Math.cos(rad(p.latitude)));
  return { latitude: +(p.latitude + dLat).toFixed(6), longitude: +(p.longitude + dLng).toFixed(6) };
}

// 线性同余随机数：给定种子永远出同一串
function rng(seed) {
  let s = seed >>> 0;
  return function () {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 4294967296;
  };
}

function nearestPoi(p) {
  let best = POIS[0], bd = Infinity;
  POIS.forEach(x => { const d = distance(p, x); if (d < bd) { bd = d; best = x; } });
  return best;
}

// 附近车辆：中心按约 300 米的网格取整做种子
function genBikes(center) {
  const gx = Math.round(center.latitude * 370), gy = Math.round(center.longitude * 320);
  const r = rng(gx * 7919 + gy * 104729);
  const n = 10 + Math.floor(r() * 7);
  const list = [];
  for (let i = 0; i < n; i++) {
    const ebike = r() < 0.35;
    const p = offset(center, 60 + r() * 560, r() * 360);
    let no = (ebike ? '8' : '1') + String(Math.floor(r() * 1e8)).padStart(8, '0');
    const battery = ebike ? 28 + Math.floor(r() * 70) : 0;
    list.push({
      id: i + 1, no, type: ebike ? 'ebike' : 'bike', battery,
      range: ebike ? Math.round(battery * 0.45) : 0,
      latitude: p.latitude, longitude: p.longitude,
      dist: distance(center, p)
    });
  }
  return list.sort((a, b) => a.dist - b.dist);
}

// 车号推出车型与电量（扫码 / 手输时没有车辆列表可查）
function bikeInfo(no) {
  const ebike = no.charAt(0) === '8';
  let sum = 0;
  for (let i = 0; i < no.length; i++) sum += Number(no.charAt(i)) * (i + 3);
  const battery = ebike ? 35 + (sum % 60) : 0;
  return { no, type: ebike ? 'ebike' : 'bike', battery, range: ebike ? Math.round(battery * 0.45) : 0 };
}

// 两点之间的骑行轨迹：沿直线插点，加一点横向抖动像真实道路
function track(a, b, seed) {
  const r = rng(seed || 7);
  const pts = [{ latitude: a.latitude, longitude: a.longitude }];
  const steps = 8;
  for (let i = 1; i < steps; i++) {
    const t = i / steps;
    const j = (r() - 0.5) * 0.0012;
    pts.push({
      latitude: +(a.latitude + (b.latitude - a.latitude) * t + j).toFixed(6),
      longitude: +(a.longitude + (b.longitude - a.longitude) * t - j).toFixed(6)
    });
  }
  pts.push({ latitude: b.latitude, longitude: b.longitude });
  return pts;
}

module.exports = { DEFAULT_CENTER, POIS, distance, offset, rng, nearestPoi, genBikes, bikeInfo, track };
