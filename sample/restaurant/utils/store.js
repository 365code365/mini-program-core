// 本地存储 + 购物车 / 订单的业务计算。价格一律用「分」做整数运算，展示时再转元。
const { DISHES, SIZES, EXTRAS } = require('./data.js');

const K = { table: 'rs_table', cart: 'rs_cart', orders: 'rs_orders', balance: 'rs_balance' };
const PAY_MINUTES = 15;
// 出餐节奏（演示用，真实食堂是分钟级）：付款后 8 秒进入制作中，20 秒可取餐
const STAGE_MS = [0, 8000, 20000];

function get(key, def) {
  const v = wx.getStorageSync(key);
  return v === '' || v === undefined || v === null ? def : v;
}
function set(key, v) { wx.setStorageSync(key, v); }

function yuan(fen) {
  const v = Math.round(fen) / 100;
  return v % 1 === 0 ? String(v) : v.toFixed(2).replace(/0$/, '');
}

function dishById(id) { return DISHES.find(d => d.id === id); }

// 规格 → 单价与描述
function specPrice(dish, sizeId, extras) {
  if (!dish.spec) return dish.price;
  const size = SIZES.find(s => s.id === sizeId) || SIZES[1];
  let p = dish.price + size.delta;
  (extras || []).forEach(id => { const e = EXTRAS.find(x => x.id === id); if (e) p += e.price; });
  return p;
}
function specText(dish, sizeId, spicy, extras) {
  if (!dish.spec) return '';
  const size = SIZES.find(s => s.id === sizeId) || SIZES[1];
  const parts = [size.name, spicy];
  (extras || []).forEach(id => { const e = EXTRAS.find(x => x.id === id); if (e) parts.push(e.name); });
  return parts.join(' / ');
}

function getCart() { return get(K.cart, []); }
function saveCart(cart) { set(K.cart, cart.filter(i => i.qty > 0)); }

function addToCart(item) {
  const cart = getCart();
  const hit = cart.find(i => i.key === item.key);
  if (hit) hit.qty += item.qty; else cart.push(item);
  saveCart(cart);
  return getCart();
}
function changeQty(key, delta) {
  const cart = getCart();
  const hit = cart.find(i => i.key === key);
  if (hit) hit.qty = Math.max(0, hit.qty + delta);
  saveCart(cart);
  return getCart();
}
function plainItem(dish) {
  return { key: dish.id, id: dish.id, name: dish.name, spec: '', unit: dish.price, qty: 1, bg: dish.bg, ch: dish.ch };
}

// 满减：满 30 减 5，满 60 减 12。进度条以 60 为满格。
function summary(cart) {
  let count = 0, amount = 0;
  cart.forEach(i => { count += i.qty; amount += i.unit * i.qty; });
  let discount = 0, tip = '';
  if (amount >= 6000) { discount = 1200; tip = '已享满60减12'; }
  else if (amount >= 3000) { discount = 500; tip = '已减5元，再点¥' + yuan(6000 - amount) + '可减12元'; }
  else if (amount > 0) tip = '再点¥' + yuan(3000 - amount) + '可减5元';
  else tip = '满30减5 · 满60减12';
  const progress = Math.min(100, Math.round(amount / 6000 * 100));
  return {
    count, amount, discount, pay: amount - discount, tip, progress,
    amountText: yuan(amount), discountText: yuan(discount), payText: yuan(amount - discount)
  };
}

// —— 订单 ——
const STAGES = ['已下单', '制作中', '可取餐'];

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function fmtTime(ts) {
  const d = new Date(ts);
  return (d.getMonth() + 1) + '月' + d.getDate() + '日 ' + pad(d.getHours()) + ':' + pad(d.getMinutes());
}

// 根据当前时间推进订单状态（超时取消、出餐阶段）
function refresh(o, now) {
  now = now || Date.now();
  if (o.status === 'unpaid' && now >= o.deadline) { o.status = 'canceled'; o.cancelReason = '支付超时，订单已自动取消'; }
  if (o.status === 'paid') {
    const t = now - o.paidAt;
    o.stage = t >= STAGE_MS[2] ? 2 : t >= STAGE_MS[1] ? 1 : 0;
  }
  return o;
}

function statusText(o) {
  if (o.status === 'unpaid') return '待支付';
  if (o.status === 'canceled') return '已取消';
  if (o.status === 'done') return '已完成';
  return STAGES[o.stage || 0];
}

function seedHistory() {
  const base = new Date(2026, 9, 22, 12, 10).getTime();
  const picks = [['d02', 'd22'], ['d09', 'd25'], ['d13'], ['d01', 'd07', 'd20'], ['d10', 'd24'], ['d14', 'd17'],
    ['d04', 'd06', 'd22'], ['d15', 'd23'], ['d11'], ['d03', 'd21'], ['d05', 'd12'], ['d16', 'd18', 'd26'], ['d01', 'd02'], ['d19', 'd25']];
  return picks.map((ids, i) => {
    const items = ids.map(id => plainItem(dishById(id)));
    if (i % 3 === 0) items[0].qty = 2;
    const s = summary(items);
    const created = base - (i + 1) * 24 * 3600 * 1000 + (i % 2 ? 6 * 3600 * 1000 - 5 * 60 * 1000 : -(i * 3) * 60 * 1000);
    return {
      id: 'H' + (100 + i), no: String(3200 - i * 7), table: ['A05', 'B03', 'A11', 'C02'][i % 4], items,
      amount: s.amount, discount: s.discount, pay: s.pay, remark: '', tableware: 1, dine: '堂食',
      status: i % 5 === 3 ? 'canceled' : 'done', cancelReason: i % 5 === 3 ? '用户取消' : '',
      createdAt: created, paidAt: created + 60000, deadline: created + PAY_MINUTES * 60000, stage: 2
    };
  });
}

function getOrders() {
  let list = get(K.orders, null);
  if (!list) { list = seedHistory(); set(K.orders, list); }
  const now = Date.now();
  let changed = false;
  list.forEach(o => { const before = o.status + o.stage; refresh(o, now); if (o.status + o.stage !== before) changed = true; });
  if (changed) set(K.orders, list);
  return list;
}
function saveOrders(list) { set(K.orders, list); }
function getOrder(id) { return getOrders().find(o => o.id === id); }
function updateOrder(id, patch) {
  const list = getOrders();
  const o = list.find(x => x.id === id);
  if (o) Object.assign(o, patch);
  saveOrders(list);
  return o;
}
function createOrder(info) {
  const list = getOrders();
  const now = Date.now();
  const o = Object.assign({
    id: 'R' + now, no: String(4000 + list.length * 3 + Math.floor(now / 1000) % 7),
    status: 'unpaid', createdAt: now, deadline: now + PAY_MINUTES * 60000, stage: 0
  }, info);
  list.unshift(o);
  saveOrders(list);
  return o;
}

// 「再来一单」：把订单菜品放回购物车（同 key 合并）
function reorder(o) {
  let cart = getCart();
  o.items.forEach(it => {
    const hit = cart.find(i => i.key === it.key);
    if (hit) hit.qty += it.qty; else cart.push(Object.assign({}, it));
  });
  saveCart(cart);
}

function getTable() { return get(K.table, ''); }
function setTable(t) { set(K.table, t); }
function getBalance() { return get(K.balance, 8650); }
function setBalance(v) { set(K.balance, v); }

module.exports = {
  K, PAY_MINUTES, STAGES, STAGE_MS, get, set, yuan, dishById, specPrice, specText, getCart, saveCart, addToCart,
  changeQty, plainItem, summary, fmtTime, pad, refresh, statusText, getOrders, saveOrders, getOrder, updateOrder,
  createOrder, reorder, getTable, setTable, getBalance, setBalance
};
