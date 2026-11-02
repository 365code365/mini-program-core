// 计费规则（杭州市区，按骑行时长计费；临时锁车期间照常计费）
const PRICING = {
  bike: {
    name: '青桔单车', start: 1.5, startMin: 15, unit: 1, unitMin: 15, speed: 3.4, // 米/秒，约 12 km/h
    rules: ['起步价 1.5 元，含 15 分钟', '超出后每 15 分钟 1 元，不足 15 分钟按 15 分钟计', '开锁后 2 分钟内结束骑行免费']
  },
  ebike: {
    name: '青桔电单车', start: 2.5, startMin: 15, unit: 2, unitMin: 15, speed: 5.0, // 约 18 km/h
    rules: ['起步价 2.5 元，含 15 分钟', '超出后每 15 分钟 2 元，不足 15 分钟按 15 分钟计', '开锁后 2 分钟内结束骑行免费', '骑行请佩戴头盔，限速 25 km/h']
  }
};

const FREE_SEC = 120;

const CARDS = [
  { id: 'b7', name: '单车 7 天畅骑卡', price: 5.9, days: 7, type: 'bike', desc: '每次骑行前 2 小时免费，不限次数' },
  { id: 'b30', name: '单车 30 天畅骑卡', price: 15, days: 30, type: 'bike', desc: '每次骑行前 2 小时免费，不限次数', hot: true },
  { id: 'e30', name: '电单车 30 天骑行卡', price: 39.9, days: 30, type: 'ebike', desc: '每次骑行抵扣起步价 2.5 元，每天限 3 次' }
];

function round2(n) { return Math.round(n * 100) / 100; }

// 按时长算费用明细
function calc(type, sec) {
  const p = PRICING[type] || PRICING.bike;
  if (sec <= FREE_SEC) return { start: 0, extra: 0, total: 0, free: true };
  const min = Math.ceil(sec / 60);
  const extraUnits = Math.max(0, Math.ceil((min - p.startMin) / p.unitMin));
  const extra = extraUnits * p.unit;
  return { start: p.start, extra, total: round2(p.start + extra), free: false };
}

// 骑行卡抵扣：单车卡免前 2 小时，电单车卡抵起步价
function discount(card, type, sec, total) {
  if (!card || card.type !== type || total <= 0) return 0;
  if (type === 'bike') {
    const covered = calc(type, Math.min(sec, 7200)).total;
    return round2(Math.min(total, covered));
  }
  return round2(Math.min(total, PRICING.ebike.start));
}

// 当前有效、适用该车型的卡
function validCard(cards, type, now) {
  return (cards || []).find(c => c.type === type && c.expireAt > now) || null;
}

function fmtDur(sec) {
  sec = Math.max(0, Math.floor(sec));
  const h = Math.floor(sec / 3600), m = Math.floor(sec % 3600 / 60), s = sec % 60;
  const p = n => String(n).padStart(2, '0');
  return (h ? p(h) + ':' : '') + p(m) + ':' + p(s);
}

function fmtMin(sec) {
  const m = Math.max(1, Math.ceil(sec / 60));
  return m >= 60 ? Math.floor(m / 60) + ' 小时 ' + (m % 60) + ' 分钟' : m + ' 分钟';
}

module.exports = { PRICING, CARDS, FREE_SEC, calc, discount, validCard, fmtDur, fmtMin, round2 };
