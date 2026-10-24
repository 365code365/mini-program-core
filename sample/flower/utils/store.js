// 购物车、结算单、订单：存在本地 storage
const data = require('./data.js');

function pad(v) { return v < 10 ? '0' + v : '' + v; }
function dayAfter(n) {
  const t = new Date();
  const d = new Date(t.getFullYear(), t.getMonth(), t.getDate() + n);
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());
}
function noOf(n, hm) { return 'HY' + dayAfter(n).replace(/-/g, '') + hm; }

const DEFAULT_CART = [
  { key: 'f4-11-mist', id: 'f4', stems: 11, pack: 'mist', date: dayAfter(1), slot: '15:00-18:00', card: '生日快乐，愿你永远热烈灿烂。', qty: 1, checked: true }
];
const DEFAULT_ORDERS = [
  { no: noOf(0, '100315'), status: 'shipping', time: dayAfter(0) + ' 10:03', date: dayAfter(0), slot: '18:00-21:00', receiver: '苏晚晴', phone: '139****2086',
    addr: '上海市徐汇区漕溪北路 88 号 圣爱大厦 12 楼', anonymous: true, total: 259,
    items: [{ id: 'f1', stems: 19, pack: 'mist', qty: 1 }] },
  { no: noOf(-12, '091142'), status: 'done', time: dayAfter(-12) + ' 09:11', date: dayAfter(-11), slot: '09:00-12:00', receiver: '王秀兰', phone: '137****5521',
    addr: '杭州市西湖区文三路 259 号 昌地火炬大厦', anonymous: false, total: 209,
    items: [{ id: 'f6', stems: 11, pack: 'mist', qty: 1 }] }
];

function getCart() {
  const v = wx.getStorageSync('fl_cart');
  return Array.isArray(v) ? v : JSON.parse(JSON.stringify(DEFAULT_CART));
}
function saveCart(v) { wx.setStorageSync('fl_cart', v); }
function cartCount() { return getCart().reduce((s, x) => s + x.qty, 0); }
function addToCart(item) {
  const cart = getCart();
  const key = item.id + '-' + item.stems + '-' + item.pack;
  const hit = cart.find(x => x.key === key);
  if (hit) { hit.qty += item.qty; Object.assign(hit, { date: item.date, slot: item.slot, card: item.card }); }
  else cart.unshift(Object.assign({ key, checked: true }, item));
  saveCart(cart);
}

function getOrders() {
  const v = wx.getStorageSync('fl_orders');
  return Array.isArray(v) ? v : JSON.parse(JSON.stringify(DEFAULT_ORDERS));
}
function saveOrders(v) { wx.setStorageSync('fl_orders', v); }

// 把一行（id/stems/pack/qty）展开成带名称、图片、单价的视图数据
function expand(line) {
  const p = data.productOf(line.id);
  const unit = data.unitPrice(p, line.stems, line.pack);
  return Object.assign({}, line, { name: p.name, img: p.img, unit, sum: unit * line.qty,
    spec: data.stemOf(line.stems).name + ' · ' + data.packOf(line.pack).name });
}

module.exports = { getCart, saveCart, cartCount, addToCart, getOrders, saveOrders, expand, dayAfter };
