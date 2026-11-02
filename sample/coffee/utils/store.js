// 本地状态：购物车 / 订单 / 优惠券 / 积分 / 钱包 / 地址 / 签到，全部存在 storage 里
const data = require('./data.js');
const K = {
  cart: 'cf_cart', orders: 'cf_orders', mode: 'cf_mode', seq: 'cf_seq',
  used: 'cf_used_coupons', extra: 'cf_extra_coupons',
  points: 'cf_points', pointsLog: 'cf_points_log', wallet: 'cf_wallet',
  addrs: 'cf_addrs', addrPick: 'cf_addr_pick', sign: 'cf_sign', signDays: 'cf_sign_days', exchanges: 'cf_exchanges'
};
const MIN = 60000;
const BASE_CUPS = 46;

function round2(n) {
  return Math.round(n * 100) / 100;
}
function pad(n) {
  return n < 10 ? '0' + n : '' + n;
}
function hm(ts) {
  const d = new Date(ts);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}
function dateStr(ts) {
  const d = new Date(ts);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}
function dateTime(ts) {
  return `${dateStr(ts)} ${hm(ts)}`;
}
// storage 里没有这一项时 getStorageSync 给 ''：数值不能用 `|| 默认值`（0 会被当成没有）
function num(key, def) {
  const v = wx.getStorageSync(key);
  return typeof v === 'number' ? v : def;
}
function list(key) {
  const v = wx.getStorageSync(key);
  return Array.isArray(v) ? v : [];
}

// ───────── 用餐方式 / 购物车 ─────────
function getMode() {
  return wx.getStorageSync(K.mode) || 'pickup';
}
function setMode(m) {
  wx.setStorageSync(K.mode, m);
}
function getCart() {
  return list(K.cart);
}
function saveCart(l) {
  wx.setStorageSync(K.cart, l);
}
function addonNames(ids) {
  return (ids || []).map(id => (data.addons.find(a => a.id === id) || {}).name).filter(Boolean);
}
function specOf(x) {
  return [x.size, x.temp, x.sugar].concat(addonNames(x.addons)).join(' / ');
}
// 同一饮品同一规格（含加料）合并成一行
function addToCart(item) {
  const l = getCart();
  const addons = (item.addons || []).slice().sort();
  const key = [item.drinkId, item.size, item.temp, item.sugar, addons.join('+')].join('|');
  const hit = l.find(x => x.key === key);
  if (hit) hit.qty = Math.min(99, hit.qty + item.qty);
  else l.push(Object.assign({}, item, { key, addons }));
  saveCart(l);
  return l;
}
function changeQty(key, delta) {
  const l = getCart();
  const i = l.findIndex(x => x.key === key);
  if (i < 0) return l;
  l[i].qty += delta;
  if (l[i].qty <= 0) l.splice(i, 1);
  saveCart(l);
  return l;
}
function summary() {
  const l = getCart().map(x => {
    const d = data.findDrink(x.drinkId);
    return Object.assign({}, x, { name: d.name, img: d.img, cat: d.cat, spec: specOf(x), total: round2(x.unit * x.qty) });
  });
  const count = l.reduce((n, x) => n + x.qty, 0);
  const amount = round2(l.reduce((n, x) => n + x.unit * x.qty, 0));
  return { list: l, count, amount };
}

// ───────── 会员 / 活动 ─────────
const LEVELS = [
  { lv: 1, name: '银豆会员', cups: 0 },
  { lv: 2, name: '铜豆会员', cups: 10 },
  { lv: 3, name: '金豆会员', cups: 30 },
  { lv: 4, name: '黑金会员', cups: 80 }
];
function levelOf(cups) {
  let i = 0;
  LEVELS.forEach((l, k) => { if (cups >= l.cups) i = k; });
  const cur = LEVELS[i];
  const next = LEVELS[Math.min(i + 1, LEVELS.length - 1)];
  const span = Math.max(1, next.cups - cur.cups);
  const top = i === LEVELS.length - 1;
  return {
    lv: cur.lv, name: cur.name, next: next.name, top,
    need: Math.max(0, next.cups - cups),
    percent: top ? 100 : Math.min(100, Math.round(((cups - cur.cups) / span) * 100))
  };
}
function cups() {
  return BASE_CUPS + getOrders().filter(o => o.status !== 'cancelled').reduce((n, o) => n + o.count, 0);
}
// 会员日：金豆及以上每周三饮品 8 折
function memberDay(level, now) {
  return level.lv >= 3 && new Date(now || Date.now()).getDay() === 3;
}
// 当季新品第 2 杯半价：新品按单价从高到低排开，第 2、4、6… 杯半价
function activityCut(l) {
  const unitPrices = [];
  l.filter(x => data.findDrink(x.drinkId).cat === 'new').forEach(x => {
    for (let i = 0; i < x.qty; i++) unitPrices.push(x.unit);
  });
  unitPrices.sort((a, b) => b - a);
  let cut = 0;
  for (let i = 1; i < unitPrices.length; i += 2) cut += unitPrices[i] / 2;
  return round2(cut);
}

// ───────── 优惠券 ─────────
function allCoupons() {
  const used = list(K.used);
  return list(K.extra).concat(data.coupons).map(c => (used.indexOf(c.id) >= 0 ? Object.assign({}, c, { status: 'used' }) : c));
}
function validCoupons() {
  return allCoupons().filter(c => c.status === 'valid');
}
// 一张券在这一单上能省多少（不满足条件 0）。base：活动 / 会员日之后的饮品金额
function couponCut(c, base, l, mode) {
  if (!c || base < c.min) return 0;
  if (c.rate) return mode === 'delivery' ? round2(base * (1 - c.rate)) : 0;
  if (c.fixed) {
    const cheapest = l.filter(x => x.size === '中杯' && data.findDrink(x.drinkId).cat !== 'new').map(x => x.unit).sort((a, b) => a - b)[0];
    return cheapest ? round2(Math.max(0, cheapest - c.amount)) : 0;
  }
  return round2(Math.min(c.amount, base));
}
function useCoupon(id) {
  const used = list(K.used);
  if (used.indexOf(id) < 0) used.push(id);
  wx.setStorageSync(K.used, used);
}
function restoreCoupon(id) {
  wx.setStorageSync(K.used, list(K.used).filter(x => x !== id));
}
// 发一张券（签到奖励 / 积分兑换 / 兑换码），有效期 30 天
function grantCoupon(c, prefix) {
  const extra = list(K.extra);
  const coupon = Object.assign({ unit: '元', min: 0 }, c, {
    id: c.id || (prefix || 'x') + Date.now().toString(36) + extra.length,
    status: 'valid',
    expire: c.expire || dateStr(Date.now() + 30 * 24 * 3600 * 1000)
  });
  extra.unshift(coupon);
  wx.setStorageSync(K.extra, extra);
  return coupon;
}

// ───────── 积分 ─────────
const POINTS0 = 2380;
function getPoints() {
  return num(K.points, POINTS0);
}
function pointsLog() {
  const v = wx.getStorageSync(K.pointsLog);
  if (Array.isArray(v)) return v;
  return [
    { title: '历史消费累计', delta: 1880, time: '2024-03-01 09:12' },
    { title: '会员开卡礼', delta: 500, time: '2024-02-01 10:00' }
  ];
}
function changePoints(delta, title) {
  const points = Math.max(0, getPoints() + delta);
  wx.setStorageSync(K.points, points);
  const log = pointsLog();
  log.unshift({ title, delta, time: dateTime(Date.now()) });
  wx.setStorageSync(K.pointsLog, log.slice(0, 50));
  return points;
}

// ───────── 咖啡钱包 ─────────
function getWallet() {
  const w = wx.getStorageSync(K.wallet);
  if (w && typeof w.balance === 'number') return w;
  return { balance: 128, logs: [{ title: '开通钱包 · 充 100 送 28', amount: 128, type: 'recharge', time: '2024-03-01 09:12' }] };
}
function changeWallet(amount, title, type) {
  const w = getWallet();
  w.balance = round2(w.balance + amount);
  w.logs.unshift({ title, amount: round2(amount), type, time: dateTime(Date.now()) });
  w.logs = w.logs.slice(0, 50);
  wx.setStorageSync(K.wallet, w);
  return w;
}

// ───────── 订单 ─────────
function getOrders() {
  return list(K.orders);
}
function findOrder(id) {
  return getOrders().find(o => o.id === id) || null;
}
function addOrder(o) {
  const l = getOrders();
  l.unshift(o);
  wx.setStorageSync(K.orders, l.slice(0, 30));
}
function updateOrder(id, patch) {
  const l = getOrders();
  const o = l.find(x => x.id === id);
  if (!o) return null;
  Object.assign(o, patch);
  wx.setStorageSync(K.orders, l);
  return o;
}
function nextSeq() {
  const seq = num(K.seq, 216) + 1;
  wx.setStorageSync(K.seq, seq);
  return seq;
}
// 进度按时间推进：下单 1 分钟内是「已下单」（可取消），之后制作 3 分钟，再到可取 / 配送。
// 预约单从预约时间前 5 分钟开始算
function stageOf(o, now) {
  const pickup = o.mode === 'pickup';
  const names = pickup ? ['已下单', '制作中', '待取餐', '已完成'] : ['已下单', '制作中', '配送中', '已送达'];
  if (o.status === 'cancelled') {
    return { stage: -1, names, status: '已取消', tip: '订单已取消，款项与优惠已原路退回', active: false, canCancel: false, canConfirm: false };
  }
  const elapsed = (now || Date.now()) - (o.makeAt || o.createdAt);
  let stage = 0;
  if (elapsed >= MIN) stage = 1;
  if (elapsed >= 4 * MIN) stage = 2;
  if (elapsed >= (pickup ? 30 : 35) * MIN || o.status === 'done') stage = 3;
  const tips = pickup
    ? ['订单已提交，咖啡师马上接单', '咖啡师正在为你制作，预计 3 分钟后可取', '已制作完成，请凭取餐码到吧台取餐', '订单已完成，欢迎再次光临']
    : ['订单已提交，门店马上接单', '咖啡师正在为你制作', '骑手已取餐，正在飞奔向你', '已送达，祝你享用愉快'];
  return {
    stage, names, status: names[stage], tip: o.makeAt && elapsed < 0 ? `预约订单，${hm(o.makeAt)} 开始制作` : tips[stage],
    active: stage < 3, canCancel: stage === 0, canConfirm: stage === 2
  };
}
// 取消：钱、券、积分原路退回
function cancelOrder(id) {
  const o = findOrder(id);
  if (!o || !stageOf(o).canCancel) return null;
  if (o.payBy === 'wallet') changeWallet(o.pay, `订单退款 ${o.code}`, 'refund');
  if (o.couponId) restoreCoupon(o.couponId);
  if (o.pointsUsed) changePoints(o.pointsUsed, `订单取消退回 ${o.code}`);
  if (o.earn) changePoints(-o.earn, `订单取消扣回 ${o.code}`);
  return updateOrder(id, { status: 'cancelled', cancelledAt: Date.now() });
}
// 再来一单：按原规格加回购物车（下架的饮品跳过）
function reorder(o) {
  let n = 0;
  o.items.forEach(i => {
    const d = i.drinkId ? data.findDrink(i.drinkId) : data.drinks.find(x => x.name === i.name);
    if (!d) return;
    const size = data.sizes.find(s => s.name === i.size) || data.sizes[0];
    const addons = (i.addons || []).filter(id => data.addons.some(a => a.id === id));
    const unit = d.price + size.add + addons.reduce((s, id) => s + data.addons.find(a => a.id === id).add, 0);
    addToCart({
      drinkId: d.id, size: size.name, temp: d.temps.indexOf(i.temp) >= 0 ? i.temp : d.temps[0],
      sugar: i.sugar || data.sugars[0], addons, qty: i.qty, unit
    });
    n += i.qty;
  });
  return n;
}

// ───────── 收货地址 ─────────
function getAddrs() {
  const v = wx.getStorageSync(K.addrs);
  if (Array.isArray(v)) return v;
  return [{ id: 'a1', name: '林拾光', phone: '18601230520', region: ['北京市', '北京市', '朝阳区'], detail: '望京SOHO T3 座 18 层 1806', tag: '公司', isDefault: true }];
}
function saveAddrs(l) {
  wx.setStorageSync(K.addrs, l);
}
function upsertAddr(a) {
  let l = getAddrs();
  if (!a.id) a.id = 'a' + Date.now().toString(36);
  if (a.isDefault || !l.some(x => x.id !== a.id)) {
    l = l.map(x => Object.assign({}, x, { isDefault: false }));
    a.isDefault = true;
  }
  const i = l.findIndex(x => x.id === a.id);
  if (i >= 0) l[i] = a;
  else l.unshift(a);
  saveAddrs(l);
  return a;
}
function removeAddr(id) {
  let l = getAddrs().filter(x => x.id !== id);
  if (l.length && !l.some(x => x.isDefault)) l[0].isDefault = true;
  saveAddrs(l);
  if (wx.getStorageSync(K.addrPick) === id) wx.removeStorageSync(K.addrPick);
}
function maskPhone(p) {
  return p && p.length === 11 ? p.slice(0, 3) + '****' + p.slice(7) : p;
}
// 这一单送到哪：先看下单页挑过的，再看默认地址
function currentAddr() {
  const l = getAddrs();
  const id = wx.getStorageSync(K.addrPick);
  return l.find(x => x.id === id) || l.find(x => x.isDefault) || l[0] || null;
}
function pickAddr(id) {
  wx.setStorageSync(K.addrPick, id);
}

// ───────── 签到：连续 7 天送 5 元无门槛券 ─────────
function today(ts) {
  return dateStr(ts || Date.now());
}
function signState() {
  const last = wx.getStorageSync(K.sign);
  const days = num(K.signDays, 0);
  const signed = last === today();
  const yesterday = today(Date.now() - 24 * 3600 * 1000);
  // 断签（昨天没签）或上一轮 7 天已满：从 0 开始新一轮
  const streak = signed ? days : (last === yesterday && days < 7 ? days : 0);
  return { signed, days: streak };
}
function sign() {
  const st = signState();
  if (st.signed) return null;
  const days = st.days + 1;
  let reward = null;
  changePoints(10, '每日签到');
  if (days === 7) {
    reward = grantCoupon({ amount: 5, title: '连续签到 7 天礼 · 5 元券', cond: '全场通用', min: 0 }, 'xs');
  }
  wx.setStorageSync(K.sign, today());
  wx.setStorageSync(K.signDays, days);
  return { days, reward };
}

function exchanges() {
  return list(K.exchanges);
}
function addExchange(e) {
  const l = exchanges();
  l.unshift(e);
  wx.setStorageSync(K.exchanges, l.slice(0, 30));
}

module.exports = {
  MIN, round2, pad, hm, dateStr, dateTime, today,
  getMode, setMode, getCart, saveCart, addToCart, changeQty, summary, specOf, addonNames,
  levelOf, cups, memberDay, activityCut,
  allCoupons, validCoupons, couponCut, useCoupon, restoreCoupon, grantCoupon,
  getPoints, pointsLog, changePoints, getWallet, changeWallet,
  getOrders, findOrder, addOrder, updateOrder, nextSeq, stageOf, cancelOrder, reorder,
  getAddrs, upsertAddr, removeAddr, maskPhone, currentAddr, pickAddr,
  signState, sign, exchanges, addExchange
};
