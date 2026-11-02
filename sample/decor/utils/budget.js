// 预算计算：杭州 2024 年家装市场的常见报价口径（每平方米按建筑面积计）
const GRADES = [
  { key: 'eco', name: '经济', desc: '国产一线品牌，满足基本功能', labor: 380, aux: 220, main: 520, custom: 480, soft: 260, smart: 8000, design: 0 },
  { key: 'mid', name: '品质', desc: '合资品牌，工艺与环保升级', labor: 460, aux: 300, main: 860, custom: 720, soft: 450, smart: 18000, design: 80 },
  { key: 'high', name: '高端', desc: '进口主材，定制化程度高', labor: 560, aux: 380, main: 1500, custom: 1200, soft: 900, smart: 42000, design: 220 }
];

const OPTIONS = [
  { key: 'custom', name: '全屋定制', desc: '衣柜、橱柜、玄关柜、书柜' },
  { key: 'soft', name: '软装家具', desc: '沙发、床、窗帘、灯具' },
  { key: 'smart', name: '智能家居', desc: '智能门锁、灯光、窗帘电机' },
  { key: 'heat', name: '地暖', desc: '水地暖，按 75% 地面面积铺设' }
];

const COLORS = ['#3a7d6c', '#c8894f', '#7fa89b', '#e3b26b', '#5e7fa6', '#d77a61', '#a58bc6', '#9aa3a8'];

// 返回 { total(元), items: [{name, value, percent, color}] }
function calc(area, gradeIdx, on) {
  const g = GRADES[gradeIdx] || GRADES[1];
  const items = [
    { name: '人工费', value: area * g.labor },
    { name: '辅材', value: area * g.aux },
    { name: '主材', value: area * g.main }
  ];
  if (on.custom) items.push({ name: '全屋定制', value: area * g.custom });
  if (on.soft) items.push({ name: '软装家具', value: area * g.soft });
  if (on.smart) items.push({ name: '智能家居', value: g.smart + area * 30 });
  if (on.heat) items.push({ name: '地暖', value: area * 0.75 * 230 });
  if (g.design) items.push({ name: '设计费', value: area * g.design });
  const total = items.reduce((s, it) => s + it.value, 0);
  items.forEach((it, i) => {
    it.value = Math.round(it.value / 100) * 100;
    it.percent = Math.round(it.value / total * 1000) / 10;
    it.color = COLORS[i % COLORS.length];
    it.wan = (it.value / 10000).toFixed(2);
  });
  return { total: Math.round(total / 100) * 100, wan: (total / 10000).toFixed(1), perSqm: Math.round(total / area), items };
}

module.exports = { GRADES, OPTIONS, COLORS, calc };
