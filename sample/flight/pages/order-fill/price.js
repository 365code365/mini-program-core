// 订单价格：成人 = 票价 + 民航发展基金 + 燃油附加费；儿童 = 全价 5 折、免基金、燃油减半；婴儿 = 全价 1 折、免税费。
// 保险按人按航段计。和国内航司的儿童 / 婴儿票规则一致。
const F = require('../../utils/flights');

const INS = { acc: { name: '航空意外险', price: 30, desc: '最高保额 300 万，覆盖登机到离机全程' }, delay: { name: '航班延误险', price: 20, desc: '延误 2 小时起赔 ¥300，取消赔 ¥200' } };
const r10 = v => Math.round(v / 10) * 10;

function perPax(leg, age) {
  const t = F.taxes(leg.flight.dist), full = leg.flight.full || leg.cabin.price;
  if (age === '儿童') return { ticket: Math.min(leg.cabin.price, r10(full * 0.5)), tax: t.fuel / 2 };
  if (age === '婴儿') return { ticket: r10(full * 0.1), tax: 0 };
  return { ticket: leg.cabin.price, tax: t.fund + t.fuel };
}

function calc(legs, pax, ins) {
  const rows = [];
  let total = 0;
  legs.forEach((leg, li) => {
    const groups = {};
    pax.forEach(p => { (groups[p.age] = groups[p.age] || []).push(p); });
    Object.keys(groups).forEach(age => {
      const n = groups[age].length, x = perPax(leg, age);
      rows.push({ k: li + age + 't', label: (legs.length > 1 ? (li ? '返程' : '去程') : '') + age + '票价', value: x.ticket, n });
      if (x.tax) rows.push({ k: li + age + 'x', label: (legs.length > 1 ? (li ? '返程' : '去程') : '') + age + '基金/燃油', value: x.tax, n });
      total += (x.ticket + x.tax) * n;
    });
  });
  ['acc', 'delay'].forEach(k => {
    if (!ins[k] || !pax.length) return;
    const n = pax.length * legs.length;
    rows.push({ k: 'ins' + k, label: INS[k].name, value: INS[k].price, n });
    total += INS[k].price * n;
  });
  return { rows, total };
}

module.exports = { INS, calc };
