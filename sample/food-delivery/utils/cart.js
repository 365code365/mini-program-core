// 购物车与订单：都落在本地存储里，重启小程序还在
const data = require('./data.js');

const CART_KEY = 'fd_cart';
const ORDER_KEY = 'fd_orders';

function round2(n) {
  return Math.round(n * 100) / 100;
}

function getCart() {
  const c = wx.getStorageSync(CART_KEY);
  return c && c.shopId ? c : { shopId: '', items: {} };
}

function saveCart(c) {
  wx.setStorageSync(CART_KEY, c);
}

// 换店点餐时清掉上一家的购物车（与主流外卖一致：一单一店）
function change(shopId, dishId, delta) {
  let c = getCart();
  if (c.shopId !== shopId) c = { shopId, items: {} };
  const n = (c.items[dishId] || 0) + delta;
  if (n <= 0) delete c.items[dishId];
  else c.items[dishId] = n;
  saveCart(c);
  return c;
}

function clear() {
  saveCart({ shopId: '', items: {} });
}

// 汇总：明细、件数、商品金额
function summary(shopId) {
  const c = getCart();
  const list = [];
  let count = 0;
  let amount = 0;
  if (c.shopId === shopId) {
    Object.keys(c.items).forEach(id => {
      const d = data.findDish(shopId, id);
      if (!d) return;
      const qty = c.items[id];
      list.push({ id, name: d.name, price: d.price, qty, glyph: d.glyph, c1: d.c1, c2: d.c2,
        total: round2(d.price * qty) });
      count += qty;
      amount += d.price * qty;
    });
  }
  return { list, count, amount: round2(amount), items: c.shopId === shopId ? c.items : {} };
}

function pad(n) {
  return n < 10 ? '0' + n : '' + n;
}

function fmtTime(ts) {
  const d = new Date(ts);
  return `${d.getMonth() + 1}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function hm(ts) {
  const d = new Date(ts);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

// 历史订单示例（首次打开就有内容可看）
function seedOrders() {
  const now = Date.now();
  const day = 86400000;
  return [
    { id: 'BL' + (now - 720000).toString().slice(-10), shopId: 's1', status: 'delivering', createdAt: now - 720000,
      items: [{ name: '农家小炒肉套餐', qty: 1, price: 26.8 }, { name: '肥西老母鸡汤', qty: 1, price: 22 },
        { name: '卤蛋', qty: 2, price: 3 }],
      pay: 44.8, remark: '不要香菜', tableware: '1 份', address: data.addresses[0], eta: hm(now + 18 * 60000) },
    { id: 'BL' + (now - day).toString().slice(-10), shopId: 's4', status: 'done', createdAt: now - day,
      items: [{ name: '招牌酸菜鱼（单人份）', qty: 1, price: 39.9 }, { name: '米饭', qty: 2, price: 2 }],
      pay: 41.9, remark: '少辣', address: data.addresses[0] },
    { id: 'BL' + (now - 3 * day).toString().slice(-10), shopId: 's5', status: 'done', createdAt: now - 3 * day,
      items: [{ name: '杨枝甘露', qty: 2, price: 16 }], pay: 29, remark: '无', address: data.addresses[1] },
    { id: 'BL' + (now - 6 * day).toString().slice(-10), shopId: 's3', status: 'refund', createdAt: now - 6 * day,
      items: [{ name: '招牌腊汁肉夹馍', qty: 2, price: 14 }], pay: 25, remark: '无', address: data.addresses[0] }
  ];
}

function getOrders() {
  let list = wx.getStorageSync(ORDER_KEY);
  if (!list || !list.length) {
    list = seedOrders();
    wx.setStorageSync(ORDER_KEY, list);
  }
  return list;
}

function addOrder(o) {
  const list = getOrders();
  list.unshift(o);
  wx.setStorageSync(ORDER_KEY, list);
}

function updateOrder(id, patch) {
  const list = getOrders().map(o => (o.id === id ? Object.assign({}, o, patch) : o));
  wx.setStorageSync(ORDER_KEY, list);
}

module.exports = { getCart, change, clear, summary, round2, getOrders, addOrder, updateOrder, fmtTime, hm };
