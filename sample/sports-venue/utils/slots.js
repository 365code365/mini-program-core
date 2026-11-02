// 场地 × 时段网格：每格 1 小时。已订 = 确定性散列（同一天每次打开一样）+ 本机订单；今天已过去的时段不可订。
// 高峰价：工作日 18:00 以后，周末 09:00 以后。
const F = require('./fmt.js');
const store = require('./store.js');

function hash(str) {
  let h = 2166136261;
  for (let i = 0; i < str.length; i++) { h ^= str.charCodeAt(i); h = Math.imul(h, 16777619) >>> 0; }
  return h;
}
function hours(v) {
  const p = v.open.split('-');
  const a = Number(p[0].split(':')[0]), b = Number(p[1].split(':')[0]);
  const list = [];
  for (let h = a; h < b; h++) list.push(h);
  return list;
}
function isPeak(dateKey, h) {
  const wd = F.fromKey(dateKey).getDay();
  return (wd === 0 || wd === 6) ? h >= 9 : h >= 18;
}
function priceAt(v, dateKey, h) { return isPeak(dateKey, h) ? v.peak : v.price; }

// 返回 { courts: ['1号场', ...], rows: [{ h, label, cells: [{ court, h, price, peak, state }] }] }
function grid(v, dateKey) {
  const booked = store.bookedCells(v.id, dateKey);
  const now = new Date();
  const today = F.dateKey(now) === dateKey;
  const courts = [];
  for (let c = 1; c <= v.courts; c++) courts.push(c + v.courtName);
  const rows = hours(v).map((h) => {
    const cells = [];
    for (let c = 1; c <= v.courts; c++) {
      let state = 'free';
      if (today && h <= now.getHours()) state = 'past';
      else if (booked[c + '-' + h]) state = 'mine';
      else if (hash(v.id + dateKey + c + ':' + h) % 100 < (h >= 18 ? 55 : 30)) state = 'sold';
      cells.push({ key: c + '-' + h, court: c, h, price: priceAt(v, dateKey, h), peak: isPeak(dateKey, h), state, on: false });
    }
    return { h, label: F.hh(h), cells };
  });
  return { courts, rows };
}

// 选中的格子 → 按场地合并连续时段：[{ court, courtLabel, start, end, hours, amount, text }]
function merge(v, picked) {
  const byCourt = {};
  picked.forEach((p) => { (byCourt[p.court] = byCourt[p.court] || []).push(p); });
  const segs = [];
  Object.keys(byCourt).map(Number).sort((a, b) => a - b).forEach((c) => {
    const list = byCourt[c].sort((a, b) => a.h - b.h);
    let cur = null;
    list.forEach((p) => {
      if (cur && cur.end === p.h) { cur.end += 1; cur.hours += 1; cur.amount += p.price; cur.peakHours += p.peak ? 1 : 0; }
      else { cur = { court: c, courtLabel: c + v.courtName, start: p.h, end: p.h + 1, hours: 1, amount: p.price, peakHours: p.peak ? 1 : 0 }; segs.push(cur); }
    });
  });
  segs.forEach((s) => { s.text = F.hh(s.start) + '-' + F.hh(s.end); });
  return segs;
}

module.exports = { grid, merge, isPeak, priceAt, hours };
