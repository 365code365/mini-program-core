// 跨页数据：购物车、订单、处方审核记录、用药提醒、收货地址。全部在 storage 里，退出重进还在
const { BY_ID } = require('./drugs.js');

const K = { cart: 'ph_cart', orders: 'ph_orders', rx: 'ph_rx', reminders: 'ph_reminders', taken: 'ph_taken', addr: 'ph_address', search: 'ph_search' };

function get(k, def) {
  const v = wx.getStorageSync(K[k] || k);
  return v === '' || v === undefined || v === null ? def : v;
}
function set(k, v) { wx.setStorageSync(K[k] || k, v); }

function money(n) { return (Math.round(n * 100) / 100).toFixed(2); }

// ---------- 购物车：[{key: id#规格下标, id, si, qty, sel}] ----------
function cartRaw() { return get('cart', []); }
function cartCount() { return cartRaw().reduce((a, c) => a + c.qty, 0); }
function addCart(id, si, qty) {
  si = si || 0; qty = qty || 1;
  const list = cartRaw(), key = id + '#' + si;
  const it = list.find(c => c.key === key);
  if (it) { it.qty = Math.min(99, it.qty + qty); it.sel = true; }
  else list.unshift({ key, id, si, qty, sel: true });
  set('cart', list);
  return cartCount();
}
function cartDetail() {
  return cartRaw().filter(c => BY_ID[c.id]).map(c => {
    const d = BY_ID[c.id], sp = d.specs[c.si] || d.specs[0];
    return Object.assign({}, c, { name: d.name, brand: d.brand, img: d.img, type: d.type, spec: sp.s, price: sp.p, priceText: money(sp.p) });
  });
}
function saveCart(list) { set('cart', list.map(c => ({ key: c.key, id: c.id, si: c.si, qty: c.qty, sel: !!c.sel }))); }

// ---------- 处方审核：[{no, ids, method, patient, at, status}]，审核通过 7 天内有效 ----------
const RX_VALID = 7 * 86400000;
function rxPending(ids) {
  const ok = get('rx', []).filter(r => r.status === '已通过' && Date.now() - r.at < RX_VALID);
  return ids.filter(id => BY_ID[id] && BY_ID[id].type === 'RX' && !ok.some(r => r.ids.indexOf(id) >= 0));
}
function rxFor(ids) {
  const ok = get('rx', []).filter(r => r.status === '已通过' && Date.now() - r.at < RX_VALID);
  return ok.find(r => ids.some(id => r.ids.indexOf(id) >= 0)) || null;
}

// ---------- 订单：下单后约 1 分钟药房出库（待发货 → 配送中），确认收货后已完成 ----------
function advanceOrders(list) {
  let changed = false;
  list.forEach(o => {
    if (o.status === '待发货' && Date.now() - o.createdAt > 60000) { o.status = '配送中'; o.shipAt = o.createdAt + 60000; changed = true; }
  });
  return changed;
}

// ---------- 用药提醒：[{id, drug, dose, times:['08:00'], days:[1..7], note, on}]；days 用 1=周一 … 7=周日 ----------
function todayKey(d) {
  d = d || new Date();
  return d.getFullYear() + '-' + (d.getMonth() + 1) + '-' + d.getDate();
}
function weekNo(d) { const w = (d || new Date()).getDay(); return w === 0 ? 7 : w; }
function todayDoses() {
  const wk = weekNo(), taken = (get('taken', {})[todayKey()] || []);
  const out = [];
  get('reminders', []).filter(r => r.on && r.days.indexOf(wk) >= 0).forEach(r => {
    r.times.forEach(t => out.push({ k: r.id + '@' + t, rid: r.id, time: t, drug: r.drug, dose: r.dose, note: r.note, taken: taken.indexOf(r.id + '@' + t) >= 0 }));
  });
  return out.sort((a, b) => (a.time < b.time ? -1 : 1));
}

module.exports = { K, get, set, money, cartRaw, cartCount, addCart, cartDetail, saveCart, rxPending, rxFor, advanceOrders, todayKey, weekNo, todayDoses };
