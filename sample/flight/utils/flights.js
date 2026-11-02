// 航班数据：按「航线 + 日期」做种子确定性生成，同一天同一航线反复进出结果一致。航司是虚构的，机场三字码真实。
const { getCity, distance, CITIES } = require('./cities');
const D = require('./date');

const AIRLINES = {
  YY: { name: '云翼航空', logo: '/images/airline/YY.png' },
  DY: { name: '东岳航空', logo: '/images/airline/DY.png' },
  HC: { name: '海川航空', logo: '/images/airline/HC.png' },
  NX: { name: '南星航空', logo: '/images/airline/NX.png' },
  BC: { name: '北辰航空', logo: '/images/airline/BC.png' }
};
const AIRLINE_CODES = Object.keys(AIRLINES);

// 退改时间窗与各运价的费率（与国内航司公布的退改规则同结构）
const WINDOWS = ['起飞前7天(含)以上', '起飞前7天至48小时', '起飞前48小时至4小时', '起飞前4小时内及起飞后'];
const WINDOW_HOURS = [168, 48, 4, -Infinity];
const RULES = {
  special: { refund: [0.2, 0.3, 0.5, 0.7], change: [0.1, 0.2, 0.3, 0.5], endorse: '不得签转' },
  standard: { refund: [0.1, 0.15, 0.3, 0.5], change: [0.05, 0.1, 0.2, 0.3], endorse: '不得签转' },
  flex: { refund: [0, 0.05, 0.1, 0.2], change: [0, 0, 0.05, 0.1], endorse: '可签转' },
  biz: { refund: [0.05, 0.1, 0.15, 0.2], change: [0, 0, 0.05, 0.1], endorse: '可签转' }
};

function hash(s) { let h = 2166136261; for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); } return h >>> 0; }
function rng(seed) {
  let a = seed;
  return function () { a |= 0; a = a + 0x6D2B79F5 | 0; let t = Math.imul(a ^ a >>> 15, 1 | a); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; };
}
const r10 = v => Math.round(v / 10) * 10;

function fullPrice(dist) { return r10(dist * 0.78 + 260); }
function taxes(dist) { return { fund: 50, fuel: dist >= 800 ? 40 : 20 }; }

// 某天这条航线的最低经济舱价：越临近越贵、周末贵一点
function lowPrice(fromCode, toCode, date) {
  const dist = distance(getCity(fromCode), getCity(toCode));
  const full = fullPrice(dist);
  const r = rng(hash(fromCode + toCode + date))();
  const ahead = D.diffDays(D.today(), date);
  const wd = D.parse(date).getDay();
  let f = 0.28 + r * 0.42 + (wd === 5 || wd === 0 ? 0.08 : 0) + (ahead <= 2 ? 0.18 : ahead <= 6 ? 0.08 : 0);
  return Math.max(200, r10(full * Math.min(f, 0.92)));
}

function ruleRows(key, price) {
  const r = RULES[key];
  return WINDOWS.map((w, i) => ({ win: w, refund: Math.round(price * r.refund[i]), change: Math.round(price * r.change[i]) }));
}
function fee(key, price, kind, hoursBefore) {
  const i = WINDOW_HOURS.findIndex(h => hoursBefore >= h);
  return Math.round(price * RULES[key][kind][i]);
}

function genFlights(fromCode, toCode, date) {
  const from = getCity(fromCode), to = getCity(toCode);
  const dist = distance(from, to);
  const full = fullPrice(dist), low = lowPrice(fromCode, toCode, date);
  const rand = rng(hash(fromCode + '>' + toCode + '@' + date));
  const n = 7 + Math.floor(rand() * 5);
  const cheapest = Math.floor(rand() * n);
  const list = [];
  for (let i = 0; i < n; i++) {
    const al = AIRLINE_CODES[Math.floor(rand() * AIRLINE_CODES.length)];
    const dep = Math.round((390 + i * (960 / n) + rand() * 40) / 5) * 5;
    const stop = dist > 1500 && rand() < 0.18 ? CITIES[5 + Math.floor(rand() * 10)] : null;
    const stopCity = stop && stop.code !== fromCode && stop.code !== toCode ? stop : null;
    const duration = Math.round((dist / 760 * 60 + 40 + rand() * 25 + (stopCity ? 70 : 0)) / 5) * 5;
    const wide = dist > 1800 && rand() < 0.5;
    const aircraft = wide ? (rand() < 0.5 ? '空客A330-300' : '波音787-9') : ['空客A320neo', '空客A321', '波音737-8', '国产C919'][Math.floor(rand() * 4)];
    const depAp = from.airports[Math.floor(rand() * from.airports.length)];
    const arrAp = to.airports[Math.floor(rand() * to.airports.length)];
    const base = i === cheapest ? low : Math.max(low, Math.min(r10(full * 0.95), r10(low * (1.02 + rand() * 0.45))));
    const seats = 2 + Math.floor(rand() * 8);
    const econ = [
      { id: 'Y1', cls: 'Y', name: '特价经济舱', rule: 'special', price: base, bag: '免费托运 20kg', seats: Math.min(seats, 5) },
      { id: 'Y2', cls: 'Y', name: '标准经济舱', rule: 'standard', price: Math.max(base + 30, Math.min(r10(full * 0.96), r10(base * 1.15))), bag: '免费托运 20kg · 含餐', seats: 9 },
      { id: 'Y3', cls: 'Y', name: '经济舱全价', rule: 'flex', price: full, bag: '免费托运 20kg · 优先值机', seats: 9 }
    ];
    const biz = [{ id: 'C1', cls: 'C', name: '公务舱', rule: 'biz', price: r10(full * 2.3), bag: '免费托运 30kg · 贵宾休息室', seats: 4 }];
    if (wide) biz.push({ id: 'F1', cls: 'F', name: '头等舱', rule: 'biz', price: r10(full * 3.1), bag: '免费托运 40kg · 专属通道', seats: 2 });
    [...econ, ...biz].forEach(c => { c.discount = c.price >= full ? '全价' : (c.price / full * 10).toFixed(1) + '折'; });
    const no = al + (1000 + Math.floor(rand() * 8999));
    list.push({
      id: no + '-' + date, no,
      airline: al, airlineName: AIRLINES[al].name, logo: AIRLINES[al].logo,
      fromCode, toCode, fromName: from.name, toName: to.name, date,
      depAirport: depAp.name, depCode: depAp.code, depTerm: 'T' + (1 + Math.floor(rand() * 3)),
      arrAirport: arrAp.name, arrCode: arrAp.code, arrTerm: 'T' + (1 + Math.floor(rand() * 2)),
      dep, arr: dep + duration, duration, depText: D.hm(dep), arrText: D.hm(dep + duration),
      cross: dep + duration >= 1440 ? 1 : 0, durText: D.dur(duration),
      stop: stopCity ? stopCity.name : '', aircraft, meal: dist > 800 ? '有餐食' : '小食饮料',
      onTime: 76 + Math.floor(rand() * 21), dist, full, econ, biz,
      minPrice: base, minDiscount: econ[0].discount, seats: econ[0].seats
    });
  }
  return list;
}

function findFlight(fromCode, toCode, date, no) {
  return genFlights(fromCode, toCode, date).find(f => f.no === no) || null;
}

module.exports = { AIRLINES, AIRLINE_CODES, WINDOWS, RULES, ruleRows, fee, taxes, lowPrice, genFlights, findFlight, fullPrice };
