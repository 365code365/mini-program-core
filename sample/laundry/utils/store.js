// 本地数据：购物篮、订单、优惠券、地址。全部走 storage，退出重进状态还在
const data = require('./data.js');

const K = { cart: 'ld_cart', orders: 'ld_orders', coupons: 'ld_coupons', addrs: 'ld_addrs', addrSel: 'ld_addr_sel' };

function get(key, def) {
  const v = wx.getStorageSync(key);
  return v === '' || v === undefined || v === null ? def : v;
}

function pad(n) { return n < 10 ? '0' + n : '' + n; }

function dateStr(d) {
  d = d || new Date();
  return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());
}

function timeStr(ts) {
  const d = new Date(ts);
  return dateStr(d) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes());
}

// ───── 购物篮 {itemId: 数量} ─────
function cart() { return get(K.cart, {}); }

function setQty(id, n) {
  const c = cart();
  if (n > 0) c[id] = n; else delete c[id];
  wx.setStorageSync(K.cart, c);
  return c;
}

function clearCart() { wx.removeStorageSync(K.cart); }

function cartSummary(c) {
  c = c || cart();
  const lines = Object.keys(c).map(id => {
    const it = data.item(id);
    return it ? { id, name: it.name, price: it.price, unit: it.unit, qty: c[id], sum: it.price * c[id] } : null;
  }).filter(Boolean);
  return {
    lines,
    count: lines.reduce((a, l) => a + l.qty, 0),
    total: lines.reduce((a, l) => a + l.sum, 0)
  };
}

// ───── 优惠券 ─────
// 首次打开发新人券；过期判断按今天的日期
function coupons() {
  let list = wx.getStorageSync(K.coupons);
  if (list === '' || list === undefined) {
    const y = new Date().getFullYear();
    list = [
      { id: 'cp1', title: '新客首单立减', amount: 10, min: 0, expire: y + '-12-31', scope: '全场通用', used: false },
      { id: 'cp2', title: '满 99 减 20', amount: 20, min: 99, expire: y + '-12-31', scope: '全场通用', used: false },
      { id: 'cp3', title: '满 199 减 40', amount: 40, min: 199, expire: y + '-12-31', scope: '全场通用', used: false },
      { id: 'cp4', title: '满 59 减 8', amount: 8, min: 59, expire: y + '-01-31', scope: '洗鞋专享', used: false }
    ];
    wx.setStorageSync(K.coupons, list);
  }
  const today = dateStr();
  return list.map(c => Object.assign({}, c, { state: c.used ? 'used' : (c.expire < today ? 'expired' : 'ok') }));
}

function saveCoupons(list) {
  wx.setStorageSync(K.coupons, list.map(c => ({ id: c.id, title: c.title, amount: c.amount, min: c.min, expire: c.expire, scope: c.scope, used: c.used })));
}

function useCoupon(id, used) {
  const list = coupons();
  const c = list.find(x => x.id === id);
  if (c) c.used = used;
  saveCoupons(list);
}

function addCoupon(c) {
  const list = coupons();
  if (list.find(x => x.id === c.id)) return false;
  list.unshift(Object.assign({ used: false }, c));
  saveCoupons(list);
  return true;
}

// ───── 地址 ─────
function addrs() { return get(K.addrs, []); }

function saveAddr(a) {
  const list = addrs();
  const i = list.findIndex(x => x.id === a.id);
  if (i >= 0) list[i] = a; else list.unshift(a);
  wx.setStorageSync(K.addrs, list);
  return list;
}

function removeAddr(id) {
  const list = addrs().filter(x => x.id !== id);
  wx.setStorageSync(K.addrs, list);
  if (selectedAddrId() === id) wx.removeStorageSync(K.addrSel);
  return list;
}

function selectedAddrId() { return get(K.addrSel, ''); }
function selectAddr(id) { wx.setStorageSync(K.addrSel, id); }

function selectedAddr() {
  const list = addrs();
  return list.find(x => x.id === selectedAddrId()) || list[0] || null;
}

// ───── 订单 ─────
function orders() { return get(K.orders, []); }

function order(id) { return orders().find(o => o.id === id) || null; }

function saveOrder(o) {
  const list = orders();
  const i = list.findIndex(x => x.id === o.id);
  if (i >= 0) list[i] = o; else list.unshift(o);
  wx.setStorageSync(K.orders, list);
  return o;
}

// 列表里的大状态：-1 已取消，其余看细分节点
function mainStatus(o) {
  if (o.cancelled) return -1;
  return data.STEPS[Math.min(o.step, data.STEPS.length - 1)].main;
}

function statusName(o) {
  const m = mainStatus(o);
  return m < 0 ? '已取消' : data.STATUS[m].name;
}

function newOrderId() {
  const d = new Date();
  return 'JY' + d.getFullYear() + pad(d.getMonth() + 1) + pad(d.getDate()) + pad(d.getHours()) + pad(d.getMinutes()) + String(d.getTime() % 1000 + 1000).slice(1);
}

// 洗衣袋上的袋码：4 位字母数字，扫码 / 手输都认
function newBagNo() {
  const cs = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
  let s = '';
  for (let i = 0; i < 4; i++) s += cs[Math.floor(Math.random() * cs.length)];
  return 'JY-' + s;
}

function clearAll() {
  Object.keys(K).forEach(k => wx.removeStorageSync(K[k]));
}

module.exports = {
  pad, dateStr, timeStr, cart, setQty, clearCart, cartSummary, coupons, useCoupon, addCoupon, saveCoupons,
  addrs, saveAddr, removeAddr, selectAddr, selectedAddr, selectedAddrId, orders, order, saveOrder, mainStatus, statusName,
  newOrderId, newBagNo, clearAll
};
