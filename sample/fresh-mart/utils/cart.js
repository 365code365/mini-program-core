// 购物车：[{ id, spec, qty, checked }]，同商品同规格合并
const data = require('./data.js');

const KEY = 'fm_cart';

function round2(n) {
  return Math.round(n * 100) / 100;
}

function get() {
  return wx.getStorageSync(KEY) || [];
}

function save(list) {
  wx.setStorageSync(KEY, list);
}

// 单价：规格加价 + 秒杀价
function unitPrice(id, spec) {
  const gd = data.findGoods(id);
  const f = data.flashOf(id);
  const base = f ? f.price : gd.price;
  const s = (gd.specs || []).find(x => x[0] === spec);
  return round2(base + (s ? s[1] : 0));
}

function add(id, qty, spec) {
  const gd = data.findGoods(id);
  const sp = spec || (gd.specs && gd.specs[0] ? gd.specs[0][0] : gd.spec);
  const list = get();
  const hit = list.find(x => x.id === id && x.spec === sp);
  if (hit) {
    hit.qty += qty || 1;
    hit.checked = true;
  } else {
    list.unshift({ id, spec: sp, qty: qty || 1, checked: true });
  }
  save(list);
  return count(list);
}

function count(list) {
  return (list || get()).reduce((n, x) => n + x.qty, 0);
}

module.exports = { get, save, add, count, unitPrice, round2 };
