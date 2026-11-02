// 易充电：离线演示数据（电站、终端、电价时段、车型、兑换码）。会变的状态（余额、订单、车辆、券、发票）在 store.js
// 分时电价（元/度，电费 + 服务费）
const PERIODS = [
  { name: '谷', range: '00:00-08:00', elec: 0.32, service: 0.40, color: '#19BE6B' },
  { name: '平', range: '08:00-10:00', elec: 0.68, service: 0.60, color: '#2D8CF0' },
  { name: '峰', range: '10:00-15:00', elec: 1.02, service: 0.80, color: '#FF7A1A' },
  { name: '平', range: '15:00-18:00', elec: 0.68, service: 0.60, color: '#2D8CF0' },
  { name: '峰', range: '18:00-21:00', elec: 1.02, service: 0.80, color: '#FF7A1A' },
  { name: '平', range: '21:00-24:00', elec: 0.68, service: 0.60, color: '#2D8CF0' }
];
function terminals(prefix, fast, slow, busy, fault) {
  const list = [];
  for (let i = 1; i <= fast + slow; i++) {
    const isFast = i <= fast;
    let status = 'idle';
    if (busy.indexOf(i) >= 0) status = 'busy';
    if (fault.indexOf(i) >= 0) status = 'fault';
    list.push({
      no: prefix + '-' + (i < 10 ? '0' + i : i),
      type: isFast ? '直流快充' : '交流慢充',
      power: isFast ? (i % 3 === 0 ? 160 : 120) : 7,
      status,
      soc: status === 'busy' ? 30 + (i * 17) % 60 : 0
    });
  }
  return list;
}
const stations = [
  { id: 's1', name: '星联充电·西溪银泰城站', addr: '西湖区五常大道 1 号西溪银泰城 B2 停车场', dist: 0.8, x: 58, y: 34, lat: 30.2771, lng: 120.0632, phone: '0571-8812 6601', price: 1.82, parking: '充电期间停车 2 小时免费', open: '24 小时', brand: '星联', score: 4.8, list: terminals('XX0128', 6, 4, [1, 3, 4, 8], [10]) },
  { id: 's2', name: '国网杭州文一西路充电站', addr: '余杭区文一西路 1818 号人工智能小镇东门', dist: 1.6, x: 26, y: 58, lat: 30.2836, lng: 120.0208, phone: '95598', price: 1.66, parking: '停车免费', open: '24 小时', brand: '国网', score: 4.7, list: terminals('GW2210', 8, 0, [2, 5], []) },
  { id: 's3', name: '特来电·海创园充电站', addr: '余杭区文一西路 998 号海创园 7 号楼旁', dist: 2.3, x: 74, y: 70, lat: 30.2795, lng: 120.0035, phone: '400-1300-001', price: 1.75, parking: '停车 5 元/小时', open: '06:00-24:00', brand: '特来电', score: 4.6, list: terminals('TL0907', 4, 6, [1, 2, 3, 4, 7], [9]) },
  { id: 's4', name: '小桔充电·五常地铁站 P+R', addr: '余杭区五常大道与荆长大道交叉口 P+R 停车场', dist: 3.1, x: 40, y: 20, lat: 30.2902, lng: 120.0517, phone: '400-0000-999', price: 1.58, parking: '凭充电订单停车 3 小时免费', open: '24 小时', brand: '小桔', score: 4.5, list: terminals('XJ3301', 10, 0, [1, 2, 6, 9], [4]) },
  { id: 's5', name: '蔚来超充·西溪湿地站', addr: '西湖区天目山路 518 号西溪湿地周家村入口', dist: 4.4, x: 84, y: 42, lat: 30.2688, lng: 120.0755, phone: '400-999-6699', price: 1.98, parking: '停车免费', open: '24 小时', brand: '蔚来', score: 4.9, list: terminals('NI0518', 4, 2, [2], []) }
];
stations.forEach(s => { s.distText = s.dist.toFixed(1) + 'km'; });
// 历史订单（首次打开时写进 storage，之后和新订单一样可开票）
const ORDERS = [
  { id: 'EV20261022183012', station: 's2', terminal: 'GW2210-03', plate: '浙A·D88520', kwh: 42.6, minutes: 38, date: '10-22 18:30', period: '峰', elec: 1.02, service: 0.80, couponTitle: '', cut: 0, startSoc: 21, endSoc: 73 },
  { id: 'EV20261018081544', station: 's1', terminal: 'XX0128-02', plate: '浙A·D88520', kwh: 31.2, minutes: 29, date: '10-18 08:15', period: '平', elec: 0.68, service: 0.60, couponTitle: '新用户 5 元券', cut: 5, startSoc: 40, endSoc: 78 },
  { id: 'EV20261012221006', station: 's4', terminal: 'XJ3301-05', plate: '浙A·D88520', kwh: 55.8, minutes: 52, date: '10-12 22:10', period: '平', elec: 0.68, service: 0.60, couponTitle: '', cut: 0, startSoc: 12, endSoc: 80 },
  { id: 'EV20260928140220', station: 's3', terminal: 'TL0907-08', plate: '浙A·D88520', kwh: 18.4, minutes: 160, date: '09-28 14:02', period: '峰', elec: 1.02, service: 0.80, couponTitle: '', cut: 0, startSoc: 55, endSoc: 77, invoiced: true }
];
// 常见车型与电池容量（度），新增车辆时选
const MODELS = [
  { name: '比亚迪 海豹', battery: 82.5 },
  { name: '比亚迪 秦PLUS EV', battery: 57.6 },
  { name: '特斯拉 Model 3', battery: 60 },
  { name: '特斯拉 Model Y', battery: 78.4 },
  { name: '小鹏 P7', battery: 86.2 },
  { name: '蔚来 ET5', battery: 75 },
  { name: '极氪 001', battery: 100 },
  { name: '小米 SU7', battery: 73.6 },
  { name: '其他车型', battery: 60 }
];
// 兑换码 → 券（每个码只能兑一次）
const CODES = {
  EV2026: { title: '满 60 减 10 元', kind: 'cash', amount: 10, min: 60 },
  GREEN5: { title: '服务费 5 折券', kind: 'service', rate: 0.5, min: 0 }
};
function getStation(id) { return stations.find(s => s.id === id) || stations[0]; }
function findTerminal(no) {
  const code = String(no || '').trim().toUpperCase();
  for (const s of stations) {
    const t = s.list.find(x => x.no === code);
    if (t) return { station: s, terminal: t };
  }
  return null;
}
// 当前时段：演示里固定按 14:00 的峰时价，和页面上「当前时段」一致
function currentPeriod() { return PERIODS[2]; }
module.exports = { PERIODS, stations, ORDERS, MODELS, CODES, getStation, findTerminal, currentPeriod };
