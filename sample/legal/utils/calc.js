// 法律计算器的纯函数。依据：《工伤保险条例》第三十五~三十七条、《劳动合同法》第四十、四十六、四十七、八十七条、
// 《诉讼费用交纳办法》第十三条。各地另有规定的（如一次性工伤医疗补助金）不在这里算，结果页提示。

// 一次性伤残补助金：1~10 级对应的本人工资月数
const INJURY_MONTHS = [27, 25, 23, 21, 18, 16, 13, 11, 9, 7];

function round2(n) { return Math.round(n * 100) / 100; }

function injury(wage, level, offMonths) {
  const m = INJURY_MONTHS[level - 1];
  const subsidy = wage * m;
  const off = wage * offMonths;
  // 1~4 级保留劳动关系、按月领伤残津贴：90%、85%、80%、75%
  const pension = level <= 4 ? wage * [0.9, 0.85, 0.8, 0.75][level - 1] : 0;
  return { months: m, subsidy: round2(subsidy), off: round2(off), pension: round2(pension), total: round2(subsidy + off) };
}

// 工作年限 N：满一年算 1；不满一年的部分 ≥6 个月算 1，<6 个月算 0.5
function serviceYears(start, end) {
  const s = new Date(start.replace(/-/g, '/')), e = new Date(end.replace(/-/g, '/'));
  if (!(e > s)) return { years: 0, months: 0, n: 0 };
  let months = (e.getFullYear() - s.getFullYear()) * 12 + (e.getMonth() - s.getMonth());
  if (e.getDate() < s.getDate()) months -= 1;
  const full = Math.floor(months / 12), rest = months % 12;
  const n = full + (rest >= 6 ? 1 : (rest > 0 || months === 0 ? 0.5 : 0));
  return { years: full, months: rest, n };
}

function severance(o) {
  const y = serviceYears(o.start, o.end);
  const cap = o.avg * 3;
  const capped = o.wage > cap;
  const base = capped ? cap : o.wage;
  const n = capped ? Math.min(y.n, 12) : y.n;
  const nPay = base * n;
  // 违法解除：赔偿金 = 经济补偿 × 2，且不再另付代通知金
  const illegal = o.type === 'illegal';
  const notice = !illegal && !o.notified ? o.wage : 0;
  const total = illegal ? nPay * 2 : nPay + notice;
  return { y, n, base: round2(base), capped, nPay: round2(nPay), notice: round2(notice), total: round2(total), illegal };
}

// 财产案件受理费（分段累计）
const TIERS = [
  [10000, 0, 50], [100000, 0.025], [200000, 0.02], [500000, 0.015], [1000000, 0.01],
  [2000000, 0.009], [5000000, 0.008], [10000000, 0.007], [20000000, 0.006], [Infinity, 0.005]
];
function propertyFee(amount) {
  if (amount <= 10000) return 50;
  let fee = 50, prev = 10000;
  for (let i = 1; i < TIERS.length && amount > prev; i++) {
    const top = TIERS[i][0];
    fee += (Math.min(amount, top) - prev) * TIERS[i][1];
    prev = top;
  }
  return fee;
}
function courtFee(kind, amount, simple) {
  let fee;
  if (kind === 'labor') fee = 10;
  else if (kind === 'divorce') fee = 300 + (amount > 200000 ? (amount - 200000) * 0.005 : 0);
  else fee = propertyFee(amount);
  const final = simple ? fee / 2 : fee;
  return { fee: round2(fee), final: round2(final) };
}

module.exports = { INJURY_MONTHS, injury, serviceYears, severance, courtFee };
