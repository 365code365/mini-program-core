// 保费测算：按险种的定价模型算年交保费（元）。费率参考市面同类产品的公开费率表，取整到分。
function band(table, age) {
  for (let i = 0; i < table.length; i++) if (age <= table[i][0]) return table[i][1];
  return table[table.length - 1][1];
}
const MEDICAL = [[4, 498], [10, 268], [17, 186], [25, 168], [30, 218], [35, 286], [40, 362], [45, 538], [50, 728], [55, 1086], [60, 1468], [65, 1988]];
const CHILD = [[0, 398], [3, 338], [6, 268], [12, 218], [17, 198]];
const OUTPATIENT = [[17, 368], [40, 299], [60, 459]];
const TRAVEL_DAYS = { '7天': 45, '15天': 78, '30天': 128, '90天': 298 };

function round2(n) { return Math.round(n * 100) / 100; }

// opts: { age, amount, term, gender: 'M' | 'F' }
function calc(p, opts) {
  const age = Number(opts.age);
  const amount = Number(opts.amount);
  const female = opts.gender === 'F';
  if (!(age >= p.ages[0] && age <= p.ages[1])) return 0;
  switch (p.model) {
    case 'medical': {
      const f = amount >= 6000000 ? 1 : amount >= 4000000 ? 0.92 : 0.82;
      return round2(band(MEDICAL, age) * f * (female ? 0.96 : 1));
    }
    case 'child': return round2(band(CHILD, age) * (amount >= 4000000 ? 1 : 0.9));
    case 'outpatient': return band(OUTPATIENT, age);
    case 'critical': {
      const base = opts.term === '保终身' ? 168 : 108;
      return round2(amount / 10000 * base * Math.exp(0.042 * (age - 30)) * (female ? 0.93 : 1));
    }
    case 'cancer': return round2(amount / 10000 * 18 * Math.exp(0.06 * (age - 50)) * (female ? 0.9 : 1));
    case 'life': {
      const base = opts.term === '保至70岁' ? 11.2 : 6.8;
      return round2(amount / 10000 * base * Math.exp(0.078 * (age - 30)) * (female ? 0.62 : 1));
    }
    case 'accident': {
      const price = amount >= 2000000 ? 299 : amount >= 1000000 ? 169 : 99;
      return round2(price * (age <= 50 ? 1 : age <= 60 ? 1.5 : 2.2));
    }
    case 'travel': {
      const af = amount >= 1000000 ? 1.8 : amount >= 500000 ? 1.3 : 1;
      return round2((TRAVEL_DAYS[opts.term] || 78) * af * (age > 70 ? 2 : 1));
    }
  }
  return 0;
}

// 产品卡上的「xx 元起」：可投年龄里最便宜的那一档（默认保额 / 保障期）
function priceFrom(p) {
  let min = Infinity;
  for (let a = p.ages[0]; a <= p.ages[1]; a++) {
    const v = calc(p, { age: a, amount: p.amounts[0], term: p.terms[0], gender: 'F' });
    if (v > 0 && v < min) min = v;
  }
  return min === Infinity ? 0 : min;
}

// 折线图用：可投年龄内每隔 step 岁一个点
function series(p, opts) {
  const pts = [];
  const span = p.ages[1] - p.ages[0];
  const step = span > 40 ? 5 : span > 20 ? 3 : span > 10 ? 2 : 1;
  for (let a = p.ages[0]; a <= p.ages[1]; a += step) pts.push({ age: a, v: calc(p, Object.assign({}, opts, { age: a })) });
  if (pts[pts.length - 1].age !== p.ages[1]) pts.push({ age: p.ages[1], v: calc(p, Object.assign({}, opts, { age: p.ages[1] })) });
  return pts;
}

function periodText(p) { return p.model === 'travel' ? '每次' : (p.terms.length === 1 && p.terms[0] === '1年' ? '每年' : '年交'); }

module.exports = { calc, priceFrom, series, periodText };
