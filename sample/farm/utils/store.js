// 跨页数据：购物车、订单、地址、拼团、收藏、直播预约，全部落 storage，退出重进还在。
const { goodsById } = require('./data.js');

const K = { cart: 'farm_cart', orders: 'farm_orders', addr: 'farm_addr', groups: 'farm_groups', fav: 'farm_fav', remind: 'farm_remind', seed: 'farm_seed_at' };

function get(k, def) {
  const v = wx.getStorageSync(k);
  return v === '' || v === undefined || v === null ? def : v;
}
function set(k, v) { wx.setStorageSync(k, v); }

function money(n) { return (Math.round(n * 100) / 100).toFixed(2); }
function pad(n) { return n < 10 ? '0' + n : '' + n; }
function fmtTime(ts) {
  const d = new Date(ts);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function skuText(g, key) {
  return String(key).split('|').map((i, k) => g.sku.dims[k].opts[+i]).join(' / ');
}
function skuInfo(g, key) { return g.sku.map[key]; }

// ---------- 购物车 ----------
function cart() { return get(K.cart, []); }
function addCart(goodsId, key, qty) {
  const list = cart();
  const hit = list.find((c) => c.goodsId === goodsId && c.skuKey === key);
  if (hit) hit.qty = Math.min(99, hit.qty + qty);
  else list.unshift({ goodsId, skuKey: key, qty, checked: true });
  set(K.cart, list);
  return list;
}
function saveCart(list) { set(K.cart, list); }
function cartCount() { return cart().reduce((s, c) => s + c.qty, 0); }

// ---------- 地址 ----------
function addresses() { return get(K.addr, []); }
function saveAddress(a) {
  const list = addresses();
  if (a.isDefault) list.forEach((x) => { x.isDefault = false; });
  const i = list.findIndex((x) => x.id === a.id);
  if (i >= 0) list[i] = a; else list.unshift(Object.assign({ id: 'a' + Date.now() }, a));
  if (!list.some((x) => x.isDefault)) list[0].isDefault = true;
  set(K.addr, list);
  return list;
}
function removeAddress(id) {
  const list = addresses().filter((x) => x.id !== id);
  if (list.length && !list.some((x) => x.isDefault)) list[0].isDefault = true;
  set(K.addr, list);
  return list;
}
function defaultAddress() {
  const list = addresses();
  return list.find((x) => x.isDefault) || list[0] || null;
}

// ---------- 拼团 ----------
// 别人开的团：以首次启动时刻为基准，按 24h 周期滚动，倒计时永远是正的
const OPENERS = [['江南小满', 1], ['老陈在杭州', 2], ['豆豆妈', 3], ['风起时', 1], ['青柠', 2], ['阿杰', 3]];
function seedAt() {
  let t = get(K.seed, 0);
  if (!t) { t = Date.now(); set(K.seed, t); }
  return t;
}
function openGroups(goodsId) {
  const g = goodsById(goodsId);
  const base = seedAt();
  const n = parseInt(goodsId.slice(1), 10);
  const day = 24 * 3600 * 1000;
  const mine = myGroups().filter((x) => x.goodsId === goodsId).map((x) => x.joinedFrom);
  return [0, 1].map((k) => {
    const [name, av] = OPENERS[(n + k * 3) % OPENERS.length];
    const offset = ((n * 37 + k * 211) % 600 + 90) * 60 * 1000;
    let end = base + offset;
    while (end <= Date.now()) end += day;
    const joined = Math.max(1, g.groupSize - 1 - k % 2);
    return { id: `og-${goodsId}-${k}`, goodsId, name, avatar: `/images/misc/farmer${av}.jpg`, size: g.groupSize, joined, lack: g.groupSize - joined, endAt: end };
  }).filter((x) => mine.indexOf(x.id) < 0);
}
function myGroups() { return get(K.groups, []); }
function saveGroups(list) { set(K.groups, list); }
function groupById(id) { return myGroups().find((x) => x.id === id); }
function createGroup(order, join) {
  const it = order.items[0];
  const g = goodsById(it.goodsId);
  const members = join
    ? Array.from({ length: join.joined }, (_, i) => ({ name: i === 0 ? join.name : OPENERS[(i + 2) % 6][0], avatar: i === 0 ? join.avatar : `/images/misc/farmer${(i % 3) + 1}.jpg` }))
    : [];
  members.push({ name: '我', avatar: '/images/misc/farmer1.jpg', me: true });
  const grp = {
    id: 'gp' + Date.now(), goodsId: it.goodsId, skuKey: it.skuKey, size: g.groupSize, members, orderId: order.id,
    leader: join ? join.name : '我', joinedFrom: join ? join.id : '', endAt: join ? join.endAt : Date.now() + 24 * 3600 * 1000,
    status: 'ongoing', lastJoin: Date.now()
  };
  const list = myGroups();
  list.unshift(grp);
  saveGroups(list);
  return settleGroup(grp.id);
}
// 自己开的团：拼团大厅会把团推荐给其他买家，每隔 15 秒匹配进一位，直到满员；超时未满则失败退款
function settleGroup(id) {
  const list = myGroups();
  const grp = list.find((x) => x.id === id);
  if (!grp) return null;
  if (grp.status === 'ongoing') {
    const now = Date.now();
    while (grp.members.length < grp.size && now - grp.lastJoin >= 15000) {
      const n = grp.members.length;
      grp.members.push({ name: OPENERS[(n + 4) % 6][0], avatar: `/images/misc/farmer${(n % 3) + 1}.jpg` });
      grp.lastJoin += 15000;
    }
    if (grp.members.length >= grp.size) grp.status = 'success';
    else if (now > grp.endAt) grp.status = 'fail';
    saveGroups(list);
    const o = orderById(grp.orderId);
    if (o && o.status === 'grouping') {
      if (grp.status === 'success') updateOrder(o.id, { status: 'paid', groupDoneAt: now });
      if (grp.status === 'fail') updateOrder(o.id, { status: 'refunded', refund: { reason: '拼团未成功，系统自动退款', amount: o.total, at: now } });
    }
  }
  return grp;
}

// ---------- 订单 ----------
const STATUS = {
  unpaid: '待付款', grouping: '待成团', paid: '待发货', shipped: '待收货', done: '已完成',
  refunding: '售后处理中', refunded: '已退款', cancelled: '已取消'
};
function orders() { return get(K.orders, []); }
function orderById(id) { return orders().find((o) => o.id === id); }
function addOrder(o) {
  const list = orders();
  const now = Date.now();
  const order = Object.assign({ id: 'o' + now, no: 'TY' + fmtTime(now).replace(/[- :]/g, '') + String(now % 1000).padStart(3, '0'), createdAt: now, status: 'unpaid' }, o);
  list.unshift(order);
  set(K.orders, list);
  return order;
}
function updateOrder(id, patch) {
  const list = orders();
  const o = list.find((x) => x.id === id);
  if (!o) return null;
  Object.assign(o, patch);
  set(K.orders, list);
  return o;
}
function logistics(o) {
  const t0 = o.shippedAt || Date.now();
  const city = (o.address && o.address.city) || '目的地';
  const origin = goodsById(o.items[0].goodsId).origin.split('·')[0];
  return [
    { t: t0 + 26 * 3600e3, text: `【${city}】快件已到达${city}转运中心，正在派送途中` },
    { t: t0 + 9 * 3600e3, text: `【${origin}】快件已发往${city}` },
    { t: t0 + 2 * 3600e3, text: `【${origin}】顺丰速运已揽收，冷链保鲜运输` },
    { t: t0, text: '商家已发货，产地果园完成分拣装箱' }
  ].filter((x) => x.t <= Date.now()).map((x) => ({ time: fmtTime(x.t), text: x.text }));
}

// ---------- 收藏 / 直播预约 ----------
function favs() { return get(K.fav, []); }
function toggleFav(id) {
  const list = favs();
  const i = list.indexOf(id);
  if (i >= 0) list.splice(i, 1); else list.unshift(id);
  set(K.fav, list);
  return i < 0;
}
function reminders() { return get(K.remind, []); }
function toggleRemind(id) {
  const list = reminders();
  const i = list.indexOf(id);
  if (i >= 0) list.splice(i, 1); else list.push(id);
  set(K.remind, list);
  return i < 0;
}

module.exports = {
  money, fmtTime, pad, skuText, skuInfo,
  cart, addCart, saveCart, cartCount,
  addresses, saveAddress, removeAddress, defaultAddress,
  openGroups, myGroups, groupById, createGroup, settleGroup,
  STATUS, orders, orderById, addOrder, updateOrder, logistics,
  favs, toggleFav, reminders, toggleRemind
};
